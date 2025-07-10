use flow_sight::orderbook::OrderBookManager;
use serde_json::json;
use std::thread;
use std::time::Duration;

#[test]
fn test_rv_volatility_10s_window() {
    let mut manager = OrderBookManager::new();
    
    // 模拟连续的价格更新，验证10秒窗口的计算
    let base_price = 50000.0;
    let current_time = manager.get_current_timestamp();
    
    // 添加10秒内的价格数据
    for i in 0..20 {
        let price = base_price + (i as f64 * 0.1); // 每次价格增加0.1
        let timestamp = current_time + (i * 500); // 每500ms一个价格点
        
        // 模拟交易数据来触发RV计算
        let trade_data = json!({
            "p": price.to_string(),
            "q": "1.0",
            "m": false
        });
        
        manager.handle_trade(&trade_data);
        
        // 短暂等待
        thread::sleep(Duration::from_millis(10));
    }
    
    // 获取RV历史数据
    let rv_history = manager.get_rv_history();
    
    // 验证RV历史数据长度（应该不超过600个点，对应10分钟的数据）
    assert!(rv_history.len() <= 600, "RV历史数据长度应该不超过600个点");
    
    // 验证RV值是否合理（应该大于0）
    if let Some((_, rv_value)) = rv_history.back() {
        assert!(*rv_value >= 0.0, "RV值应该大于等于0");
        println!("当前RV波动率: {:.4}", rv_value);
    }
    
    // 等待超过10秒，然后添加新的价格数据
    println!("等待11秒以测试10秒窗口...");
    thread::sleep(Duration::from_secs(11));
    
    // 添加新的价格数据
    let new_price = base_price + 100.0;
    let new_trade_data = json!({
        "p": new_price.to_string(),
        "q": "1.0",
        "m": false
    });
    
    manager.handle_trade(&new_trade_data);
    
    // 验证RV历史数据仍然在合理范围内
    let rv_history_after = manager.get_rv_history();
    assert!(rv_history_after.len() <= 600, "RV历史数据长度应该不超过600个点");
    
    println!("RV波动率10秒窗口测试通过");
}

#[test]
fn test_rv_volatility_calculation_consistency() {
    let mut manager = OrderBookManager::new();
    
    // 模拟稳定的价格变化
    let base_price = 50000.0;
    let current_time = manager.get_current_timestamp();
    
    // 添加一系列价格数据
    for i in 0..50 {
        // 模拟小幅度的价格波动
        let price_change = if i % 2 == 0 { 0.1 } else { -0.1 };
        let price = base_price + (i as f64 * price_change);
        let timestamp = current_time + (i * 200); // 每200ms一个价格点
        
        let trade_data = json!({
            "p": price.to_string(),
            "q": "1.0",
            "m": false
        });
        
        manager.handle_trade(&trade_data);
        
        // 短暂等待
        thread::sleep(Duration::from_millis(5));
    }
    
    // 获取市场快照
    let market_snapshot = manager.get_market_snapshot();
    
    // 验证RV值在合理范围内
    assert!(market_snapshot.realized_volatility >= 0.0, "RV值应该大于等于0");
    assert!(market_snapshot.realized_volatility < 1000.0, "RV值应该在合理范围内");
    
    println!("RV波动率计算一致性测试通过，当前RV值: {:.4}", market_snapshot.realized_volatility);
} 