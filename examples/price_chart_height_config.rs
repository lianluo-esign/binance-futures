// 价格图表高度配置示例
//
// 这个示例展示了如何配置价格图表的固定高度

use binance_futures::gui::unified_orderbook_widget::UnifiedOrderBookWidget;

fn main() {
    // 创建订单簿widget
    let mut widget = UnifiedOrderBookWidget::new();

    // 示例1: 设置价格图表高度为200像素
    widget.set_price_chart_height(200.0);
    println!("价格图表高度设置为: {}像素", widget.get_price_chart_height());

    // 示例2: 设置价格图表高度为400像素
    widget.set_price_chart_height(400.0);
    println!("价格图表高度设置为: {}像素", widget.get_price_chart_height());

    // 示例3: 设置价格图表高度为600像素
    widget.set_price_chart_height(600.0);
    println!("价格图表高度设置为: {}像素", widget.get_price_chart_height());

    // 示例4: 隐藏价格图表（设置为0）
    widget.set_price_chart_height(0.0);
    println!("价格图表高度设置为: {}像素", widget.get_price_chart_height());

    // 示例5: 超出范围的值会被自动限制在0.0-800.0之间
    widget.set_price_chart_height(1000.0); // 会被限制为800.0
    println!("尝试设置1000像素，实际值: {}像素", widget.get_price_chart_height());

    widget.set_price_chart_height(-50.0); // 会被限制为0.0
    println!("尝试设置-50像素，实际值: {}像素", widget.get_price_chart_height());
}

/*
使用说明:

1. 固定高度参数范围: 0.0 - 800.0 像素
   - 0.0: 不显示价格图表，整个右侧区域都是预留区域
   - 300.0: 默认高度
   - 800.0: 最大高度限制

2. 布局效果:
   - 左侧50%: 订单簿表格（不变）
   - 右侧50%:
     * 上部分: 价格图表（固定高度，例如300像素）
     * 下部分: 预留区域（剩余高度，至少50像素）

3. 实际应用场景:
   - 200像素: 紧凑布局，适合小屏幕
   - 300像素: 默认布局，平衡的显示效果
   - 400-500像素: 强调价格图表的场景
   - 600-800像素: 大屏幕下的详细图表显示

4. 智能限制:
   - 图表高度不会超过可用区域高度减去50像素（确保预留区域可见）
   - 超出范围的值会被自动限制在有效范围内
   - 可以在运行时动态调整，界面会立即响应变化

5. 优势:
   - 固定高度确保图表显示一致性
   - 不受窗口大小变化影响
   - 更精确的布局控制
*/
