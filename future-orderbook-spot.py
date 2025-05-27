import datetime
import time
import json
# from binance.websocket.um_futures.websocket_client import UMFuturesWebsocketClient
from binance.websocket.spot.websocket_stream import SpotWebsocketStreamClient
from colorama import Fore, Back, Style
from prompt_toolkit import Application
from prompt_toolkit.layout import Layout, Window, HSplit, FormattedTextControl
from prompt_toolkit.key_binding import KeyBindings
from threading import Lock
import threading
from prompt_toolkit.styles import Style

class OrderBookDisplay:
    def __init__(self):
        self.lock = Lock()
        self.current_text = []
        self.kb = KeyBindings()
        self.scroll_offset = 0
        self.max_visible_rows = 50  # 可见行数
        
        # 修改滚动相关参数
        self.edge_threshold = 15    # 边界阈值（距离顶部或底部的行数）
        self.scroll_speed = 3       # 滚动速度
        self.price_position = 35    # 当前价格在可见区域的目标位置
        
        # 修改样式定义，添加加粗样式
        self.style = Style.from_dict({
            'ask': 'ansired',        # 卖单红色
            'bid': 'ansigreen',      # 买单绿色
            'price': 'ansiwhite',    # 价格白色
            'header': 'ansiyellow',  # 表头黄色
            'current_row': 'bg:ansiwhite fg:ansiblack',  # 当前价格行背景色
            'volume_bold': 'bold',   # 加粗样式
            'bid_bold': 'ansigreen bold',  # 买单绿色加粗
            'ask_bold': 'ansired bold',    # 卖单红色加粗
        })

        @self.kb.add('c-c')
        def _(event):
            event.app.exit()

        @self.kb.add('up')
        def _(event):
            self.scroll_offset = max(0, self.scroll_offset - self.scroll_speed)
            event.app.invalidate()

        @self.kb.add('down')
        def _(event):
            self.scroll_offset += self.scroll_speed
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
            height=self.max_visible_rows + 3,  # 额外空间用于表头
            wrap_lines=False
        )
        self.layout = Layout(self.window)
        
        self.app = Application(
            layout=self.layout,
            key_bindings=self.kb,
            full_screen=True,
            mouse_support=True,
            style=self.style,
            color_depth='DEPTH_24_BIT'
        )

        self.refresh_interval = 0.1
        self._running = True
        self._refresh_thread = None
        self.trader = None

    def set_trader(self, trader):
        self.trader = trader

    def start_refresh_thread(self):
        def refresh_loop():
            while self._running:
                self.app.invalidate()
        
        self._refresh_thread = threading.Thread(target=refresh_loop, daemon=True)
        self._refresh_thread.start()

    def stop_refresh_thread(self):
        self._running = False
        if self._refresh_thread:
            self._refresh_thread.join()

    def get_formatted_text(self):
        with self.lock:
            return self.current_text

    def update_display(self, orderbook_data):
        with self.lock:
            self.current_text = []
            
            if not orderbook_data.current_price:
                return
            
            current_price = float(orderbook_data.current_price)
            price_rows = []
            current_price_index = None
            
            # 获取所有价格并排序
            all_prices = sorted(orderbook_data.price_levels.keys(), key=lambda x: float(x), reverse=True)
            
            # 生成价格行
            for i, price in enumerate(all_prices):
                price_float = float(price)
                if abs(price_float - current_price) < 0.000001:
                    current_price_index = i
                
                data = orderbook_data.price_levels.get(price, {'ask': 0, 'bid': 0})
                bid_vol = data['bid']
                ask_vol = data['ask']
                
                # 格式化数值，使其居中显示
                bid_str = f"{bid_vol:.1f}" if bid_vol > 0 else ""
                ask_str = f"{ask_vol:.1f}" if ask_vol > 0 else ""
                price_str = f"{price_float:.2f}"
                
                # 计算居中所需的空格
                bid_space = (12 - len(bid_str)) // 2
                ask_space = (12 - len(ask_str)) // 2
                price_space = (11 - len(price_str)) // 2
                
                # 构建行数据
                if abs(price_float - current_price) < 0.000001:
                    row = [
                        ('class:normal', "│"),
                        ('class:normal', " " * (bid_space + 1)),
                        ('class:bid_bold', bid_str if bid_vol > 0 else " " * len(bid_str)),  # 加粗的买单
                        ('class:normal', " " * (12 - len(bid_str) - bid_space)),
                        ('class:normal', "│"),
                        ('class:normal', " " * (price_space + 1)),
                        ('class:price', price_str),
                        ('class:normal', " " * (11 - len(price_str) - price_space)),
                        ('class:normal', "│"),
                        ('class:normal', " " * (ask_space + 1)),
                        ('class:ask_bold', ask_str if ask_vol > 0 else " " * len(ask_str)),  # 加粗的卖单
                        ('class:normal', " " * (12 - len(ask_str) - ask_space)),
                        ('class:normal', "│\n")
                    ]
                else:
                    show_bid = price_float < current_price
                    show_ask = price_float > current_price
                    
                    row = [
                        ('class:normal', "│"),
                        ('class:normal', " " * (bid_space + 1)),
                        ('class:bid_bold', bid_str if show_bid and bid_vol > 0 else " " * len(bid_str)),  # 加粗的买单
                        ('class:normal', " " * (12 - len(bid_str) - bid_space)),
                        ('class:normal', "│"),
                        ('class:normal', " " * (price_space + 1)),
                        ('class:price', price_str),
                        ('class:normal', " " * (11 - len(price_str) - price_space)),
                        ('class:normal', "│"),
                        ('class:normal', " " * (ask_space + 1)),
                        ('class:ask_bold', ask_str if show_ask and ask_vol > 0 else " " * len(ask_str)),  # 加粗的卖单
                        ('class:normal', " " * (12 - len(ask_str) - ask_space)),
                        ('class:normal', "│\n")
                    ]
                price_rows.append(row)

            # 更新滚动位置
            if current_price_index is not None:
                # 计算当前价格在可视区域中的位置
                visible_position = current_price_index - self.scroll_offset
                
                # 检查是否需要滚动
                if visible_position < self.edge_threshold:
                    # 当前价格太靠近顶部，向上滚动
                    self.scroll_offset = max(0, self.scroll_offset - self.scroll_speed)
                elif visible_position > (self.max_visible_rows - self.edge_threshold):
                    # 当前价格太靠近底部，向下滚动
                    self.scroll_offset = min(
                        len(price_rows) - self.max_visible_rows,
                        self.scroll_offset + self.scroll_speed
                    )

            # 确保滚动范围有效
            total_rows = len(price_rows)
            self.scroll_offset = min(max(0, self.scroll_offset), max(0, total_rows - self.max_visible_rows))
            
            # 确定显示范围
            start_idx = self.scroll_offset
            end_idx = min(start_idx + self.max_visible_rows, total_rows)
            
            # 组合最终显示内容
            self.current_text = [item for row in price_rows[start_idx:end_idx] for item in row]

