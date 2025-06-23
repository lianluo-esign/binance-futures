# 轴刻度格式化优化更新

## 更新概述

根据用户需求，对时间维度足迹图表的X轴和Y轴进行了重大优化，实现了固定的刻度间距和可读的时间/价格格式化显示，大幅提升了图表的可读性和专业性。

## 核心优化

### 1. X轴时间刻度优化

#### 固定间距设计
- **刻度间距**: 固定1分钟间距
- **网格稳定性**: 避免动态缩放导致的刻度跳跃
- **视觉一致性**: 无论缩放级别如何，时间间距保持固定

#### 时间格式化
- **显示格式**: HH:MM (例如: 20:54, 20:55, 20:56)
- **时区处理**: 使用UTC时间进行统一显示
- **可读性**: 清晰的时分格式，便于快速识别

### 2. Y轴价格刻度优化

#### 固定间距设计
- **刻度间距**: 固定5美元间距
- **价格精度**: 优化显示密度，提升可读性
- **网格稳定性**: 价格刻度线位置固定，不随数据变化

#### 价格格式化
- **显示格式**: 整数价格 (例如: 101480, 101485, 101490)
- **精度控制**: 显示为整数，避免小数点混乱
- **间距优化**: 5美元间距，减少视觉噪音
- **数值清晰**: 直观显示具体价格水平

## 技术实现

### 1. 新增依赖
```toml
chrono = { version = "0.4", features = ["serde"] }
```

### 2. 核心组件

#### 自定义网格间距器
```rust
/// X轴时间网格间距器 - 固定1分钟间距
fn time_grid_spacer(input: GridInput) -> Vec<GridMark> {
    let mut marks = Vec::new();
    let step_size = 1.0; // 1分钟对应1.0单位
    
    let start_minute = (input.bounds.0.floor() as i64).max(0);
    let end_minute = input.bounds.1.ceil() as i64;
    
    for minute in start_minute..=end_minute {
        let value = minute as f64;
        if value >= input.bounds.0 && value <= input.bounds.1 {
            marks.push(GridMark { value, step_size });
        }
    }
    marks
}

/// Y轴价格网格间距器 - 固定5美元间距
fn price_grid_spacer(input: GridInput) -> Vec<GridMark> {
    let mut marks = Vec::new();
    let step_size = 5.0; // 固定5美元间距

    // 计算起始和结束的价格标记，向下和向上取整到5的倍数
    let start_price = ((input.bounds.0 / 5.0).floor() as i64) * 5;
    let end_price = ((input.bounds.1 / 5.0).ceil() as i64) * 5;

    // 生成每5美元的网格标记
    let mut price = start_price;
    while price <= end_price {
        let value = price as f64;
        if value >= input.bounds.0 && value <= input.bounds.1 {
            marks.push(GridMark { value, step_size });
        }
        price += 5; // 每次增加5美元
    }
    marks
}
```

#### 自定义格式化器
```rust
/// X轴时间格式化器 - 显示为 HH:MM 格式
fn format_time_axis(mark: GridMark, _axis_index: usize, _range: &std::ops::RangeInclusive<f64>) -> String {
    let minute_timestamp = (mark.value as u64) * 60000; // 转换为毫秒时间戳
    let datetime = Utc.timestamp_millis_opt(minute_timestamp as i64)
        .single()
        .unwrap_or_else(|| Utc::now());
    datetime.format("%H:%M").to_string()
}

/// Y轴价格格式化器 - 显示为整数价格
fn format_price_axis(mark: GridMark, _axis_index: usize, _range: &std::ops::RangeInclusive<f64>) -> String {
    format!("{:.0}", mark.value)
}
```

### 3. 图表配置集成
```rust
let plot = Plot::new("time_footprint_chart")
    // ... 其他配置
    .x_grid_spacer(Self::time_grid_spacer)
    .x_axis_formatter(Self::format_time_axis)
    .y_grid_spacer(Self::price_grid_spacer)
    .y_axis_formatter(Self::format_price_axis);
```

## 功能特性

### 1. 固定网格系统
- **时间轴**: 每分钟一个刻度，间距恒定
- **价格轴**: 每美元一个刻度，间距恒定
- **缩放稳定**: 缩放时刻度位置保持稳定
- **视觉一致**: 提供稳定的参考框架

### 2. 智能格式化
- **时间显示**: 24小时制 HH:MM 格式
- **价格显示**: 整数格式，无小数点干扰
- **自动适应**: 根据数据范围自动生成合适的刻度
- **性能优化**: 高效的格式化算法

### 3. 用户体验提升
- **可读性**: 清晰的时间和价格标识
- **专业性**: 符合金融图表标准的显示格式
- **一致性**: 固定间距提供稳定的视觉参考
- **直观性**: 便于快速读取时间和价格信息

## 使用效果

### 1. X轴时间显示示例
```
20:54  20:55  20:56  20:57  20:58  20:59  21:00
  |      |      |      |      |      |      |
```

### 2. Y轴价格显示示例
```
101490 ─
101485 ─
101480 ─
101475 ─
101470 ─
```

### 3. 网格稳定性
- 无论如何缩放，时间刻度始终以1分钟为间隔
- 无论如何平移，价格刻度始终以5美元为间隔
- 提供稳定的视觉参考框架，减少视觉噪音

## 性能优化

### 1. 计算效率
- **静态方法**: 避免不必要的self引用
- **整数运算**: 使用整数计算提高性能
- **范围限制**: 只生成可见范围内的刻度

### 2. 内存管理
- **按需生成**: 根据显示范围动态生成刻度
- **无缓存开销**: 不需要额外的缓存机制
- **轻量级**: 最小化内存占用

### 3. 渲染优化
- **固定间距**: 减少重新计算的需要
- **批量处理**: 一次性生成所有刻度
- **高效格式化**: 优化的字符串格式化

## 兼容性

### 1. 向后兼容
- **数据结构**: 不影响现有数据结构
- **API接口**: 保持现有接口不变
- **功能完整**: 所有原有功能正常工作

### 2. 扩展性
- **自定义格式**: 易于修改显示格式
- **间距调整**: 可以轻松调整刻度间距
- **多时区支持**: 可扩展支持不同时区

## 配置选项

### 当前配置
- **时间间距**: 1分钟 (固定)
- **价格间距**: 5美元 (固定)
- **时间格式**: HH:MM (24小时制)
- **价格格式**: 整数显示

### 未来扩展可能
- 支持不同的时间间距 (30秒、5分钟等)
- 支持不同的价格精度 (0.1美元、10美元等)
- 支持本地时区显示
- 支持自定义格式化模板

## 故障排除

### 常见问题
1. **时间显示异常**: 检查系统时间和时区设置
2. **价格格式错误**: 确认数据源的价格精度
3. **刻度密度过高**: 调整显示窗口大小

### 解决方案
- 确保chrono依赖正确安装
- 验证时间戳数据的有效性
- 检查网格间距配置是否合理

这次优化显著提升了时间维度足迹图表的专业性和可读性，为用户提供了更好的数据分析体验。
