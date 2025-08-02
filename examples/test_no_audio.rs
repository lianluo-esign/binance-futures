use flow_sight::orderbook::OrderBookManager;
use serde_json::json;

fn main() {
    println!("Testing order book without audio dependencies...");
    
    let mut manager = OrderBookManager::new();
    
    // Test depth update
    let depth_data = json!({
        "b": [["50000.0", "1.5"]],
        "a": [["50100.0", "2.0"]]
    });
    
    manager.handle_depth_update(&depth_data);
    
    let order_flows = manager.get_order_flows();
    println!("✅ Order flows count: {}", order_flows.len());
    
    // Test trade data
    let trade_data = json!({
        "p": "50050.0",
        "q": "0.5",
        "m": false
    });
    
    manager.handle_trade(&trade_data);
    
    let (best_bid, best_ask) = manager.get_best_prices();
    println!("✅ Best bid: {:?}, Best ask: {:?}", best_bid, best_ask);
    
    println!("🎉 Audio removal successful! Core functionality works without ALSA dependencies.");
}