class OrderBookData:
    def __init__(self, price_range=4):
        self.price_levels = {}  # 价格层级数据 {price: {'ask': volume, 'bid': volume}}
        self.current_price = None  # 当前成交价格
        self.price_range = price_range * 3600  # 转换为价格刻度（4小时）

    def update(self, price, volume, side):
        """更新指定价格和方向的挂单量"""
        if volume > 0:
            if price not in self.price_levels:
                self.price_levels[price] = {'ask': 0, 'bid': 0}
            self.price_levels[price][side] = volume
        else:
            # 如果量为0，删除该价格层级的对应方向数据
            if price in self.price_levels:
                self.price_levels[price][side] = 0
                # 如果买卖双方都为0，则删除整个价格层级
                if self.price_levels[price]['ask'] == 0 and self.price_levels[price]['bid'] == 0:
                    del self.price_levels[price]

    def update_current_price(self, price):
        """更新当前成交价格"""
        self.current_price = str(price)  # 直接使用原始价格

    def _update_best_prices(self):
        """更新最优买卖价格"""
        best_bid = None
        best_ask = None
        
        for price, data in self.price_levels.items():
            price_float = float(price)
            if data['bid'] > 0:
                if best_bid is None or price_float > best_bid:
                    best_bid = price_float
            if data['ask'] > 0:
                if best_ask is None or price_float < best_ask:
                    best_ask = price_float
        
        self.best_bid = best_bid
        self.best_ask = best_ask
        
        # 更新当前价格为最高买价
        if self.best_bid is not None:
            self.current_price = str(self.best_bid)  # 直接使用原始价格

    def clean_old_levels(self):
        """清理超出价格范围的数据"""
        if not self.current_price:
            return
        
        current = float(self.current_price)
        min_price = current - self.price_range
        max_price = current + self.price_range
        
        # 删除超出范围的价格层级
        self.price_levels = {
            price: data for price, data in self.price_levels.items()
            if min_price <= float(price) <= max_price
        }

