use crate::events::{Event, EventType, EventPriority};
use crate::handlers::HandlerContext;

/// 全局事件处理器 - 处理所有事件的通用逻辑
pub fn handle_global_event(event: &Event, context: &HandlerContext) {
    // 记录事件日志
    log_event(event);
    
    // 更新事件统计
    update_event_statistics(event);
    
    // 检查事件优先级并采取相应行动
    handle_event_priority(event, context);
    
    // 检查事件是否过期
    check_event_expiration(event);
    
    // 性能监控
    monitor_event_processing_performance(event);
}

/// 记录事件统计（不输出到控制台）
fn log_event(event: &Event) {
    // 只记录统计信息，不输出到控制台以避免干扰UI
    increment_event_count(&format!("{:?}_{}", event.event_type.type_name(), event.priority.as_str()));

    // 只有关键错误才写入日志文件
    if matches!(event.priority, EventPriority::Critical) {
        // 这些会写入日志文件而不是控制台
        log::error!("[CRITICAL] 事件: {} 来源: {} 时间: {}",
            event.event_type.type_name(),
            event.source,
            event.timestamp
        );
    }
}

/// 更新事件统计信息
fn update_event_statistics(event: &Event) {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    
    lazy_static::lazy_static! {
        static ref EVENT_STATS: Arc<Mutex<HashMap<String, EventStats>>> = 
            Arc::new(Mutex::new(HashMap::new()));
    }
    
    if let Ok(mut stats) = EVENT_STATS.lock() {
        let event_type = event.event_type.type_name();
        let stat = stats.entry(event_type.to_string()).or_insert_with(EventStats::new);
        
        stat.total_count += 1;
        stat.last_seen = event.timestamp;
        
        match event.priority {
            EventPriority::Critical => stat.critical_count += 1,
            EventPriority::High => stat.high_count += 1,
            EventPriority::Normal => stat.normal_count += 1,
            EventPriority::Low => stat.low_count += 1,
        }
        
        // 更新来源统计
        *stat.sources.entry(event.source.clone()).or_insert(0) += 1;
    }
}

/// 处理事件优先级
fn handle_event_priority(event: &Event, context: &HandlerContext) {
    match event.priority {
        EventPriority::Critical => {
            // 关键事件需要立即处理和通知
            handle_critical_event(event, context);
        }
        EventPriority::High => {
            // 高优先级事件需要快速处理
            handle_high_priority_event(event, context);
        }
        _ => {
            // 普通和低优先级事件按正常流程处理
        }
    }
}

/// 处理关键事件
fn handle_critical_event(event: &Event, _context: &HandlerContext) {
    // 只记录到日志文件，不输出到控制台
    log::error!("处理关键事件: {:?}", event);

    // 关键事件可能需要立即停止某些操作
    match &event.event_type {
        EventType::WebSocketError(_) => {
            // WebSocket错误可能需要重连或停止交易
            log::error!("关键WebSocket错误，考虑停止交易");
        }
        EventType::RiskEvent(_) => {
            // 风险事件需要立即响应
            log::error!("关键风险事件，立即执行风险控制措施");
        }
        _ => {}
    }
}

/// 处理高优先级事件
fn handle_high_priority_event(event: &Event, _context: &HandlerContext) {
    // 不输出到控制台，只记录统计
    increment_event_count(&format!("high_priority_{}", event.event_type.type_name()));

    // 高优先级事件需要快速响应
    match &event.event_type {
        EventType::Signal(_) => {
            // 信号事件需要快速处理以抓住交易机会
            increment_event_count("high_priority_signal_processed");
        }
        EventType::OrderRequest(_) | EventType::OrderCancel(_) => {
            // 交易相关事件需要快速执行
            increment_event_count("high_priority_trading_processed");
        }
        _ => {}
    }
}

