pub mod market_data;
pub mod signals;
pub mod trading;
pub mod errors;
pub mod global;

use std::sync::{Arc, Mutex};
use crate::events::EventBus;

/// 事件处理器上下文 - 提供处理器所需的共享资源
#[derive(Clone)]
pub struct HandlerContext {
    pub event_bus: Arc<Mutex<EventBus>>,
    // 可以添加其他共享资源，如数据库连接、配置等
}

impl HandlerContext {
    pub fn new(event_bus: Arc<Mutex<EventBus>>) -> Self {
        Self { event_bus }
    }

    /// 发布新事件到总线
    pub fn publish_event(&self, event: crate::events::Event) {
        if let Ok(mut bus) = self.event_bus.lock() {
            bus.publish(event);
        }
    }

    /// 获取事件总线统计信息
    pub fn get_stats(&self) -> Option<crate::events::event_bus::EventBusStats> {
        if let Ok(bus) = self.event_bus.lock() {
            Some(bus.stats().clone())
        } else {
            None
        }
    }
}
