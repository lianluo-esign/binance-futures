# FlowSight - 币安期货订单流分析系统

<div align="center">

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)]()

**专业级实时加密货币订单流分析工具**

*基于 Rust 构建的高性能交易数据可视化应用*

[快速开始](#-快速开始) • [功能特性](#-核心特性) • [文档](#-文档) • [下载](#运行应用)

![FlowSight Screenshot](docs/images/screenshot.png)

</div>

---

## 🎯 项目简介

FlowSight 是一个专为专业交易员设计的**高性能实时订单流分析系统**，采用 Rust 语言开发，具备以下核心优势：

- 🚀 **超低延迟**: 亚毫秒级数据处理，1ms UI 刷新
- 📊 **专业可视化**: 统一订单簿与交易足迹数据展示
- ⚡ **高性能架构**: 事件驱动 + 无锁设计，支持 >10K events/sec
- 🎨 **现代界面**: 基于 egui 的原生桌面应用，支持中文

## ✨ 核心特性

<table>
<tr>
<td width="50%">

### 🎯 专业交易界面
- **实时订单簿**: ±40 价格级别，1美元聚合
- **交易足迹**: 主动买卖量实时监控
- **专业配色**: 蓝色买单/红色卖单
- **自动跟踪**: 价格居中显示
- **响应式布局**: 自适应窗口大小

### ⚡ 高性能架构
- **亚毫秒延迟**: 事件处理 < 1ms
- **高吞吐量**: >10,000 events/sec
- **无锁设计**: Lock-free 数据结构
- **内存优化**: 64字节缓存行对齐
- **CPU 亲和性**: 核心绑定优化

</td>
<td width="50%">

### 📊 智能分析
- **数据聚合**: 智能价格级别合并
- **噪音过滤**: 自动清洗无效订单
- **成交量可视化**: 比例缩放条形图
- **历史足迹**: 5秒窗口累积数据
- **实时监控**: 完整性能指标

### 🔧 技术特性
- **事件驱动**: EventBus 架构
- **自动重连**: 智能连接管理
- **跨平台**: Windows/macOS/Linux
- **模块化**: 易于扩展维护
- **24小时连接**: 符合币安 API 要求

</td>
</tr>
</table>

## 🚀 快速开始

### 📋 系统要求

| 项目 | 最低要求 | 推荐配置 |
|------|----------|----------|
| **操作系统** | Windows 10, macOS 10.15, Ubuntu 18.04 | 最新版本 |
| **Rust** | 1.70+ | 最新稳定版 |
| **内存** | 4GB RAM | 8GB+ RAM |
| **网络** | 稳定互联网连接 | 低延迟连接 |
| **显示** | 1200x800 | 1920x1080+ |

### ⚡ 一键安装

```bash
# 1. 安装 Rust (如果尚未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. 克隆并构建
git clone https://github.com/lianluo-esign/binance-futures.git
cd binance-futures
cargo build --release

# 3. 运行应用
cargo run --release
```

> **Windows 用户**: 请先从 [rustup.rs](https://rustup.rs/) 安装 Rust

### 🎮 运行应用

```bash
# 默认 BTCUSDT 交易对
cargo run --release

# 指定其他交易对
cargo run --release ETHUSDT
cargo run --release ADAUSDT
```

### 📱 界面预览

```
┌─────────────────────────────────────────────────────────────┐
│ FlowSight - BTCUSDT │ ✓连接 │ 15ms │ 1.2K events/s │ 60fps │ ← 状态栏
├─────────────────────────────────────────────────────────────┤
│ 卖单量 │   价格   │ 买单量 │ 主动买量 │ 主动卖量 │           │
│ ████   │ 45,250   │        │   1.2K   │         │           │
│        │ 45,249   │ ██     │          │   0.8K  │           │
│ ██     │ 45,248   │ █████  │   2.1K   │         │ ← 当前价格 │
│        │ 45,247   │ ███    │   0.9K   │         │           │
│ █████  │ 45,246   │        │          │   1.5K  │           │
└─────────────────────────────────────────────────────────────┘
```

**界面说明**:
- 🔴 **卖单量**: 深红色条形图显示卖方流动性
- 🔵 **买单量**: 蓝色条形图显示买方流动性
- 🟢 **主动买量**: 绿色粗体显示主动买入交易
- 🔴 **主动卖量**: 红色粗体显示主动卖出交易
- ⚡ **实时更新**: 1ms 刷新，自动价格跟踪

## 📊 性能表现

<div align="center">

| 指标 | 数值 | 说明 |
|------|------|------|
| **延迟** | < 1ms | P99 端到端处理延迟 |
| **吞吐量** | > 10K/s | 事件处理速度 |
| **内存** | ~50MB | 稳态运行内存占用 |
| **CPU** | < 20% | 单核正常负载 |
| **刷新率** | 60 FPS | 界面更新频率 |

</div>

### 🔧 性能优化特性

- ⚡ **无锁算法**: 避免锁竞争，提升并发性能
- 🧠 **缓存优化**: 64字节对齐，CPU 缓存友好
- 🔄 **批处理**: 高效的批量事件处理
- 💾 **内存池**: 减少动态分配开销
- 🎯 **CPU 亲和性**: 核心绑定优化

## 🧪 测试验证

```bash
# 完整测试套件
cargo test

# 性能基准测试
cargo test --release performance_test

# 集成测试
cargo test integration_test
```

## ⚙️ 配置选项

<details>
<summary><b>🔧 基本配置</b></summary>

```rust
use binance_futures::Config;

let config = Config::new("BTCUSDT".to_string())
    .with_buffer_size(65536)        // 事件缓冲区大小
    .with_max_reconnects(5)         // 最大重连次数
    .with_max_visible_rows(3000)    // UI 最大显示行数
    .with_price_precision(0.01);    // 价格聚合精度
```

</details>

<details>
<summary><b>⚡ 性能优化配置</b></summary>

```rust
// 高性能配置
let config = Config::new("BTCUSDT".to_string())
    .with_buffer_size(65536)        // 更大缓冲区
    .with_cpu_affinity(Some(0))     // CPU 核心绑定
    .with_ui_refresh_rate(1);       // 1ms 刷新率

// 环境变量
export RUST_LOG=warn               // 减少日志输出
export CPU_GOVERNOR=performance    // Linux 性能模式
```

</details>

## 🏗️ 架构设计

<details>
<summary><b>📁 项目结构</b></summary>

```
src/
├── core/                    # 🔧 核心数据结构 (RingBuffer, 无锁实现)
├── events/                  # ⚡ 事件系统 (EventBus, 分发器)
├── handlers/                # 🎯 事件处理器 (市场数据, 交易逻辑)
├── orderbook/               # 📊 订单簿管理 (数据结构, 流分析)
├── websocket/               # 🌐 网络层 (连接管理, 币安API)
├── gui/                     # 🎨 界面组件 (egui, 可视化组件)
├── app/                     # 🚀 应用层 (主逻辑, 状态管理)
└── monitoring/              # 📈 性能监控

tests/                       # 🧪 测试套件
docs/                        # 📚 完整文档
```

**核心组件**:
- `main.rs` - 应用入口和 GUI 初始化
- `ReactiveApp` - 核心应用逻辑和状态管理
- `OrderBookManager` - 订单簿数据管理
- `WebSocketManager` - 币安 API 连接管理

</details>

> 📖 **详细架构**: 查看 [ARCHITECTURE.CN.md](ARCHITECTURE.CN.md) 了解完整的系统设计

## 📚 文档

| 文档 | 描述 | 链接 |
|------|------|------|
| 📖 **文档中心** | 完整文档导航 | [docs/](docs/) |
| 🚀 **安装指南** | 详细安装说明 | [INSTALLATION.md](docs/INSTALLATION.md) |
| 👤 **用户指南** | 界面操作指南 | [USER_GUIDE.md](docs/USER_GUIDE.md) |
| 🔧 **配置指南** | 配置和优化 | [CONFIGURATION.md](docs/CONFIGURATION.md) |
| 🏗️ **架构文档** | 系统架构设计 | [ARCHITECTURE.CN.md](ARCHITECTURE.CN.md) |
| 📝 **更新日志** | 版本更新记录 | [CHANGELOG.md](CHANGELOG.md) |

## 🔧 故障排除

<details>
<summary><b>❓ 常见问题</b></summary>

**Q: 无法连接到币安 API**
A: 检查网络连接，确保可访问 `stream.binance.com`，某些地区需要 VPN

**Q: 中文字体显示异常**
A: 确保系统安装中文字体 (Windows: 微软雅黑, Linux: `sudo apt install fonts-noto-cjk`)

**Q: CPU 占用过高**
A: 降低刷新率或减少显示级别，检查其他高 CPU 进程

**Q: 数据延迟或中断**
A: 检查网络延迟，使用更稳定的网络连接

</details>

<details>
<summary><b>⚡ 性能优化</b></summary>

```bash
# Linux 系统优化
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# 编译优化 (PGO)
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" cargo build --release
RUSTFLAGS="-Cprofile-use=/tmp/pgo-data" cargo build --release

# CPU 亲和性
taskset -c 0 cargo run --release
```

</details>

## 🤝 参与贡献

我们欢迎社区贡献！

<details>
<summary><b>🔧 开发流程</b></summary>

```bash
# 1. Fork 并克隆项目
git clone https://github.com/your-username/binance-futures.git
cd binance-futures

# 2. 创建特性分支
git checkout -b feature/amazing-feature

# 3. 开发和测试
cargo test && cargo fmt && cargo clippy

# 4. 提交并推送
git commit -m 'feat: add amazing feature'
git push origin feature/amazing-feature
```

**代码规范**: 遵循 Rust 官方风格，添加文档注释，编写测试

</details>

**问题报告**: [GitHub Issues](https://github.com/lianluo-esign/binance-futures/issues)
**功能讨论**: [GitHub Discussions](https://github.com/lianluo-esign/binance-futures/discussions)

---

## 📄 许可证与免责声明

<div align="center">

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**本项目采用 MIT 许可证**

⚠️ **重要提示**: 本工具仅用于教育和研究目的
使用本工具进行实际交易的风险由用户自行承担

</div>

## 🙏 致谢

感谢以下优秀的开源项目：

- [egui](https://github.com/emilk/egui) - 即时模式 GUI 框架
- [tungstenite](https://github.com/snapview/tungstenite-rs) - WebSocket 客户端
- [serde](https://github.com/serde-rs/serde) - 序列化框架
- Rust 社区的持续支持

---

<div align="center">

**FlowSight v0.1.0** | **活跃开发中** | **最后更新: 2025-06-23**

[⭐ Star](https://github.com/lianluo-esign/binance-futures) • [🐛 Issues](https://github.com/lianluo-esign/binance-futures/issues) • [💬 Discussions](https://github.com/lianluo-esign/binance-futures/discussions)

</div>
