# Coinbase WebSocket连接实现

## 概述

本文档详细介绍了Coinbase WebSocket连接的实现，包括连接管理、数据订阅、消息处理等功能。

## 架构设计

### 核心组件

1. **CoinbaseWebSocketManager** - 主要管理器类
2. **CoinbaseWebSocketMessage** - 消息结构体
3. **CoinbaseSubscriptionRequest** - 订阅请求结构体
4. **CoinbaseChannel** - 频道配置结构体

### 实现特点

- 实现了`ExchangeWebSocketManager` trait
- 支持自动重连机制
- 完整的错误处理和统计信息
- 支持多种数据类型订阅
- 异步消息处理

## API端点

### 生产环境
- **URL**: `wss://ws-feed.exchange.coinbase.com`
- **协议**: WebSocket
- **认证**: 无需认证（公共市场数据）

### 测试环境
- **URL**: `wss://ws-feed-public.sandbox.exchange.coinbase.com`
- **协议**: WebSocket
- **认证**: 无需认证（公共市场数据）

## 支持的数据类型

### 1. 深度数据 (Level2)
- **频道名**: `level2`
- **产品ID**: `BTC-USD`
- **数据类型**: 订单簿快照和增量更新
- **消息类型**: `snapshot`, `l2update`

### 2. 交易数据 (Matches)
- **频道名**: `matches`
- **产品ID**: `BTC-USD`
- **数据类型**: 实时交易记录
- **消息类型**: `match`

### 3. 最优买卖价 (Ticker)
- **频道名**: `ticker`
- **产品ID**: `BTC-USD`
- **数据类型**: 实时最优买卖价
- **消息类型**: `ticker`

## 消息格式

### 订阅请求
```json
{
    "type": "subscribe",
    "channels": [
        {
            "name": "level2",
            "product_ids": ["BTC-USD"]
        }
    ]
}
```

### 深度数据快照
```json
{
    "type": "snapshot",
    "product_id": "BTC-USD",
    "bids": [
        ["30000.00", "0.5"]
    ],
    "asks": [
        ["30001.00", "0.3"]
    ]
}
```

### 深度数据更新
```json
{
    "type": "l2update",
    "product_id": "BTC-USD",
    "changes": [
        ["buy", "30000.00", "0.6"],
        ["sell", "30001.00", "0.0"]
    ]
}
```

### 交易数据
```json
{
    "type": "match",
    "product_id": "BTC-USD",
    "price": "30000.50",
    "size": "0.1",
    "side": "buy",
    "trade_id": 12345
}
```

### 最优买卖价
```json
{
    "type": "ticker",
    "product_id": "BTC-USD",
    "best_bid": "30000.00",
    "best_ask": "30001.00",
    "best_bid_size": "0.5",
    "best_ask_size": "0.3"
}
```

## 数据转换

### 符号转换
- 输入格式: `BTCUSDT`
- Coinbase格式: `BTC-USD`
- 转换逻辑: `symbol.replace("USDT", "-USD")`

### 价格处理
- 原始价格: 字符串格式 (如 "30000.50")
- 内部处理: 转换为浮点数
- 存储格式: 使用整数键避免浮点数精度问题 (`(price * 100.0) as i64`)

### 数量处理
- 原始数量: 字符串格式
- 内部处理: 转换为浮点数
- 特殊处理: 数量为0表示删除该价格层级

## 连接管理

### 连接流程
1. 创建WebSocket连接
2. 分离发送和接收流
3. 启动异步消息处理任务
4. 发送订阅请求

### 心跳机制
- 发送ping消息保持连接
- 监控pong响应时间
- 超时自动重连

### 重连逻辑
- 检测连接状态
- 自动重连机制
- 重新订阅之前的频道
- 统计重连次数

## 错误处理

### 连接错误
- 记录连接失败次数
- 自动重连
- 错误日志记录

### 解析错误
- 记录解析失败的消息
- 统计解析错误次数
- 继续处理其他消息

### 订阅错误
- 处理订阅失败
- 重试机制
- 错误状态通知

## 统计信息

### 连接统计
- 总消息数
- 总字节数
- 解析错误数
- 连接错误数
- 重连次数

### 性能监控
- 最后消息时间
- 消息处理速度
- 连接健康状态

## 使用示例

### 基本使用
```rust
use flow_sight::websocket::exchanges::CoinbaseWebSocketManager;
use flow_sight::websocket::exchange_trait::{ExchangeWebSocketManager, ExchangeConfig};

// 创建配置
let config = ExchangeConfig {
    exchange_name: "Coinbase".to_string(),
    symbol: "BTCUSDT".to_string(),
    testnet: false,
    api_key: None,
    api_secret: None,
};

// 创建管理器
let mut manager = CoinbaseWebSocketManager::new(config);

// 连接
manager.connect().await?;

// 订阅深度数据
manager.subscribe_depth("BTCUSDT").await?;

// 订阅交易数据
manager.subscribe_trades("BTCUSDT").await?;

// 订阅最优买卖价
manager.subscribe_book_ticker("BTCUSDT").await?;

// 读取消息
let messages = manager.read_messages().await?;
```

### 多交易所集成
```rust
use flow_sight::websocket::{MultiExchangeManagerBuilder, ExchangeType};

let mut manager = MultiExchangeManagerBuilder::new()
    .with_exchanges(vec![ExchangeType::Coinbase])
    .with_exchange_config(ExchangeType::Coinbase, coinbase_config)
    .build(event_bus);

manager.initialize().await?;
manager.start().await?;
manager.subscribe_btcusdt_perpetual().await?;
```

## 注意事项

### 限制
- 每IP每秒8个请求
- 突发请求最多20个
- 每个连接每秒100个消息

### 数据特点
- 订单簿数据可能有丢失
- 需要处理序列号间隙
- 建议使用level2频道保证数据完整性

### 产品ID
- Coinbase使用`BTC-USD`格式
- 注意与其他交易所的符号转换
- 永续合约在Coinbase中为现货交易

## 故障排除

### 常见问题
1. **连接失败**: 检查网络连接和URL正确性
2. **订阅失败**: 确认产品ID格式正确
3. **消息解析失败**: 检查消息格式是否符合预期
4. **重连频繁**: 检查网络稳定性和心跳设置

### 调试方法
1. 启用详细日志记录
2. 检查统计信息
3. 监控连接状态
4. 验证消息格式

## 更新日志

### v1.0.0
- 初始实现
- 支持基本的WebSocket连接
- 实现深度数据、交易数据、最优买卖价订阅
- 完整的错误处理和重连机制
- 统计信息收集
- 多交易所管理器集成 