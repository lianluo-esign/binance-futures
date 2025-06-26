use binance_futures::orderbook::{OrderBookManager, OrderFlow};
use serde_json::json;
use std::thread;
use std::time::Duration;

#[test]
fn test_orderbook_data_cleanup_after_5_seconds() {
    let mut manager = OrderBookManager::new();
    
    // 创建模拟的深度更新数据
    let depth_data = json!({
        "b": [["50000.0", "1.5"]], // 买单：价格50000，数量1.5
        "a": [["50100.0", "2.0"]]  // 卖单：价格50100，数量2.0
    });
    
    // 处理深度更新
    manager.handle_depth_update(&depth_data);
    
    // 验证数据已经被添加
    let order_flows = manager.get_order_flows();
    assert!(!order_flows.is_empty(), "订单流数据应该不为空");
    
    // 检查特定价格的数据
    let price_50000 = ordered_float::OrderedFloat(50000.0);
    let price_50100 = ordered_float::OrderedFloat(50100.0);
    
    assert!(order_flows.contains_key(&price_50000), "应该包含50000价格的数据");
    assert!(order_flows.contains_key(&price_50100), "应该包含50100价格的数据");
    
    // 验证初始数据
    let flow_50000 = &order_flows[&price_50000];
    let flow_50100 = &order_flows[&price_50100];
    
    assert!(flow_50000.bid_ask.bid > 0.0, "50000价格应该有买单数据");
    assert!(flow_50100.bid_ask.ask > 0.0, "50100价格应该有卖单数据");
    
    println!("初始数据验证通过");
    
    // 等待6秒（超过5秒的清理阈值）
    println!("等待6秒以测试数据清理...");
    thread::sleep(Duration::from_secs(6));
    
    // 执行清理
    manager.cleanup_expired_data();
    
    // 重新获取数据
    let order_flows_after_cleanup = manager.get_order_flows();
    
    // 验证数据是否被清理
    if let Some(flow_50000_after) = order_flows_after_cleanup.get(&price_50000) {
        assert_eq!(flow_50000_after.bid_ask.bid, 0.0, "50000价格的买单数据应该被清理");
    }
    
    if let Some(flow_50100_after) = order_flows_after_cleanup.get(&price_50100) {
        assert_eq!(flow_50100_after.bid_ask.ask, 0.0, "50100价格的卖单数据应该被清理");
    }
    
    println!("数据清理验证通过");
}

#[test]
fn test_orderbook_data_not_cleaned_within_5_seconds() {
    let mut manager = OrderBookManager::new();
    
    // 创建模拟的深度更新数据
    let depth_data = json!({
        "b": [["51000.0", "1.0"]], // 买单：价格51000，数量1.0
        "a": [["51100.0", "1.5"]]  // 卖单：价格51100，数量1.5
    });
    
    // 处理深度更新
    manager.handle_depth_update(&depth_data);
    
    // 验证数据已经被添加
    let order_flows = manager.get_order_flows();
    let price_51000 = ordered_float::OrderedFloat(51000.0);
    let price_51100 = ordered_float::OrderedFloat(51100.0);
    
    assert!(order_flows.contains_key(&price_51000), "应该包含51000价格的数据");
    assert!(order_flows.contains_key(&price_51100), "应该包含51100价格的数据");
    
    // 等待3秒（少于5秒的清理阈值）
    println!("等待3秒，数据不应该被清理...");
    thread::sleep(Duration::from_secs(3));
    
    // 执行清理
    manager.cleanup_expired_data();
    
    // 重新获取数据
    let order_flows_after_cleanup = manager.get_order_flows();
    
    // 验证数据没有被清理
    if let Some(flow_51000_after) = order_flows_after_cleanup.get(&price_51000) {
        assert!(flow_51000_after.bid_ask.bid > 0.0, "51000价格的买单数据不应该被清理");
    }
    
    if let Some(flow_51100_after) = order_flows_after_cleanup.get(&price_51100) {
        assert!(flow_51100_after.bid_ask.ask > 0.0, "51100价格的卖单数据不应该被清理");
    }
    
    println!("数据保留验证通过");
}

#[test]
fn test_order_flow_clean_expired_price_levels() {
    let mut order_flow = OrderFlow::new();
    
    // 获取当前时间戳
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    // 设置一个旧的时间戳（6秒前）
    let old_timestamp = current_time - 6000; // 6秒前
    
    // 更新价格层级数据
    order_flow.update_price_level(100.0, 200.0, old_timestamp);
    
    // 验证数据存在
    assert_eq!(order_flow.bid_ask.bid, 100.0);
    assert_eq!(order_flow.bid_ask.ask, 200.0);
    assert_eq!(order_flow.bid_ask.timestamp, old_timestamp);
    
    // 执行清理（5秒阈值）
    order_flow.clean_expired_price_levels(current_time, 5000);
    
    // 验证数据被清理
    assert_eq!(order_flow.bid_ask.bid, 0.0, "过期的买单数据应该被清理");
    assert_eq!(order_flow.bid_ask.ask, 0.0, "过期的卖单数据应该被清理");
    assert_eq!(order_flow.bid_ask.timestamp, old_timestamp, "时间戳应该保持不变");
    
    println!("OrderFlow清理功能验证通过");
}

#[test]
fn test_order_flow_keep_recent_price_levels() {
    let mut order_flow = OrderFlow::new();
    
    // 获取当前时间戳
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    // 设置一个较新的时间戳（3秒前）
    let recent_timestamp = current_time - 3000; // 3秒前
    
    // 更新价格层级数据
    order_flow.update_price_level(150.0, 250.0, recent_timestamp);
    
    // 验证数据存在
    assert_eq!(order_flow.bid_ask.bid, 150.0);
    assert_eq!(order_flow.bid_ask.ask, 250.0);
    assert_eq!(order_flow.bid_ask.timestamp, recent_timestamp);
    
    // 执行清理（5秒阈值）
    order_flow.clean_expired_price_levels(current_time, 5000);
    
    // 验证数据没有被清理
    assert_eq!(order_flow.bid_ask.bid, 150.0, "最近的买单数据不应该被清理");
    assert_eq!(order_flow.bid_ask.ask, 250.0, "最近的卖单数据不应该被清理");
    assert_eq!(order_flow.bid_ask.timestamp, recent_timestamp, "时间戳应该保持不变");
    
    println!("OrderFlow保留功能验证通过");
}
