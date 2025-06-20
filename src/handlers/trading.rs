use crate::events::{Event, EventType};
use crate::handlers::HandlerContext;
use serde_json::Value;

/// 处理订单请求事件
pub fn handle_order_request(event: &Event, context: &HandlerContext) {
    if let EventType::OrderRequest(data) = &event.event_type {
        // 记录订单请求统计，不输出到控制台
        crate::handlers::global::increment_event_count("order_request_processed");
        
        // 在这里实现订单处理逻辑
        if let Some(signal_type) = data["signal_type"].as_str() {
            match signal_type {
                "trading_recommendation" => handle_trading_recommendation(data, context),
                _ => {
                    // 记录未知订单请求类型统计，不输出到控制台
                    crate::handlers::global::increment_event_count("unknown_order_request_type");
                }
            }
        }
    }
}

/// 处理仓位更新事件
pub fn handle_position_update(event: &Event, context: &HandlerContext) {
    if let EventType::PositionUpdate(data) = &event.event_type {
        // 记录仓位更新统计，不输出到控制台
        crate::handlers::global::increment_event_count("position_update_processed");
        
        // 在这里实现仓位更新逻辑
        // 例如：更新风险管理参数、调整止损止盈等
        
        if let Some(position_size) = data["size"].as_f64() {
            // 检查仓位是否超过风险限制
            if position_size.abs() > get_max_position_size() {
                let risk_event = Event::new(
                    EventType::RiskEvent(create_position_risk_signal(position_size)),
                    "position_monitor".to_string()
                );
                context.publish_event(risk_event);
            }
        }
    }
}

/// 处理订单取消事件
pub fn handle_order_cancel(event: &Event, context: &HandlerContext) {
    if let EventType::OrderCancel(data) = &event.event_type {
        // 记录订单取消统计，不输出到控制台
        crate::handlers::global::increment_event_count("order_cancel_processed");
        
        // 在这里实现订单取消逻辑
        if let Some(order_id) = data["order_id"].as_str() {
            // 执行订单取消操作
            if cancel_order(order_id) {
                crate::handlers::global::increment_event_count("order_cancel_success");
            } else {
                crate::handlers::global::increment_event_count("order_cancel_failed");
                // 只有失败才写入日志文件
                log::error!("订单 {} 取消失败", order_id);
                
                // 发布错误事件
                let error_event = Event::new(
                    EventType::WebSocketError(format!("订单取消失败: {}", order_id)),
                    "order_manager".to_string()
                );
                context.publish_event(error_event);
            }
        }
    }
}

// 辅助函数

fn handle_trading_recommendation(data: &Value, context: &HandlerContext) {
    if let (Some(direction), Some(strength), Some(confidence)) = (
        data["direction"].as_str(),
        data["strength"].as_str(),
        data["confidence"].as_f64()
    ) {
        // 记录交易建议统计，不输出到控制台
        crate::handlers::global::increment_event_count(&format!("trading_recommendation_{}_{}", direction, strength));
        
        // 根据交易建议执行相应操作
        match strength {
            "strong" if confidence > 0.8 => {
                // 高置信度的强信号，执行交易
                execute_trade(direction, calculate_position_size(confidence));
            }
            "medium" if confidence > 0.6 => {
                // 中等信号，执行较小仓位的交易
                execute_trade(direction, calculate_position_size(confidence) * 0.5);
            }
            _ => {
                // 记录信号不足统计，不输出到控制台
                crate::handlers::global::increment_event_count("trading_signal_insufficient");
            }
        }
    }
}

fn execute_trade(direction: &str, size: f64) {
    // 记录交易执行统计，不输出到控制台
    crate::handlers::global::increment_event_count(&format!("trade_executed_{}", direction));
    
    // 在这里实现实际的交易执行逻辑
    // 例如：调用交易所API、更新本地仓位记录等
    
    // 模拟交易执行
    let success = simulate_trade_execution(direction, size);
    
    if success {
        crate::handlers::global::increment_event_count("trade_execution_success");
    } else {
        crate::handlers::global::increment_event_count("trade_execution_failed");
        // 只有失败才写入日志文件
        log::error!("交易执行失败");
    }
}

fn calculate_position_size(confidence: f64) -> f64 {
    // 根据置信度计算仓位大小
    // 这里使用简单的线性关系，实际应用中可能需要更复杂的算法
    let base_size = 1000.0; // 基础仓位大小
    let max_multiplier = 3.0; // 最大倍数
    
    base_size * (confidence * max_multiplier).min(max_multiplier)
}

fn get_max_position_size() -> f64 {
    // 返回最大允许的仓位大小
    10000.0
}

fn cancel_order(order_id: &str) -> bool {
    // 模拟订单取消操作，不输出到控制台
    crate::handlers::global::increment_event_count("order_cancel_attempt");
    
    // 在实际应用中，这里会调用交易所API
    // 现在只是返回模拟结果
    true
}

fn simulate_trade_execution(direction: &str, size: f64) -> bool {
    // 模拟交易执行
    // 在实际应用中，这里会调用交易所API执行实际交易
    
    // 简单的成功率模拟（90%成功率）
    use rand::Rng;
    let mut rng = rand::thread_rng();
    rng.gen::<f64>() < 0.9
}

fn create_position_risk_signal(position_size: f64) -> Value {
    serde_json::json!({
        "risk_type": "position_limit_exceeded",
        "position_size": position_size,
        "max_position_size": get_max_position_size(),
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        "description": format!("仓位超限: 当前={:.2}, 最大={:.2}", position_size, get_max_position_size())
    })
}
