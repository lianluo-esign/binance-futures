# 订单簿居中显示修复

## 问题描述

在运行一段时间后，当价格下跌时，当前窗口没有居中best bid，导致向下偏移，看不到bid挂单只有ask挂单。

## 问题分析

1. **根本原因**：原有的居中逻辑仅依赖于`best_bid_price`，但在价格快速下跌时：
   - `best_bid_price`可能滞后于实际市场价格
   - 价格聚合过程中可能导致最优价格的偏移
   - 居中计算没有考虑当前交易价格作为更准确的参考点

2. **具体表现**：
   - 价格下跌后窗口不能及时跟随
   - bid挂单区域被推出可视范围
   - 用户只能看到ask挂单，无法看到完整的市场深度

## 修复方案

### 1. 改进价格参考逻辑

**文件**: `src/gui/orderbook_renderer.rs`

```rust
// 修复前：仅使用best_bid_price
if let Some(best_bid) = render_data.best_bid_price {
    let center_offset = self.price_tracker.calculate_center_offset(best_bid, &price_levels, visible_rows);
}

// 修复后：优先使用当前交易价格
let reference_price = render_data.current_trade_price
    .or(render_data.best_bid_price)
    .or(render_data.best_ask_price);
```

### 2. 增强价格跟踪器

**文件**: `src/gui/price_tracker.rs`

- 提高价格变化敏感度：`center_threshold: 1.0` (从0.1提升到1.0美元)
- 加快响应速度：`min_center_interval: 200ms` (从500ms降低到200ms)
- 改进价格匹配容差：`price_tolerance: 0.5` (从0.001提升到0.5)

### 3. 优化价格匹配算法

```rust
// 对于价格下跌的情况，如果目标价格在两个价格层级之间，
// 优先选择较低的价格层级（更接近实际的bid价格）
if target_price < current_price && target_price > lower_price {
    let distance_to_current = (current_price - target_price).abs();
    let distance_to_lower = (lower_price - target_price).abs();
    
    // 如果距离相近，优先选择较低的价格（对bid更友好）
    if distance_to_lower <= distance_to_current * 1.2 {
        closest_index += 1;
    }
}
```

### 4. 改进订单簿管理器

**文件**: `src/orderbook/manager.rs`

- 添加`recalculate_best_prices()`方法，确保最优价格的准确性
- 在深度更新时重新计算最优价格，而不是简单的增量更新
- 增加价格合理性检查，确保bid < ask的基本市场规则

### 5. 增强价格跟踪逻辑

**文件**: `src/app/reactive_app.rs`

```rust
// 对于价格下跌，使用更敏感的阈值
let threshold = if current_price < last_price {
    self.price_change_threshold * 0.5 // 价格下跌时使用更小的阈值
} else {
    self.price_change_threshold
};
```

## 修复效果

1. **实时跟踪**：窗口现在能够实时跟踪当前交易价格，而不仅仅是best_bid
2. **快速响应**：价格下跌时能够更快地重新居中显示
3. **准确定位**：使用多重价格参考（交易价格 > best_bid > best_ask）确保准确定位
4. **平滑体验**：减少了窗口跳跃，提供更平滑的用户体验

## 测试建议

1. **价格下跌场景**：在价格快速下跌时观察窗口是否能正确居中
2. **价格上涨场景**：确保修复不影响价格上涨时的正常显示
3. **横盘整理场景**：验证在价格波动较小时不会频繁重新居中
4. **极端价格跳跃**：测试价格大幅跳跃时的表现

## 相关文件

- `src/gui/orderbook_renderer.rs` - 主要渲染逻辑
- `src/gui/price_tracker.rs` - 价格跟踪和居中计算
- `src/orderbook/manager.rs` - 订单簿数据管理
- `src/app/reactive_app.rs` - 应用主逻辑和价格跟踪

## 注意事项

1. 修复保持了向后兼容性，不会影响现有功能
2. 所有修改都有详细的日志输出，便于调试
3. 价格容差和阈值可以根据实际使用情况进一步调整