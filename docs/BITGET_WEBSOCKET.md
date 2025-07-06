# Bitget WebSocket 实现

## 概述

本文档描述了Bitget WebSocket连接的实现，用于接收BTCUSDT永续合约的实时市场数据。

## 实现特性

### 支持的数据类型
- **深度数据 (books)**: 实时订单簿数据
- **成交数据 (trade)**: 实时成交记录
- **最优买卖价 (ticker)**: 实时最优买卖价信息

### 技术规格
- **WebSocket端点**: `wss://ws.bitget.com/v2/ws/public`
- **产品类型**: UMCBL (USDT永续合约)
- **符号格式**: BTCUSDT_UMCBL
- **协议版本**: V2

## API 接口

### 订阅消息格式

#### 深度数据订阅
```json
{
    "op": "subscribe",
    "args": [{
        "instType": "UMCBL",
        "channel": "books",
        "instId": "BTCUSDT_UMCBL"
    }]
}
```

#### 成交数据订阅
```json
{
    "op": "subscribe",
    "args": [{
        "instType": "UMCBL",
        "channel": "trade",
        "instId": "BTCUSDT_UMCBL"
    }]
}
```

#### 最优买卖价订阅
```json
{
    "op": "subscribe",
    "args": [{
        "instType": "UMCBL",
        "channel": "ticker",
        "instId": "BTCUSDT_UMCBL"
    }]
}
```

### 心跳机制
```json
{
    "op": "ping"
}
```

## 数据格式

### 深度数据响应
```json
{
    "arg": {
        "instType": "UMCBL",
        "channel": "books",
        "instId": "BTCUSDT_UMCBL"
    },
    "data": [{
        "asks": [["43500.0", "0.5"], ["43501.0", "1.0"]],
        "bids": [["43499.0", "0.8"], ["43498.0", "1.2"]],
        "ts": "1703123456789"
    }]
}
```

### 成交数据响应
```json
{
    "arg": {
        "instType": "UMCBL",
        "channel": "trade",
        "instId": "BTCUSDT_UMCBL"
    },
    "data": [{
        "price": "43500.0",
        "size": "0.1",
        "side": "buy",
        "ts": "1703123456789"
    }]
}
```

### 最优买卖价响应
```json
{
    "arg": {
        "instType": "UMCBL",
        "channel": "ticker",
        "instId": "BTCUSDT_UMCBL"
    },
    "data": [{
        "bidPx": "43499.0",
        "askPx": "43501.0",
        "ts": "1703123456789"
    }]
}
```

## 实现细节

### BitgetWebSocketManager 结构
```rust
pub struct BitgetWebSocketManager {
    connection_state: ExchangeConnectionState,
    stats: ExchangeStats,
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    event_sender: Option<mpsc::UnboundedSender<Event>>,
}
```

### 主要方法

#### 连接管理
- `connect()`: 建立WebSocket连接
- `disconnect()`: 断开连接
- `attempt_reconnect()`: 重连逻辑

#### 数据订阅
- `subscribe_depth()`: 订阅深度数据
- `subscribe_trades()`: 订阅成交数据
- `subscribe_book_ticker()`: 订阅最优买卖价

#### 消息处理
- `handle_message()`: 处理接收到的消息
- `parse_message()`: 解析消息内容
- `send_heartbeat()`: 发送心跳

### 符号转换
Bitget期货使用特殊的符号格式：
- 输入: `BTCUSDT`
- 输出: `BTCUSDT_UMCBL`

```rust
fn convert_symbol(&self, symbol: &str) -> String {
    if symbol == "BTCUSDT" {
        "BTCUSDT_UMCBL".to_string()
    } else {
        format!("{}_UMCBL", symbol)
    }
}
```

## 错误处理

### 连接错误
- 自动重连机制
- 连接状态跟踪
- 错误统计

### 消息解析错误
- 解析错误计数
- 错误日志记录
- 继续处理其他消息

### 心跳机制
- 定期发送ping消息
- 处理pong响应
- 连接保活

## 统计信息

### 连接统计
- 总消息数
- 总字节数
- 解析错误数
- 连接错误数
- 重连次数

### 状态监控
- 连接状态
- 最后消息时间
- 连接开始时间

## 使用示例

### 基本使用
```rust
use flow_sight::websocket::exchanges::bitget::BitgetWebSocketManager;
use flow_sight::websocket::exchange_trait::ExchangeWebSocketManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = BitgetWebSocketManager::new();
    
    // 连接
    manager.connect().await?;
    
    // 订阅数据
    manager.subscribe_btcusdt_perpetual().await?;
    
    // 处理消息
    loop {
        let messages = manager.read_messages().await?;
        for message in messages {
            println!("收到消息: {}", message);
        }
    }
}
```

### 与多交易所管理器集成
```rust
use flow_sight::websocket::{MultiExchangeManagerBuilder, ExchangeType, ExchangeConfig};

let bitget_config = ExchangeConfig {
    exchange_name: "Bitget".to_string(),
    symbol: "BTCUSDT".to_string(),
    testnet: false,
    api_key: None,
    api_secret: None,
};

let mut manager = MultiExchangeManagerBuilder::new()
    .with_exchanges(vec![ExchangeType::Bitget])
    .with_exchange_config(ExchangeType::Bitget, bitget_config)
    .build(event_bus);
```

## 注意事项

1. **符号格式**: Bitget期货使用`_UMCBL`后缀
2. **数据格式**: 所有价格和数量都是字符串格式
3. **心跳**: 需要定期发送ping消息保持连接
4. **重连**: 支持自动重连机制
5. **错误处理**: 单个消息解析错误不会影响整个连接

## 测试

运行Bitget WebSocket测试：
```bash
cargo run --example multi_exchange_test
```

这将启动包含Bitget在内的多交易所测试程序。

## 版本信息

- **实现版本**: 1.0.0
- **Bitget API版本**: V2
- **支持的合约类型**: UMCBL (USDT永续合约)
- **最后更新**: 2024-01-XX 