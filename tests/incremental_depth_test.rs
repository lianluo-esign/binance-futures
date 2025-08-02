use flow_sight::orderbook::OrderBookManager;
use serde_json::json;
use std::thread;
use std::time::Duration;

#[test]
fn test_incremental_depth_update_preserves_unchanged_levels() {
    let mut manager = OrderBookManager::new();
    
    // 第一次深度更新 - 模拟完整深度快照
    let initial_depth = json!({
        "b": [
            ["50000.0", "1.0"],
            ["49999.0", "2.0"],
            ["49998.0", "3.0"]
        ],
        "a": [
            ["50001.0", "1.5"],
            ["50002.0", "2.5"],
            ["50003.0", "3.5"]
        ]
    });
    
    manager.handle_depth_update(&initial_depth);
    
    // 验证初始数据
    let order_flows = manager.get_order_flows();
    assert_eq!(order_flows.len(), 6);
    
    let price_50000 = ordered_float::OrderedFloat(50000.0);
    let price_49999 = ordered_float::OrderedFloat(49999.0);
    let price_50001 = ordered_float::OrderedFloat(50001.0);
    
    assert_eq!(order_flows[&price_50000].bid_ask.bid, 1.0);
    assert_eq!(order_flows[&price_49999].bid_ask.bid, 2.0);
    assert_eq!(order_flows[&price_50001].bid_ask.ask, 1.5);
    
    println!("初始深度数据验证通过");
    
    // 第二次深度更新 - 只更新部分价格层级（增量更新）
    let incremental_depth = json!({
        "b": [
            ["50000.0", "1.5"], // 更新50000价格的数量
            ["49997.0", "4.0"]  // 新增49997价格层级
        ],
        "a": [
            ["50001.0", "0.0"]  // 移除50001价格层级
        ]
    });
    
    manager.handle_depth_update(&incremental_depth);
    
    // 验证增量更新结果
    let order_flows_after = manager.get_order_flows();
    
    // 验证更新的价格层级
    assert_eq!(order_flows_after[&price_50000].bid_ask.bid, 1.5, "50000价格应该被更新为1.5");
    
    // 验证未在更新中提到的价格层级应该保持不变
    assert_eq!(order_flows_after[&price_49999].bid_ask.bid, 2.0, "49999价格应该保持不变");
    
    // 验证新添加的价格层级
    let price_49997 = ordered_float::OrderedFloat(49997.0);
    assert!(order_flows_after.contains_key(&price_49997), "应该包含新的49997价格层级");
    assert_eq!(order_flows_after[&price_49997].bid_ask.bid, 4.0);
    
    // 验证被移除的价格层级（数量为0）
    assert_eq!(order_flows_after[&price_50001].bid_ask.ask, 0.0, "50001价格的ask应该被移除");
    
    // 验证其他未提到的ask价格层级保持不变
    let price_50002 = ordered_float::OrderedFloat(50002.0);
    let price_50003 = ordered_float::OrderedFloat(50003.0);
    assert_eq!(order_flows_after[&price_50002].bid_ask.ask, 2.5, "50002价格应该保持不变");
    assert_eq!(order_flows_after[&price_50003].bid_ask.ask, 3.5, "50003价格应该保持不变");
    
    println!("增量深度更新验证通过");
}

#[test]
fn test_depth_data_not_auto_cleared_over_time() {
    let mut manager = OrderBookManager::new();
    
    // 添加深度数据
    let depth_data = json!({
        "b": [["51000.0", "2.0"]],
        "a": [["51100.0", "3.0"]]
    });
    
    manager.handle_depth_update(&depth_data);
    
    // 验证数据存在
    let price_51000 = ordered_float::OrderedFloat(51000.0);
    let price_51100 = ordered_float::OrderedFloat(51100.0);
    
    let order_flows_initial = manager.get_order_flows();
    assert_eq!(order_flows_initial[&price_51000].bid_ask.bid, 2.0);
    assert_eq!(order_flows_initial[&price_51100].bid_ask.ask, 3.0);
    
    // 等待10秒（远超过之前的500ms清理阈值）
    println!("等待10秒，测试数据是否会被自动清理...");
    thread::sleep(Duration::from_secs(10));
    
    // 执行清理
    manager.cleanup_expired_data();
    
    // 验证数据仍然存在（因为是深度数据，不应该被自动清理）
    let order_flows_after = manager.get_order_flows();
    assert!(order_flows_after.contains_key(&price_51000), "51000价格层级应该仍然存在");
    assert!(order_flows_after.contains_key(&price_51100), "51100价格层级应该仍然存在");
    assert_eq!(order_flows_after[&price_51000].bid_ask.bid, 2.0, "51000价格的bid数据不应该被清理");
    assert_eq!(order_flows_after[&price_51100].bid_ask.ask, 3.0, "51100价格的ask数据不应该被清理");
    
    println!("深度数据持久化验证通过");
}

