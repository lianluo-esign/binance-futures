# 币安期货订单流分析系统

基于EventBus架构的高性能币安期货订单流分析工具，使用Rust构建。

## 🚀 特性

- **事件驱动架构**: 基于高性能RingBuffer构建的EventBus系统
- **模块化设计**: 清晰的模块分离，易于扩展和维护
- **高性能**: 优化的数据结构和算法，支持高频数据处理
- **实时分析**: 实时订单流分析、价格速度计算、波动率监控
- **可视化界面**: 基于ratatui的终端UI界面
- **自动重连**: 智能的WebSocket连接管理和错误恢复

## 🏗️ 架构优势

1. **纯事件驱动**: 信号生成通过事件发布，不直接修改状态
2. **解耦合**: 各模块间通过EventBus解耦，提高可维护性
3. **高性能处理**: 基于优化的RingBuffer，支持高频数据处理
4. **统一时间系统**: 使用统一的时间戳保持一致性
5. **可扩展性**: 易于添加新的事件类型和处理器

## 📦 安装

确保你已经安装了Rust (1.70+):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

克隆项目并构建：

```bash
git clone https://github.com/your-username/binance-futures.git
cd binance-futures
cargo build --release
```

## 🚀 使用

### 基本使用

运行默认交易对 (BTCUSDT):
```bash
cargo run
```

指定交易对:
```bash
cargo run ETHUSDT
```

### 操作说明

- `↑/↓`: 滚动订单簿
- `Home/End`: 跳转到首尾
- `Space`: 切换自动滚动
- `q`: 退出程序

## 🧪 测试

运行所有测试：
```bash
cargo test
```

运行特定测试：
```bash
cargo test test_event_bus_basic_functionality
```

## 📊 性能

系统经过优化，支持：
- 高频事件处理 (>10,000 events/sec)
- 低延迟数据处理 (<1ms)
- 内存高效使用
- CPU缓存友好的数据结构

## 🔧 配置

可以通过Config结构体自定义配置：

```rust
let config = Config::new("BTCUSDT".to_string())
    .with_buffer_size(10000)        // 事件缓冲区大小
    .with_max_reconnects(5)         // 最大重连次数
    .with_log_level("info".to_string()); // 日志级别
```

## 📁 项目结构

```
src/
├── core/                   # 核心数据结构
├── events/                # 事件系统
├── handlers/              # 事件处理器
├── orderbook/             # 订单簿管理
├── websocket/             # WebSocket管理
├── app/                   # 应用程序
└── main.rs                # 主程序
```

详细架构说明请参考 [ARCHITECTURE.md](ARCHITECTURE.md)

## 🤝 贡献

欢迎贡献代码！请遵循以下步骤：

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 打开 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## ⚠️ 免责声明

本工具仅用于教育和研究目的。使用本工具进行实际交易的风险由用户自行承担。
6. 内存效率 ：避免在 OrderBook 中存储信号历史
7. UI界面的渲染规则为：UI基于orderbook的数据状态实时渲染，纯数据状态驱动渲染，而不是基于事件流触发。
8. ringbuffer支持充分利用L1/L2缓存的功能

需求1:
实时计算撤单/增加订单功能，每次当前新的depth update的获取，和上一次update的时候的订单薄的数据中的order_flow数据的snapshot 做diff， 如果当前的order_flow的bid变多了 就是增加了挂单，如果bid变少了 就是撤单了，

并且对当前的tick trade价格的同价位的order做减法去除主动订单的消耗得出真正的撤单和增加订单的数量。实时撤单保存在realtime_order_cancel

增加订单保存在realtime_increase_order


需求2:
首先对handle_book_ticker的事件进行处理，增加对bookTicker的快照，也就是每次更新完orderflow之后临时保存一份bookticker的数据到booktickerSnapshot，后面要用到。

当tick trade 被触发，如果是一个主动的buy trade且成交量大于booktickerSnapshot的best ask量，则判定为订单冲击，产生一个订单冲击买入的信号投放到事件缓冲区。 

如果是一个主动的sell trade且成交量大于booktickerSnapshot的best bid量，则判定为订单冲击，产生一个订单冲击卖出的信号。