class OrderBookTrader:
    def __init__(self, symbol="btcusdt"):
        self.symbol = symbol.lower()
        self.display = OrderBookDisplay()
        self.display.set_trader(self)
        self.orderbook = OrderBookData(price_range=4)
        self.orderbook_lock = Lock()
        
        # 添加重连相关的属性
        self.is_running = False
        self.reconnect_delay = 1  # 初始重连延迟1秒
        self.max_reconnect_delay = 60  # 最大重连延迟60秒
        self.last_message_time = time.time()
        self.heartbeat_interval = 5  # 心跳检测间隔（秒）
        
        # 创建WebSocket客户端
        self.create_websocket_client()

    def create_websocket_client(self):
        """创建新的WebSocket客户端"""
        if hasattr(self, 'umfclient') and self.umfclient:
            try:
                self.umfclient.stop()
            except:
                pass
        self.umfclient = SpotWebsocketStreamClient(
            on_message=self.message_handler,
            on_close=self.handle_disconnect
        )

    def handle_disconnect(self, *args):
        """处理断开连接事件"""
        if not self.is_running:
            return
        
        print(f"WebSocket断开连接，{self.reconnect_delay}秒后尝试重连...")
        time.sleep(self.reconnect_delay)
        
        # 指数退避重连
        self.reconnect_delay = min(self.reconnect_delay * 2, self.max_reconnect_delay)
        
        try:
            self.create_websocket_client()
            # 重新订阅数据
            self.subscribe_data()
            print("重连成功！")
            # 重连成功后重置重连延迟
            self.reconnect_delay = 1
        except Exception as e:
            print(f"重连失败: {e}")
            # 递归调用自身继续尝试重连
            self.handle_disconnect()

    def subscribe_data(self):
        """订阅数据"""
        try:
            # 现货的订阅格式需要加上 symbol@
            self.umfclient.diff_book_depth(
                symbol=self.symbol,
                speed=100,
                id=1
            )
            self.umfclient.agg_trade(
                symbol=self.symbol,
                id=2
            )
        except Exception as e:
            print(f"订阅数据失败: {e}")
            raise

    def start_heartbeat_thread(self):
        """启动心跳检测线程"""
        def heartbeat_check():
            while self.is_running:
                current_time = time.time()
                if current_time - self.last_message_time > self.heartbeat_interval:
                    print("检测到连接异常，准备重连...")
                    self.handle_disconnect()
                time.sleep(1)

        self.heartbeat_thread = threading.Thread(target=heartbeat_check, daemon=True)
        self.heartbeat_thread.start()

    def message_handler(self, _, message):
        try:
            # 更新最后收到消息的时间
            self.last_message_time = time.time()
            # 重置重连延迟
            self.reconnect_delay = 1
            
            data = json.loads(message)
            
            with self.orderbook_lock:
                # 处理深度数据
                if data.get('e') == 'depthUpdate':
                    # 更新卖单
                    for ask in data.get('a', []):
                        try:
                            price = self.round_price(ask[0])
                            volume = float(ask[1])
                            if price:
                                self.orderbook.update(price, volume, 'ask')
                        except (ValueError, TypeError):
                            continue
                    
                    # 更新买单
                    for bid in data.get('b', []):
                        try:
                            price = self.round_price(bid[0])
                            volume = float(bid[1])
                            if price:
                                self.orderbook.update(price, volume, 'bid')
                        except (ValueError, TypeError):
                            continue
                
                # 处理成交价格
                elif data.get('e') == 'aggTrade':
                    price = data.get('p')
                    if price:
                        self.orderbook.update_current_price(price)
                
                # 更新显示
                if self.orderbook.current_price:
                    self.display.update_display(self.orderbook)
        
        except Exception as e:
            print(f"处理消息异常: {e}")

    def round_price(self, price):
        """保持原始价格精度"""
        try:
            return str(price)  # 直接返回原始价格字符串
        except (ValueError, TypeError):
            return None

    def start(self):
        self.is_running = True
        try:
            # 启动心跳检测线程
            self.start_heartbeat_thread()
            # 订阅数据
            self.subscribe_data()
            # 启动显示
            self.display.start_refresh_thread()
            self.display.app.run()
        finally:
            self.shutdown()

    def shutdown(self):
        self.is_running = False
        self.display.stop_refresh_thread()
        if self.umfclient:
            try:
                self.umfclient.stop()
            except:
                pass

if __name__ == "__main__":
    trader = OrderBookTrader(symbol="btcusdt")
    try:
        trader.start()
    except KeyboardInterrupt:
        trader.shutdown()
        print("\n安全退出")
