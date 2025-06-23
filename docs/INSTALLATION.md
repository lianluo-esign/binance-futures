# FlowSight 安装指南

本文档提供 FlowSight 的详细安装和配置说明。

## 系统要求

### 最低要求
- **操作系统**: Windows 10, macOS 10.15, Ubuntu 18.04 或更高版本
- **内存**: 4GB RAM
- **存储**: 500MB 可用空间
- **网络**: 稳定的互联网连接

### 推荐配置
- **内存**: 8GB+ RAM
- **CPU**: 4核心以上现代处理器
- **网络**: 低延迟连接到币安服务器
- **显示**: 1920x1080 或更高分辨率

## 安装 Rust

### Windows

1. 访问 [rustup.rs](https://rustup.rs/)
2. 下载并运行 `rustup-init.exe`
3. 按照安装向导完成安装
4. 重启命令提示符或 PowerShell

### macOS

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Linux (Ubuntu/Debian)

```bash
# 安装必要的依赖
sudo apt update
sudo apt install curl build-essential

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 安装 GUI 依赖
sudo apt install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

### 验证安装

```bash
rustc --version
cargo --version
```

## 获取源代码

```bash
git clone https://github.com/lianluo-esign/binance-futures.git
cd binance-futures
```

## 构建应用

### 开发构建

```bash
cargo build
```

### 发布构建（推荐）

```bash
cargo build --release
```

### 验证构建

```bash
cargo test
```

## 首次运行

```bash
# 使用默认配置运行
cargo run --release

# 指定交易对
cargo run --release ETHUSDT
```

## 系统优化（可选）

### Linux 性能优化

```bash
# 设置 CPU 性能模式
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# 禁用透明大页
echo never | sudo tee /sys/kernel/mm/transparent_hugepage/enabled

# 增加网络缓冲区
echo 'net.core.rmem_max = 16777216' | sudo tee -a /etc/sysctl.conf
echo 'net.core.wmem_max = 16777216' | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

### Windows 性能优化

1. 设置高性能电源计划
2. 关闭不必要的后台应用
3. 确保防火墙允许应用网络访问

## 故障排除

### 编译错误

**错误**: `linker 'cc' not found`
**解决**: 安装 C 编译器
```bash
# Ubuntu/Debian
sudo apt install build-essential

# CentOS/RHEL
sudo yum groupinstall "Development Tools"

# macOS
xcode-select --install
```

**错误**: `failed to run custom build command for openssl-sys`
**解决**: 安装 OpenSSL 开发库
```bash
# Ubuntu/Debian
sudo apt install libssl-dev pkg-config

# CentOS/RHEL
sudo yum install openssl-devel

# macOS
brew install openssl
```

### 运行时错误

**错误**: 网络连接失败
**解决**: 
1. 检查网络连接
2. 确认可以访问 `stream.binance.com`
3. 检查防火墙设置
4. 考虑使用 VPN（某些地区）

**错误**: 字体显示问题
**解决**: 安装中文字体
```bash
# Ubuntu/Debian
sudo apt install fonts-noto-cjk

# CentOS/RHEL
sudo yum install google-noto-cjk-fonts
```

## 下一步

安装完成后，请参考：
- [用户指南](USER_GUIDE.md) - 学习如何使用应用
- [配置指南](CONFIGURATION.md) - 自定义应用设置
- [架构文档](../ARCHITECTURE.CN.md) - 了解系统架构
