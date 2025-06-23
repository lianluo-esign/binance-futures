# FlowSight - 币安期货订单流分析系统

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

FlowSight 是一个基于 Rust 构建的高性能实时加密货币交易分析应用，专为需要超低延迟市场数据处理和可视化的专业交易员设计。系统采用事件驱动架构和无锁数据结构，实现亚毫秒级处理时间。

![FlowSight Screenshot](docs/images/screenshot.png)

## ✨ 核心特性

### 🎯 专业交易界面
- **现代GUI界面**: 基于 egui 框架的原生桌面应用，支持中文字体
- **实时订单簿**: BTCUSDT 实时深度数据，±40 价格级别显示
- **统一数据视图**: 订单簿深度与交易足迹数据融合展示
- **1毫秒刷新**: 超低延迟的实时数据更新
- **专业配色**: 蓝色买单/深红色卖单的专业交易配色方案
- **自适应布局**: 响应式设计，充分利用窗口空间

### ⚡ 高性能架构
- **事件驱动系统**: 基于高性能 RingBuffer 的 EventBus 架构
- **无锁设计**: Lock-free 数据结构，支持高频数据处理
- **内存优化**: 缓存行对齐（64字节），CPU 缓存预取优化
- **亚毫秒延迟**: 事件处理延迟 < 1ms
- **高吞吐量**: 支持 >10,000 events/sec 的数据处理
- **CPU 亲和性**: 支持核心绑定优化性能

### 📊 智能数据分析
- **价格级别聚合**: 1美元价格级别智能聚合，减少噪音
- **主动交易监控**: 实时区分主动买入/卖出交易
- **历史足迹分析**: 5秒窗口累积交易数据
- **成交量可视化**: 水平条形图展示订单数量，比例缩放
- **自动价格跟踪**: 智能滚动跟随当前价格，保持居中显示
- **数据清洗**: 自动过滤无效订单，基于最佳买卖价

### 🔧 技术优势
- **模块化设计**: 清晰的模块分离，易于扩展和维护
- **自动重连**: 智能的 WebSocket 连接管理和错误恢复
- **24小时连接管理**: 符合币安 API 要求的连接生命周期
- **实时监控**: 完整的性能指标和健康监控
- **跨平台支持**: Windows、macOS、Linux 原生支持

## 🚀 快速开始

### 系统要求

- **操作系统**: Windows 10+, macOS 10.15+, Linux (Ubuntu 18.04+)
- **Rust版本**: 1.70 或更高版本
- **内存**: 最少 4GB RAM（推荐 8GB+）
- **CPU**: 现代多核处理器（支持 CPU 亲和性优化）
- **网络**: 稳定的互联网连接（访问币安 API）
- **显示**: 支持 1200x800 或更高分辨率

### 安装步骤

1. **安装 Rust 开发环境**
```bash
# Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Windows (PowerShell)
# 下载并运行 https://rustup.rs/ 提供的安装程序
```

2. **克隆项目**
```bash
git clone https://github.com/lianluo-esign/binance-futures.git
cd binance-futures
```

3. **构建项目**
```bash
# 开发构建（包含调试信息）
cargo build

# 优化发布构建（推荐用于生产）
cargo build --release

# 运行测试确保一切正常
cargo test
```

### 运行应用

**启动 FlowSight GUI 应用**
```bash
# 默认交易对 (BTCUSDT)
cargo run --release

# 指定其他交易对
cargo run --release ETHUSDT
cargo run --release ADAUSDT
```

**命令行参数**
- 第一个参数：交易对符号（默认：BTCUSDT）
- 支持所有币安期货交易对

### 应用界面说明

#### 主界面布局
- **顶部状态栏 (5%)**: 显示连接状态、性能指标和应用信息
- **主表格区域 (95%)**: 统一的订单流分析表格

#### 表格列说明
1. **卖单列**: 显示卖方订单数量，深红色背景条
2. **价格列**: 当前价格级别，1美元聚合
3. **买单列**: 显示买方订单数量，蓝色背景条
4. **主动买量**: 主动买入交易量，绿色粗体显示
5. **主动卖量**: 主动卖出交易量，红色粗体显示

#### 实时功能
- **自动价格跟踪**: 自动滚动跟随当前价格，保持居中
- **实时数据更新**: 1毫秒刷新间隔
- **动态价格级别**: 维持 ±40 价格级别显示
- **成交量可视化**: 水平条形图按比例显示订单量

### GUI 操作指南

- **自动跟踪**: 应用自动跟随当前价格滚动
- **手动滚动**: 可以手动滚动查看其他价格级别
- **连接监控**: 顶部显示 WebSocket 连接状态
- **性能指标**: 实时显示事件处理速度和延迟
- **窗口调整**: 支持窗口大小调整，布局自适应

