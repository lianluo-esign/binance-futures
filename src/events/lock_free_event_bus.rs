use crate::core::LockFreeRingBuffer;
use crate::events::event_types::Event;
use crate::events::event_bus::EventBusStats;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// 无锁事件总线实现
/// 
/// 使用无锁环形缓冲区替代互斥锁，提供高性能的事件处理能力
/// 支持多生产者单消费者模式，适合高频事件处理场景
pub struct LockFreeEventBus {
    /// 无锁环形缓冲区存储事件
    events: LockFreeRingBuffer<Event>,
    
    /// 同步事件处理器
    sync_handlers: HashMap<String, Vec<Box<dyn Fn(&Event) + Send + Sync>>>,
    
    /// 全局事件处理器（处理所有事件）
    global_handlers: Vec<Box<dyn Fn(&Event) + Send + Sync>>,
    
    /// 事件过滤器
    filters: Vec<Box<dyn Fn(&Event) -> bool + Send + Sync>>,
    
    /// 统计信息（使用原子操作）
    total_events_published: AtomicU64,
    total_events_processed: AtomicU64,
    handler_errors: AtomicU64,
}

impl LockFreeEventBus {
    /// 创建新的无锁事件总线
    /// 
    /// # 参数
    /// * `capacity` - 事件缓冲区容量
    pub fn new(capacity: usize) -> Self {
        Self {
            events: LockFreeRingBuffer::new(capacity),
            sync_handlers: HashMap::new(),
            global_handlers: Vec::new(),
            filters: Vec::new(),
            total_events_published: AtomicU64::new(0),
            total_events_processed: AtomicU64::new(0),
            handler_errors: AtomicU64::new(0),
        }
    }
    
    /// 发布事件到总线
    /// 
    /// 这个方法是无锁的，可以从多个线程并发调用
    /// 
    /// # 参数
    /// * `event` - 要发布的事件
    /// 
    /// # 返回值
    /// * `true` - 事件成功发布
    /// * `false` - 缓冲区已满，事件被丢弃
    pub fn publish(&self, event: Event) -> bool {
        // 应用过滤器
        for filter in &self.filters {
            if !filter(&event) {
                return false; // 事件被过滤器拒绝
            }
        }
        
        // 尝试将事件推入缓冲区
        match self.events.try_push(event) {
            Ok(()) => {
                self.total_events_published.fetch_add(1, Ordering::Relaxed);
                true
            }
            Err(_) => {
                // 缓冲区已满，事件被丢弃
                log::warn!("事件缓冲区已满，丢弃事件");
                false
            }
        }
    }
    
    /// 批量发布事件
    /// 
    /// # 参数
    /// * `events` - 要发布的事件列表
    /// 
    /// # 返回值
    /// 成功发布的事件数量
    pub fn publish_batch(&self, events: Vec<Event>) -> usize {
        let mut published = 0;
        for event in events {
            if self.publish(event) {
                published += 1;
            }
        }
        published
    }
    
