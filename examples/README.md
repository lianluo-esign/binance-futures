# 交易所WebSocket示例测试

本目录包含了各个交易所的WebSocket连接示例测试，用于验证各个交易所的WebSocket接口功能。

## 支持的交易所

目前支持以下交易所的WebSocket测试：

- **Bitfinex** - 支持永续合约 (tBTCF0:USTF0)
- **Bitget** - 支持UMCBL合约 (BTCUSDT_UMCBL)
- **Bybit** - 支持线性合约 (BTCUSDT)
- **Coinbase** - 支持现货交易 (BTC-USD)
- **MEXC** - 支持永续合约 (BTC_USDT)
- **OKX** - 支持永续合约 (BTC-USDT-SWAP)
- **Gate.io** - 支持永续合约 (BTCUSDT)

## 运行示例

### 单个交易所测试

运行特定交易所的测试：

```bash
# Bitfinex测试
cargo run --example bitfinex_test

# Bitget测试
cargo run --example bitget_test

# Bybit测试
cargo run --example bybit_test

# Coinbase测试
cargo run --example coinbase_test

# MEXC测试
cargo run --example mexc_test

# OKX测试
cargo run --example okx_test

# Gate.io测试
cargo run --example gateio_improved_test
```

### 多交易所测试

运行多个交易所的综合测试：

```bash
# 无锁线程化多交易所测试（推荐）
cargo run --example lock_free_threaded_multi_exchange_test
```

### 多交易所管理器特性

| 特性 | lock_free_threaded_multi_exchange_test |
|------|----------------------------------------|
| 架构 | 多线程+无锁事件总线 |
| 性能 | 高性能 |
| 延迟 | 低延迟 |
| 并发性 | 真正并发 |
| 适用场景 | 高频交易 |

## 测试功能

每个测试文件都会执行以下操作：

1. **连接WebSocket** - 建立与交易所的WebSocket连接
2. **订阅数据流** - 订阅深度数据、成交数据和ticker数据
3. **接收消息** - 接收并解析WebSocket消息
4. **消息分析** - 分析不同类型的消息并输出统计信息
5. **心跳维护** - 定期发送心跳保持连接
6. **统计显示** - 显示连接统计信息
7. **优雅断开** - 正确关闭WebSocket连接

## 日志配置

测试使用`env_logger`进行日志输出，默认日志级别为`info`。可以通过环境变量调整日志级别：

```bash
# 显示详细调试信息
RUST_LOG=debug cargo run --example bitfinex_test

# 只显示错误信息
RUST_LOG=error cargo run --example bitfinex_test
```

## 测试参数

### 消息数量
每个测试默认接收25条消息后停止。可以修改代码中的`max_messages`变量来调整。

### 心跳间隔
每8条消息发送一次心跳。可以修改代码中的心跳逻辑来调整频率。

### 连接超时
连接建立后等待2秒让连接稳定。可以修改`sleep(Duration::from_secs(2))`来调整。

## 交易所特定配置

### Bitfinex
- 使用永续合约格式：`tBTCF0:USTF0`
- 支持深度、成交和ticker数据
- 使用ping/pong心跳机制

### Bitget
- 使用UMCBL合约格式：`BTCUSDT_UMCBL`
- 支持books、trade和ticker频道
- 使用op/args订阅格式

### Bybit
- 使用线性合约：`BTCUSDT`
- 支持orderbook、publicTrade和tickers
- 需要ExchangeConfig配置

### Coinbase
- 使用现货交易对：`BTC-USD`
- 支持l2update、match和ticker
- 需要ExchangeConfig配置

### MEXC
- 使用永续合约：`BTC_USDT`
- 支持sub.depth和sub.deal
- 使用method/param订阅格式

### OKX
- 使用永续合约：`BTC-USDT-SWAP`
- 支持books5、trades和bbo-tbt
- 需要ExchangeConfig配置

## 故障排除

### 连接失败
- 检查网络连接
- 确认交易所WebSocket地址是否正确
- 查看是否有防火墙阻止连接

### 订阅失败
- 检查交易对符号格式是否正确
- 确认交易所是否支持该交易对
- 查看订阅消息格式是否符合API要求

### 消息解析错误
- 检查消息格式是否发生变化
- 确认JSON解析逻辑是否正确
- 查看交易所API文档更新

## 开发说明

如需添加新的交易所支持：

1. 在`src/websocket/exchanges/`目录下创建新的交易所实现
2. 实现`ExchangeWebSocketManager` trait
3. 在`examples/`目录下创建对应的测试文件
4. 更新本README文档

## 注意事项

- 这些测试仅用于验证WebSocket连接功能
- 不涉及实际交易操作
- 建议在测试环境中运行
- 请遵守各交易所的API使用限制 