# FlowSight 配置指南

本文档详细说明 FlowSight 的各种配置选项和自定义方法。

## 配置概览

FlowSight 支持多种配置方式：
1. **代码配置**: 通过 `Config` 结构体
2. **环境变量**: 运行时环境配置
3. **命令行参数**: 启动时参数
4. **系统优化**: 操作系统级别优化

## 基本配置

### Config 结构体

```rust
use binance_futures::Config;

let config = Config::new("BTCUSDT".to_string())
    .with_buffer_size(10000)        // 事件缓冲区大小
    .with_max_reconnects(5)         // 最大重连次数
    .with_max_visible_rows(3000)    // UI 最大显示行数
    .with_price_precision(0.01);    // 价格聚合精度
```

### 配置参数详解

#### 基础参数

- **symbol**: 交易对符号
  - 默认: `"BTCUSDT"`
  - 支持: 所有币安期货交易对
  - 示例: `"ETHUSDT"`, `"ADAUSDT"`

- **buffer_size**: 事件缓冲区大小
  - 默认: `8192`
  - 范围: `1024` - `65536`
  - 影响: 内存使用和处理能力

- **max_reconnects**: 最大重连次数
  - 默认: `3`
  - 范围: `1` - `10`
  - 影响: 网络故障恢复能力

#### UI 参数

- **max_visible_rows**: 最大显示行数
  - 默认: `2000`
  - 范围: `100` - `5000`
  - 影响: 内存使用和渲染性能

- **price_precision**: 价格聚合精度
  - 默认: `0.01` (1分)
  - 选项: `0.01`, `0.1`, `1.0`
  - 影响: 数据粒度和显示清晰度

- **ui_refresh_rate**: UI 刷新间隔（毫秒）
  - 默认: `16` (约60 FPS)
  - 范围: `1` - `100`
  - 影响: 界面流畅度和 CPU 使用

#### 性能参数

- **cpu_affinity**: CPU 核心绑定
  - 默认: `None` (自动)
  - 选项: `Some(0)`, `Some(1)`, etc.
  - 影响: 性能一致性

- **enable_batch_processing**: 批处理模式
  - 默认: `true`
  - 影响: 吞吐量和延迟

## 环境变量配置

### 日志配置

```bash
# 设置日志级别
export RUST_LOG=info          # debug, info, warn, error
export RUST_LOG_STYLE=always  # 彩色输出

# 禁用控制台输出
export FLOWSIGHT_QUIET=1
```

### 性能配置

```bash
# CPU 性能模式 (Linux)
export CPU_GOVERNOR=performance

# 内存配置
export MALLOC_ARENA_MAX=2      # 限制内存分配器线程数
export MALLOC_MMAP_THRESHOLD_=131072  # 大内存分配阈值
```

### 网络配置

```bash
# WebSocket 配置
export WS_CONNECT_TIMEOUT=10   # 连接超时（秒）
export WS_READ_TIMEOUT=30      # 读取超时（秒）
export WS_PING_INTERVAL=20     # 心跳间隔（秒）
```

## 高级配置

### 自定义配置文件

创建 `config.toml` 文件：

```toml
[application]
symbol = "BTCUSDT"
log_level = "info"

[performance]
buffer_size = 16384
max_visible_rows = 3000
ui_refresh_rate = 1
enable_batch_processing = true

[network]
max_reconnects = 5
connect_timeout = 10
read_timeout = 30

[ui]
price_precision = 0.01
window_width = 1400
window_height = 900
theme = "dark"
```

### 编程配置示例

```rust
// 高性能配置
let high_perf_config = Config::new("BTCUSDT".to_string())
    .with_buffer_size(32768)
    .with_ui_refresh_rate(1)        // 1ms 刷新
    .with_cpu_affinity(Some(0))     // 绑定到核心 0
    .with_batch_processing(true);

// 低资源配置
let low_resource_config = Config::new("BTCUSDT".to_string())
    .with_buffer_size(4096)
    .with_ui_refresh_rate(50)       // 20 FPS
    .with_max_visible_rows(1000);

// 调试配置
let debug_config = Config::new("BTCUSDT".to_string())
    .with_log_level("debug")
    .with_enable_debug_window(true)
    .with_performance_monitoring(true);
```

