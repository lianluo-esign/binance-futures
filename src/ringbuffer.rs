use crossbeam_queue::ArrayQueue;
use std::sync::Arc;
use serde_json::Value;
use std::time::Instant;

#[derive(Clone, Debug)]
pub enum EventType {
    WebSocketData(Value),
    WebSocketConnected,
    WebSocketDisconnected,
    Timer(TimerType),
    Render,
    Quit,
}

#[derive(Clone, Debug)]
pub enum TimerType {
    SignalGeneration,
    DataCleanup,
    Reconnect,
}

#[derive(Clone, Debug)]
pub struct Event {
    pub event_type: EventType,
    pub timestamp: Instant,
}

impl Event {
    pub fn new(event_type: EventType) -> Self {
        Self {
            event_type,
            timestamp: Instant::now(),
        }
    }
}

// 单一RingBuffer事件队列，所有事件同等重要
pub struct EventRingBuffer {
    queue: Arc<ArrayQueue<Event>>,
}

impl EventRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: Arc::new(ArrayQueue::new(capacity)),
        }
    }
    
    pub fn push(&self, event: Event) -> Result<(), Event> {
        self.queue.push(event)
    }
    
    pub fn pop(&self) -> Option<Event> {
        self.queue.pop()
    }
    
    pub fn len(&self) -> usize {
        self.queue.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
    
    pub fn is_full(&self) -> bool {
        self.queue.is_full()
    }
    
    pub fn capacity(&self) -> usize {
        self.queue.capacity()
    }
    
    pub fn clone_producer(&self) -> Self {
        Self {
            queue: Arc::clone(&self.queue),
        }
    }
}

impl Clone for EventRingBuffer {
    fn clone(&self) -> Self {
        Self {
            queue: Arc::clone(&self.queue),
        }
    }
}

// 定时器管理器
pub struct TimerManager {
    signal_timer: Instant,
    cleanup_timer: Instant,
    reconnect_timer: Instant,
    render_timer: Instant,
    signal_interval: std::time::Duration,
    cleanup_interval: std::time::Duration,
    reconnect_interval: std::time::Duration,
    render_interval: std::time::Duration,
}

impl TimerManager {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            signal_timer: now,
            cleanup_timer: now,
            reconnect_timer: now,
            render_timer: now,
            signal_interval: std::time::Duration::from_millis(100),
            cleanup_interval: std::time::Duration::from_millis(1000),
            reconnect_interval: std::time::Duration::from_millis(5000),
            render_interval: std::time::Duration::from_millis(16), // ~60 FPS
        }
    }
    
    pub fn check_and_push_events(&mut self, event_buffer: &EventRingBuffer) {
        let now = Instant::now();
        
        if now.duration_since(self.signal_timer) >= self.signal_interval {
            let _ = event_buffer.push(Event::new(EventType::Timer(TimerType::SignalGeneration)));
            self.signal_timer = now;
        }
        
        if now.duration_since(self.cleanup_timer) >= self.cleanup_interval {
            let _ = event_buffer.push(Event::new(EventType::Timer(TimerType::DataCleanup)));
            self.cleanup_timer = now;
        }
        
        if now.duration_since(self.reconnect_timer) >= self.reconnect_interval {
            let _ = event_buffer.push(Event::new(EventType::Timer(TimerType::Reconnect)));
            self.reconnect_timer = now;
        }
        
        if now.duration_since(self.render_timer) >= self.render_interval {
            let _ = event_buffer.push(Event::new(EventType::Render));
            self.render_timer = now;
        }
    }
    
    pub fn next_timeout(&self) -> std::time::Duration {
        let now = Instant::now();
        let signal_remaining = self.signal_interval.saturating_sub(now.duration_since(self.signal_timer));
        let cleanup_remaining = self.cleanup_interval.saturating_sub(now.duration_since(self.cleanup_timer));
        let reconnect_remaining = self.reconnect_interval.saturating_sub(now.duration_since(self.reconnect_timer));
        let render_remaining = self.render_interval.saturating_sub(now.duration_since(self.render_timer));
        
        signal_remaining
            .min(cleanup_remaining)
            .min(reconnect_remaining)
            .min(render_remaining)
            .max(std::time::Duration::from_micros(1)) // 最小1微秒
    }
}