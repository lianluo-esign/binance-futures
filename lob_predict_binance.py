import json
import numpy as np
import torch
from binance.websocket.um_futures.websocket_client import UMFuturesWebsocketClient
from collections import deque
from deeplob_model import DeepLOB  # 你的 DeepLOB 模型
from preprocess import preprocess_lob  # 预处理数据

class BinanceLOBStreamer:
    def __init__(self, symbol="BTCUSDT"):
        self.symbol = symbol.lower()
        self.client = UMFuturesWebsocketClient(on_message=self.message_handler)
        self.lob_queue = deque(maxlen=100)  # 存储最近100次盘口数据
        self.model = torch.load("deeplob_model.pth")  # 加载已训练的 DeepLOB 模型
        self.model.eval()

    def start(self):
        """启动 WebSocket 监听 LOB 数据"""
        self.client.subscribe(stream=f"{self.symbol}@depth20@100ms")

    def message_handler(self, message):
        """解析 Binance 盘口数据并进行预测"""
        data = json.loads(message)
        if "b" in data and "a" in data:
            bids = np.array(data["b"][:10], dtype=np.float32)  # 前 10 档买单
            asks = np.array(data["a"][:10], dtype=np.float32)  # 前 10 档卖单
            lob_snapshot = np.concatenate([bids.flatten(), asks.flatten()])
            
            self.lob_queue.append(lob_snapshot)
            
            # 盘口数据填满后进行预测
            if len(self.lob_queue) == 100:
                X_input = np.array(self.lob_queue)
                X_input = preprocess_lob(X_input)  # 归一化等预处理
                X_tensor = torch.tensor(X_input, dtype=torch.float32).unsqueeze(0)

                # 预测短期市场方向
                with torch.no_grad():
                    prediction = self.model(X_tensor)
                    pred_class = torch.argmax(prediction, dim=1).item()

                print(f"📊 预测方向：{'上涨' if pred_class == 2 else '下跌' if pred_class == 0 else '震荡'}")

# 启动监听
lob_streamer = BinanceLOBStreamer()
lob_streamer.start()
