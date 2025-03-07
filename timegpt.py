import pandas as pd
import matplotlib.pyplot as plt

# 假设已安装 nixtla 的 TimeGPT 库，具体库名和方法请参考官方文档
from nixtla import NixtlaClient

nixtla_client = NixtlaClient(
    api_key = 'nixak-NnHvfO0Xrow2LNqlZEcxePQ17RPJgB7svTer5LljWbHWY6jAYDA2uJ4nrUxz7RrxS4AvyfIpTG10c4A4'
)

# nixtla_client.validate_api_key()
df = pd.read_csv('https://raw.githubusercontent.com/Nixtla/transfer-learning-time-series/main/datasets/air_passengers.csv')
df.head()

nixtla_client.plot(df, time_col='timestamp', target_col='value')
timegpt_fcst_df = nixtla_client.forecast(df=df, h=12, freq='MS', time_col='timestamp', target_col='value')
timegpt_fcst_df.head()

# # ----------------------------
# # 1. 加载并预处理数据
# # ----------------------------
# # 读取现货黄金 XAUUSD 5分钟 K 线数据
# df = pd.read_csv("XAUUSD_5M.csv")

# # 转换 timestamp 为 datetime 类型，并按时间排序
# df["timestamp"] = pd.to_datetime(df["timestamp"])
# df = df.sort_values("timestamp")

# # 选择收盘价数据，重命名为 'value'
# series = df[["timestamp", "close"]].rename(columns={"close": "value"})

# # ----------------------------
# # 2. 加载预训练好的 TimeGPT 模型
# # ----------------------------
# # 加载预训练模型（无需重新训练）
# model = TimeGPT.load_pretrained()

# # ----------------------------
# # 3. 进行预测
# # ----------------------------
# # 预测未来12个时间点（5分钟一根K线，12根即未来60分钟）
# forecast_horizon = 12
# forecast = model.forecast(series, horizon=forecast_horizon)

# # 输出预测结果
# print("预测结果:")
# print(forecast)

# # ----------------------------
# # 4. 绘制历史数据与预测结果
# # ----------------------------
# plt.figure(figsize=(12, 6))
# plt.plot(series["timestamp"], series["value"], label="历史收盘价")
# plt.plot(forecast["timestamp"], forecast["forecast"], label="预测收盘价", linestyle="--")
# plt.xlabel("时间")
# plt.ylabel("价格")
# plt.title("XAUUSD 5分钟K线 收盘价预测")
# plt.legend()
# plt.show()
