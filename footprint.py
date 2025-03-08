import datetime
import time
import json
import copy
from binance.websocket.um_futures.websocket_client import UMFuturesWebsocketClient
from colorama import Fore, Back, Style

class OrderFlowTrader:
    def __init__(self, symbol="btcusdt"):
        self.symbol = symbol.lower()
        
        # 初始化 UM Futures WebSocket 客户端
        self.umfclient = UMFuturesWebsocketClient(on_message=self.spot_message_handler)
        
        # ------------------- 参数设置 -------------------
        self.imbalance_threshold = 3.0          # 整体失衡阈值（用于分钟级别判断）
        self.volume_threshold_multiplier = 1.5  # 当前分钟总成交量大于历史平均成交量1.5倍视为放量
        self.order_quantity = 0.01              # 下单数量（BTC）
        
        self.TICK_SIZE = 1.0  # 价格档位间隔：每1美元一个档
        
        self.HISTORY_LENGTH = 1440  # 用于记录过去多少分钟的订单流数据
        self.orderflow_history = []  # 存储每分钟的 footprint 数据
            
        # ------------------- 实时变量 -------------------
        self.current_minute = None
        self.footprint = self.new_minute_footprint()
        self.imbalance_checked = False  # 当前分钟是否已进行连续失衡检测

        # 新增支撑压力位分析相关参数
        self.support_resistance_levels = []  # 存储识别出的支撑压力位
        self.sr_volume_threshold = 0.1  # 支撑压力位的成交量阈值（占总成交量的比例）
        self.sr_price_range = 5  # 支撑压力位的价格范围（美元）
        self.reversal_threshold = 2.0  # 反转信号的失衡比例阈值

    def new_minute_footprint(self):
        """返回一个新的 minute 级别的 footprint 数据结构，并重置检测标记"""
        self.imbalance_checked = False
        return {
            "time": None,           # 新增time字段，记录这一分钟的时间戳
            "open": None,           # 本分钟第一个成交价
            "high": None,           # 本分钟最高成交价
            "low": None,           # 本分钟最低成交价
            "close": None,         # 本分钟最后成交价
            "total_volume": 0.0,   # 本分钟累计成交量
            "buy_volume": 0.0,     # 本分钟累计买量
            "sell_volume": 0.0,    # 本分钟累计卖量
            "delta": 0.0,          # 买量 - 卖量
            "order_flows": {}      # 存放各价格档位的订单流数据
        }

    def get_minute_str(self, timestamp_ms):
        """将毫秒级时间戳转换为分钟级字符串，例如 '202503061230' 表示2025-03-06 12:30。"""
        dt = datetime.datetime.fromtimestamp(timestamp_ms / 1000)
        return dt.strftime('%Y%m%d%H%M')

    def analyze_support_resistance(self):
        """分析最近24小时数据中的支撑位和压力位"""
        if not self.orderflow_history:
            return []

        # 统计所有价格层级的成交量
        price_volumes = {}
        total_volume = 0
        
        # 遍历历史数据
        for minute_data in self.orderflow_history:
            for price_str, orders in minute_data["order_flows"].items():
                price = int(price_str)
                volume = sum(order['volume'] for order in orders)
                price_volumes[price] = price_volumes.get(price, 0) + volume
                total_volume += volume

        # 识别高成交量价格区域
        significant_levels = []
        volume_threshold = total_volume * self.sr_volume_threshold

        for price, volume in price_volumes.items():
            if volume >= volume_threshold:
                # 计算该价位的买卖比例
                buy_volume = sum(
                    sum(o['volume'] for o in minute["order_flows"].get(str(price), [])
                        if o['side'] == 'buy')
                    for minute in self.orderflow_history
                )
                sell_volume = sum(
                    sum(o['volume'] for o in minute["order_flows"].get(str(price), [])
                        if o['side'] == 'sell')
                    for minute in self.orderflow_history
                )
                
                level_type = "支撑" if buy_volume > sell_volume else "压力"
                significant_levels.append({
                    'price': price,
                    'volume': volume,
                    'type': level_type,
                    'buy_ratio': buy_volume / (buy_volume + sell_volume) if (buy_volume + sell_volume) > 0 else 0
                })

        # 合并接近的价格水平
        merged_levels = []
        significant_levels.sort(key=lambda x: x['price'])
        
        i = 0
        while i < len(significant_levels):
            current_level = significant_levels[i]
            j = i + 1
            merged_volume = current_level['volume']
            merged_buy_ratio = current_level['buy_ratio'] * current_level['volume']
            
            while j < len(significant_levels) and \
                  significant_levels[j]['price'] - significant_levels[i]['price'] <= self.sr_price_range:
                merged_volume += significant_levels[j]['volume']
                merged_buy_ratio += significant_levels[j]['buy_ratio'] * significant_levels[j]['volume']
                j += 1
            
            avg_buy_ratio = merged_buy_ratio / merged_volume
            level_type = "支撑" if avg_buy_ratio > 0.5 else "压力"
            
            merged_levels.append({
                'price': significant_levels[i]['price'],
                'volume': merged_volume,
                'type': level_type,
                'strength': merged_volume / total_volume  # 该位置的强度
            })
            
            i = j

        self.support_resistance_levels = merged_levels
        return merged_levels

    def check_reversal_signals(self):
        """检查当前价格是否接近支撑压力位，并分析是否有反转信号"""
        if not self.support_resistance_levels or not self.footprint['close']:
            return None

        current_price = self.footprint['close']
        
        # 遍历所有支撑压力位
        for level in self.support_resistance_levels:
            price_diff = abs(current_price - level['price'])
            
            # 如果当前价格接近支撑压力位（在5美元范围内）
            if price_diff <= 5:
                # 分析当前分钟的买卖失衡
                buy_volume = self.footprint['buy_volume']
                sell_volume = self.footprint['sell_volume']
                
                # 计算价格变动
                price_change = self.footprint['close'] - self.footprint['open']
                
                # 判断是否有反转信号
                if level['type'] == "压力" and price_change < 0:
                    # 接近压力位且价格下跌
                    if sell_volume > 0 and (buy_volume / sell_volume) > self.reversal_threshold:
                        return {
                            'signal': 'BUY',
                            'price': current_price,
                            'level_price': level['price'],
                            'level_type': level['type'],
                            'strength': level['strength'],
                            'imbalance_ratio': buy_volume / sell_volume
                        }
                        
                elif level['type'] == "支撑" and price_change > 0:
                    # 接近支撑位且价格上涨
                    if buy_volume > 0 and (sell_volume / buy_volume) > self.reversal_threshold:
                        return {
                            'signal': 'SELL',
                            'price': current_price,
                            'level_price': level['price'],
                            'level_type': level['type'],
                            'strength': level['strength'],
                            'imbalance_ratio': sell_volume / buy_volume
                        }
        
        return None

    def evaluate_minute(self):
        """1分钟结束后，更新 delta，并打印 minute 级别的统计数据；同时将 footprint 记录到历史数据中。"""
        # 更新 delta
        self.footprint["delta"] = self.footprint["buy_volume"] - self.footprint["sell_volume"]
        total_volume = self.footprint["buy_volume"] + self.footprint["sell_volume"]

        # 转换时间戳为可读格式
        time_str = datetime.datetime.fromtimestamp(self.footprint["time"] / 1000).strftime('%Y-%m-%d %H:%M:%S')
        
        print("====== 分钟结束，Minute Footprint 数据 ======")
        print(f"Time: {time_str}")
        print(f"Open: {self.footprint['open']:.2f}, High: {self.footprint['high']:.2f}, "
              f"Low: {self.footprint['low']:.2f}, Close: {self.footprint['close']:.2f}")
        print(f"Total Volume: {self.footprint['total_volume']:.6f}, Buy Volume: {self.footprint['buy_volume']:.6f}, "
              f"Sell Volume: {self.footprint['sell_volume']:.6f}, Delta: {self.footprint['delta']:.6f}")
        
        # 统计 order_flows 里各档位的累计买入和卖出成交量
        total_buy_flows = 0.0
        total_sell_flows = 0.0

        print("------ 订单流数据 (按1美元档) ------")

        for price_level, orders in sorted(self.footprint["order_flows"].items(), key=lambda x: -float(x[0])):
            level_buy = sum(o['volume'] for o in orders if o['side'] == 'buy')
            level_sell = sum(o['volume'] for o in orders if o['side'] == 'sell')
            total_buy_flows += level_buy
            total_sell_flows += level_sell
            print(f"价格档位: {price_level}, 订单数: {len(orders)}, 累计成交量: {level_buy + level_sell:.6f}, "
                  f"买量: {level_buy:.6f}, 卖量: {level_sell:.6f}")

        print(f"所有价位累计 - 买量: {total_buy_flows:.6f}, 卖量: {total_sell_flows:.6f}")
        
        # 判断1分钟内的整体失衡表现
        imbalance = 0
        if self.footprint["sell_volume"] > self.footprint["buy_volume"]:
            imbalance = self.footprint["sell_volume"] / self.footprint["buy_volume"]
            if imbalance > self.imbalance_threshold:
                print(f"{Fore.RED}当前1分钟严重失衡: Imbalance:{imbalance}{Style.RESET_ALL}")
        elif self.footprint["sell_volume"] < self.footprint["buy_volume"]:
            imbalance = self.footprint["buy_volume"] / self.footprint["sell_volume"]
            if imbalance > self.imbalance_threshold:
                print(f"{Fore.RED}当前1分钟严重失衡: Imbalance:{imbalance}{Style.RESET_ALL}")

        # 检查连续价位失衡（移到这里）
        print("\n------ 连续价位失衡分析 ------")
        self.check_consecutive_imbalances()
        
        # 将本分钟的 footprint 深拷贝后存入历史记录
        self.orderflow_history.append(copy.deepcopy(self.footprint))
        if len(self.orderflow_history) > self.HISTORY_LENGTH:
            self.orderflow_history.pop(0)

        # 分析支撑压力位
        sr_levels = self.analyze_support_resistance()
        
        print("\n====== 支撑压力位分析 ======")
        for level in sr_levels:
            print(f"{level['type']}位: ${level['price']}, 强度: {level['strength']*100:.2f}%")

        # 检查反转信号
        reversal_signal = self.check_reversal_signals()
        if reversal_signal:
            print(f"\n{Fore.GREEN}检测到反转信号:")
            print(f"方向: {reversal_signal['signal']}")
            print(f"当前价格: ${reversal_signal['price']:.2f}")
            print(f"接近{reversal_signal['level_type']}位: ${reversal_signal['level_price']}")
            print(f"位置强度: {reversal_signal['strength']*100:.2f}%")
            print(f"失衡比例: {reversal_signal['imbalance_ratio']:.2f}{Style.RESET_ALL}")

    def check_consecutive_imbalances(self):
        """
        检查当前 footprint 的 order_flows 中是否存在连续三个价位满足失衡条件，
        定义：在该价位上，一方成交量 > 3 * 另一方成交量，
        并且三个连续价位的方向一致（全部为多头或全部为空头）。
        """
        # 获取所有存在订单流的价位（转换为整数，并排序）
        price_levels = sorted([int(k) for k in self.footprint["order_flows"].keys()])
        for i in range(len(price_levels) - 2):
            p1, p2, p3 = price_levels[i], price_levels[i+1], price_levels[i+2]
            # 检查是否为连续三个价位（例如 90130, 90131, 90132）
            if p2 == p1 + 1 and p3 == p2 + 1:
                def agg_vol(level):
                    key = str(level)
                    orders = self.footprint["order_flows"].get(key, [])
                    buy_vol = sum(o['volume'] for o in orders if o['side'] == 'buy')
                    sell_vol = sum(o['volume'] for o in orders if o['side'] == 'sell')
                    return buy_vol, sell_vol
                b1, s1 = agg_vol(p1)
                b2, s2 = agg_vol(p2)
                b3, s3 = agg_vol(p3)
                def imbalance_direction(b, s):
                    if b > 3 * s:
                        return "多头"  # Long
                    elif s > 3 * b:
                        return "空头"  # Short
                    else:
                        return None
                d1 = imbalance_direction(b1, s1)
                d2 = imbalance_direction(b2, s2)
                d3 = imbalance_direction(b3, s3)
                if d1 and d2 and d3 and (d1 == d2 == d3):
                    print(f"{Fore.YELLOW}检测到连续三个价位失衡，价位 {p1}, {p2}, {p3}，方向为 {d1}{Style.RESET_ALL}")
                    return True
        return False

    def spot_message_handler(self, _, data):
        """
        处理 aggTrade 数据，累计 1 分钟内的整体订单流统计，
        并将各价格档位的订单流数据存放于 order_flows 中。
        
        Binance aggTrade 事件示例：
        {
            "e": "aggTrade",
            "E": 123456789,
            "s": "BTCUSDT",
            "a": 12345,
            "p": "90000.0",
            "q": "0.5",
            "f": 100,
            "l": 105,
            "T": 123456785,
            "m": false,
            "M": true
        }
        """
        try:
            message = json.loads(data)
        except Exception as e:
            print("JSON解析异常:", e)
            return
        
        if message.get('e') != 'aggTrade':
            return

        trade_time = message.get('T')
        minute_str = self.get_minute_str(trade_time)

        # 判断是否进入新的分钟
        if self.current_minute is None:
            self.current_minute = minute_str
            self.footprint = self.new_minute_footprint()
            self.footprint["time"] = trade_time  # 设置分钟开始的时间戳
        elif minute_str != self.current_minute:
            self.evaluate_minute()
            self.current_minute = minute_str
            self.footprint = self.new_minute_footprint()
            self.footprint["time"] = trade_time  # 设置新分钟的时间戳

        try:
            price = float(message.get('p'))
        except Exception as e:
            print("价格转换异常:", e)
            return

        try:
            volume = float(message.get('q', 0))
        except Exception as e:
            print("成交量转换异常:", e)
            return

        side = 'sell' if message.get('m', False) else 'buy'

        # 更新 minute 级别的价格数据
        if self.footprint["open"] is None:
            self.footprint["open"] = price
            self.footprint["high"] = price
            self.footprint["low"] = price
        else:
            self.footprint["close"] = price
            if price > self.footprint["high"]:
                self.footprint["high"] = price
            if price < self.footprint["low"]:
                self.footprint["low"] = price

        self.footprint["total_volume"] += volume
        if side == 'buy':
            self.footprint["buy_volume"] += volume
        else:
            self.footprint["sell_volume"] += volume

        # 根据价格划分档位（1美元档），将成交价格向下取整
        price_level = int(price)
        price_str = str(price_level)

        order_flow_record = {
            'price': price,
            'volume': volume,
            'side': side,
            'timestamp': message.get('T')
        }
        if price_str not in self.footprint["order_flows"]:
            self.footprint["order_flows"][price_str] = []
        self.footprint["order_flows"][price_str].append(order_flow_record)

    def start(self):
        self.umfclient.agg_trade(self.symbol)

    def shutdown(self):
        self.umfclient.stop()

if __name__ == "__main__":
    trader = OrderFlowTrader()
    try:
        trader.start()
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        trader.shutdown()
        print(f"\n{Fore.YELLOW}安全退出{Style.RESET_ALL}")
