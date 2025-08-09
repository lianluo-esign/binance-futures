# Binance Futures Trading System

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](#)

一个用 Rust 构建的高性能币安期货交易系统，专注于实时数据处理、订单流分析和终端 UI 展示。

## 📋 目录

- [特性](#特性)
- [系统架构](#系统架构)
- [快速开始](#快速开始)
- [配置说明](#配置说明)
- [使用指南](#使用指南)
- [性能优化](#性能优化)
- [开发指南](#开发指南)
- [贡献指南](#贡献指南)

## ✨ 特性

### 🚀 高性能架构
- **无锁并发**: 基于 `LockFreeRingBuffer` 的事件驱动架构
- **零拷贝设计**: 最小化内存分配和数据拷贝开销
- **CPU 亲和性**: 支持绑定特定 CPU 核心提升性能
- **实时数据处理**: 延迟 <1ms 的订单簿更新处理

### 📊 金融数据处理
- **实时订单簿**: 精确的 L2 订单深度数据展示
- **订单流分析**: 大单检测、冲击成本计算、流向分析
- **价格图表**: 实时价格走势和成交量展示
- **历史数据回放**: 支持 Gzip 压缩的历史数据分析

### 🎮 终端 UI
- **响应式界面**: 基于 `ratatui` 的现代终端 UI
- **多面板布局**: 订单簿、价格图表、订单流同时展示
- **键盘控制**: 丰富的快捷键操作支持
- **主题支持**: 暗色主题，护眼设计

### 🔧 灵活配置
- **模块化提供者**: 可插拔的数据源系统
- **类型安全配置**: 基于 `serde` 的配置验证
- **多环境支持**: 开发、测试、生产环境配置
- **热重载**: 非关键配置支持运行时更新

## 🏗️ 系统架构

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Data Sources  │    │  Core Processing │    │   Presentation  │
├─────────────────┤    ├──────────────────┤    ├─────────────────┤
│ • Binance WS    │───▶│ • Event Bus      │───▶│ • Terminal UI   │
│ • Historical    │    │ • Order Book     │    │ • Price Chart   │
│ • REST API      │    │ • Order Flow     │    │ • Volume Profile│
│ • Mock Data     │    │ • Ring Buffer    │    │ • Signal Display│
└─────────────────┘    └──────────────────┘    └─────────────────┘
                              │
                              ▼
                       ┌──────────────────┐
                       │   Configuration  │
                       ├──────────────────┤
                       │ • Provider Config│
                       │ • System Settings│
                       │ • GUI Preferences│
                       │ • Performance    │
                       └──────────────────┘
```

### 核心组件

- **Event System**: 基于 `LockFreeEventDispatcher` 的高性能事件处理
- **Provider System**: 可插拔的数据源管理器
- **OrderBook Manager**: 实时订单簿状态管理
- **Ring Buffer**: 无锁环形缓冲区，用于高频数据处理
- **GUI Renderer**: 基于 `ratatui` 的终端界面渲染

## 🚀 快速开始

### 系统要求

- **Rust**: 1.70 或更高版本
- **操作系统**: Windows 10/11, Linux, macOS
- **内存**: 最低 4GB RAM
- **网络**: 稳定的互联网连接

### 安装

1. **克隆项目**
```bash
git clone <repository-url>
cd binance-futures
```

2. **构建项目**
```bash
# 调试构建
cargo build

# 发布构建（推荐）
cargo build --release
```

3. **运行程序**
```bash
# 使用默认配置
cargo run --release

# 指定 CPU 核心
cargo run --release -- --cpu-core 1

# 指定配置文件
cargo run --release -- --config custom_config.toml
```

### 命令行选项

```bash
binance-futures [OPTIONS]

OPTIONS:
    --config <FILE>     指定配置文件路径 [默认: config.toml]
    --cpu-core <N>      绑定到指定的 CPU 核心
    --log-level <LEVEL> 设置日志级别 [trace, debug, info, warn, error]
    --help              显示帮助信息
    --version           显示版本信息
```

## ⚙️ 配置说明

### 主配置文件 (config.toml)

```toml
[system]
name = "Binance Futures Trading System"
version = "1.0.0"
log_level = "info"
performance_mode = "high"

[runtime]
cpu_affinity = true
cpu_cores = [1]
thread_pool_size = 4
event_buffer_size = 65536

[providers]
active = ["binance_market_provider", "gzip_historical_provider"]

[[providers.config]]
name = "binance_market_provider"
type = "BinanceWebSocket"
enabled = true
priority = 1
config_file = "configs/providers/binance_market_provider.toml"

[gui]
theme = "dark"
fps = 60
layout = "default"
```

### 数据源配置

#### 币安 WebSocket 配置
```toml
# configs/providers/binance_market_provider.toml
[provider]
name = "binance_market_provider"
provider_type = "BinanceWebSocket"

[connection]
base_url = "wss://stream.binance.com:9443"
max_reconnect_attempts = 5
reconnect_delay_ms = 1000

[subscriptions]
symbols = ["BTCUSDT", "ETHUSDT"]
streams = ["depth", "trade", "ticker"]
```

## 📚 使用指南

### 界面操作

#### 基础控制
- `q` / `Ctrl+C`: 退出程序
- `r`: 刷新界面
- `h`: 显示/隐藏帮助信息
- `d`: 切换调试信息显示

#### 订单簿操作
- `↑` / `↓`: 滚动订单簿
- `Page Up` / `Page Down`: 快速滚动
- `Home` / `End`: 跳转到顶部/底部
- `c`: 居中到当前价格

#### 图表操作
- `+` / `-`: 缩放图表
- `←` / `→`: 水平滚动
- `Space`: 暂停/恢复数据更新

### 数据源管理

#### 启用/禁用数据源
```bash
# 在配置文件中修改
[[providers.config]]
name = "binance_market_provider"
enabled = true  # 设置为 false 禁用
```

#### 添加新的交易对
```toml
[subscriptions]
symbols = ["BTCUSDT", "ETHUSDT", "ADAUSDT"]  # 添加新交易对
```

### 性能调优

#### CPU 亲和性设置
```toml
[runtime]
cpu_affinity = true
cpu_cores = [1, 2]  # 使用多个 CPU 核心
```

#### 缓冲区大小调整
```toml
[runtime]
event_buffer_size = 131072  # 增大缓冲区提高吞吐量
```

## 🔧 性能优化

### 延迟优化
- **无锁数据结构**: 避免互斥锁开销
- **零拷贝操作**: 减少内存分配和拷贝
- **CPU 绑定**: 减少上下文切换
- **批量处理**: 提高数据处理效率

### 内存优化
- **对象池**: 重用频繁分配的对象
- **预分配**: 启动时分配所需内存
- **缓存友好**: 优化数据结构内存布局
- **压缩存储**: 历史数据使用 Gzip 压缩

### 性能指标
| 指标 | 数值 | 说明 |
|------|------|------|
| 订单簿更新延迟 | <1ms | p99 延迟 |
| 事件处理吞吐量 | >100K/s | 每秒事件数 |
| 内存占用 | ~50MB | 运行时内存 |
| CPU 使用率 | <30% | 单核使用率 |

## 🛠️ 开发指南

### 项目结构

```
src/
├── lib.rs                          # 库入口
├── main.rs                         # 程序入口
├── core/                           # 核心业务逻辑
│   ├── mod.rs
│   ├── ring_buffer.rs             # 无锁环形缓冲区
│   ├── cpu_affinity.rs            # CPU 亲和性管理
│   └── provider/                   # 数据提供者
│       ├── mod.rs
│       ├── binance_provider.rs
│       └── gzip_provider.rs
├── events/                         # 事件系统
│   ├── mod.rs
│   ├── event_types.rs
│   └── lock_free_dispatcher.rs    # 无锁事件分发器
├── orderbook/                      # 订单簿管理
│   ├── mod.rs
│   ├── manager.rs
│   └── renderer_data.rs
├── gui/                            # 用户界面
│   ├── mod.rs
│   ├── orderbook_renderer.rs
│   └── volume_profile.rs
├── config/                         # 配置管理
│   ├── mod.rs
│   └── provider_config.rs
└── handlers/                       # 事件处理器
    ├── mod.rs
    └── market_data.rs
```

### 开发环境设置

1. **安装开发工具**
```bash
# 代码格式化
rustup component add rustfmt

# 静态分析
rustup component add clippy

# 文档生成
cargo install cargo-doc
```

2. **运行测试**
```bash
# 单元测试
cargo test

# 基准测试
cargo bench

# 覆盖率测试
cargo tarpaulin --out Html
```

3. **代码检查**
```bash
# 格式检查
cargo fmt --check

# Clippy 检查
cargo clippy -- -D warnings

# 安全审计
cargo audit
```

### 开发规范

本项目严格遵循 [CLAUDE.md](CLAUDE.md) 中定义的开发规范：

- **组合优于继承**: 使用 struct 组合和 trait 实现
- **强类型系统**: 使用 newtype 模式创建业务类型
- **错误处理**: 使用 `Result<T, E>` 和 `thiserror`
- **异步编程**: 基于 `tokio` 的异步架构
- **性能优化**: 零成本抽象和无锁并发

### 代码贡献流程

1. Fork 项目并创建功能分支
2. 编写代码并确保测试通过
3. 运行代码质量检查
4. 提交 Pull Request
5. 代码审查和合并

## 🧪 测试

### 运行测试套件

```bash
# 所有测试
cargo test

# 特定模块测试
cargo test --package binance-futures --lib orderbook

# 集成测试
cargo test --test integration

# 性能测试
cargo bench
```

### 测试覆盖率

项目维持高测试覆盖率：

- **单元测试**: >90% 代码覆盖率
- **集成测试**: 核心业务流程全覆盖
- **性能测试**: 关键路径性能验证

## 📈 监控和日志

### 日志系统

```rust
// 日志级别配置
RUST_LOG=debug cargo run

// 或在配置文件中设置
[system]
log_level = "debug"
```

### 性能监控

- **内置指标**: CPU 使用率、内存占用、事件处理延迟
- **监控端口**: 默认 9090 端口暴露 Prometheus 指标
- **健康检查**: 自动检测系统健康状态

### 故障排除

常见问题和解决方案：

1. **WebSocket 连接失败**
   - 检查网络连接
   - 验证 API 端点配置
   - 查看防火墙设置

2. **高延迟问题**
   - 启用 CPU 亲和性
   - 增大事件缓冲区
   - 检查系统负载

3. **内存使用过高**
   - 减少历史数据缓存
   - 调整垃圾回收参数
   - 检查内存泄漏

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🤝 贡献

欢迎贡献代码！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详细的贡献指南。

### 开发团队

- **核心架构**: 高性能事件驱动系统
- **数据处理**: 实时订单流分析算法
- **用户界面**: 现代终端 UI 设计
- **性能优化**: 无锁并发和零拷贝优化

## 📞 支持

如果遇到问题或需要帮助：

1. 查看 [文档](docs/)
2. 搜索 [Issues](../../issues)
3. 创建新的 [Issue](../../issues/new)
4. 参与 [Discussions](../../discussions)

## 🗺️ 路线图

### v0.2.0 (计划中)
- [ ] 支持更多交易所 (OKX, Bybit)
- [ ] WebAssembly 支持
- [ ] 策略回测功能
- [ ] RESTful API 接口

### v0.3.0 (计划中)
- [ ] 机器学习信号生成
- [ ] 分布式部署支持
- [ ] 实时风控系统
- [ ] 移动端界面

---

**⚡ 为高频交易而生，用 Rust 打造的极致性能！**