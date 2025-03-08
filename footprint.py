import datetime
import time
import json
import copy
from binance.websocket.um_futures.websocket_client import UMFuturesWebsocketClient
from colorama import Fore, Back, Style
from prompt_toolkit import Application
from prompt_toolkit.layout import Layout, Window, HSplit, FormattedTextControl
from prompt_toolkit.key_binding import KeyBindings
from threading import Lock
import asyncio
import threading
from prompt_toolkit.styles import Style

class FootprintDisplay:

    def __init__(self):
        self.lock = Lock()
        self.current_text = []
        self.kb = KeyBindings()
        self.scroll_offset = 0
        self.max_visible_rows = 80  # 可见行数
        
        # 添加样式
        self.style = Style.from_dict({
            'buy_strong': 'ansigreen bold',  # 绿色加粗
            'sell_strong': 'ansired bold',  # 红色加粗
            'normal': 'ansiwhite',  # 白色
            'header': 'ansiyellow',  # 黄色
            'time': 'ansicyan',    # 青色
            'price': 'ansiwhite',   # 白色
            'ohlc': 'ansicyan',    # 青色
            'volume': 'ansiwhite',   # 白色
            'current_row': 'bg:ansiwhite fg:ansiblack'  # 当前价格层级的背景色
        })

        @self.kb.add('c-c')
        def _(event):
            event.app.exit()

        @self.kb.add('up')
        def _(event):
            self.scroll_offset = max(0, self.scroll_offset - 1)
            event.app.invalidate()

        @self.kb.add('down')
        def _(event):
            self.scroll_offset += 1
            event.app.invalidate()

        @self.kb.add('pageup')
        def _(event):
            self.scroll_offset = max(0, self.scroll_offset - self.max_visible_rows)
            event.app.invalidate()

        @self.kb.add('pagedown')
        def _(event):
            self.scroll_offset += self.max_visible_rows
            event.app.invalidate()

        @self.kb.add('home')
        def _(event):
            self.scroll_offset = 0
            event.app.invalidate()

        self.text_control = FormattedTextControl(text=self.get_formatted_text)
        self.window = Window(
            content=self.text_control,
            height=self.max_visible_rows + 5,  # 额外空间用于头部信息
            wrap_lines=False
        )
        self.layout = Layout(self.window)
        
        self.app = Application(
            layout=self.layout,
            key_bindings=self.kb,
            full_screen=True,
            mouse_support=True,
            style=self.style,  # 添加样式
            color_depth='DEPTH_24_BIT'  # 启用24位真彩色
        )

        # 添加定时刷新
        self.refresh_interval = 0.1  # 100ms 刷新一次
        self._running = True
        self._refresh_thread = None

    def start_refresh_thread(self):
        def refresh_loop():
            while self._running:
                self.app.invalidate()
                time.sleep(self.refresh_interval)
        
        self._refresh_thread = threading.Thread(target=refresh_loop, daemon=True)
        self._refresh_thread.start()

    def stop_refresh_thread(self):
        self._running = False
        if self._refresh_thread:
            self._refresh_thread.join()

    def get_formatted_text(self):
        with self.lock:
            return self.current_text

    def update_display(self, footprint_data):
        with self.lock:
            self.current_text = []
            
            # 添加时间和OHLC信息
            time_str = datetime.datetime.fromtimestamp(footprint_data["time"] / 1000).strftime('%Y-%m-%d %H:%M:%S')
            
            # 处理可能为None的OHLC值
            open_price = footprint_data['open'] if footprint_data['open'] is not None else 0.0
            high_price = footprint_data['high'] if footprint_data['high'] is not None else 0.0
            low_price = footprint_data['low'] if footprint_data['low'] is not None else 0.0
            close_price = footprint_data['close'] if footprint_data['close'] is not None else 0.0
            
            header_info = [
                ('class:time', f"Time: {time_str}\n"),
                ('class:ohlc', f"Open: {open_price:.2f}, High: {high_price:.2f}, "
                              f"Low: {low_price:.2f}, Close: {close_price:.2f}\n"),
                ('class:volume', f"Total Volume: {footprint_data['total_volume']:.3f}, "
                               f"Buy Volume: {footprint_data['buy_volume']:.3f}, "
                               f"Sell Volume: {footprint_data['sell_volume']:.3f}, "
                               f"Delta: {footprint_data['delta']:.3f}\n\n")
            ]
            
            # 添加表格头部
            table_header = [
                ('class:header', "┌" + "─" * 15 + "┬" + "─" * 12 + "┬" + "─" * 16 + "┬" + "─" * 16 + "┬" + "─" * 16 + "┐\n"),
                ('class:header', "│ Price Level   │ Orders     │ Total Volume   │ Buy Volume     │ Sell Volume    │\n"),
                ('class:header', "├" + "─" * 15 + "┼" + "─" * 12 + "┼" + "─" * 16 + "┼" + "─" * 16 + "┼" + "─" * 16 + "┤\n")
            ]
            
            # 获取当前价格层级
            current_price_level = str(int(footprint_data['close']))
            
            # 生成所有价格层级数据行
            price_rows = []
            current_price_index = None  # 用于记录当前价格所在行的索引
            
            for i, (price_level, level_data) in enumerate(sorted(footprint_data["order_flows"].items(), key=lambda x: -float(x[0]))):
                if price_level == current_price_level:
                    current_price_index = i
                
                buy_vol = level_data["buy_volume"]
                sell_vol = level_data["sell_volume"]
                total_vol = buy_vol + sell_vol
                
                # 根据买卖比例设置样式
                if price_level == current_price_level:
                    # 当前价格层级使用背景色
                    style_class = 'current_row'
                    price_text = f"{price_level:13}"
                    buy_text = f"{buy_vol:14.3f}"
                    sell_text = f"{sell_vol:14.3f}"
                    total_text = f"{total_vol:14.3f}"
                    orders_text = f"{level_data['order_count']:10}"
                    
                    row = [
                        ('class:current_row', "│ "),
                        ('class:current_row', price_text),
                        ('class:current_row', " │ "),
                        ('class:current_row', orders_text),
                        ('class:current_row', " │ "),
                        ('class:current_row', total_text),
                        ('class:current_row', " │ "),
                        ('class:current_row', buy_text),
                        ('class:current_row', " │ "),
                        ('class:current_row', sell_text),
                        ('class:current_row', " │\n")
                    ]
                else:
                    # 设置买卖量的颜色样式
                    if sell_vol > 0 and buy_vol >= 1 and buy_vol / sell_vol >= 2:
                        buy_style = 'buy_strong'
                        sell_style = 'normal'
                    elif buy_vol > 0 and sell_vol >= 1 and sell_vol / buy_vol >= 2:
                        buy_style = 'normal'
                        sell_style = 'sell_strong'
                    else:
                        buy_style = 'normal'
                        sell_style = 'normal'
                    
                    row = [
                        ('class:normal', "│ "),
                        ('class:price', f"{price_level:13}"),
                        ('class:normal', " │ "),
                        ('class:normal', f"{level_data['order_count']:10}"),
                        ('class:normal', " │ "),
                        ('class:normal', f"{total_vol:14.3f}"),
                        ('class:normal', " │ "),
                        (f'class:{buy_style}', f"{buy_vol:14.3f}"),
                        ('class:normal', " │ "),
                        (f'class:{sell_style}', f"{sell_vol:14.3f}"),
                        ('class:normal', " │\n")
                    ]
                price_rows.append(row)

            # 自动调整滚动位置，使当前价格保持在窗口中间
            total_rows = len(price_rows)
            if current_price_index is not None:
                # 计算理想的滚动位置（当前价格位于窗口中间）
                ideal_scroll = max(0, current_price_index - self.max_visible_rows // 2)
                # 平滑滚动：每次最多移动一定行数
                max_scroll_change = 3  # 每次最多移动3行
                if abs(ideal_scroll - self.scroll_offset) > max_scroll_change:
                    if ideal_scroll > self.scroll_offset:
                        self.scroll_offset += max_scroll_change
                    else:
                        self.scroll_offset -= max_scroll_change
                else:
                    self.scroll_offset = ideal_scroll

            # 确保滚动位置在有效范围内
            self.scroll_offset = min(max(0, self.scroll_offset), max(0, total_rows - self.max_visible_rows))
            start_idx = self.scroll_offset
            end_idx = min(start_idx + self.max_visible_rows, total_rows)
            
            # 组合最终显示内容
            self.current_text = (
                header_info +
                table_header +
                [item for row in price_rows[start_idx:end_idx] for item in row] +
                [('class:header', "└" + "─" * 15 + "┴" + "─" * 12 + "┴" + "─" * 16 + "┴" + "─" * 16 + "┴" + "─" * 16 + "┘\n")]
            )

class OrderFlowTrader:
    def __init__(self, symbol="btcusdt"):
        self.symbol = symbol.lower()
        self.display = FootprintDisplay()
        
        # 初始化 UM Futures WebSocket 客户端
        self.umfclient = UMFuturesWebsocketClient(on_message=self.spot_message_handler)
        
        # ------------------- 参数设置 -------------------
        self.imbalance_threshold = 3.0          # 整体失衡阈值
        self.volume_threshold_multiplier = 1.5  # 当前成交量大于历史平均成交量1.5倍视为放量
        self.order_quantity = 0.01              # 下单数量（BTC）
        
        self.TICK_SIZE = 1.0  # 价格档位间隔：每1美元一个档
        
        self.HISTORY_LENGTH = 288  # 用于记录过去24小时的数据 (24 * 12) 因为是5分钟一个周期
        self.orderflow_history = []  # 存储每5分钟的 footprint 数据
            
        # ------------------- 实时变量 -------------------
        self.current_minute = None
        self.footprint = self.new_minute_footprint()
        self.imbalance_checked = False

        # 新增支撑压力位分析相关参数
        self.support_resistance_levels = []  # 存储识别出的支撑压力位
        self.sr_volume_threshold = 0.1  # 支撑压力位的成交量阈值（占总成交量的比例）
        self.sr_price_range = 5  # 支撑压力位的价格范围（美元）
        self.reversal_threshold = 2.0  # 反转信号的失衡比例阈值

    def get_minute_str(self, timestamp_ms):
        """将毫秒级时间戳转换为5分钟级字符串"""
        dt = datetime.datetime.fromtimestamp(timestamp_ms / 1000)
        # 将分钟向下取整到最近的5分钟
        minute = dt.minute - (dt.minute % 5)
        return dt.strftime(f'%Y%m%d%H') + f'{minute:02d}'

    def new_minute_footprint(self):
        """返回一个新的 5分钟 级别的 footprint 数据结构，并重置检测标记"""
        self.imbalance_checked = False
        return {
            "time": None,           # 时间戳
            "open": None,           # 本5分钟第一个成交价
            "high": None,           # 本5分钟最高成交价
            "low": None,           # 本5分钟最低成交价
            "close": None,         # 本5分钟最后成交价
            "total_volume": 0.0,   # 本5分钟累计成交量
            "buy_volume": 0.0,     # 本5分钟累计买量
            "sell_volume": 0.0,    # 本5分钟累计卖量
            "delta": 0.0,          # 买量 - 卖量
            "order_flows": {}      # 价格层级数据
        }

    def analyze_support_resistance(self):
        """分析最近24小时数据中的支撑位和压力位"""
        if not self.orderflow_history:
            return []

        # 统计所有价格层级的成交量
        price_volumes = {}
        total_volume = 0
        
        # 遍历历史数据
        for minute_data in self.orderflow_history:
            for price_str, level_data in minute_data["order_flows"].items():
                price = int(price_str)
                volume = level_data["buy_volume"] + level_data["sell_volume"]
                price_volumes[price] = price_volumes.get(price, 0) + volume
                total_volume += volume

        # 识别高成交量价格区域
        significant_levels = []
        volume_threshold = total_volume * self.sr_volume_threshold

        for price, volume in price_volumes.items():
            if volume >= volume_threshold:
                # 计算该价位的买卖比例
                buy_volume = sum(
                    minute["order_flows"].get(str(price), {"buy_volume": 0})["buy_volume"]
                    for minute in self.orderflow_history
                )
                sell_volume = sum(
                    minute["order_flows"].get(str(price), {"sell_volume": 0})["sell_volume"]
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
        """5分钟后，更新历史数据"""
        self.footprint["delta"] = self.footprint["buy_volume"] - self.footprint["sell_volume"]
        
        # 将本5分钟的 footprint 深拷贝后存入历史记录
        self.orderflow_history.append(copy.deepcopy(self.footprint))
        if len(self.orderflow_history) > self.HISTORY_LENGTH:
            self.orderflow_history.pop(0)

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
                # 获取每个价位的买卖量
                level1 = self.footprint["order_flows"][str(p1)]
                level2 = self.footprint["order_flows"][str(p2)]
                level3 = self.footprint["order_flows"][str(p3)]
                
                b1, s1 = level1["buy_volume"], level1["sell_volume"]
                b2, s2 = level2["buy_volume"], level2["sell_volume"]
                b3, s3 = level3["buy_volume"], level3["sell_volume"]

                def imbalance_direction(b, s):
                    if b > 3 * s and s > 0:
                        return "多头"  # Long
                    elif s > 3 * b and b > 0:
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
        try:
            message = json.loads(data)
        except Exception as e:
            print("JSON解析异常:", e)
            return
        
        if message.get('e') != 'aggTrade':
            return

        trade_time = message.get('T')
        minute_str = self.get_minute_str(trade_time)

        try:
            price = float(message.get('p'))
            volume = float(message.get('q', 0))
        except Exception as e:
            print("数据转换异常:", e)
            return

        # 判断是否进入新的5分钟
        if self.current_minute is None:
            self.current_minute = minute_str
            self.footprint = self.new_minute_footprint()
            self.footprint["time"] = trade_time
            # 初始化第一个价格
            self.footprint["open"] = price
            self.footprint["high"] = price
            self.footprint["low"] = price
            self.footprint["close"] = price
        elif minute_str != self.current_minute:
            self.evaluate_minute()  # 只保存历史数据，不打印
            self.current_minute = minute_str
            self.footprint = self.new_minute_footprint()
            self.footprint["time"] = trade_time
            # 初始化新5分钟的第一个价格
            self.footprint["open"] = price
            self.footprint["high"] = price
            self.footprint["low"] = price
            self.footprint["close"] = price
        else:
            # 更新5分钟级别的价格数据
            self.footprint["close"] = price
            if price > self.footprint["high"]:
                self.footprint["high"] = price
            if price < self.footprint["low"]:
                self.footprint["low"] = price

        side = 'sell' if message.get('m', False) else 'buy'

        # 更新总成交量统计
        self.footprint["total_volume"] += volume
        if side == 'buy':
            self.footprint["buy_volume"] += volume
        else:
            self.footprint["sell_volume"] += volume

        # 更新价格层级数据
        price_level = str(int(price))
        if price_level not in self.footprint["order_flows"]:
            self.footprint["order_flows"][price_level] = {
                "buy_volume": 0.0,
                "sell_volume": 0.0,
                "order_count": 0
            }
        
        # 更新该价格层级的统计数据
        level_data = self.footprint["order_flows"][price_level]
        if side == 'buy':
            level_data["buy_volume"] += volume
        else:
            level_data["sell_volume"] += volume
        level_data["order_count"] += 1

        # 更新delta
        self.footprint["delta"] = self.footprint["buy_volume"] - self.footprint["sell_volume"]

        # 实时更新显示
        self.display.update_display(self.footprint)

    def start(self):
        self.umfclient.agg_trade(self.symbol)
        try:
            self.display.start_refresh_thread()  # 启动刷新线程
            self.display.app.run()
        finally:
            self.shutdown()

    def shutdown(self):
        self.display.stop_refresh_thread()  # 停止刷新线程
        self.umfclient.stop()

if __name__ == "__main__":
    trader = OrderFlowTrader()
    try:
        trader.start()
    except KeyboardInterrupt:
        trader.shutdown()
        print(f"\n{Fore.YELLOW}安全退出{Style.RESET_ALL}")
