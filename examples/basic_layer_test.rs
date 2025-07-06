use flow_sight::core::{BasicLayer, BasicLayerStats};
use flow_sight::events::event_types::{Event, EventType};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    // 初始化日志
    env_logger::init();

    println!("=== BasicLayer 基础数据层测试 ===");
    
    // 创建BasicLayer实例
    let mut basic_layer = BasicLayer::new();
    
    println!("✅ BasicLayer 创建成功");
    
    // 显示支持的交易所
    let supported_exchanges = basic_layer.get_supported_exchanges();
    println!("📋 支持的交易所: {:?}", supported_exchanges);
    
    // 模拟一些事件数据
    simulate_market_data(&mut basic_layer);
    
    // 显示统计信息
    display_statistics(&basic_layer);
    
    // 显示各交易所的数据
    display_exchange_data(&basic_layer);
    
    println!("\n🎉 BasicLayer 测试完成！");
}

fn simulate_market_data(basic_layer: &mut BasicLayer) {
    println!("\n📊 模拟市场数据...");
    
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    // 模拟不同交易所的深度数据
    let exchanges = vec!["binance", "okx", "bybit", "coinbase"];
    let base_price = 108000.0;
    
    for (i, exchange) in exchanges.iter().enumerate() {
        let price_offset = i as f64 * 10.0;
        
        // 模拟深度数据
        let depth_event = Event::new_with_exchange(
            EventType::DepthUpdate(json!({
                "bids": [
                    [(base_price - 1.0 + price_offset).to_string(), "1.5"],
                    [(base_price - 2.0 + price_offset).to_string(), "2.3"],
                    [(base_price - 3.0 + price_offset).to_string(), "0.8"]
                ],
                "asks": [
                    [(base_price + 1.0 + price_offset).to_string(), "1.2"],
                    [(base_price + 2.0 + price_offset).to_string(), "2.1"],
                    [(base_price + 3.0 + price_offset).to_string(), "0.9"]
                ]
            })),
            "websocket".to_string(),
            exchange.to_string()
        );
        
        basic_layer.handle_event(&depth_event);
        
        // 模拟成交数据
        for j in 0..5 {
            let trade_price = base_price + price_offset + (j as f64 * 0.1);
            let trade_event = Event::new_with_exchange(
                EventType::Trade(json!({
                    "price": trade_price.to_string(),
                    "quantity": (0.1 + j as f64 * 0.05).to_string(),
                    "side": if j % 2 == 0 { "buy" } else { "sell" },
                    "timestamp": current_time + j as u64 * 1000
                })),
                "websocket".to_string(),
                exchange.to_string()
            );
            
            basic_layer.handle_event(&trade_event);
        }
        
        // 模拟BookTicker数据
        let book_ticker_event = Event::new_with_exchange(
            EventType::BookTicker(json!({
                "bidPrice": (base_price - 0.5 + price_offset).to_string(),
                "askPrice": (base_price + 0.5 + price_offset).to_string(),
                "bidQty": "1.8",
                "askQty": "1.6"
            })),
            "websocket".to_string(),
            exchange.to_string()
        );
        
        basic_layer.handle_event(&book_ticker_event);
        
        println!("  ✅ {} 数据模拟完成", exchange);
    }
}

fn display_statistics(basic_layer: &BasicLayer) {
    println!("\n📈 BasicLayer 全局统计:");
    
    let stats = basic_layer.get_global_stats();
    println!("  总交易所数: {}", stats.total_exchanges);
    println!("  活跃交易所数: {}", stats.active_exchanges);
    println!("  总成交记录: {}", stats.total_trades);
    println!("  总深度更新: {}", stats.total_depth_updates);
    println!("  总BookTicker更新: {}", stats.total_book_ticker_updates);
    println!("  最后更新时间: {}", stats.last_update);
    
    // 显示活跃交易所
    let active_exchanges = basic_layer.get_active_exchanges();
    println!("  活跃交易所: {:?}", active_exchanges);
}

