# 多交易所WebSocket管理器使用指南

## 概述

多交易所管理器(`MultiExchangeManager`)是一个统一管理多个交易所WebSocket连接的组件，支持同时连接到多个交易所并处理它们的市场数据。

## 主要特性

- **统一接口**: 通过单一接口管理多个交易所连接
- **自动重连**: 支持连接断开时的自动重连机制
- **健康监控**: 实时监控各交易所连接状态
- **事件驱动**: 与事件总线集成，支持异步消息处理
- **配置灵活**: 支持动态添加/移除交易所
- **统计信息**: 提供详细的连接和消息统计

## 支持的交易所

当前支持的交易所：
- ✅ OKX
- ⬜ Binance (待实现)
- ⬜ Bybit (待实现)
- ⬜ Coinbase (待实现)
- ⬜ Bitget (待实现)
- ⬜ Bitfinex (待实现)
- ⬜ Gate.io (待实现)
- ⬜ MEXC (待实现)

## 基本使用

### 1. 创建事件总线

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use flow_sight::events::event_bus::EventBus;

let event_bus = Arc::new(RwLock::new(EventBus::new(1000)));
```

### 2. 配置交易所

```rust
use flow_sight::websocket::{ExchangeConfig, ExchangeType};

let okx_config = ExchangeConfig {
    exchange_name: "OKX".to_string(),
    symbol: "BTCUSDT".to_string(),
    testnet: false,
    api_key: None,
    api_secret: None,
};
```

### 3. 创建管理器

```rust
use flow_sight::websocket::MultiExchangeManagerBuilder;

let mut manager = MultiExchangeManagerBuilder::new()
    .with_exchanges(vec![ExchangeType::Okx])
    .with_exchange_config(ExchangeType::Okx, okx_config)
    .with_auto_reconnect(true)
    .with_reconnect_interval(5)
    .with_max_reconnect_attempts(3)
    .build(event_bus.clone());
```

### 4. 初始化和启动

```rust
// 初始化管理器
manager.initialize().await?;

// 启动连接
manager.start().await?;

// 订阅BTCUSDT永续合约数据
manager.subscribe_btcusdt_perpetual().await?;
```

### 5. 处理消息

```rust
// 在主循环中处理消息
loop {
    // 处理消息
    if let Err(e) = manager.process_messages().await {
        eprintln!("处理消息时出错: {}", e);
    }
    
    // 检查连接健康状态
    let unhealthy = manager.check_health().await;
    if !unhealthy.is_empty() {
        println!("发现不健康的交易所连接: {:?}", unhealthy);
        manager.reconnect_all().await?;
    }
    
    // 短暂休眠
    tokio::time::sleep(Duration::from_millis(100)).await;
}
```

### 6. 停止管理器

```rust
manager.stop().await?;
```

## 高级功能

### 获取统计信息

```rust
let stats = manager.get_stats().await;
println!("总连接数: {}", stats.total_connections);
println!("活跃连接数: {}", stats.active_connections);
println!("总消息数: {}", stats.total_messages);
println!("总错误数: {}", stats.total_errors);
```

### 获取连接状态

```rust
let states = manager.get_connection_states().await;
for (exchange, state) in states {
    println!("{} 连接状态: {:?}", exchange.name(), state);
}
```

### 重连特定交易所

```rust
manager.reconnect_exchange(ExchangeType::Okx).await?;
```

### 动态添加交易所

```rust
let new_config = ExchangeConfig {
    exchange_name: "Binance".to_string(),
    symbol: "BTCUSDT".to_string(),
    testnet: false,
    api_key: None,
    api_secret: None,
};

manager.add_exchange_config(ExchangeType::Binance, new_config);
```

### 移除交易所

```rust
manager.remove_exchange(ExchangeType::Okx).await?;
```

## 配置选项

### MultiExchangeConfig

- `enabled_exchanges`: 启用的交易所列表
- `exchange_configs`: 各交易所的配置
- `auto_reconnect`: 是否启用自动重连
- `reconnect_interval`: 重连间隔（秒）
- `max_reconnect_attempts`: 最大重连次数

### ExchangeConfig

- `exchange_name`: 交易所名称
- `symbol`: 交易对符号
- `testnet`: 是否使用测试网
- `api_key`: API密钥（可选）
- `api_secret`: API密钥（可选）

## 错误处理

管理器提供了多层错误处理：

1. **连接级错误**: 单个交易所连接失败不会影响其他交易所
2. **自动重连**: 连接断开时自动尝试重连
3. **健康检查**: 定期检查连接状态
4. **错误统计**: 记录各种错误类型的统计信息

## 事件集成

管理器与事件总线紧密集成：

- 接收到的市场数据会发送到事件总线
- 支持事件驱动的消息处理
- 可以订阅特定类型的事件

## 性能优化

- 异步处理: 所有操作都是异步的
- 并发连接: 支持同时连接多个交易所
- 内存优化: 合理管理消息缓冲区
- 网络优化: 支持心跳和断线重连

## 示例程序

完整的示例程序请参考 `examples/multi_exchange_test.rs`。

## 注意事项

1. 确保网络连接稳定
2. 合理设置重连参数
3. 监控内存使用情况
4. 处理API限流
5. 妥善保管API密钥

## 故障排除

### 连接失败
- 检查网络连接
- 验证交易所API地址
- 确认防火墙设置

### 重连频繁
- 调整重连间隔
- 检查网络稳定性
- 验证API限流设置

### 内存使用过高
- 调整事件总线容量
- 检查消息处理速度
- 优化数据结构

## 后续开发

计划添加的功能：
- 更多交易所支持
- 数据分层管理
- 高级过滤功能
- 性能监控面板
- 配置热更新 