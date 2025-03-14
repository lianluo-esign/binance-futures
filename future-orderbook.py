import datetime
import time
import json
from binance.websocket.um_futures.websocket_client import UMFuturesWebsocketClient
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
        
        # 滚动相关参数
        self.edge_threshold = 5     # 边界阈值（距离顶部或底部的行数）
        self.scroll_speed = 3       # 滚动速度
        
        # 添加样式
        self.style = Style.from_dict({
            'ask': 'ansired',        # 卖单红色
            'bid': 'ansigreen',      # 买单绿色
            'price': 'ansiwhite',    # 价格白色
            'header': 'ansiyellow',  # 表头黄色
            'current_row': 'bg:ansiwhite fg:ansiblack',  # 当前价格行背景色
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
            
            # 添加表头
            table_header = [
                ('class:header', "┌" + "─" * 16 + "┬" + "─" * 15 + "┬" + "─" * 16 + "┐\n"),
                ('class:header', "│     Ask Vol   │  Price      │    Bid Vol    │\n"),
                ('class:header', "├" + "─" * 16 + "┼" + "─" * 15 + "┼" + "─" * 16 + "┤\n")
            ]
            
            # 获取当前价格层级
            current_price = str(int(float(orderbook_data.get('current_price', 0))))
            
            # 生成价格层级数据行
            price_rows = []
            current_price_index = None
            
            # 合并所有价格层级
            all_prices = set()
            for price in orderbook_data['asks'].keys():
                all_prices.add(int(float(price)))
            for price in orderbook_data['bids'].keys():
                all_prices.add(int(float(price)))
            
            # 按价格排序（从高到低）
            sorted_prices = sorted(all_prices, reverse=True)
            
            for i, price in enumerate(sorted_prices):
                if str(price) == current_price:
                    current_price_index = i
                
                ask_vol = orderbook_data['asks'].get(str(price), 0.0)
                bid_vol = orderbook_data['bids'].get(str(price), 0.0)
                
                # 根据是否为当前价格设置样式
                if str(price) == current_price:
                    row = [
                        ('class:current_row', "│ "),
                        ('class:current_row', f"{ask_vol:12.3f}"),
                        ('class:current_row', " │ "),
                        ('class:current_row', f"{price:11}"),
                        ('class:current_row', " │ "),
                        ('class:current_row', f"{bid_vol:12.3f}"),
                        ('class:current_row', " │\n")
                    ]
                else:
                    row = [
                        ('class:normal', "│ "),
                        ('class:ask', f"{ask_vol:12.3f}" if ask_vol > 0 else " " * 12),
                        ('class:normal', " │ "),
                        ('class:price', f"{price:11}"),
                        ('class:normal', " │ "),
                        ('class:bid', f"{bid_vol:12.3f}" if bid_vol > 0 else " " * 12),
                        ('class:normal', " │\n")
                    ]
                price_rows.append(row)

            # 自动调整滚动位置
            total_rows = len(price_rows)
            if current_price_index is not None:
                visible_position = current_price_index - self.scroll_offset
                
                if visible_position >= (self.max_visible_rows - self.edge_threshold):
                    target_scroll = current_price_index - (self.max_visible_rows - self.edge_threshold)
                    if abs(target_scroll - self.scroll_offset) > self.scroll_speed:
                        self.scroll_offset += self.scroll_speed
                    else:
                        self.scroll_offset = target_scroll
                
                elif visible_position <= self.edge_threshold:
                    target_scroll = current_price_index - self.edge_threshold
                    if abs(target_scroll - self.scroll_offset) > self.scroll_speed:
                        self.scroll_offset -= self.scroll_speed
                    else:
                        self.scroll_offset = target_scroll

            # 确保滚动位置在有效范围内
            self.scroll_offset = min(max(0, self.scroll_offset), max(0, total_rows - self.max_visible_rows))
            start_idx = self.scroll_offset
            end_idx = min(start_idx + self.max_visible_rows, total_rows)
            
            # 组合最终显示内容
            self.current_text = (
                table_header +
                [item for row in price_rows[start_idx:end_idx] for item in row] +
                [('class:header', "└" + "─" * 16 + "┴" + "─" * 15 + "┴" + "─" * 16 + "┘\n")]
            )

class OrderBookTrader:
    def __init__(self, symbol="btcusdt"):
        self.symbol = symbol.lower()
        self.display = OrderBookDisplay()
        self.display.set_trader(self)
        
        # 初始化订单簿数据
        self.orderbook = {
            'asks': {},  # 卖单 price -> volume
            'bids': {},  # 买单 price -> volume
            'current_price': None
        }
        self.orderbook_lock = Lock()
        self.snapshot_received = False  # 添加标志位判断是否收到快照
        
        # 初始化WebSocket客户端
        self.umfclient = UMFuturesWebsocketClient(on_message=self.message_handler)

    def round_price(self, price):
        """将价格四舍五入到最近的整数"""
        try:
            return str(int(float(price) + 0.5))
        except (ValueError, TypeError):
            return None

    def message_handler(self, _, message):
        try:
            data = json.loads(message)
            
            # 处理深度数据
            if data.get('e') == 'depthUpdate':
                with self.orderbook_lock:
                    # 如果是第一次收到数据，作为快照保存
                    if not self.snapshot_received:
                        self.orderbook['asks'].clear()
                        self.orderbook['bids'].clear()
                        self.snapshot_received = True
                    
                    # 更新卖单
                    for ask in data.get('a', []):
                        try:
                            price = ask[0]
                            volume = ask[1]
                            if price is not None and volume is not None:
                                price = self.round_price(price)
                                volume = float(volume)
                                if volume > 0:
                                    self.orderbook['asks'][price] = volume
                                elif price in self.orderbook['asks']:
                                    del self.orderbook['asks'][price]  # 移除量为0的价格
                        except (IndexError, ValueError, TypeError):
                            continue
                    
                    # 更新买单
                    for bid in data.get('b', []):
                        try:
                            price = bid[0]
                            volume = bid[1]
                            if price is not None and volume is not None:
                                price = self.round_price(price)
                                volume = float(volume)
                                if volume > 0:
                                    self.orderbook['bids'][price] = volume
                                elif price in self.orderbook['bids']:
                                    del self.orderbook['bids'][price]  # 移除量为0的价格
                        except (IndexError, ValueError, TypeError):
                            continue
            
            # 处理最新价格
            elif data.get('e') == 'aggTrade':
                price = data.get('p')
                if price is not None:
                    with self.orderbook_lock:
                        self.orderbook['current_price'] = price
                
            # 更新显示
            if self.orderbook['current_price'] is not None and self.snapshot_received:
                self.display.update_display(self.orderbook)
            
        except Exception as e:
            print(f"处理消息异常: {e}")

    def start(self):
        # 订阅深度数据和最新成交
        # self.umfclient.diff_book_depth(self.symbol, speed=100)  # 改用diff_book_depth获取增量更新
        self.umfclient.partial_book_depth(self.symbol, speed=100, level=20)
        # self.umfclient.agg_trade(self.symbol)
        
        try:
            self.display.start_refresh_thread()
            self.display.app.run()
        finally:
            self.shutdown()

    def shutdown(self):
        self.display.stop_refresh_thread()
        self.umfclient.stop()

if __name__ == "__main__":
    trader = OrderBookTrader(symbol="btcusdt")
    try:
        trader.start()
    except KeyboardInterrupt:
        trader.shutdown()
        print(f"\n{Fore.YELLOW}安全退出{Style.RESET_ALL}")