fn display_exchange_data(basic_layer: &BasicLayer) {
    println!("\n🏢 各交易所详细数据:");
    
    for exchange in basic_layer.get_active_exchanges() {
        if let Some(manager) = basic_layer.get_exchange_manager(&exchange) {
            println!("\n  📊 {} 交易所:", exchange);
            
            // 显示统计信息
            let stats = manager.get_stats();
            println!("    成交记录: {}", stats.total_trades);
            println!("    深度更新: {}", stats.total_depth_updates);
            println!("    BookTicker更新: {}", stats.total_book_ticker_updates);
            println!("    成交数据窗口大小: {}", manager.get_trades_count());
            println!("    价格层级数量: {}", manager.get_price_levels_count());
            
            // 显示最优买卖价
            if let (Some(best_bid), Some(best_ask)) = (manager.best_bid, manager.best_ask) {
                println!("    最优买价: ${:.2}", best_bid);
                println!("    最优卖价: ${:.2}", best_ask);
                println!("    价差: ${:.2}", best_ask - best_bid);
            }
            
            // 显示最近的成交数据
            let recent_trades = manager.get_recent_trades(3);
            if !recent_trades.is_empty() {
                println!("    最近3笔成交:");
                for (i, trade) in recent_trades.iter().enumerate() {
                    println!("      {}. ${:.2} - {:.4} BTC - {} - {}ms", 
                        i + 1, 
                        trade.price, 
                        trade.quantity, 
                        trade.side,
                        trade.timestamp
                    );
                }
            }
            
            // 显示订单簿快照
            let orderbook = manager.get_orderbook_snapshot();
            if !orderbook.bids.is_empty() && !orderbook.asks.is_empty() {
                println!("    订单簿快照 (前3层):");
                println!("      买单:");
                for (i, (price, qty)) in orderbook.bids.iter().take(3).enumerate() {
                    println!("        {}. ${:.2} - {:.4} BTC", i + 1, price, qty);
                }
                println!("      卖单:");
                for (i, (price, qty)) in orderbook.asks.iter().take(3).enumerate() {
                    println!("        {}. ${:.2} - {:.4} BTC", i + 1, price, qty);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_layer_functionality() {
        let mut basic_layer = BasicLayer::new();
        
        // 测试初始状态
        let stats = basic_layer.get_global_stats();
        assert_eq!(stats.total_trades, 0);
        assert_eq!(stats.active_exchanges, 0);
        
        // 模拟一个事件
        let event = Event::new_with_exchange(
            EventType::Trade(json!({
                "price": "108000.0",
                "quantity": "0.1",
                "side": "buy"
            })),
            "websocket".to_string(),
            "binance".to_string()
        );
        
        basic_layer.handle_event(&event);
        
        // 验证统计信息更新
        let stats = basic_layer.get_global_stats();
        assert_eq!(stats.total_trades, 1);
        assert_eq!(stats.active_exchanges, 1);
        
        // 验证交易所数据
        let active_exchanges = basic_layer.get_active_exchanges();
        assert!(active_exchanges.contains(&"binance".to_string()));
        
        if let Some(manager) = basic_layer.get_exchange_manager("binance") {
            assert_eq!(manager.get_trades_count(), 1);
        }
    }
    
    #[test]
    fn test_multiple_exchanges() {
        let mut basic_layer = BasicLayer::new();
        
        let exchanges = vec!["binance", "okx", "bybit"];
        
        for exchange in &exchanges {
            let event = Event::new_with_exchange(
                EventType::Trade(json!({
                    "price": "108000.0",
                    "quantity": "0.1",
                    "side": "buy"
                })),
                "websocket".to_string(),
                exchange.to_string()
            );
            
            basic_layer.handle_event(&event);
        }
        
        let stats = basic_layer.get_global_stats();
        assert_eq!(stats.total_trades, 3);
        assert_eq!(stats.active_exchanges, 3);
        
        let active_exchanges = basic_layer.get_active_exchanges();
        for exchange in &exchanges {
            assert!(active_exchanges.contains(&exchange.to_string()));
        }
    }
    
    #[test]
    fn test_trades_window_limit() {
        let mut basic_layer = BasicLayer::new();
        
        // 添加超过10000条成交记录
        for i in 0..10005 {
            let event = Event::new_with_exchange(
                EventType::Trade(json!({
                    "price": (108000.0 + i as f64 * 0.01).to_string(),
                    "quantity": "0.001",
                    "side": if i % 2 == 0 { "buy" } else { "sell" }
                })),
                "websocket".to_string(),
                "binance".to_string()
            );
            
            basic_layer.handle_event(&event);
        }
        
        // 验证滑动窗口限制
        if let Some(manager) = basic_layer.get_exchange_manager("binance") {
            assert_eq!(manager.get_trades_count(), 10000);
        }
    }
} 