新增如下任务:

1. 多WebSocket管理器, 支持更多的交易所的数据源websocket连接用于后续的订阅他们的深度数据和逐笔成交数据 websocket管理器只负责往LockFreeEventBus 投放event数据 所有的后续操作都由handler去处理 注：通过搜索不同交易所的最新的开发文档来实现

2- 实现不同交易所的数据格式转换为统一格式 每条数据要带上交易所的name字段
     - binance (当前版本使用的交易所)
     - okx 
     - bybit
     - coinbase
     - bitget
     - bitfinex
     - gate.io
     - mexc
     - hyperliquid
      这些是目前主流的交易所的名字

2- 分层设计
   - 第一层基础数据层： 多个不同的交易所的orderbook管理和现有的orderbook数据结构保持一致 和 trades len=10000 条的数据的滑动窗口，经过统一格式转换之后都要管理保存，
   - 第二层聚合数据管理层：
         