    /// 订阅同步事件处理器
    /// 
    /// 注意：这个方法需要可变引用，应该在初始化阶段调用
    pub fn subscribe<F>(&mut self, event_type: &str, handler: F)
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        let handlers = self.sync_handlers.entry(event_type.to_string()).or_insert_with(Vec::new);
        handlers.push(Box::new(handler));
    }
    
    /// 订阅全局事件处理器（处理所有事件）
    /// 
    /// 注意：这个方法需要可变引用，应该在初始化阶段调用
    pub fn subscribe_global<F>(&mut self, handler: F)
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        self.global_handlers.push(Box::new(handler));
    }
    
    /// 添加事件过滤器
    /// 
    /// 注意：这个方法需要可变引用，应该在初始化阶段调用
    pub fn add_filter<F>(&mut self, filter: F)
    where
        F: Fn(&Event) -> bool + Send + Sync + 'static,
    {
        self.filters.push(Box::new(filter));
    }
    
    /// 处理单个事件（非阻塞）
    /// 
    /// # 返回值
    /// * `true` - 成功处理一个事件
    /// * `false` - 没有待处理的事件
    pub fn process_next_event(&self) -> bool {
        if let Some(event) = self.events.try_pop() {
            self.process_event_sync(&event);
            self.total_events_processed.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
    
    /// 处理所有待处理事件
    pub fn process_all_events(&self) {
        while self.process_next_event() {
            // 继续处理直到没有更多事件
        }
    }
    
    /// 处理指定数量的事件
    /// 
    /// # 参数
    /// * `max_events` - 最大处理事件数量
    /// 
    /// # 返回值
    /// 实际处理的事件数量
    pub fn process_events(&self, max_events: usize) -> usize {
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
    
    /// 轮询单个事件（不处理）
    /// 
    /// # 返回值
    /// * `Some(event)` - 获取到事件
    /// * `None` - 没有待处理的事件
    pub fn poll_event(&self) -> Option<Event> {
        self.events.try_pop()
    }
    
    /// 同步处理单个事件
    fn process_event_sync(&self, event: &Event) {
        let event_type = event.event_type.type_name();
        
        // 处理全局处理器
        for handler in &self.global_handlers {
            if let Err(_) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                handler(event);
            })) {
                self.handler_errors.fetch_add(1, Ordering::Relaxed);
            }
        }
        
        // 处理特定类型的同步处理器
        if let Some(handlers) = self.sync_handlers.get(event_type) {
            for handler in handlers {
                if let Err(_) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    handler(event);
                })) {
                    self.handler_errors.fetch_add(1, Ordering::Relaxed);
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
    pub fn clear_events(&self) {
        self.events.clear();
    }
    
    /// 获取统计信息
    pub fn stats(&self) -> EventBusStats {
        EventBusStats {
            total_events_published: self.total_events_published.load(Ordering::Relaxed),
            total_events_processed: self.total_events_processed.load(Ordering::Relaxed),
            events_dropped: 0, // TODO: 实现事件丢弃统计
            handler_errors: self.handler_errors.load(Ordering::Relaxed),
        }
    }
    
    /// 重置统计信息
    pub fn reset_stats(&self) {
        self.total_events_published.store(0, Ordering::Relaxed);
        self.total_events_processed.store(0, Ordering::Relaxed);
        self.handler_errors.store(0, Ordering::Relaxed);
    }
}

// 实现Send和Sync，但需要注意处理器的线程安全性
unsafe impl Send for LockFreeEventBus {}
unsafe impl Sync for LockFreeEventBus {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::event_types::{Event, EventType};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_basic_publish_and_process() {
        let mut bus = LockFreeEventBus::new(100);
        
        // 创建测试事件
        let event = Event::new(
            EventType::DepthUpdate(serde_json::json!({"test": "data"})),
            "test".to_string()
        );
        
        // 发布事件
        assert!(bus.publish(event));
        assert_eq!(bus.pending_events(), 1);
        
        // 处理事件
        assert!(bus.process_next_event());
        assert_eq!(bus.pending_events(), 0);
        
        // 检查统计信息
        let stats = bus.stats();
        assert_eq!(stats.total_events_published, 1);
        assert_eq!(stats.total_events_processed, 1);
    }

    #[test]
    fn test_concurrent_publish() {
        let bus = Arc::new(LockFreeEventBus::new(1000));
        let processed_count = Arc::new(AtomicUsize::new(0));
        
        // 启动多个生产者线程
        let mut handles = Vec::new();
        for i in 0..4 {
            let bus_clone = bus.clone();
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    let event = Event::new(
                        EventType::Trade(serde_json::json!({"thread": i, "index": j})),
                        format!("producer-{}", i)
                    );
                    while !bus_clone.publish(event.clone()) {
                        thread::yield_now();
                    }
                }
            });
            handles.push(handle);
        }
        
        // 启动消费者线程
        let bus_consumer = bus.clone();
        let count_clone = processed_count.clone();
        let consumer = thread::spawn(move || {
            while count_clone.load(Ordering::Relaxed) < 400 {
                if bus_consumer.process_next_event() {
                    count_clone.fetch_add(1, Ordering::Relaxed);
                } else {
                    thread::yield_now();
                }
            }
        });
        
        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }
        consumer.join().unwrap();
        
        // 验证结果
        assert_eq!(processed_count.load(Ordering::Relaxed), 400);
        let stats = bus.stats();
        assert_eq!(stats.total_events_published, 400);
        assert_eq!(stats.total_events_processed, 400);
    }
}