#[test]
fn test_mixed_bid_ask_updates() {
    let mut manager = OrderBookManager::new();
    
    // 初始深度数据 - 同一价格有bid和ask
    let initial_depth = json!({
        "b": [["50000.0", "1.0"]],
        "a": [["50000.0", "2.0"]]
    });
    
    manager.handle_depth_update(&initial_depth);
    
    let price_50000 = ordered_float::OrderedFloat(50000.0);
    let order_flows = manager.get_order_flows();
    
    // 验证同一价格层级同时有bid和ask
    assert_eq!(order_flows[&price_50000].bid_ask.bid, 1.0);
    assert_eq!(order_flows[&price_50000].bid_ask.ask, 2.0);
    
    // 增量更新 - 只更新bid，保持ask不变
    let bid_update = json!({
        "b": [["50000.0", "3.0"]]
    });
    
    manager.handle_depth_update(&bid_update);
    
    let order_flows_after_bid = manager.get_order_flows();
    // bid应该被更新，ask应该保持不变
    assert_eq!(order_flows_after_bid[&price_50000].bid_ask.bid, 3.0, "bid应该被更新");
    assert_eq!(order_flows_after_bid[&price_50000].bid_ask.ask, 2.0, "ask应该保持不变");
    
    // 增量更新 - 只更新ask，保持bid不变
    let ask_update = json!({
        "a": [["50000.0", "4.0"]]
    });
    
    manager.handle_depth_update(&ask_update);
    
    let order_flows_after_ask = manager.get_order_flows();
    // ask应该被更新，bid应该保持不变
    assert_eq!(order_flows_after_ask[&price_50000].bid_ask.bid, 3.0, "bid应该保持不变");
    assert_eq!(order_flows_after_ask[&price_50000].bid_ask.ask, 4.0, "ask应该被更新");
    
    println!("混合bid/ask更新验证通过");
}

#[test]
fn test_zero_quantity_removes_price_level() {
    let mut manager = OrderBookManager::new();
    
    // 添加初始深度数据
    let initial_depth = json!({
        "b": [["50000.0", "1.0"]],
        "a": [["50001.0", "2.0"]]
    });
    
    manager.handle_depth_update(&initial_depth);
    
    let price_50000 = ordered_float::OrderedFloat(50000.0);
    let price_50001 = ordered_float::OrderedFloat(50001.0);
    
    // 验证初始数据
    let order_flows_initial = manager.get_order_flows();
    assert_eq!(order_flows_initial[&price_50000].bid_ask.bid, 1.0);
    assert_eq!(order_flows_initial[&price_50001].bid_ask.ask, 2.0);
    
    // 发送数量为0的更新，应该移除相应的价格层级
    let remove_update = json!({
        "b": [["50000.0", "0.0"]],
        "a": [["50001.0", "0.0"]]
    });
    
    manager.handle_depth_update(&remove_update);
    
    let order_flows_after = manager.get_order_flows();
    
    // 验证数量为0的价格层级被正确处理
    assert_eq!(order_flows_after[&price_50000].bid_ask.bid, 0.0, "bid应该被移除（设为0）");
    assert_eq!(order_flows_after[&price_50001].bid_ask.ask, 0.0, "ask应该被移除（设为0）");
    
    println!("零数量移除价格层级验证通过");
}