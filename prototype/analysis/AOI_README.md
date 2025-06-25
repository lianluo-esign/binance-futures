# AOI (Active Order Imbalance) 指标分析工具

## 📊 功能概述

AOI指标分析工具是一个专门用于高频交易数据分析的Python工具，实现了主动成交不平衡指标的计算、可视化和回测分析功能。

### 🎯 核心功能

1. **AOI指标计算**: 实现多种窗口类型的AOI指标计算
2. **Delta Volume分析**: 计算买卖成交量差值
3. **交易信号生成**: 基于AOI阈值生成买卖信号
4. **可视化分析**: 生成时间序列图、信号图等
5. **统计报告**: 生成详细的分析报告和统计信息

## 📈 AOI指标原理

### 计算公式
```
AOI = (买入成交量 - 卖出成交量) / (买入成交量 + 卖出成交量)
Delta Volume = 买入成交量 - 卖出成交量
```

### 指标含义
- **AOI值范围**: [-1, 1]
- **AOI > 0**: 买方主导，市场偏向上涨
- **AOI < 0**: 卖方主导，市场偏向下跌
- **AOI接近±1**: 极端不平衡，可能出现强烈价格变动

### 窗口类型
1. **时间窗口**: 固定时间段内的交易数据聚合
2. **Tick窗口**: 固定交易笔数的数据聚合
3. **成交量窗口**: 固定成交量的数据聚合

## 🚀 使用方法

### 1. 环境要求
```bash
pip install pandas numpy matplotlib pymongo
```

### 2. 基本使用

#### 使用真实数据（需要MongoDB）
```python
from task1_AOI import AOIAnalyzer
from connect_mongodb import MongoDBConnector

# 连接MongoDB
mongo = MongoDBConnector(host='localhost', port=27017, db_name='crypto_data')

# 创建分析器
analyzer = AOIAnalyzer(mongo)

# 加载数据并分析
trades_df = analyzer.load_trade_data('btcusdt', start_time, end_time)
aoi_df = analyzer.calculate_aoi_series(trades_df, window_configs)
```

#### 使用模拟数据演示
```python
python task1_AOI_demo.py
```

### 3. 窗口配置示例
```python
window_configs = [
    {'type': 'time', 'size': 60},    # 60秒时间窗口
    {'type': 'time', 'size': 300},   # 5分钟时间窗口
    {'type': 'tick', 'size': 100},   # 100笔交易窗口
    {'type': 'tick', 'size': 500},   # 500笔交易窗口
]
```

## 📊 输出结果

### 1. 可视化图表
- **AOI时间序列图**: 显示不同窗口配置的AOI变化趋势
- **交易信号图**: 在价格图上标记买卖信号点
- **Delta Volume图**: 显示成交量不平衡情况

### 2. 统计报告
```json
{
  "analysis_time": "2025-06-25 13:52:49",
  "data_period": {
    "start": "2025-06-25 11:52:17",
    "end": "2025-06-25 13:52:17",
    "duration_hours": 2.0
  },
  "configurations": {
    "time_60": {
      "window_type": "time",
      "window_size": 60,
      "total_samples": 2400,
      "statistics": {
        "aoi_mean": -0.0040,
        "aoi_std": 0.3497,
        "positive_aoi_ratio": 0.5196,
        "strong_buy_ratio": 0.0775,
        "strong_sell_ratio": 0.0908
      }
    }
  }
}
```

### 3. 数据文件
- `aoi_data.csv`: 完整的AOI计算结果
- `signals_data.csv`: 交易信号数据
- `aoi_analysis_report.json`: 详细分析报告

## 📋 分析结果解读

### AOI统计指标
- **aoi_mean**: AOI平均值，反映整体市场偏向
- **aoi_std**: AOI标准差，反映市场波动性
- **positive_aoi_ratio**: 正向AOI比例，买方主导时间占比
- **strong_buy_ratio**: 强买入信号比例（AOI > 0.5）
- **strong_sell_ratio**: 强卖出信号比例（AOI < -0.5）

### 交易信号统计
- **BUY**: 买入信号数量
- **SELL**: 卖出信号数量  
- **HOLD**: 持有信号数量

## 🔧 参数调优

### 1. AOI阈值调整
```python
# 生成交易信号时可调整阈值
signals_df = analyzer.generate_signals(aoi_df, aoi_threshold=0.6)
```

### 2. 窗口大小优化
- **短窗口**: 更敏感，信号更频繁但可能有噪音
- **长窗口**: 更平滑，信号更可靠但可能滞后

### 3. 多窗口组合
- 使用多个窗口配置进行交叉验证
- 短期和长期AOI趋势一致时信号更可靠

## 📈 实际应用场景

### 1. 高频交易策略
- 基于AOI极值进行快速进出场
- 结合其他技术指标提高信号质量

### 2. 市场微观结构分析
- 分析不同时间段的市场主导力量
- 识别机构交易行为模式

### 3. 风险管理
- 监控市场不平衡程度
- 预警极端市场条件

## 🎯 演示结果分析

基于模拟数据的分析结果显示：

### 窗口效果对比
1. **60秒窗口**: 
   - AOI波动较大（标准差0.35）
   - 信号较频繁（7.75%强买入，9.08%强卖出）
   - 适合短期交易

2. **300秒窗口**: 
   - AOI更平滑（标准差0.17）
   - 信号较少但更可靠（0.04%强买入，0.75%强卖出）
   - 适合中期趋势判断

### 信号质量
- 大部分时间为HOLD信号，符合市场常态
- 极端信号相对较少，避免过度交易
- 不同窗口配置提供多层次分析视角

## 🔮 后续扩展

1. **多指标融合**: 结合OBI、Fill Ratio等其他指标
2. **机器学习**: 使用ML模型优化信号生成
3. **实时分析**: 接入实时数据流进行在线分析
4. **策略回测**: 完整的策略回测和绩效评估框架

## 📝 注意事项

1. **数据质量**: 确保交易数据的完整性和准确性
2. **参数敏感性**: 不同市场条件下需要调整参数
3. **延迟考虑**: 实际交易中需要考虑信号延迟
4. **风险控制**: 结合止损和仓位管理策略

---

*本工具为高频交易数据分析的研究工具，实际交易请谨慎使用并充分测试。*
