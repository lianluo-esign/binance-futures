use crate::events::{Event, EventType};
use crate::handlers::HandlerContext;

/// 处理WebSocket错误事件
pub fn handle_websocket_error(event: &Event, context: &HandlerContext) {
    if let EventType::WebSocketError(error_msg) = &event.event_type {
        // WebSocket错误是关键错误，写入日志文件但不输出到控制台
        log::error!("WebSocket错误: {}", error_msg);
        crate::handlers::global::increment_event_count("websocket_error");
        
        // 根据错误类型采取不同的处理策略
        if error_msg.contains("connection") {
            handle_connection_error(error_msg, context);
        } else if error_msg.contains("timeout") {
            handle_timeout_error(error_msg, context);
        } else if error_msg.contains("authentication") {
            handle_auth_error(error_msg, context);
        } else {
            handle_generic_error(error_msg, context);
        }
    }
}

/// 处理连接错误
fn handle_connection_error(error_msg: &str, context: &HandlerContext) {
    // 连接错误记录到日志文件，不输出到控制台
    log::warn!("处理连接错误: {}", error_msg);
    crate::handlers::global::increment_event_count("connection_error_handled");
    
    // 连接错误通常需要重连
    // 这里可以发布重连事件或直接执行重连逻辑
    
    // 记录错误统计
    increment_error_counter("connection_error");
    
    // 如果连接错误频繁发生，可能需要暂停交易
    if get_error_count("connection_error") > 5 {
        // 关键错误写入日志文件
        log::error!("连接错误过于频繁，暂停交易");
        
        let risk_event = Event::new(
            EventType::RiskEvent(serde_json::json!({
                "risk_type": "connection_instability",
                "error_count": get_error_count("connection_error"),
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
                "description": "连接不稳定，建议暂停交易"
            })),
            "error_monitor".to_string()
        );
        context.publish_event(risk_event);
    }
}

/// 处理超时错误
fn handle_timeout_error(error_msg: &str, context: &HandlerContext) {
    // 超时错误记录到日志文件，不输出到控制台
    log::warn!("处理超时错误: {}", error_msg);
    crate::handlers::global::increment_event_count("timeout_error_handled");
    
    // 超时错误可能是网络延迟导致的
    increment_error_counter("timeout_error");
    
    // 检查网络延迟是否过高
    if get_error_count("timeout_error") > 3 {
        // 网络延迟警告写入日志文件
        log::warn!("网络延迟过高，可能影响交易执行");
        
        let warning_event = Event::new(
            EventType::RiskEvent(serde_json::json!({
                "risk_type": "high_latency",
                "timeout_count": get_error_count("timeout_error"),
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
                "description": "网络延迟过高，可能影响交易时机"
            })),
            "latency_monitor".to_string()
        );
        context.publish_event(warning_event);
    }
}

/// 处理认证错误
fn handle_auth_error(error_msg: &str, context: &HandlerContext) {
    // 认证错误是关键错误，写入日志文件
    log::error!("处理认证错误: {}", error_msg);
    crate::handlers::global::increment_event_count("auth_error_handled");
    
    // 认证错误是严重问题，需要立即停止交易
    increment_error_counter("auth_error");
    
    let critical_event = Event::new(
        EventType::RiskEvent(serde_json::json!({
            "risk_type": "authentication_failure",
            "error_message": error_msg,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            "description": "认证失败，立即停止所有交易操作"
        })),
        "auth_monitor".to_string()
    );
    context.publish_event(critical_event);
}

/// 处理通用错误
fn handle_generic_error(_error_msg: &str, context: &HandlerContext) {
    // 通用错误记录统计，不输出到控制台
    crate::handlers::global::increment_event_count("generic_error_handled");
    
    increment_error_counter("generic_error");
    
    // 对于通用错误，记录日志并监控频率
    if get_error_count("generic_error") > 10 {
        // 高频错误警告写入日志文件
        log::warn!("通用错误频率过高，需要关注");
        
        let monitoring_event = Event::new(
            EventType::RiskEvent(serde_json::json!({
                "risk_type": "high_error_rate",
                "error_count": get_error_count("generic_error"),
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
                "description": "错误频率过高，建议检查系统状态"
            })),
            "error_monitor".to_string()
        );
        context.publish_event(monitoring_event);
    }
}

// 错误统计相关函数（简化实现）
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

lazy_static::lazy_static! {
    static ref ERROR_COUNTERS: Arc<Mutex<HashMap<String, u32>>> = Arc::new(Mutex::new(HashMap::new()));
}

fn increment_error_counter(error_type: &str) {
    if let Ok(mut counters) = ERROR_COUNTERS.lock() {
        *counters.entry(error_type.to_string()).or_insert(0) += 1;
    }
}

fn get_error_count(error_type: &str) -> u32 {
    if let Ok(counters) = ERROR_COUNTERS.lock() {
        counters.get(error_type).copied().unwrap_or(0)
    } else {
        0
    }
}

/// 重置错误计数器（可以定期调用以避免计数器无限增长）
pub fn reset_error_counters() {
    if let Ok(mut counters) = ERROR_COUNTERS.lock() {
        counters.clear();
    }
}

/// 获取所有错误统计信息
pub fn get_error_stats() -> HashMap<String, u32> {
    if let Ok(counters) = ERROR_COUNTERS.lock() {
        counters.clone()
    } else {
        HashMap::new()
    }
}
