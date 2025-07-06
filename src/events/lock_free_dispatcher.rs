use crate::events::{Event, LockFreeEventBus};
use crate::events::event_bus::EventBusStats;
use std::sync::Arc;

/// 无锁事件分发器
/// 
/// 使用无锁事件总线提供高性能的事件分发能力
/// 完全无锁设计，避免了互斥锁的性能开销
pub struct LockFreeEventDispatcher {
    /// 无锁事件总线
    event_bus: Arc<LockFreeEventBus>,
}

impl LockFreeEventDispatcher {
    /// 创建新的无锁事件分发器
    /// 
    /// # 参数
    /// * `capacity` - 事件缓冲区容量
    pub fn new(capacity: usize) -> Self {
        Self {
            event_bus: Arc::new(LockFreeEventBus::new(capacity)),
        }
    }
    
    /// 从现有的无锁事件总线创建分发器
    pub fn from_event_bus(event_bus: Arc<LockFreeEventBus>) -> Self {
        Self { event_bus }
    }
    
    /// 获取事件总线的引用（用于注册处理器）
    /// 
    /// 注意：这个方法返回Arc，因为LockFreeEventBus的订阅方法需要可变引用
    /// 在实际使用中，应该在初始化阶段完成所有处理器的注册
    pub fn get_event_bus(&self) -> Arc<LockFreeEventBus> {
        self.event_bus.clone()
    }
    
    /// 发布事件到总线 - 完全无锁
    /// 
    /// # 参数
    /// * `event` - 要发布的事件
    /// 
    /// # 返回值
    /// * `true` - 事件成功发布
    /// * `false` - 缓冲区已满，事件被丢弃
    pub fn publish(&self, event: Event) -> bool {
        self.event_bus.publish(event)
    }
    
    /// 批量发布事件 - 完全无锁
    /// 
    /// # 参数
    /// * `events` - 要发布的事件列表
    /// 
    /// # 返回值
    /// 成功发布的事件数量
    pub fn publish_batch(&self, events: Vec<Event>) -> usize {
        self.event_bus.publish_batch(events)
    }
    
    /// 处理事件循环 - 完全无锁
    /// 
    /// 处理所有待处理的事件，直到队列为空
    pub fn process_events(&self) {
        self.event_bus.process_all_events();
    }
    
    /// 处理指定数量的事件 - 完全无锁
    /// 
    /// # 参数
    /// * `max_events` - 最大处理事件数量
    /// 
    /// # 返回值
    /// 实际处理的事件数量
    pub fn process_events_batch(&self, max_events: usize) -> usize {
        self.event_bus.process_events(max_events)
    }
    
    /// 轮询单个事件 - 完全无锁
    /// 
    /// # 返回值
    /// * `Some(event)` - 获取到事件
    /// * `None` - 没有待处理的事件
    pub fn poll_event(&self) -> Option<Event> {
        self.event_bus.poll_event()
    }
    
    /// 获取待处理事件数量 - 完全无锁
    pub fn pending_events(&self) -> usize {
        self.event_bus.pending_events()
    }
    
    /// 获取事件总线统计信息 - 完全无锁
    pub fn get_stats(&self) -> Option<EventBusStats> {
        Some(self.event_bus.stats())
    }
    
    /// 获取缓冲区使用情况 (当前大小, 最大容量) - 完全无锁
    pub fn get_buffer_usage(&self) -> (usize, usize) {
        (self.event_bus.pending_events(), self.event_bus.capacity())
    }
    
    /// 检查是否有待处理事件 - 完全无锁
    pub fn has_pending_events(&self) -> bool {
        self.event_bus.has_pending_events()
    }
    
    /// 清空所有待处理事件
    pub fn clear_events(&self) {
        self.event_bus.clear_events();
    }
    
    /// 重置统计信息
    pub fn reset_stats(&self) {
        self.event_bus.reset_stats();
    }
}

// 实现Clone以便在多个地方使用同一个分发器
impl Clone for LockFreeEventDispatcher {
    fn clone(&self) -> Self {
        Self {
            event_bus: self.event_bus.clone(),
        }
    }
}

// 实现Send和Sync，使分发器可以在多线程环境中使用
unsafe impl Send for LockFreeEventDispatcher {}
unsafe impl Sync for LockFreeEventDispatcher {}

