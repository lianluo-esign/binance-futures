# MEXC WebSocket API 文档

## 概述

MEXC WebSocket API 提供实时合约市场数据，包括深度数据和成交数据。该API专门针对MEXC合约交易平台。

## 连接信息

- **WebSocket URL**: `wss://contract.mexc.com/edge`
- **协议版本**: v1
- **认证**: 公共市场数据无需认证

## 消息格式

### 订阅消息格式

#### 深度数据订阅（增量模式，启用压缩）
```json
{
  "method": "sub.depth",
  "param": {
    "symbol": "BTC_USDT",
    "compress": true
  }
}
```

#### 成交数据订阅
```json
{
  "method": "sub.deal",
  "param": {
    "symbol": "BTC_USDT"
  }
}
```

### 响应消息格式

#### 深度数据响应
```json
{
  "channel": "push.depth",
  "data": {
    "asks": [
      [109419.6, 1546632, 1],
      [109464.6, 0, 0]
    ],
    "bids": [
      [108182.7, 950165, 1],
      [108032.4, 1532291, 1]
    ],
    "begin": 26012861157,
    "end": 26012861281,
    "version": 26012861281
  },
  "symbol": "BTC_USDT",
  "ts": 1751820030820
}
```

#### 成交数据响应
```json
{
  "channel": "push.deal",
  "data": {
    "M": 1,
    "O": 3,
    "T": 1,
    "p": 108709.4,
    "t": 1751820031121,
    "v": 322
  },
  "symbol": "BTC_USDT",
  "ts": 1751820031121
}
```

## 数据字段说明

### 深度数据字段
- `asks`: 卖盘数组，每个元素格式为 [价格, 合约张数, 订单数量]
- `bids`: 买盘数组，每个元素格式为 [价格, 合约张数, 订单数量]
- `begin`: 开始版本号
- `end`: 结束版本号
- `version`: 当前版本号
- `symbol`: 交易对符号
- `ts`: 时间戳

**注意**: 当合约张数为0时，表示该价位的订单已被撤销或成交完毕，应从订单簿中移除。

### 成交数据字段
- `p`: 成交价格
- `v`: 成交数量（合约张数）
- `T`: 成交方向，1:买入，2:卖出
- `O`: 是否是开仓，1:新增仓位，2:减少仓位，3:仓位不变
- `M`: 是否为自成交，1:是，2:否
- `t`: 成交时间戳
- `symbol`: 交易对符号
- `ts`: 推送时间戳

## 符号格式

MEXC合约使用下划线分隔的符号格式：
- 标准格式：`BTC_USDT`
- 转换规则：`BTCUSDT` → `BTC_USDT`

## 心跳机制

MEXC WebSocket连接支持标准的ping/pong心跳机制：
- 客户端发送：`Message::Ping(vec![])`
- 服务器响应：`Message::Pong`

## 错误处理

### 连接错误
- 网络连接失败
- 服务器拒绝连接
- 认证失败（私有频道）

### 订阅错误
- 无效的交易对符号
- 不支持的频道类型
- 参数格式错误

### 数据解析错误
- JSON格式错误
- 缺少必要字段
- 数据类型不匹配

## 使用示例

### Rust实现示例
```rust
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};

// 连接到MEXC WebSocket
let (mut ws_stream, _) = connect_async("wss://contract.mexc.com/edge").await?;

// 订阅深度数据
let depth_msg = json!({
    "method": "sub.depth",
    "param": {
        "symbol": "BTC_USDT",
        "compress": true
    }
});
ws_stream.send(Message::Text(depth_msg.to_string())).await?;

// 订阅成交数据
let trades_msg = json!({
    "method": "sub.deal",
    "param": {
        "symbol": "BTC_USDT"
    }
});
ws_stream.send(Message::Text(trades_msg.to_string())).await?;
```

## 版本更新历史

- **2024-01-31**: WebSocket地址更新为 `wss://contract.mexc.com/edge`
- **2025-04-09**: 订阅深度数据时默认启用压缩模式

## 性能特点

- **低延迟**: 毫秒级数据推送
- **高频率**: 支持高频交易数据
- **稳定性**: 自动重连机制
- **压缩**: 支持数据压缩减少带宽使用

## 注意事项

1. **版本控制**: 深度数据包含版本号，用于维护数据一致性
2. **增量更新**: 深度数据为增量推送，需要维护本地订单簿
3. **数据精度**: 价格和数量字段使用高精度浮点数
4. **频率限制**: 建议合理控制订阅频率，避免触发限流 