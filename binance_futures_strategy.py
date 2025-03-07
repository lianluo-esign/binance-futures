import time
from binance.websocket.um_futures.websocket_client import UMFuturesWebsocketClient
from collections import defaultdict
import json
from colorama import Fore, Back, Style

class EnhancedOFITrader:
    def __init__(self, symbol="btcusdt", window_seconds=60, threshold=100):
        self.symbol = symbol
        self.window = window_seconds
        self.threshold = threshold
        
        # 订单流统计
        self.ofi_buffer = defaultdict(float)
        self.current_window = None
        
        # 实时速度追踪
        self.last_ofi = None
        self.last_timestamp_ms = None
        
        # 订单簿深度维护
        self.bid_dict = {}  # {价格: 数量}
        self.ask_dict = {}  # {价格: 数量}
        self.last_depth_update = 0  # 最后更新时间戳
        
        # 初始化WebSocket
        self.client = UMFuturesWebsocketClient(on_message=self.message_handler)
        
    def message_handler(self, _, raw_message):
        try:
            data = json.loads(raw_message)
            
            # 处理深度更新
            if 'e' in data and data['e'] == 'depthUpdate':
                self.handle_depth(data)
                return
                
            # 原始交易数据处理逻辑
            current_timestamp_ms = data['T']
            current_volume = float(data['q'])
            is_seller = data['m']
            price = float(data['p'])
        
            # 计算当前OFI贡献值
            current_ofi = current_volume if not is_seller else -current_volume
            
            # ================= 实时速度计算 =================
            if self.last_ofi is not None and self.last_timestamp_ms is not None:
                delta_ofi = current_ofi - self.last_ofi
                speed_color = Fore.WHITE
                if delta_ofi > 2:
                    speed_color = Fore.GREEN
                    print(f"{speed_color}[实时] 速度: {delta_ofi:+.3f} 价格：{price:.2f}{Style.RESET_ALL}")
                elif delta_ofi < -2:
                    speed_color = Fore.RED
                    print(f"{speed_color}[实时] 速度: {delta_ofi:+.3f} 价格：{price:.2f}{Style.RESET_ALL}")
                
            
            # 更新追踪变量
            self.last_ofi = current_ofi
            self.last_timestamp_ms = current_timestamp_ms
            
            # ================= 时间窗口统计 =================
            timestamp_sec = current_timestamp_ms // 1000
            window_start = (timestamp_sec // self.window) * self.window
            self.ofi_buffer[window_start] += current_ofi
            
            # 当时间窗口切换时处理
            if self.current_window != window_start:
                if self.current_window is not None:
                    prev_ofi = self.ofi_buffer[self.current_window]
                    
                    # 交易信号
                    if prev_ofi > self.threshold:
                        print(f"\n{Back.GREEN}{Fore.BLACK}🚀 买入信号（OFI: {prev_ofi:.2f}）{Style.RESET_ALL}\n")
                    elif prev_ofi < -self.threshold:
                        print(f"\n{Back.RED}{Fore.BLACK}💣 卖出信号（OFI: {prev_ofi:.2f}）{Style.RESET_ALL}\n")
                    
                    # 窗口统计显示
                    if prev_ofi < -20:
                        window_color = Back.RED
                    elif prev_ofi > 20:
                        window_color = Back.GREEN
                    else:
                        window_color = Back.WHITE
                    print(f"{window_color}{Fore.BLACK}[窗口 {time.ctime(self.current_window)}] OFI: {prev_ofi:.2f}{Style.RESET_ALL}")
                
                self.current_window = window_start
            
        except Exception as e:
            print(f"处理错误: {str(e)}")

    def handle_depth(self, data):
        """处理深度更新"""
        try:
            # 更新买盘
            for price, qty in data['b']:
                price_f = float(price)
                qty_f = float(qty)
                if qty_f == 0:
                    self.bid_dict.pop(price_f, None)
                else:
                    self.bid_dict[price_f] = qty_f
                    
            # 更新卖盘
            for price, qty in data['a']:
                price_f = float(price)
                qty_f = float(qty)
                if qty_f == 0:
                    self.ask_dict.pop(price_f, None)
                else:
                    self.ask_dict[price_f] = qty_f
                    
            # 生成排序列表
            sorted_bids = sorted(self.bid_dict.items(), key=lambda x: -x[0])[:20]
            sorted_asks = sorted(self.ask_dict.items(), key=lambda x: x[0])[:20]
            
            # 计算压力值
            buy_pressure = self.calculate_pressure(sorted_bids, sorted_asks)
            self.display_pressure(buy_pressure)
            
        except Exception as e:
            print(f"深度处理错误: {str(e)}")

    def calculate_pressure(self, bids, asks):
        """计算买卖压力值（-1到1区间）"""
        total_bid = sum(qty for _, qty in bids)
        total_ask = sum(qty for _, qty in asks)
        
        total = total_bid + total_ask
        if total == 0:
            return 0.0  # 无挂单时显示中性
        
        # 计算压力值：买方占比 - 卖方占比
        return (total_bid - total_ask) / total

    def display_pressure(self, pressure):
        """带正负号的压力值显示"""
        # 颜色判断逻辑
        if pressure > 0.3:
            color = Back.GREEN + Fore.BLACK
        elif pressure < -0.3:
            color = Back.RED + Fore.BLACK
        else:
            color = Back.WHITE + Fore.BLACK
        
        # 带符号的百分比显示（保持5字符宽度：例如+12.3%）
        # print(f"{color}[深度压力] {pressure*100:+05.1f}%{Style.RESET_ALL}")
        
    def start(self):
        """启动监控"""
        # 同时订阅交易和深度数据
        self.client.agg_trade(symbol=self.symbol)
        self.client.subscribe(stream=f"{self.symbol.lower()}@depth20@100ms")
        
        print(f"{Fore.CYAN}启动 {self.symbol} 监控...{Style.RESET_ALL}")
        while True: 
            time.sleep(1)

    def shutdown(self):
        self.client.stop()

if __name__ == "__main__":
    trader = EnhancedOFITrader(window_seconds=60, threshold=10)
    try:
        trader.start()
    except KeyboardInterrupt:
        trader.shutdown()
        print(f"\n{Fore.YELLOW}安全退出{Style.RESET_ALL}")