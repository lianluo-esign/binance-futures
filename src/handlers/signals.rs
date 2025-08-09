use crate::events::{Event, EventType};
use crate::handlers::HandlerContext;
use serde_json::Value;

/// 处理信号事件
pub fn handle_signal(event: &Event, context: &HandlerContext) {
    if let EventType::Signal(data) = &event.event_type {
        if let Some(signal_type) = data["signal_type"].as_str() {
            match signal_type {
                "imbalance" => handle_imbalance_signal(data, context),
                "order_impact" => handle_order_impact_signal(data, context),
                "price_move" => handle_price_move_signal(data, context),
                "big_trade" => handle_big_trade_signal(data, context),
                "spread_anomaly" => handle_spread_anomaly_signal(data, context),
                _ => {
                    // 记录未知信号统计，不输出到控制台
                    crate::handlers::global::increment_event_count("unknown_signal_type");
                }
            }
        }
    }
}

/// 处理不平衡信号
fn handle_imbalance_signal(data: &Value, context: &HandlerContext) {
    if let (Some(direction), Some(ratio)) = (
        data["direction"].as_str(),
        data["ratio"].as_f64()
    ) {
        // 记录不平衡信号统计，不输出到控制台
        crate::handlers::global::increment_event_count(&format!("imbalance_signal_{}", direction));

        // 根据不平衡信号生成交易建议
        if ratio > 0.8 {  // 强烈不平衡
            let trading_signal = create_trading_signal(direction, "strong", ratio);
            let trading_event = Event::new(
                EventType::OrderRequest(trading_signal),
                "signal_processor".to_string()
            );
            context.publish_event(trading_event);
        }
    }
}

/// 处理订单冲击信号
fn handle_order_impact_signal(data: &Value, context: &HandlerContext) {
    if let (Some(direction), Some(impact_ratio)) = (
        data["direction"].as_str(),
        data["impact_ratio"].as_f64()
    ) {
        // 记录订单冲击信号统计，不输出到控制台
        crate::handlers::global::increment_event_count(&format!("order_impact_signal_{}", direction));

        // 订单冲击可能预示着价格变动
        if impact_ratio > 1.5 {  // 冲击比率超过1.5倍
            let momentum_signal = create_momentum_signal(direction, impact_ratio);
            let signal_event = Event::new(
                EventType::Signal(momentum_signal),
                "impact_analyzer".to_string()
            );
            context.publish_event(signal_event);
        }
    }
}

/// 处理价格变动信号
fn handle_price_move_signal(data: &Value, _context: &HandlerContext) {
    if let Some(_price) = data["price"].as_f64() {
        // 记录价格变动信号统计，不输出到控制台
        crate::handlers::global::increment_event_count("price_move_signal");
        
        // 可以在这里添加价格变动的后续处理逻辑
        // 例如：调整止损、更新仓位等
    }
}

/// 处理大额交易信号
fn handle_big_trade_signal(data: &Value, context: &HandlerContext) {
    if let (Some(_quantity), Some(_price), Some(value)) = (
        data["quantity"].as_f64(),
        data["price"].as_f64(),
        data["value"].as_f64()
    ) {
        // 记录大额交易信号统计，不输出到控制台
        crate::handlers::global::increment_event_count("big_trade_signal");
        
        // 大额交易可能影响市场，生成风险事件
        if value > 500000.0 {  // 超过50万的交易
            let risk_signal = create_risk_signal("big_trade", value);
            let risk_event = Event::new(
                EventType::RiskEvent(risk_signal),
                "risk_monitor".to_string()
            );
            context.publish_event(risk_event);
        }
    }
}

/// 处理价差异常信号
fn handle_spread_anomaly_signal(data: &Value, context: &HandlerContext) {
    if let Some(spread_pct) = data["spread_percentage"].as_f64() {
        // 记录价差异常信号统计，不输出到控制台
        crate::handlers::global::increment_event_count("spread_anomaly_signal");
        
        // 价差异常可能影响交易执行，生成风险事件
        if spread_pct > 0.5 {  // 价差超过0.5%
            let risk_signal = create_risk_signal("spread_anomaly", spread_pct);
            let risk_event = Event::new(
                EventType::RiskEvent(risk_signal),
                "spread_monitor".to_string()
            );
            context.publish_event(risk_event);
        }
    }
}

// 辅助函数

fn create_trading_signal(direction: &str, strength: &str, ratio: f64) -> Value {
    serde_json::json!({
        "signal_type": "trading_recommendation",
        "direction": direction,
        "strength": strength,
        "confidence": ratio,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        "description": format!("{}交易建议，强度: {}, 置信度: {:.4}", 
            match direction {
                "bull" => "多头",
                "bear" => "空头",
                _ => "未知"
            }, 
            strength, 
            ratio
        )
    })
}

fn create_momentum_signal(direction: &str, impact_ratio: f64) -> Value {
    serde_json::json!({
        "signal_type": "momentum",
        "direction": direction,
        "impact_ratio": impact_ratio,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        "description": format!("{}动量信号，冲击比率: {:.4}", 
            match direction {
                "buy" => "买入",
                "sell" => "卖出",
                _ => "未知"
            }, 
            impact_ratio
        )
    })
}

fn create_risk_signal(risk_type: &str, value: f64) -> Value {
    serde_json::json!({
        "risk_type": risk_type,
        "value": value,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        "description": format!("风险事件: {}, 值: {:.2}", risk_type, value)
    })
}
