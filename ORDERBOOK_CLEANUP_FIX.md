# 订单薄数据清理功能修复

## 问题描述

GUI界面上出现了一个小问题：1美元精度聚合的深度订单薄数据超过5秒没有更新时没有被清除。

## 解决方案

### 1. 修改 `OrderBookManager::cleanup_expired_data` 方法

在 `src/orderbook/manager.rs` 文件中，为订单薄管理器的清理方法添加了对挂单数据的过期检查：

```rust
/// 定期清理过期数据
pub fn cleanup_expired_data(&mut self) {
    let current_time = self.get_current_timestamp();

    // 清理5秒内的实时交易数据
    for (_, order_flow) in self.order_flows.iter_mut() {
        order_flow.clean_expired_trades(current_time, 5000); // 5秒
        order_flow.clean_expired_cancels(current_time, 5000); // 5秒
        order_flow.clean_expired_increases(current_time, 5000); // 5秒
        
        // 新增：清理超过5秒没有更新的挂单数据（1美元精度聚合的深度订单薄数据）
        order_flow.clean_expired_price_levels(current_time, 5000); // 5秒
    }

    // 清理空的或过期的订单流条目
    self.order_flows.retain(|_, order_flow| {
        // 保留有挂单数据或最近有活动的条目
        order_flow.bid_ask.bid > 0.0 ||
        order_flow.bid_ask.ask > 0.0 ||
        order_flow.has_recent_activity(current_time, 60000) // 保留60秒内有活动的
    });
}
```

### 2. 添加 `OrderFlow::clean_expired_price_levels` 方法

在 `src/orderbook/order_flow.rs` 文件中，为订单流结构体添加了新的清理方法：

```rust
/// 清理超过指定时间没有更新的挂单数据（价格层级数据）
pub fn clean_expired_price_levels(&mut self, current_time: u64, max_age: u64) {
    // 检查挂单数据的时间戳，如果超过max_age（5秒）没有更新，则清除
    if current_time.saturating_sub(self.bid_ask.timestamp) > max_age {
        // 清除过期的挂单数据
        self.bid_ask.bid = 0.0;
        self.bid_ask.ask = 0.0;
        // 注意：不重置timestamp，保持原有时间戳用于后续判断
    }
}
```

## 功能特点

1. **精确的时间控制**：超过5秒没有更新的挂单数据会被自动清除
2. **保持时间戳**：清理后保持原有时间戳，用于后续的过期判断
3. **不影响其他数据**：只清理挂单数据（bid/ask），不影响交易记录等其他数据
4. **自动调用**：通过应用程序的事件循环自动调用，每秒执行一次清理

## 测试验证

创建了完整的测试套件来验证功能：

1. `test_orderbook_data_cleanup_after_5_seconds` - 验证超过5秒的数据被清理
2. `test_orderbook_data_not_cleaned_within_5_seconds` - 验证5秒内的数据不被清理
3. `test_order_flow_clean_expired_price_levels` - 验证OrderFlow级别的清理功能
4. `test_order_flow_keep_recent_price_levels` - 验证最近数据的保留功能

所有测试都通过，确保功能正常工作。

## 调用流程

```
ReactiveApp::event_loop()
    ↓
ReactiveApp::cleanup_expired_data_if_needed() (每秒调用一次)
    ↓
OrderBookManager::cleanup_expired_data()
    ↓
OrderFlow::clean_expired_price_levels() (对每个价格层级调用)
```

## 影响范围

- **修改文件**：
  - `src/orderbook/manager.rs` - 添加清理调用
  - `src/orderbook/order_flow.rs` - 添加清理方法
  - `tests/orderbook_cleanup_test.rs` - 添加测试用例

- **功能影响**：
  - 改善GUI界面显示，避免显示过期的订单薄数据
  - 减少内存使用，定期清理无效数据
  - 提高数据准确性，确保显示的都是最新的市场数据

## 配置参数

- **清理间隔**：5秒（5000毫秒）
- **调用频率**：每秒检查一次
- **数据类型**：1美元精度聚合的深度订单薄数据（bid/ask价格层级）
