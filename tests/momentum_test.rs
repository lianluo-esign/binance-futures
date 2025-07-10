use flow_sight::orderbook::OrderBookManager;
use serde_json::json;
use std::thread;
use std::time::Duration;

#[test]
fn test_volume_weighted_momentum() {
    let mut manager = OrderBookManager::new();
    
    // 模拟连续的交易数据，验证成交量加权动量计算
    let base_price = 50000.0;
    let current_time = manager.get_current_timestamp();
    
    // 模拟多头趋势：价格上涨 + 买单量大于卖单量
    println!("测试多头趋势...");
    for i in 0..10 {
        let price = base_price + (i as f64 * 1.0); // 每次价格上涨1美元
        let volume = 1.0 + (i as f64 * 0.1); // 逐渐增加的成交量
        let is_buy = i % 3 != 0; // 大部分是买单
        
        // 模拟交易数据
        let trade_data = json!({
            "p": price.to_string(),
            "q": volume.to_string(),
            "m": !is_buy // maker是卖单，taker是买单
        });
        
        manager.handle_trade(&trade_data);
        
        // 短暂等待
        thread::sleep(Duration::from_millis(10));
    }
    
    // 获取动量数据
    let momentum = manager.get_volume_weighted_momentum();
    let momentum_history_len = manager.get_momentum_history().len();
    
    println!("多头趋势测试结果:");
    println!("当前动量值: {:.6}", momentum);
    println!("动量历史数据点数: {}", momentum_history_len);
    
    // 验证多头趋势应该产生正动量
    assert!(momentum > 0.0, "多头趋势应该产生正动量，实际值: {}", momentum);
    assert!(momentum_history_len > 0, "应该有动量历史数据");
    
    // 模拟空头趋势：价格下跌 + 卖单量大于买单量
    println!("\n测试空头趋势...");
    for i in 0..10 {
        let price = base_price + 10.0 - (i as f64 * 1.0); // 每次价格下跌1美元
        let volume = 1.0 + (i as f64 * 0.1); // 逐渐增加的成交量
        let is_buy = i % 3 == 0; // 大部分是卖单
        
        // 模拟交易数据
        let trade_data = json!({
            "p": price.to_string(),
            "q": volume.to_string(),
            "m": !is_buy // maker是卖单，taker是买单
        });
        
        manager.handle_trade(&trade_data);
        
        // 短暂等待
        thread::sleep(Duration::from_millis(10));
    }
    
    // 获取更新后的动量数据
    let momentum_after = manager.get_volume_weighted_momentum();
    let momentum_history_after_len = manager.get_momentum_history().len();
    
    println!("空头趋势测试结果:");
    println!("当前动量值: {:.6}", momentum_after);
    println!("动量历史数据点数: {}", momentum_history_after_len);
    
    // 验证空头趋势应该产生负动量
    assert!(momentum_after < 0.0, "空头趋势应该产生负动量，实际值: {}", momentum_after);
    assert!(momentum_history_after_len > momentum_history_len, "应该有更多历史数据");
    
    // 验证历史数据长度限制
    assert!(momentum_history_after_len <= 600, "历史数据应该不超过600个点");
    
    println!("\n成交量加权动量指标测试通过！");
    println!("多头趋势动量: {:.6}", momentum);
    println!("空头趋势动量: {:.6}", momentum_after);
}

#[test]
fn test_z_score_momentum_calculation() {
    let mut manager = OrderBookManager::new();
    
    // 设置测试参数
    manager.set_momentum_window_size(5);
    manager.set_momentum_threshold(1.5);
    
    // 模拟上涨趋势的价格序列
    let test_prices = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0];
    let test_volumes = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
    
    // 添加测试数据
    for (i, &price) in test_prices.iter().enumerate() {
        let volume = test_volumes[i];
        let timestamp = i as u64 * 1000; // 每秒一个数据点
        manager.calculate_volume_weighted_momentum(timestamp, price, volume, true); // 假设都是买单
    }
    
    // 验证动量值应该为正（上涨趋势）
    let momentum = manager.get_volume_weighted_momentum();
    assert!(momentum > 0.0, "上涨趋势的动量值应该为正，实际值: {}", momentum);
    
    // 验证历史数据长度
    let history = manager.get_momentum_history();
    assert!(!history.is_empty(), "应该有历史数据");
}

#[test]
fn test_z_score_momentum_downward_trend() {
    let mut manager = OrderBookManager::new();
    
    // 设置测试参数
    manager.set_momentum_window_size(5);
    manager.set_momentum_threshold(1.5);
    
    // 模拟下跌趋势的价格序列
    let test_prices = vec![105.0, 104.0, 103.0, 102.0, 101.0, 100.0];
    let test_volumes = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
    
    // 添加测试数据
    for (i, &price) in test_prices.iter().enumerate() {
        let volume = test_volumes[i];
        let timestamp = i as u64 * 1000; // 每秒一个数据点
        manager.calculate_volume_weighted_momentum(timestamp, price, volume, false); // 假设都是卖单
    }
    
    // 验证动量值应该为负（下跌趋势）
    let momentum = manager.get_volume_weighted_momentum();
    assert!(momentum < 0.0, "下跌趋势的动量值应该为负，实际值: {}", momentum);
}

