use crate::events::{Event, EventType};
use crate::handlers::HandlerContext;
use serde_json::Value;

/// 处理Tick价格事件
pub fn handle_tick_price(event: &Event, context: &HandlerContext) {
    if let EventType::TickPrice(data) = &event.event_type {
        // 处理tick价格数据
        if let Some(price) = data["price"].as_f64() {
            // 不输出到控制台，只记录统计
            crate::handlers::global::increment_event_count("tick_price_processed");
            
            // 可以在这里添加价格分析逻辑
            // 例如：检测价格突破、计算移动平均等
            
            // 如果检测到重要信号，可以发布新事件
            if is_significant_price_move(price, data) {
                let signal_event = Event::new(
                    EventType::Signal(create_price_signal(price, data)),
                    "price_analyzer".to_string()
                );
                context.publish_event(signal_event);
            }
        }
    }
}

/// 处理深度更新事件
pub fn handle_depth_update(event: &Event, context: &HandlerContext) {
    if let EventType::DepthUpdate(data) = &event.event_type {
        // 不输出到控制台，只记录统计
        crate::handlers::global::increment_event_count("depth_update_processed");
        
        // 处理订单簿深度数据
        // 可以在这里分析订单流、检测大单等
        
        // 检测订单簿不平衡
        if let Some(imbalance_signal) = detect_order_book_imbalance(data) {
            let signal_event = Event::new(
                EventType::Signal(imbalance_signal),
                "depth_analyzer".to_string()
            );
            context.publish_event(signal_event);
        }
    }
}

/// 处理交易事件
pub fn handle_trade(event: &Event, context: &HandlerContext) {
    if let EventType::Trade(data) = &event.event_type {
        // 不输出到控制台，只记录统计
        crate::handlers::global::increment_event_count("trade_processed");
        
        // 处理交易数据
        // 可以在这里分析交易量、检测异常交易等
        
        // 检测大额交易
        if let Some(big_trade_signal) = detect_big_trade(data) {
            let signal_event = Event::new(
                EventType::Signal(big_trade_signal),
                "trade_analyzer".to_string()
            );
            context.publish_event(signal_event);
        }
    }
}

/// 处理BookTicker事件
pub fn handle_book_ticker(event: &Event, context: &HandlerContext) {
    if let EventType::BookTicker(data) = &event.event_type {
        // 不输出到控制台，只记录统计
        crate::handlers::global::increment_event_count("book_ticker_processed");
        
        // 处理最优买卖价数据
        // 可以在这里计算价差、检测流动性变化等
        
        // 检测价差异常
        if let Some(spread_signal) = detect_spread_anomaly(data) {
            let signal_event = Event::new(
                EventType::Signal(spread_signal),
                "spread_analyzer".to_string()
            );
            context.publish_event(signal_event);
        }
    }
}

// 辅助函数

fn is_significant_price_move(_price: f64, _data: &Value) -> bool {
    // 实现价格变动检测逻辑
    // 这里只是示例，实际实现需要根据具体需求
    false
}

fn create_price_signal(price: f64, _data: &Value) -> Value {
    serde_json::json!({
        "signal_type": "price_move",
        "price": price,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        "description": "Significant price movement detected"
    })
}

fn detect_order_book_imbalance(_data: &Value) -> Option<Value> {
    // 实现订单簿不平衡检测逻辑
    // 返回None表示没有检测到不平衡
    None
}

fn detect_big_trade(data: &Value) -> Option<Value> {
    // 实现大额交易检测逻辑
    if let (Some(quantity), Some(price)) = (data["q"].as_str(), data["p"].as_str()) {
        if let (Ok(qty), Ok(px)) = (quantity.parse::<f64>(), price.parse::<f64>()) {
            let trade_value = qty * px;
            
            // 假设交易额超过100000为大额交易
            if trade_value > 100000.0 {
                return Some(serde_json::json!({
                    "signal_type": "big_trade",
                    "quantity": qty,
                    "price": px,
                    "value": trade_value,
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                    "description": format!("Big trade detected: {} @ {}", qty, px)
                }));
            }
        }
    }
    None
}

fn detect_spread_anomaly(data: &Value) -> Option<Value> {
    // 实现价差异常检测逻辑
    if let (Some(bid_str), Some(ask_str)) = (data["b"].as_str(), data["a"].as_str()) {
        if let (Ok(bid), Ok(ask)) = (bid_str.parse::<f64>(), ask_str.parse::<f64>()) {
            let spread = ask - bid;
            let spread_pct = spread / bid * 100.0;
            
            // 假设价差超过0.1%为异常
            if spread_pct > 0.1 {
                return Some(serde_json::json!({
                    "signal_type": "spread_anomaly",
                    "bid": bid,
                    "ask": ask,
                    "spread": spread,
                    "spread_percentage": spread_pct,
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                    "description": format!("Abnormal spread detected: {:.4}%", spread_pct)
                }));
            }
        }
    }
    None
}
