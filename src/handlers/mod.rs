pub mod market_data;
pub mod signals;
pub mod trading;
pub mod errors;
pub mod global;

use std::sync::Arc;
use crate::events::LockFreeEventDispatcher;

/// 事件处理器上下文 - 提供处理器所需的共享资源
#[derive(Clone)]
pub struct HandlerContext {
    pub event_dispatcher: Arc<LockFreeEventDispatcher>,
    // 可以添加其他共享资源，如数据库连接、配置等
}

impl HandlerContext {
    pub fn new(event_dispatcher: Arc<LockFreeEventDispatcher>) -> Self {
        Self { event_dispatcher }
    }

    /// 发布新事件到总线
    pub fn publish_event(&self, event: crate::events::Event) {
        self.event_dispatcher.publish(event);
    }

    /// 获取事件总线统计信息
    pub fn get_stats(&self) -> Option<crate::events::lock_free_event_bus::EventBusStats> {
        self.event_dispatcher.get_stats()
    }
}