#[test]
fn test_z_score_momentum_volume_weighting() {
    let mut manager = OrderBookManager::new();
    
    // 设置测试参数
    manager.set_momentum_window_size(5);
    manager.set_momentum_threshold(1.5);
    
    // 模拟价格序列
    let test_prices = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0];
    let test_volumes = vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0]; // 递增的成交量
    
    // 添加测试数据
    for (i, &price) in test_prices.iter().enumerate() {
        let volume = test_volumes[i];
        let timestamp = i as u64 * 1000;
        manager.calculate_volume_weighted_momentum(timestamp, price, volume, true);
    }
    
    // 验证动量值应该为正且受成交量影响
    let momentum = manager.get_volume_weighted_momentum();
    assert!(momentum > 0.0, "上涨趋势的动量值应该为正，实际值: {}", momentum);
    
    // 验证历史数据
    let history = manager.get_momentum_history();
    assert!(!history.is_empty(), "应该有历史数据");
}

#[test]
fn test_z_score_momentum_threshold_signals() {
    let mut manager = OrderBookManager::new();
    
    // 设置测试参数
    manager.set_momentum_window_size(5);
    manager.set_momentum_threshold(1.0); // 设置较低的阈值便于测试
    
    // 模拟强上涨趋势的价格序列
    let test_prices = vec![100.0, 102.0, 104.0, 106.0, 108.0, 110.0]; // 每次上涨2%
    let test_volumes = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
    
    // 添加测试数据
    for (i, &price) in test_prices.iter().enumerate() {
        let volume = test_volumes[i];
        let timestamp = i as u64 * 1000;
        manager.calculate_volume_weighted_momentum(timestamp, price, volume, true);
    }
    
    // 验证动量值应该超过阈值（买入信号）
    let momentum = manager.get_volume_weighted_momentum();
    let threshold = manager.get_momentum_threshold();
    assert!(momentum > threshold, "强上涨趋势应该产生买入信号，动量值: {}, 阈值: {}", momentum, threshold);
}

#[test]
fn test_z_score_momentum_window_size() {
    let mut manager = OrderBookManager::new();
    
    // 测试不同的窗口大小
    let test_window_sizes = vec![5, 10, 20];
    
    for window_size in test_window_sizes {
        manager.set_momentum_window_size(window_size);
        assert_eq!(manager.get_momentum_window_size(), window_size, "窗口大小设置失败");
        
        // 验证窗口大小限制
        manager.set_momentum_window_size(200); // 超过最大值
        assert_eq!(manager.get_momentum_window_size(), 100, "窗口大小应该被限制在100");
        
        manager.set_momentum_window_size(1); // 低于最小值
        assert_eq!(manager.get_momentum_window_size(), 5, "窗口大小应该被限制在5");
    }
}

#[test]
fn test_z_score_momentum_threshold_limits() {
    let mut manager = OrderBookManager::new();
    
    // 测试阈值限制
    manager.set_momentum_threshold(10.0); // 超过最大值
    assert_eq!(manager.get_momentum_threshold(), 5.0, "阈值应该被限制在5.0");
    
    manager.set_momentum_threshold(0.01); // 低于最小值
    assert_eq!(manager.get_momentum_threshold(), 0.1, "阈值应该被限制在0.1");
    
    // 测试正常范围
    manager.set_momentum_threshold(2.0);
    assert_eq!(manager.get_momentum_threshold(), 2.0, "正常阈值设置失败");
}

#[test]
fn test_momentum_large_dataset() {
    let mut manager = OrderBookManager::new();
    
    // 设置测试参数
    manager.set_momentum_window_size(10);
    manager.set_momentum_threshold(1.5);
    
    // 模拟大量数据点（超过UI显示限制）
    let test_count = 1000;
    let base_price = 100.0;
    
    for i in 0..test_count {
        let price = base_price + (i as f64 * 0.01); // 逐渐上涨
        let volume = 1.0;
        let timestamp = i as u64 * 1000;
        manager.calculate_volume_weighted_momentum(timestamp, price, volume, true);
    }
    
    // 验证历史数据长度
    let history = manager.get_momentum_history();
    assert!(history.len() > 600, "应该有超过600个历史数据点，实际: {}", history.len());
    assert!(history.len() <= 3000, "历史数据点不应超过3000个，实际: {}", history.len());
    
    // 验证动量值应该为正（上涨趋势）
    let momentum = manager.get_volume_weighted_momentum();
    assert!(momentum > 0.0, "上涨趋势的动量值应该为正，实际值: {}", momentum);
} 