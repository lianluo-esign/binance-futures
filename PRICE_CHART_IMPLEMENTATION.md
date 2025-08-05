# 价格图表 Widget 实现总结

## 概述

我已经成功实现了一个新的价格图表 widget，放置在整体布局的最右侧，占据剩余的宽度。这个 widget 使用 ratatui 的线型图展示实时的价格走势。

## 主要特性

### 1. 布局集成
- **位置**: 放置在最右侧，占据剩余宽度（40%）
- **布局**: 四列布局 - 订单薄(20%) + Volume Profile(30%) + 信号(10%) + 价格图表(40%)
- **自适应**: 根据终端大小自动调整

### 2. 数据管理
- **滑动窗口**: 默认支持10000个数据点的滑动窗口
- **实时更新**: 从交易数据中实时获取价格并更新图表
- **数据源**: 使用 `ReactiveApp` 的 `current_price` 作为数据源

### 3. 图表显示
- **X轴**: 代表数据点序号（逐笔成交订单的序列）
- **Y轴**: 价格，刻度间隔为1美元
- **线型图**: 使用 ratatui 的 Chart widget 和 Braille 标记
- **颜色**: 青色线条，白色边框

### 4. 核心功能
- **价格跟踪**: 实时跟踪和显示最新交易价格
- **滑动窗口**: 自动维护固定大小的数据窗口
- **动态刻度**: Y轴刻度根据价格范围自动调整
- **统计信息**: 提供价格范围、平均值、标准差等统计数据

## 文件结构

### 新增文件
- `src/gui/price_chart.rs` - 价格图表渲染器实现
- `tests/price_chart_test.rs` - 价格图表功能测试

### 修改文件
- `src/gui/mod.rs` - 添加价格图表模块导出
- `src/main.rs` - 集成价格图表到主程序布局

## 核心组件

### PriceChartRenderer
```rust
pub struct PriceChartRenderer {
    data_points: VecDeque<PricePoint>,
    max_data_points: usize,
    sequence_counter: u64,
    min_price: f64,
    max_price: f64,
    price_scale_interval: f64,
}
```

### 主要方法
- `new(max_data_points)` - 创建新的图表渲染器
- `add_price_point(price)` - 添加新的价格数据点
- `render(frame, area)` - 渲染图表到指定区域
- `get_stats()` - 获取统计信息
- `clear_data()` - 清空数据

### PricePoint 数据结构
```rust
pub struct PricePoint {
    pub timestamp: u64,
    pub price: f64,
    pub sequence: u64,
}
```

## 集成方式

### 1. 数据更新
```rust
fn update_price_chart(price_chart_renderer: &mut PriceChartRenderer, app: &ReactiveApp) {
    let market_snapshot = app.get_market_snapshot();
    if let Some(current_price) = market_snapshot.current_price {
        price_chart_renderer.add_price_point(current_price);
    }
}
```

### 2. 渲染集成
```rust
fn render_price_chart(f: &mut Frame, price_chart_renderer: &PriceChartRenderer, area: Rect) {
    price_chart_renderer.render(f, area);
}
```

### 3. 布局配置
```rust
let horizontal_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
        Constraint::Percentage(20), // 订单薄
        Constraint::Percentage(30), // Volume Profile
        Constraint::Percentage(10), // 信号
        Constraint::Percentage(40), // 价格图表
    ])
    .split(size);
```

## 测试覆盖

实现了全面的单元测试，包括：
- 基本功能测试
- 滑动窗口测试
- 数据清理测试
- 配置管理测试
- 边界情况测试

所有测试都通过验证，确保功能的稳定性和可靠性。

## 技术特点

### 1. 性能优化
- 使用 `VecDeque` 实现高效的滑动窗口
- 自动管理内存，防止数据积累过多
- 实时更新，无需重新计算历史数据

### 2. 可配置性
- 可调整最大数据点数量
- 可设置价格刻度间隔
- 支持动态配置更新

### 3. 错误处理
- 空数据时显示占位图表
- 价格范围过小时自动调整显示范围
- 边界情况的安全处理

## 使用方式

1. 启动程序后，价格图表会自动显示在最右侧
2. 随着交易数据的到来，图表会实时更新
3. X轴显示数据点序号，Y轴显示价格
4. 图表会自动维护10000个数据点的滑动窗口

这个实现完全满足了需求：在整体布局上创建了一个新的 widget，命名为 price chart，放在最右侧占据剩余宽度，使用 ratatui 的线型图展示实时价格走势，X轴代表滑动窗口的数据点，Y轴为价格，刻度为1美元间隔。