import time
from pathlib import Path
from threading import Lock, Thread
from tinydb import TinyDB, Query
import json
from binance.websocket.um_futures.websocket_client import UMFuturesWebsocketClient
import signal
import sys
from colorama import Fore, Style
from datetime import datetime, timedelta

class TradesRecorder:
    def __init__(self, symbol="btcusdt"):
        self.symbol = symbol.lower()
        
        # 初始化数据库设置
        self.db_folder = Path("/Users/Administrator/Documents/trades_history")
        self.db_folder.mkdir(exist_ok=True)  # 创建存储文件夹
        
        # 初始化当前周的信息
        self.current_week_start = self._get_week_start()
        self.current_db_path = self._get_db_path(self.current_week_start)
        
        # 交易历史数据库设置
        self.trades_db = TinyDB(self.current_db_path)
        self.trades_table = self.trades_db.table(f'trades_history_{self.symbol}')
        
        # 存储队列和线程
        self.trades_queue = []
        self.trades_lock = Lock()
        self.trades_thread = None
        self._trades_running = True
        
        # WebSocket客户端
        self.ws_client = UMFuturesWebsocketClient(on_message=self.message_handler)
        
        # 启动存储线程
        self.start_storage_thread()
        
        print(f"{Fore.GREEN}交易数据记录器已启动 - 正在记录 {self.symbol} 的交易数据{Style.RESET_ALL}")
        print(f"{Fore.CYAN}当前数据文件: {self.current_db_path.name}{Style.RESET_ALL}")

    def _get_week_start(self, timestamp=None):
        """获取所在周的开始时间（以周一为第一天）"""
        if timestamp is None:
            dt = datetime.now()
        else:
            dt = datetime.fromtimestamp(timestamp / 1000)
        # 获取周一的日期，并设置时间为00:00:00
        monday = dt - timedelta(days=dt.weekday())
        return monday.replace(hour=0, minute=0, second=0, microsecond=0)

    def _get_db_path(self, week_start):
        """根据周开始时间生成数据库文件路径"""
        return self.db_folder / f"trades_{self.symbol}_{week_start.strftime('%Y%m%d')}.json"

    def _check_and_switch_db(self):
        """检查是否需要切换到新的周数据文件"""
        current_week_start = self._get_week_start()
        
        # 如果时间相同，不需要切换
        if current_week_start == self.current_week_start:
            return
        
        try:
            print(f"{Fore.YELLOW}开始新的一周，切换数据文件...{Style.RESET_ALL}")
            
            # 先保存所有待处理的数据
            with self.trades_lock:
                if self.trades_queue:
                    try:
                        self.trades_table.insert_multiple(self.trades_queue)
                        print(f"{Fore.GREEN}已保存剩余 {len(self.trades_queue)} 条交易数据{Style.RESET_ALL}")
                        self.trades_queue.clear()
                    except Exception as e:
                        print(f"{Fore.RED}保存剩余交易数据失败: {e}{Style.RESET_ALL}")
            
            # 关闭当前数据库连接
            self.trades_db.close()
            
            # 更新周信息和数据库路径
            self.current_week_start = current_week_start
            self.current_db_path = self._get_db_path(self.current_week_start)
            
            # 打开新的数据库连接
            self.trades_db = TinyDB(self.current_db_path)
            self.trades_table = self.trades_db.table(f'trades_history_{self.symbol}')
            
            print(f"{Fore.CYAN}新数据文件: {self.current_db_path.name}{Style.RESET_ALL}")
            
            # 清理旧文件
            self._cleanup_old_files()
            
        except Exception as e:
            print(f"{Fore.RED}切换数据文件失败: {e}{Style.RESET_ALL}")

    def _cleanup_old_files(self):
        """清理超过8周的旧文件"""
        try:
            current_time = datetime.now()
            for file in self.db_folder.glob(f"trades_{self.symbol}_*.json"):
                try:
                    # 从文件名提取日期
                    date_str = file.stem.split('_')[-1]
                    file_date = datetime.strptime(date_str, '%Y%m%d')
                    
                    # 如果文件超过8周，则删除
                    if (current_time - file_date).days > 56:  # 8周 = 56天
                        file.unlink()
                        print(f"{Fore.YELLOW}已删除旧文件: {file.name}{Style.RESET_ALL}")
                except Exception as e:
                    print(f"{Fore.RED}处理文件 {file.name} 时出错: {e}{Style.RESET_ALL}")
        except Exception as e:
            print(f"{Fore.RED}清理旧文件失败: {e}{Style.RESET_ALL}")

    def message_handler(self, _, message):
        """处理WebSocket消息"""
        try:
            data = json.loads(message)
            if data.get('e') != 'aggTrade':
                return

            # 提取交易数据
            trade_time = data.get('T')
            price = float(data.get('p'))
            volume = float(data.get('q', 0))
            side = 'sell' if data.get('m', False) else 'buy'
            
            # 保存交易数据
            self.save_trade(trade_time, price, volume, side)
            
        except Exception as e:
            print(f"{Fore.RED}处理消息失败: {e}{Style.RESET_ALL}")

    def start_storage_thread(self):
        """启动异步存储线程"""
        def trades_storage_loop():
            last_save_time = time.time()
            batch_size = 0
            
            while self._trades_running:
                current_time = time.time()
                
                # 检查是否需要切换数据文件
                self._check_and_switch_db()
                
                with self.trades_lock:
                    if self.trades_queue:
                        try:
                            # 批量保存数据
                            trades_to_save = self.trades_queue[:1000]  # 每次最多处理1000条
                            self.trades_table.insert_multiple(trades_to_save)
                            batch_size += len(trades_to_save)
                            del self.trades_queue[:len(trades_to_save)]
                            
                            # 每60秒打印一次状态
                            if current_time - last_save_time >= 60:
                                print(f"{Fore.CYAN}已保存 {batch_size} 条交易记录{Style.RESET_ALL}")
                                last_save_time = current_time
                                batch_size = 0
                                
                        except Exception as e:
                            print(f"{Fore.RED}保存交易数据失败: {e}{Style.RESET_ALL}")
                
                time.sleep(0.1)  # 避免CPU占用过高

        self.trades_thread = Thread(target=trades_storage_loop, daemon=True)
        self.trades_thread.start()

    def save_trade(self, trade_time, price, volume, side):
        """将交易数据添加到存储队列"""
        try:
            # 准备要保存的交易数据
            trade_data = {
                'symbol': self.symbol,
                'timestamp': trade_time,
                'price': price,
                'volume': volume,
                'side': side,
                'created_at': int(time.time())
            }
            
            # 添加到交易数据队列
            with self.trades_lock:
                self.trades_queue.append(trade_data)
            
        except Exception as e:
            print(f"{Fore.RED}准备交易数据失败: {e}{Style.RESET_ALL}")

    def cleanup_old_data(self):
        """清理超过7天的历史数据"""
        try:
            # 获取7天前的时间戳
            time_7d_ago = int(time.time() * 1000) - (7 * 24 * 60 * 60 * 1000)
            
            # 删除旧的交易数据
            History = Query()
            deleted_count = len(self.trades_table.remove(History.timestamp < time_7d_ago))
            print(f"{Fore.YELLOW}已清理 {deleted_count} 条过期数据{Style.RESET_ALL}")
            
        except Exception as e:
            print(f"{Fore.RED}清理历史数据失败: {e}{Style.RESET_ALL}")

    def start(self):
        """启动记录器"""
        try:
            # 订阅交易数据
            self.ws_client.agg_trade(self.symbol)
            
            # 每天自动清理一次旧数据
            def cleanup_loop():
                while self._trades_running:
                    self.cleanup_old_data()
                    time.sleep(24 * 3600)  # 24小时
            
            cleanup_thread = Thread(target=cleanup_loop, daemon=True)
            cleanup_thread.start()
            
            # 保持程序运行
            while self._trades_running:
                time.sleep(1)
                
        except KeyboardInterrupt:
            self.shutdown()
        except Exception as e:
            print(f"{Fore.RED}运行出错: {e}{Style.RESET_ALL}")
            self.shutdown()

    def shutdown(self):
        """关闭记录器"""
        print(f"\n{Fore.YELLOW}正在关闭交易数据记录器...{Style.RESET_ALL}")
        
        # 停止WebSocket连接
        self.ws_client.stop()
        
        # 停止存储线程
        self._trades_running = False
        
        if self.trades_thread:
            self.trades_thread.join(timeout=2)
        
        # 保存剩余的交易数据
        with self.trades_lock:
            if self.trades_queue:
                try:
                    self.trades_table.insert_multiple(self.trades_queue)
                    print(f"{Fore.GREEN}已保存剩余 {len(self.trades_queue)} 条交易数据{Style.RESET_ALL}")
                except Exception as e:
                    print(f"{Fore.RED}保存剩余交易数据失败: {e}{Style.RESET_ALL}")
        
        # 清理旧数据
        self.cleanup_old_data()
        # 关闭数据库连接
        self.trades_db.close()
        print(f"{Fore.GREEN}交易数据记录器已安全关闭{Style.RESET_ALL}")

def signal_handler(signum, frame):
    """处理进程信号"""
    print(f"\n{Fore.YELLOW}收到退出信号，正在安全关闭...{Style.RESET_ALL}")
    if 'recorder' in globals():
        recorder.shutdown()
    sys.exit(0)

if __name__ == "__main__":
    # 注册信号处理
    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)
    
    # 创建记录器实例
    recorder = TradesRecorder("btcusdt")
    
    try:
        recorder.start()
    except KeyboardInterrupt:
        recorder.shutdown()
    except Exception as e:
        print(f"{Fore.RED}程序异常退出: {e}{Style.RESET_ALL}")
        recorder.shutdown()