## 系统级优化

### Linux 优化

```bash
#!/bin/bash
# performance_setup.sh

# CPU 性能模式
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# 禁用透明大页
echo never | sudo tee /sys/kernel/mm/transparent_hugepage/enabled

# 网络优化
echo 'net.core.rmem_max = 16777216' | sudo tee -a /etc/sysctl.conf
echo 'net.core.wmem_max = 16777216' | sudo tee -a /etc/sysctl.conf
echo 'net.ipv4.tcp_rmem = 4096 65536 16777216' | sudo tee -a /etc/sysctl.conf
echo 'net.ipv4.tcp_wmem = 4096 65536 16777216' | sudo tee -a /etc/sysctl.conf

# 应用设置
sudo sysctl -p

# 设置进程优先级
echo 'flowsight soft rtprio 99' | sudo tee -a /etc/security/limits.conf
echo 'flowsight hard rtprio 99' | sudo tee -a /etc/security/limits.conf
```

### Windows 优化

```powershell
# PowerShell 脚本 (以管理员身份运行)

# 设置高性能电源计划
powercfg -setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c

# 禁用 Windows Defender 实时保护（可选）
Set-MpPreference -DisableRealtimeMonitoring $true

# 设置网络优化
netsh int tcp set global autotuninglevel=normal
netsh int tcp set global chimney=enabled
netsh int tcp set global rss=enabled
```

## 监控和调试

### 性能监控配置

```rust
let config = Config::new("BTCUSDT".to_string())
    .with_performance_monitoring(true)
    .with_metrics_interval(1000)    // 1秒统计间隔
    .with_enable_profiling(true);
```

### 调试配置

```bash
# 启用详细日志
export RUST_LOG=binance_futures=debug

# 启用性能分析
export FLOWSIGHT_PROFILE=1

# 启用内存调试
export RUST_BACKTRACE=1
```

## 配置验证

### 配置检查工具

```rust
// 验证配置有效性
fn validate_config(config: &Config) -> Result<(), String> {
    if config.buffer_size < 1024 {
        return Err("Buffer size too small".to_string());
    }
    
    if config.ui_refresh_rate < 1 {
        return Err("Refresh rate too low".to_string());
    }
    
    // 更多验证...
    Ok(())
}
```

### 性能基准测试

```bash
# 运行性能测试
cargo test --release performance_test

# 基准测试
cargo bench

# 内存使用分析
valgrind --tool=massif cargo run --release
```

## 故障排除

### 配置问题诊断

1. **检查配置语法**
   ```bash
   cargo check
   ```

2. **验证环境变量**
   ```bash
   env | grep FLOWSIGHT
   env | grep RUST_LOG
   ```

3. **测试网络连接**
   ```bash
   ping stream.binance.com
   telnet stream.binance.com 9443
   ```

### 常见配置错误

- **缓冲区过小**: 导致数据丢失
- **刷新率过高**: 导致 CPU 占用过高
- **网络超时过短**: 导致频繁重连
- **日志级别过低**: 影响调试能力

## 最佳实践

1. **生产环境**
   - 使用发布构建 (`--release`)
   - 设置适当的缓冲区大小
   - 启用批处理模式
   - 绑定 CPU 核心

2. **开发环境**
   - 启用调试日志
   - 使用较小的缓冲区
   - 启用性能监控

3. **测试环境**
   - 使用模拟数据
   - 启用所有监控
   - 记录性能指标

## 配置模板

### 生产配置

```rust
Config::new("BTCUSDT".to_string())
    .with_buffer_size(16384)
    .with_max_reconnects(5)
    .with_ui_refresh_rate(16)
    .with_cpu_affinity(Some(0))
    .with_log_level("warn")
```

### 开发配置

```rust
Config::new("BTCUSDT".to_string())
    .with_buffer_size(8192)
    .with_max_reconnects(3)
    .with_ui_refresh_rate(33)
    .with_log_level("debug")
    .with_enable_debug_window(true)
```