## 🧪 测试

### 运行测试套件

```bash
# 运行所有测试
cargo test

# 运行特定测试模块
cargo test test_event_bus_basic_functionality
cargo test integration_test
cargo test performance_test

# 运行测试并显示输出
cargo test -- --nocapture

# 运行性能基准测试
cargo test --release performance_test
```

### 测试覆盖
- **单元测试**: 核心组件功能测试
- **集成测试**: 端到端系统测试
- **性能测试**: 延迟和吞吐量基准测试
- **错误处理测试**: 网络中断和恢复测试

## 📊 性能指标

### 系统性能
- **事件处理吞吐量**: >10,000 events/sec
- **端到端延迟**: <1ms (P99)
- **内存使用**: ~50MB 稳态运行
- **CPU 使用**: 单核 <20% (正常负载)
- **网络延迟**: 取决于到币安服务器的网络延迟

### 优化特性
- **缓存行对齐**: 64字节对齐的关键数据结构
- **CPU 缓存预取**: 可预测访问模式的优化
- **无锁算法**: 避免锁竞争和上下文切换
- **批处理**: 高效的批量事件处理
- **内存池**: 减少动态内存分配

## 🔧 配置选项

### 基本配置

通过 `Config` 结构体自定义应用行为：

```rust
use binance_futures::Config;

let config = Config::new("BTCUSDT".to_string())
    .with_buffer_size(10000)        // 事件缓冲区大小
    .with_max_reconnects(5)         // 最大重连次数
    .with_max_visible_rows(3000)    // UI 最大显示行数
    .with_price_precision(0.01);    // 价格聚合精度 (1分)
```

### 高级配置

```rust
// 性能优化配置
let config = Config::new("BTCUSDT".to_string())
    .with_buffer_size(20000)        // 更大缓冲区用于高频交易
    .with_cpu_affinity(Some(0))     // 绑定到特定 CPU 核心
    .with_log_level("warn")         // 减少日志输出
    .with_ui_refresh_rate(1);       // 1ms UI 刷新率
```

### 环境变量

```bash
# 设置日志级别
export RUST_LOG=info

# 设置 CPU 性能模式 (Linux)
export CPU_GOVERNOR=performance

# 禁用调试输出
export FLOWSIGHT_QUIET=1
```

## 📁 项目结构

```
src/
├── core/                          # 核心数据结构
│   ├── mod.rs                     # 模块导出
│   ├── ring_buffer.rs             # 传统环形缓冲区
│   └── lock_free_ring_buffer.rs   # 无锁实现
├── events/                        # 事件系统
│   ├── mod.rs                     # 事件系统导出
│   ├── event_types.rs             # 事件定义
│   ├── event_bus.rs               # EventBus 实现
│   ├── dispatcher.rs              # 事件分发器
│   ├── lock_free_dispatcher.rs    # 无锁分发器
│   └── lock_free_event_bus.rs     # 无锁事件总线
├── handlers/                      # 事件处理器
│   ├── mod.rs                     # 处理器导出
│   ├── market_data.rs             # 市场数据处理
│   ├── trading.rs                 # 交易事件处理
│   ├── errors.rs                  # 错误处理
│   ├── signals.rs                 # 信号处理
│   └── global.rs                  # 全局事件监控
├── orderbook/                     # 订单簿管理
│   ├── mod.rs                     # 订单簿导出
│   ├── manager.rs                 # OrderBookManager
│   ├── data_structures.rs         # 数据类型
│   └── order_flow.rs              # 订单流分析
├── websocket/                     # WebSocket 层
│   ├── mod.rs                     # WebSocket 导出
│   ├── manager.rs                 # WebSocketManager
│   └── connection.rs              # 连接处理
├── gui/                           # GUI 组件
│   ├── mod.rs                     # GUI 导出
│   ├── egui_app.rs                # 主应用程序
│   ├── unified_orderbook_widget.rs # 统一订单簿显示
│   ├── orderbook_widget.rs        # 传统组件
│   ├── trade_footprint_widget.rs  # 交易足迹组件
│   └── debug_window.rs            # 调试界面
├── app/                           # 应用层
│   ├── mod.rs                     # 应用导出
│   ├── reactive_app.rs            # 主应用逻辑
│   └── ui.rs                      # UI 逻辑
├── monitoring/                    # 性能监控
│   └── mod.rs                     # 监控系统
├── image/                         # 应用资源
│   ├── logo.png                   # 应用图标
│   └── ...                       # 其他图标文件
├── lib.rs                         # 库接口
└── main.rs                        # 应用入口点

tests/
├── integration_test.rs            # 集成测试
└── performance_test.rs            # 性能测试

docs/
├── ARCHITECTURE.CN.md             # 中文架构文档
├── architecture.md                # 英文架构文档
├── PRD.CN.md                      # 中文产品需求文档
└── PRD.md                         # 英文产品需求文档
```