/// 检查事件过期
fn check_event_expiration(event: &Event) -> bool {
    // 检查不同类型事件的过期时间
    let max_age = match &event.event_type {
        EventType::TickPrice(_) | EventType::Trade(_) => 1000,  // 1秒
        EventType::DepthUpdate(_) | EventType::BookTicker(_) => 2000,  // 2秒
        EventType::Signal(_) => 5000,  // 5秒
        EventType::OrderRequest(_) => 10000,  // 10秒
        _ => 30000,  // 30秒
    };
    
    if event.is_expired(max_age) {
        // 记录过期事件统计，不输出到控制台
        increment_event_count("expired_events");
        return false;
    }

    true
}

/// 监控事件处理性能
fn monitor_event_processing_performance(event: &Event) {
    use std::sync::{Arc, Mutex};
    
    lazy_static::lazy_static! {
        static ref PERFORMANCE_MONITOR: Arc<Mutex<PerformanceMonitor>> = 
            Arc::new(Mutex::new(PerformanceMonitor::new()));
    }
    
    if let Ok(mut monitor) = PERFORMANCE_MONITOR.lock() {
        monitor.record_event_processing(event);
    }
}

// 辅助数据结构

#[derive(Debug, Clone)]
struct EventStats {
    total_count: u64,
    critical_count: u64,
    high_count: u64,
    normal_count: u64,
    low_count: u64,
    last_seen: u64,
    sources: std::collections::HashMap<String, u64>,
}

impl EventStats {
    fn new() -> Self {
        Self {
            total_count: 0,
            critical_count: 0,
            high_count: 0,
            normal_count: 0,
            low_count: 0,
            last_seen: 0,
            sources: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug)]
struct PerformanceMonitor {
    event_processing_times: std::collections::VecDeque<(String, u64)>,
    max_records: usize,
}

impl PerformanceMonitor {
    fn new() -> Self {
        Self {
            event_processing_times: std::collections::VecDeque::new(),
            max_records: 1000,
        }
    }
    
    fn record_event_processing(&mut self, event: &Event) {
        let processing_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64 - event.timestamp;
        
        self.event_processing_times.push_back((
            event.event_type.type_name().to_string(),
            processing_time
        ));
        
        // 保持记录数量在限制内
        if self.event_processing_times.len() > self.max_records {
            self.event_processing_times.pop_front();
        }
        
        // 如果处理时间过长，记录统计而不是输出到控制台
        if processing_time > 100 {  // 超过100ms
            increment_event_count("slow_processing_events");
            // 只有非常慢的事件才写入日志文件
            if processing_time > 1000 {  // 超过1秒
                log::warn!("事件处理时间过长: {} 耗时: {}ms",
                    event.event_type.type_name(),
                    processing_time
                );
            }
        }
    }
}

/// 增加事件计数器
pub fn increment_event_count(event_name: &str) {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    lazy_static::lazy_static! {
        static ref EVENT_COUNTERS: Arc<Mutex<HashMap<String, u64>>> =
            Arc::new(Mutex::new(HashMap::new()));
    }

    if let Ok(mut counters) = EVENT_COUNTERS.lock() {
        *counters.entry(event_name.to_string()).or_insert(0) += 1;
    }
}

/// 获取事件计数器
pub fn get_event_count(event_name: &str) -> u64 {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    lazy_static::lazy_static! {
        static ref EVENT_COUNTERS: Arc<Mutex<HashMap<String, u64>>> =
            Arc::new(Mutex::new(HashMap::new()));
    }

    if let Ok(counters) = EVENT_COUNTERS.lock() {
        counters.get(event_name).copied().unwrap_or(0)
    } else {
        0
    }
}

/// 获取事件统计信息（用于监控和调试）
pub fn get_event_statistics() -> std::collections::HashMap<String, EventStats> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    lazy_static::lazy_static! {
        static ref EVENT_STATS: Arc<Mutex<HashMap<String, EventStats>>> =
            Arc::new(Mutex::new(HashMap::new()));
    }

    if let Ok(stats) = EVENT_STATS.lock() {
        stats.clone()
    } else {
        HashMap::new()
    }
}
