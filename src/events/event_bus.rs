use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::core::RingBuffer;
use super::event_types::{Event, EventType, EventPriority};

/// 事件处理器类型定义
pub type EventHandler = Box<dyn Fn(&Event) + Send + Sync>;
pub type AsyncEventHandler = Box<dyn Fn(&Event) -> Box<dyn std::future::Future<Output = ()> + Send> + Send + Sync>;

/// 事件总线 - 在RingBuffer基础上构建的抽象层
pub struct EventBus {
    // 使用RingBuffer作为底层存储
    events: RingBuffer<Event>,
    // 同步事件处理器
    sync_handlers: HashMap<String, Vec<EventHandler>>,
    // 异步事件处理器
    async_handlers: HashMap<String, Vec<AsyncEventHandler>>,
    // 全局事件处理器（处理所有事件）
    global_handlers: Vec<EventHandler>,
    // 事件过滤器
    filters: Vec<Box<dyn Fn(&Event) -> bool + Send + Sync>>,
    // 统计信息
    stats: EventBusStats,
}

#[derive(Debug, Default, Clone)]
pub struct EventBusStats {
    pub total_events_published: u64,
    pub total_events_processed: u64,
    pub events_dropped: u64,
    pub handler_errors: u64,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        Self {
            events: RingBuffer::new(capacity),
            sync_handlers: HashMap::new(),
            async_handlers: HashMap::new(),
            global_handlers: Vec::new(),
            filters: Vec::new(),
            stats: EventBusStats::default(),
        }
    }

    /// 发布事件到总线
    pub fn publish(&mut self, event: Event) {
        self.stats.total_events_published += 1;

        // 应用过滤器
        for filter in &self.filters {
            if !filter(&event) {
                self.stats.events_dropped += 1;
                return;
            }
        }

        // 将事件添加到环形缓冲区，等待统一处理
        // 这样可以保证事件的严格顺序性
        if !self.events.push(event) {
            self.stats.events_dropped += 1;
        }
    }

    /// 批量发布事件
    pub fn publish_batch(&mut self, events: Vec<Event>) {
        for event in events {
            self.publish(event);
        }
    }

    /// 订阅同步事件处理器
    pub fn subscribe<F>(&mut self, event_type: &str, handler: F)
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        let handlers = self.sync_handlers.entry(event_type.to_string()).or_insert_with(Vec::new);
        handlers.push(Box::new(handler));
    }

    /// 订阅异步事件处理器
    pub fn subscribe_async<F, Fut>(&mut self, event_type: &str, handler: F)
    where
        F: Fn(&Event) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let handlers = self.async_handlers.entry(event_type.to_string()).or_insert_with(Vec::new);
        handlers.push(Box::new(move |event| Box::new(handler(event))));
    }

    /// 订阅全局事件处理器（处理所有事件）
    pub fn subscribe_global<F>(&mut self, handler: F)
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        self.global_handlers.push(Box::new(handler));
    }

    /// 添加事件过滤器
    pub fn add_filter<F>(&mut self, filter: F)
    where
        F: Fn(&Event) -> bool + Send + Sync + 'static,
    {
        self.filters.push(Box::new(filter));
    }

    /// 处理单个事件 - 同步处理保证顺序性
    pub fn process_next_event(&mut self) -> bool {
        if let Some(event) = self.events.pop() {
            self.process_event_sync(&event);
            self.stats.total_events_processed += 1;
            true
        } else {
            false
        }
    }

    /// 处理所有待处理事件
    pub fn process_all_events(&mut self) {
        while self.process_next_event() {
            // 继续处理直到没有更多事件
        }
    }

    /// 处理指定数量的事件
    pub fn process_events(&mut self, max_events: usize) -> usize {
        let mut processed = 0;
        for _ in 0..max_events {
            if self.process_next_event() {
                processed += 1;
            } else {
                break;
            }
        }
        processed
    }

    /// 同步处理单个事件
    fn process_event_sync(&mut self, event: &Event) {
        let event_type = event.event_type.type_name();

        // 处理全局处理器
        for handler in &self.global_handlers {
            if let Err(_) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                handler(event);
            })) {
                self.stats.handler_errors += 1;
            }
        }

        // 处理特定类型的同步处理器
        if let Some(handlers) = self.sync_handlers.get(event_type) {
            for handler in handlers {
                if let Err(_) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    handler(event);
                })) {
                    self.stats.handler_errors += 1;
                }
            }
        }
    }

    /// 获取待处理事件数量
    pub fn pending_events(&self) -> usize {
        self.events.len()
    }

    /// 获取事件总线容量
    pub fn capacity(&self) -> usize {
        self.events.capacity()
    }

    /// 检查是否有待处理事件
    pub fn has_pending_events(&self) -> bool {
        !self.events.is_empty()
    }

    /// 清空所有待处理事件
    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    /// 获取统计信息
    pub fn stats(&self) -> &EventBusStats {
        &self.stats
    }

    /// 重置统计信息
    pub fn reset_stats(&mut self) {
        self.stats = EventBusStats::default();
    }
}

// 实现Send和Sync，使EventBus可以在多线程环境中使用
unsafe impl Send for EventBus {}
unsafe impl Sync for EventBus {}