/// 创建配置好的无锁事件分发器
/// 
/// 这个函数创建一个预配置的事件分发器，包含常用的事件处理器
/// 
/// # 参数
/// * `capacity` - 事件缓冲区容量
/// 
/// # 返回值
/// 配置好的无锁事件分发器
pub fn create_configured_lock_free_dispatcher(capacity: usize) -> LockFreeEventDispatcher {
    let dispatcher = LockFreeEventDispatcher::new(capacity);
    
    // 注意：由于LockFreeEventBus的订阅方法需要可变引用，
    // 在实际使用中需要在创建后立即配置处理器
    // 这里返回未配置的分发器，由调用者负责配置
    
    dispatcher
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::event_types::{Event, EventType};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_basic_dispatch() {
        let dispatcher = LockFreeEventDispatcher::new(100);
        
        // 创建测试事件
        let event = Event::new(
            EventType::DepthUpdate(serde_json::json!({"test": "data"})),
            "test".to_string()
        );
        
        // 发布事件
        assert!(dispatcher.publish(event));
        assert_eq!(dispatcher.pending_events(), 1);
        
        // 处理事件
        let processed = dispatcher.process_events_batch(10);
        assert_eq!(processed, 1);
        assert_eq!(dispatcher.pending_events(), 0);
    }

    #[test]
    fn test_concurrent_dispatch() {
        let dispatcher = Arc::new(LockFreeEventDispatcher::new(1000));
        let processed_count = Arc::new(AtomicUsize::new(0));
        
        // 启动多个生产者线程
        let mut handles = Vec::new();
        for i in 0..4 {
            let dispatcher_clone = dispatcher.clone();
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    let event = Event::new(
                        EventType::Trade(serde_json::json!({"thread": i, "index": j})),
                        format!("producer-{}", i)
                    );
                    while !dispatcher_clone.publish(event.clone()) {
                        thread::yield_now();
                    }
                }
            });
            handles.push(handle);
        }
        
        // 启动消费者线程
        let dispatcher_consumer = dispatcher.clone();
        let count_clone = processed_count.clone();
        let consumer = thread::spawn(move || {
            while count_clone.load(Ordering::Relaxed) < 400 {
                let processed = dispatcher_consumer.process_events_batch(10);
                if processed > 0 {
                    count_clone.fetch_add(processed, Ordering::Relaxed);
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
        
        // 检查统计信息
        if let Some(stats) = dispatcher.get_stats() {
            assert_eq!(stats.total_events_published, 400);
            assert_eq!(stats.total_events_processed, 400);
        }
    }

    #[test]
    fn test_buffer_usage() {
        let dispatcher = LockFreeEventDispatcher::new(10);
        
        // 发布一些事件
        for i in 0..5 {
            let event = Event::new(
                EventType::Trade(serde_json::json!({"index": i})),
                "test".to_string()
            );
            assert!(dispatcher.publish(event));
        }
        
        // 检查缓冲区使用情况
        let (current, capacity) = dispatcher.get_buffer_usage();
        assert_eq!(current, 5);
        assert!(capacity >= 10); // 容量可能会被向上舍入到2的幂
        
        // 处理一些事件
        let processed = dispatcher.process_events_batch(3);
        assert_eq!(processed, 3);
        
        // 再次检查缓冲区使用情况
        let (current, _) = dispatcher.get_buffer_usage();
        assert_eq!(current, 2);
    }

    #[test]
    fn test_batch_operations() {
        let dispatcher = LockFreeEventDispatcher::new(100);
        
        // 创建批量事件
        let events: Vec<Event> = (0..10).map(|i| {
            Event::new(
                EventType::BookTicker(serde_json::json!({"index": i})),
                "batch_test".to_string()
            )
        }).collect();
        
        // 批量发布
        let published = dispatcher.publish_batch(events);
        assert_eq!(published, 10);
        assert_eq!(dispatcher.pending_events(), 10);
        
        // 批量处理
        let processed = dispatcher.process_events_batch(5);
        assert_eq!(processed, 5);
        assert_eq!(dispatcher.pending_events(), 5);
        
        // 处理剩余事件
        dispatcher.process_events();
        assert_eq!(dispatcher.pending_events(), 0);
    }
}
