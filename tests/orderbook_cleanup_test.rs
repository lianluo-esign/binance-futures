use flow_sight::orderbook::{OrderBookManager, OrderFlow};
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

#[test]
fn test_depth_cleanup_logic() {
    let mut manager = OrderBookManager::new();
    
    // 模拟第一次深度更新 - 20档数据
    let first_depth_data = json!({
        "b": [
            ["50000.0", "1.0"],
            ["49999.0", "2.0"],
            ["49998.0", "3.0"],
            ["49997.0", "4.0"],
            ["49996.0", "5.0"],
            ["49995.0", "6.0"],
            ["49994.0", "7.0"],
            ["49993.0", "8.0"],
            ["49992.0", "9.0"],
            ["49991.0", "10.0"],
            ["49990.0", "11.0"],
            ["49989.0", "12.0"],
            ["49988.0", "13.0"],
            ["49987.0", "14.0"],
            ["49986.0", "15.0"],
            ["49985.0", "16.0"],
            ["49984.0", "17.0"],
            ["49983.0", "18.0"],
            ["49982.0", "19.0"],
            ["49981.0", "20.0"]
        ],
        "a": [
            ["50001.0", "1.0"],
            ["50002.0", "2.0"],
            ["50003.0", "3.0"],
            ["50004.0", "4.0"],
            ["50005.0", "5.0"],
            ["50006.0", "6.0"],
            ["50007.0", "7.0"],
            ["50008.0", "8.0"],
            ["50009.0", "9.0"],
            ["50010.0", "10.0"],
            ["50011.0", "11.0"],
            ["50012.0", "12.0"],
            ["50013.0", "13.0"],
            ["50014.0", "14.0"],
            ["50015.0", "15.0"],
            ["50016.0", "16.0"],
            ["50017.0", "17.0"],
            ["50018.0", "18.0"],
            ["50019.0", "19.0"],
            ["50020.0", "20.0"]
        ]
    });
    
    manager.handle_depth_update(&first_depth_data);
    
    // 验证第一次更新后的数据
    let order_flows = manager.get_order_flows();
    assert_eq!(order_flows.len(), 40); // 20个bid + 20个ask
    
    // 模拟第二次深度更新 - 价格范围发生变化，只有15档数据
    let second_depth_data = json!({
        "b": [
            ["50010.0", "1.0"],
            ["50009.0", "2.0"],
            ["50008.0", "3.0"],
            ["50007.0", "4.0"],
            ["50006.0", "5.0"],
            ["50005.0", "6.0"],
            ["50004.0", "7.0"],
            ["50003.0", "8.0"],
            ["50002.0", "9.0"],
            ["50001.0", "10.0"],
            ["50000.0", "11.0"],
            ["49999.0", "12.0"],
            ["49998.0", "13.0"],
            ["49997.0", "14.0"],
            ["49996.0", "15.0"]
        ],
        "a": [
            ["50011.0", "1.0"],
            ["50012.0", "2.0"],
            ["50013.0", "3.0"],
            ["50014.0", "4.0"],
            ["50015.0", "5.0"],
            ["50016.0", "6.0"],
            ["50017.0", "7.0"],
            ["50018.0", "8.0"],
            ["50019.0", "9.0"],
            ["50020.0", "10.0"],
            ["50021.0", "11.0"],
            ["50022.0", "12.0"],
            ["50023.0", "13.0"],
            ["50024.0", "14.0"],
            ["50025.0", "15.0"]
        ]
    });
    
    manager.handle_depth_update(&second_depth_data);
    
    // 验证第二次更新后的数据 - 应该只保留新的15档数据
    let order_flows_after = manager.get_order_flows();
    assert_eq!(order_flows_after.len(), 30); // 15个bid + 15个ask
    
    // 验证旧的价格层级已被清除
    assert!(!order_flows_after.contains_key(&49981.0.into())); // 旧的bid价格
    assert!(!order_flows_after.contains_key(&50020.0.into())); // 旧的ask价格
    
    // 验证新的价格层级存在
    assert!(order_flows_after.contains_key(&50010.0.into())); // 新的bid价格
    assert!(order_flows_after.contains_key(&50025.0.into())); // 新的ask价格
}

#[test]
fn test_depth_cleanup_with_trades() {
    let mut manager = OrderBookManager::new();
    
    // 先添加一些深度数据
    let depth_data = json!({
        "b": [
            ["50000.0", "1.0"],
            ["49999.0", "2.0"]
        ],
        "a": [
            ["50001.0", "1.0"],
            ["50002.0", "2.0"]
        ]
    });
    
    manager.handle_depth_update(&depth_data);
    
    // 添加一些交易数据
    let trade_data = json!({
        "p": "50000.5",
        "q": "0.5",
        "m": false
    });
    
    manager.handle_trade(&trade_data);
    
    // 验证交易数据被正确添加
    let order_flows = manager.get_order_flows();
    assert!(order_flows.contains_key(&50000.5.into()));
    
    // 更新深度数据，不包含交易价格
    let new_depth_data = json!({
        "b": [
            ["50010.0", "1.0"],
            ["50009.0", "2.0"]
        ],
        "a": [
            ["50011.0", "1.0"],
            ["50012.0", "2.0"]
        ]
    });
    
    manager.handle_depth_update(&new_depth_data);
    
    // 验证深度数据被清理，但交易数据仍然保留（因为交易数据不是通过深度更新添加的）
    let order_flows_after = manager.get_order_flows();
    assert!(!order_flows_after.contains_key(&50000.0.into())); // 旧的深度数据被清除
    assert!(order_flows_after.contains_key(&50000.5.into())); // 交易数据仍然保留
}
