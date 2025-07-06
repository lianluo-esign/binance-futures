# 多交易所WebSocket集成项目任务列表

## 任务状态说明
- ⬜ 未完成
- 🟨 进行中  
- ✅ 已完成

## 一、WebSocket连接管理器开发

### 1.1 基础架构设计
- ✅ 设计通用的ExchangeWebSocketManager trait
- ✅ 定义统一的WebSocket消息格式和错误处理机制
- ✅ 扩展EventBus支持exchange字段
- ✅ 实现多交易所管理器(MultiExchangeManager)
- ✅ 实现交易所类型枚举和配置管理

### 1.2 各交易所WebSocket实现
- ✅ Binance WebSocket连接实现（已完成）
- ✅ OKX WebSocket连接实现
  - ✅ 研究OKX API文档
  - ✅ 实现连接管理
  - ✅ 实现订阅BTCUSDT永续合约深度和成交数据
- ✅ Bybit WebSocket连接实现
  - ✅ 研究Bybit API文档
  - ✅ 实现连接管理
  - ✅ 实现订阅BTCUSDT永续合约深度和成交数据
- 🟨 Coinbase WebSocket连接实现
  - 🟨 研究Coinbase API文档
  - 🟨 实现连接管理
  - 🟨 实现订阅功能
- ✅ Bitget WebSocket连接实现
  - ✅ 研究Bitget API文档
  - ✅ 实现连接管理
  - ✅ 实现订阅功能
- ✅ Bitfinex WebSocket连接实现
  - ✅ 研究Bitfinex API文档
  - ✅ 实现连接管理
  - ✅ 实现订阅功能
- ✅ Gate.io WebSocket连接实现
  - ✅ 研究Gate.io API文档
  - ✅ 实现连接管理
  - ✅ 实现订阅功能
- ✅ MEXC WebSocket连接实现
  - ✅ 研究MEXC API文档
  - ✅ 实现连接管理
  - ✅ 实现订阅功能

## 二、数据格式转换Handler

### 2.1 统一数据格式设计
- ⬜ 定义统一的OrderBook数据结构（支持exchange字段）
- ⬜ 定义统一的Trade数据结构（支持exchange字段）
- ⬜ 设计合约张数到BTC数量的转换接口

### 2.2 各交易所数据转换实现
- ⬜ OKX数据格式转换Handler（合约张数→BTC）
- ⬜ Bybit数据格式转换Handler（合约张数→BTC）
- ⬜ Coinbase数据格式转换Handler
- ⬜ Bitget数据格式转换Handler（合约张数→BTC）
- ⬜ Bitfinex数据格式转换Handler
- ⬜ Gate.io数据格式转换Handler（合约张数→BTC）
- ⬜ MEXC数据格式转换Handler

## 三、分层数据管理架构

### 3.1 基础数据层(BasicLayer)
- ✅ 设计BasicLayer数据结构
- ✅ 实现每个交易所独立的OrderBook管理
- ✅ 实现每个交易所独立的Trades滑动窗口（10000条）
- ✅ 实现数据存储和查询接口

### 3.2 聚合数据层(AggLayer)
- ⬜ 设计AggLayer数据结构
- ⬜ 实现多交易所OrderBook按价格聚合（1美元精度）
  - ⬜ 价格向下取整逻辑
  - ⬜ 多交易所深度数据合并
- ⬜ 实现成交数据聚合统计
  - ⬜ 主动买单聚合
  - ⬜ 主动卖单聚合
  - ⬜ 历史累计买单
  - ⬜ 历史累计卖单
  - ⬜ 历史总计
  - ⬜ 历史delta计算

### 3.3 数据层管理器
- ⬜ 实现DataLayerManager统一管理接口
- ⬜ 实现层级切换功能
- ⬜ 实现数据订阅和推送机制

## 四、UI界面改造

### 4.1 OrderBook表格更新
- ⬜ 修改UnifiedOrderBookWidget使用AggLayer数据
- ⬜ 添加交易所来源显示
- ⬜ 优化聚合数据展示效果

### 4.2 Price Chart改造
- ⬜ 调整Price Chart布局（右侧全宽，高度40%）
- ⬜ 实现多交易所价格线显示
  - ⬜ 每个交易所使用不同颜色
  - ⬜ 添加交易所图例
  - ⬜ 实现10000条数据的滑动窗口显示
- ⬜ 添加交易所选择器（显示/隐藏某个交易所的线）

### 4.3 信号窗口改造
- ⬜ 添加交易所图标资源
  - ⬜ 收集各交易所logo图标
  - ⬜ 统一图标尺寸和格式
- ⬜ 修改信号显示逻辑
  - ⬜ 按交易所分组显示信号
  - ⬜ 在信号前添加交易所图标
- ⬜ 实现交易所过滤功能

## 五、测试和优化

### 5.1 功能测试
- ⬜ 各交易所WebSocket连接稳定性测试
- ⬜ 数据格式转换正确性测试
- ⬜ 数据聚合准确性测试
- ⬜ UI显示效果测试

### 5.2 性能优化
- ⬜ WebSocket连接池优化
- ⬜ 数据处理性能优化
- ⬜ UI渲染性能优化
- ⬜ 内存使用优化

### 5.3 错误处理
- ⬜ WebSocket断线重连机制
- ⬜ 数据异常处理
- ⬜ 交易所API限流处理
- ⬜ 用户友好的错误提示

## 六、文档和部署

### 6.1 文档更新
- ⬜ 更新架构文档
- ⬜ 更新配置文档
- ⬜ 更新用户使用指南
- ⬜ 添加各交易所API说明

### 6.2 配置管理
- ⬜ 添加交易所配置选项
- ⬜ 实现动态启用/禁用交易所
- ⬜ 添加API密钥管理（如需要）

## 开发优先级

1. **第一阶段**：WebSocket基础架构和OKX集成 ✅
   - ✅ 完成基础架构设计
   - ✅ 实现OKX WebSocket连接和数据转换
   - ✅ 实现多交易所管理器(MultiExchangeManager)
   - ✅ 实现BasicLayer基础功能

2. **第二阶段**：数据分层和聚合
   - 完成AggLayer实现
   - 更新OrderBook显示使用聚合数据

3. **第三阶段**：其他交易所集成
   - 按优先级逐个实现其他交易所
   - Bybit → Coinbase → Bitget → 其他

4. **第四阶段**：UI优化和完善
   - Price Chart多交易所显示
   - 信号窗口改造
   - 性能优化

## 备注
- 所有开发都针对BTCUSDT永续合约
- 优先保证系统稳定性和数据准确性
- 每完成一个交易所集成都要进行充分测试 