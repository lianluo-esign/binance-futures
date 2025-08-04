use super::{Service, ServiceError, ServiceHealth, ServiceStats};
use crate::events::{Event, LockFreeEventDispatcher};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::time::Instant;

/// 事件服务 - 负责统一的事件处理
pub struct EventService {
    /// 运行状态
    is_running: AtomicBool,
    /// 启动时间
    start_time: Option<Instant>,
    /// 事件分发器
    dispatcher: Arc<LockFreeEventDispatcher>,
    /// 统计信息
    stats: EventServiceStats,
}

#[derive(Debug)]
struct EventServiceStats {
    events_processed: AtomicU64,
    events_published: AtomicU64,
    error_count: AtomicU64,
}

impl Default for EventServiceStats {
    fn default() -> Self {
        Self {
            events_processed: AtomicU64::new(0),
            events_published: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
        }
    }
}

impl EventService {
    pub fn new(capacity: usize) -> Self {
        Self {
            is_running: AtomicBool::new(false),
            start_time: None,
            dispatcher: Arc::new(LockFreeEventDispatcher::new(capacity)),
            stats: EventServiceStats::default(),
        }
    }

    pub async fn publish_event(&self, event: Event) -> Result<(), ServiceError> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        if self.dispatcher.publish(event) {
            self.stats.events_published.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            self.stats.error_count.fetch_add(1, Ordering::Relaxed);
            Err(ServiceError::InternalError("事件发布失败".to_string()))
        }
    }

    pub async fn process_events(&self, max_events: usize) -> Result<usize, ServiceError> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        let processed = self.dispatcher.process_events_batch(max_events);
        self.stats.events_processed.fetch_add(processed as u64, Ordering::Relaxed);
        Ok(processed)
    }
}

impl Service for EventService {
    fn name(&self) -> &'static str {
        "EventService"
    }

    fn start(&mut self) -> Result<(), ServiceError> {
        if self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::AlreadyRunning);
        }

        self.is_running.store(true, Ordering::Relaxed);
        self.start_time = Some(Instant::now());
        
        log::info!("事件服务已启动");
        Ok(())
    }

    fn stop(&mut self) -> Result<(), ServiceError> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        self.is_running.store(false, Ordering::Relaxed);
        
        log::info!("事件服务已停止");
        Ok(())
    }

    fn health_check(&self) -> ServiceHealth {
        if !self.is_running.load(Ordering::Relaxed) {
            return ServiceHealth::Unhealthy("事件服务未运行".to_string());
        }

        let pending_events = self.dispatcher.pending_events();
        if pending_events > 5000 {
            ServiceHealth::Warning(format!("待处理事件过多: {}", pending_events))
        } else {
            ServiceHealth::Healthy
        }
    }

    fn stats(&self) -> ServiceStats {
        ServiceStats {
            service_name: self.name().to_string(),
            is_running: self.is_running.load(Ordering::Relaxed),
            start_time: self.start_time,
            requests_processed: self.stats.events_processed.load(Ordering::Relaxed),
            error_count: self.stats.error_count.load(Ordering::Relaxed),
            avg_response_time_ms: 0.0,
            memory_usage_bytes: 0,
        }
    }
}

/// 事件处理器trait
pub trait EventProcessor: Send + Sync {
    fn can_handle(&self, event: &Event) -> bool;
    fn process(&self, event: &Event) -> Result<(), String>;
}