# Bitfinex WebSocket实现文档

## 概述

本文档详细介绍了Bitfinex WebSocket连接的实现，包括API架构、消息格式、订阅方式和数据处理逻辑。

## Bitfinex WebSocket API架构

### 连接信息
- **WebSocket URL**: `wss://api-pub.bitfinex.com/ws/2`
- **协议版本**: V2
- **认证**: 公共频道无需认证
- **永续合约符号**: `tBTCF0:USTF0`

### 支持的频道类型
1. **订单簿 (book)** - 实时订单簿数据
2. **交易 (trades)** - 实时成交记录
3. **行情 (ticker)** - 实时价格信息

## 消息格式

### 订阅消息
```json
{
    "event": "subscribe",
    "channel": "book",
    "symbol": "tBTCF0:USTF0",
    "prec": "P0",
    "freq": "F0", 
    "len": "25"
}
```

### 订阅确认
```json
{
    "event": "subscribed",
    "channel": "book",
    "chanId": 12345,
    "symbol": "tBTCF0:USTF0",
    "prec": "P0",
    "freq": "F0",
    "len": "25",
    "pair": "BTCF0:USTF0"
}
```

### 数据消息格式

#### 订单簿快照
```json
[
    12345,  // CHANNEL_ID
    [
        [7254.7, 3, 3.3],    // [PRICE, COUNT, AMOUNT]
        [7254.6, 2, 1.5],
        ...
    ]
]
```

#### 订单簿更新
```json
[
    12345,      // CHANNEL_ID
    [7254.5, 0, 1]  // [PRICE, COUNT, AMOUNT]
]
```

#### 交易数据
```json
[
    12345,          // CHANNEL_ID
    "te",           // 交易执行标识
    "1234-BTCUSD",  // 序列号
    1443659698,     // 时间戳
    236.42,         // 价格
    0.49064538      // 数量
]
```

#### 心跳消息
```json
[12345, "hb"]
```

## 实现特点

### 1. 符号转换
- 输入: `BTCUSDT`
- 输出: `tBTCF0:USTF0` (永续合约格式)

### 2. 消息解析
- 订阅确认消息处理
- 数据消息数组格式解析
- 心跳消息识别和处理
- 错误消息处理

### 3. 数据处理
- 订单簿快照和更新分离处理
- 交易数据实时处理
- 事件转换为统一格式

### 4. 连接管理
- 自动重连机制
- 心跳保活
- 状态管理

## 订单簿算法

### 维护逻辑
1. 订阅频道后接收快照
2. 根据更新消息维护订单簿：
   - `COUNT > 0`: 添加或更新价格档位
   - `COUNT = 0`: 删除价格档位
   - `AMOUNT > 0`: 买单 (bid)
   - `AMOUNT < 0`: 卖单 (ask)

### 删除逻辑
- `COUNT = 0` 且 `AMOUNT = 1`: 删除买单
- `COUNT = 0` 且 `AMOUNT = -1`: 删除卖单

## 错误处理

### 错误代码
- `10011`: 未知订单簿精度
- `10012`: 未知订单簿长度
- `10000`: 未知事件
- `10001`: 未知交易对

### 重连策略
- 连接失败后等待5秒重试
- 指数退避重连
- 最大重连次数限制

## 配置参数

### 订单簿精度 (prec)
- `P0`: 5位有效数字
- `P1`: 4位有效数字
- `P2`: 3位有效数字
- `P3`: 2位有效数字
- `P4`: 1位有效数字

### 更新频率 (freq)
- `F0`: 实时更新
- `F1`: 2秒更新一次

### 深度长度 (len)
- `"1"`: 1档深度
- `"25"`: 25档深度 (默认)
- `"100"`: 100档深度
- `"250"`: 250档深度

## 使用示例

```rust
use crate::websocket::exchanges::bitfinex::BitfinexWebSocketManager;
use crate::websocket::exchange_trait::ExchangeWebSocketManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = BitfinexWebSocketManager::new();
    
    // 连接到Bitfinex
    manager.connect().await?;
    
    // 订阅深度数据
    manager.subscribe_depth("BTCUSDT").await?;
    
    // 订阅交易数据
    manager.subscribe_trades("BTCUSDT").await?;
    
    // 订阅行情数据
    manager.subscribe_ticker("BTCUSDT").await?;
    
    // 启动消息循环
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    manager.start_message_loop(tx).await?;
    
    // 处理接收到的事件
    while let Some(event) = rx.recv().await {
        println!("收到Bitfinex事件: {:?}", event);
    }
    
    Ok(())
}
```

## 性能考虑

### 消息处理
- 异步消息解析
- 批量事件处理
- 内存池优化

### 连接优化
- 连接复用
- 压缩支持
- 流量控制

## 监控和日志

### 关键指标
- 连接状态
- 消息接收率
- 错误计数
- 延迟统计

### 日志级别
- `INFO`: 连接状态变化
- `DEBUG`: 消息详情
- `WARN`: 连接问题
- `ERROR`: 严重错误

## 测试

### 单元测试
- 消息解析测试
- 符号转换测试
- 错误处理测试

### 集成测试
- 连接测试
- 订阅测试
- 数据接收测试

## 注意事项

1. **永续合约**: 使用`tBTCF0:USTF0`格式
2. **消息格式**: 数组格式，非JSON对象
3. **心跳处理**: 必须正确处理心跳消息
4. **错误恢复**: 实现完善的错误恢复机制
5. **限流**: 注意API限流要求

## 相关文档

- [Bitfinex WebSocket API官方文档](https://docs.bitfinex.com/v2/reference)
- [永续合约说明](https://docs.bitfinex.com/docs/derivatives)
- [多交易所管理器文档](./MULTI_EXCHANGE_MANAGER.md) 