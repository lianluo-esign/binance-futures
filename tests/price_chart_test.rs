use binance_futures::gui::PriceChartRenderer;

#[test]
fn test_price_chart_basic_functionality() {
    let mut chart = PriceChartRenderer::new(100);
    
    // 测试初始状态
    assert_eq!(chart.get_data_count(), 0);
    assert_eq!(chart.get_latest_price(), None);
    
    // 添加一些价格数据点（现在需要包含交易信息）
    chart.add_price_point(100000.0, 0.001, false); // 买单
    chart.add_price_point(100001.0, 0.002, true);  // 卖单
    chart.add_price_point(100002.0, 0.001, false); // 买单
    chart.add_price_point(99999.0, 0.003, true);   // 卖单
    chart.add_price_point(100003.0, 0.001, false); // 买单
    
    // 验证数据点数量
    assert_eq!(chart.get_data_count(), 5);
    
    // 验证最新价格
    assert_eq!(chart.get_latest_price(), Some(100003.0));
    
    // 验证价格范围
    let (min_price, max_price) = chart.get_price_range();
    assert_eq!(min_price, 99999.0);
    assert_eq!(max_price, 100003.0);
    
    // 测试统计信息
    let stats = chart.get_stats();
    assert_eq!(stats.data_points, 5);
    assert_eq!(stats.min_price, 99999.0);
    assert_eq!(stats.max_price, 100003.0);
    assert_eq!(stats.latest_price, Some(100003.0));
    assert_eq!(stats.price_range, 4.0);
}

#[test]
fn test_price_chart_sliding_window() {
    let mut chart = PriceChartRenderer::new(3); // 只保留3个数据点
    
    // 添加超过窗口大小的数据点
    chart.add_price_point(100.0, 0.001, false);
    chart.add_price_point(101.0, 0.001, true);
    chart.add_price_point(102.0, 0.001, false);
    assert_eq!(chart.get_data_count(), 3);
    
    // 添加第4个数据点，应该移除第1个
    chart.add_price_point(103.0, 0.001, true);
    assert_eq!(chart.get_data_count(), 3);
    assert_eq!(chart.get_latest_price(), Some(103.0));
    
    // 验证价格范围已更新（不再包含100.0）
    let (min_price, max_price) = chart.get_price_range();
    assert_eq!(min_price, 101.0);
    assert_eq!(max_price, 103.0);
}

#[test]
fn test_price_chart_clear_data() {
    let mut chart = PriceChartRenderer::new(100);
    
    // 添加一些数据
    chart.add_price_point(100.0, 0.001, false);
    chart.add_price_point(101.0, 0.001, true);
    assert_eq!(chart.get_data_count(), 2);
    
    // 清空数据
    chart.clear_data();
    assert_eq!(chart.get_data_count(), 0);
    assert_eq!(chart.get_latest_price(), None);
}

#[test]
fn test_price_chart_configuration() {
    let mut chart = PriceChartRenderer::new(1000);
    
    // 测试设置最大数据点数量
    chart.set_max_data_points(5);
    
    // 添加超过新限制的数据点
    for i in 0..10 {
        chart.add_price_point(100.0 + i as f64, 0.001, i % 2 == 0); // 交替买卖单
    }
    
    // 应该只保留最后5个数据点
    assert_eq!(chart.get_data_count(), 5);
    assert_eq!(chart.get_latest_price(), Some(109.0));
    
    // 测试设置价格刻度间隔
    chart.set_price_scale_interval(5.0);
    // 这个测试主要确保函数不会panic，具体的刻度逻辑在渲染时验证
}

#[test]
fn test_price_chart_edge_cases() {
    let mut chart = PriceChartRenderer::new(100);
    
    // 测试相同价格
    chart.add_price_point(100.0, 0.001, false);
    chart.add_price_point(100.0, 0.001, true);
    chart.add_price_point(100.0, 0.001, false);
    
    let (min_price, max_price) = chart.get_price_range();
    // 当所有价格相同时，min和max应该相等
    assert_eq!(min_price, 100.0);
    assert_eq!(max_price, 100.0);
    
    // 测试极端价格值
    chart.clear_data();
    chart.add_price_point(0.01, 0.001, false);
    chart.add_price_point(1000000.0, 0.001, true);
    
    let (min_price, max_price) = chart.get_price_range();
    assert_eq!(min_price, 0.01);
    assert_eq!(max_price, 1000000.0);
}