### 关键文件说明

- **main.rs**: 应用程序入口，GUI 初始化和配置
- **lib.rs**: 库接口，导出主要 API
- **Config**: 应用配置管理
- **TradingGUI**: 主 GUI 应用类
- **ReactiveApp**: 核心应用逻辑和状态管理

详细架构说明请参考 [ARCHITECTURE.CN.md](ARCHITECTURE.CN.md)

## � 文档

### 完整文档
- [📖 文档中心](docs/) - 完整的文档导航
- [🚀 安装指南](docs/INSTALLATION.md) - 详细安装说明
- [👤 用户指南](docs/USER_GUIDE.md) - 界面操作指南
- [🔧 配置指南](docs/CONFIGURATION.md) - 配置和优化
- [🏗️ 架构文档](ARCHITECTURE.CN.md) - 系统架构设计

### 快速链接
- [更新日志](CHANGELOG.md) - 版本更新记录
- [许可证](LICENSE) - MIT 许可证
- [问题报告](https://github.com/lianluo-esign/binance-futures/issues) - Bug 报告和功能请求

## �🔧 故障排除

### 常见问题

**Q: 应用启动时显示"无法连接到币安 API"**
A: 检查网络连接，确保可以访问 `stream.binance.com`。某些地区可能需要 VPN。

**Q: GUI 界面显示乱码或中文字体不正确**
A: 确保系统安装了中文字体（Windows: 微软雅黑，macOS/Linux: 需要安装中文字体包）。

**Q: 应用占用 CPU 过高**
A: 尝试降低刷新率或减少显示的价格级别数量。检查是否有其他高 CPU 使用的进程。

**Q: 数据更新延迟或不连续**
A: 检查网络延迟到币安服务器。考虑使用更接近币安服务器的网络连接。

### 性能优化建议

1. **系统配置**
```bash
# Linux 系统优化
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
echo never | sudo tee /sys/kernel/mm/transparent_hugepage/enabled
```

2. **编译优化**
```bash
# 使用 PGO (Profile-Guided Optimization)
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" cargo build --release
# 运行应用生成配置文件数据
RUSTFLAGS="-Cprofile-use=/tmp/pgo-data" cargo build --release
```

3. **运行时优化**
```bash
# 设置 CPU 亲和性
taskset -c 0 cargo run --release
```

## 🤝 贡献指南

我们欢迎社区贡献！请遵循以下步骤：

### 开发流程

1. **Fork 项目**
   ```bash
   git clone https://github.com/your-username/binance-futures.git
   cd binance-futures
   ```

2. **创建特性分支**
   ```bash
   git checkout -b feature/amazing-feature
   ```

3. **开发和测试**
   ```bash
   # 运行测试确保没有破坏现有功能
   cargo test

   # 运行格式化
   cargo fmt

   # 运行 linting
   cargo clippy
   ```

4. **提交更改**
   ```bash
   git commit -m 'feat: add amazing feature'
   ```

5. **推送并创建 PR**
   ```bash
   git push origin feature/amazing-feature
   ```

### 代码规范

- 遵循 Rust 官方代码风格
- 添加适当的文档注释
- 为新功能编写测试
- 保持提交信息清晰明确

### 报告问题

请使用 GitHub Issues 报告 bug 或请求新功能：
- 提供详细的错误描述
- 包含系统信息和日志
- 提供重现步骤

## 📄 许可证

本项目采用 MIT 许可证。详情请查看 [LICENSE](LICENSE) 文件。

## ⚠️ 免责声明

**重要提示**: 本工具仅用于教育和研究目的。

- 本软件按"原样"提供，不提供任何明示或暗示的保证
- 使用本工具进行实际交易的风险由用户自行承担
- 开发者不对因使用本软件而导致的任何损失负责
- 请在充分了解风险的情况下使用，建议先在模拟环境中测试

## 📞 支持与联系

- **GitHub Issues**: [报告问题或请求功能](https://github.com/lianluo-esign/binance-futures/issues)
- **讨论**: [GitHub Discussions](https://github.com/lianluo-esign/binance-futures/discussions)
- **文档**: [完整文档](docs/)

## 🙏 致谢

感谢以下开源项目和社区：

- [egui](https://github.com/emilk/egui) - 优秀的即时模式 GUI 框架
- [tungstenite](https://github.com/snapview/tungstenite-rs) - WebSocket 客户端库
- [serde](https://github.com/serde-rs/serde) - 序列化框架
- Rust 社区的持续支持和贡献

---

**项目状态**: 活跃开发中
**版本**: 0.1.0
**最后更新**: 2025-06-23
