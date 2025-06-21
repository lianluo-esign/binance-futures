use std::sync::{Arc, Mutex};
use super::event_bus::EventBus;
use super::event_types::{Event, EventType};
use crate::handlers;

/// 事件分发器 - 负责初始化和管理事件处理器
pub struct EventDispatcher {
    event_bus: Arc<Mutex<EventBus>>,
}

impl EventDispatcher {
    pub fn new(event_bus: Arc<Mutex<EventBus>>) -> Self {
        Self { event_bus }
    }
    
    /// 初始化所有事件处理函数
    pub fn init_handlers(&self) {
        // 创建事件处理器实例
        let handler_context = handlers::HandlerContext::new(self.event_bus.clone());

        // 注册市场数据处理器
        self.register_sync_handler("TickPrice", {
            let context = handler_context.clone();
            move |event| handlers::market_data::handle_tick_price(event, &context)
        });

        self.register_sync_handler("DepthUpdate", {
            let context = handler_context.clone();
            move |event| handlers::market_data::handle_depth_update(event, &context)
        });

        self.register_sync_handler("Trade", {
            let context = handler_context.clone();
            move |event| handlers::market_data::handle_trade(event, &context)
        });

        self.register_sync_handler("BookTicker", {
            let context = handler_context.clone();
            move |event| handlers::market_data::handle_book_ticker(event, &context)
        });

        // 注册信号处理器
        self.register_sync_handler("Signal", {
            let context = handler_context.clone();
            move |event| handlers::signals::handle_signal(event, &context)
        });

        // 注册交易处理器
        self.register_sync_handler("OrderRequest", {
            let context = handler_context.clone();
            move |event| handlers::trading::handle_order_request(event, &context)
        });

        self.register_sync_handler("PositionUpdate", {
            let context = handler_context.clone();
            move |event| handlers::trading::handle_position_update(event, &context)
        });

        self.register_sync_handler("OrderCancel", {
            let context = handler_context.clone();
            move |event| handlers::trading::handle_order_cancel(event, &context)
        });

        // 注册错误处理器
        self.register_sync_handler("WebSocketError", {
            let context = handler_context.clone();
            move |event| handlers::errors::handle_websocket_error(event, &context)
        });

        // 注册全局事件监听器（用于日志记录等）
        self.register_global_handler({
            let context = handler_context.clone();
            move |event| handlers::global::handle_global_event(event, &context)
        });

        // 添加事件过滤器
        self.add_event_filters();
    }
    
    /// 注册同步事件处理函数
    fn register_sync_handler<F>(&self, event_type: &str, handler: F)
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        if let Ok(mut bus) = self.event_bus.lock() {
            bus.subscribe(event_type, handler);
        }
    }

    /// 注册全局事件处理函数
    fn register_global_handler<F>(&self, handler: F)
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        if let Ok(mut bus) = self.event_bus.lock() {
            bus.subscribe_global(handler);
        }
    }

    /// 添加事件过滤器
    fn add_event_filters(&self) {
        if let Ok(mut bus) = self.event_bus.lock() {
            // 过滤过期事件（超过5秒的事件）
            bus.add_filter(|event| !event.is_expired(5000));

            // 过滤重复的错误事件（可以根据需要实现更复杂的去重逻辑）
            bus.add_filter(|event| {
                match &event.event_type {
                    EventType::WebSocketError(msg) => {
                        // 简单的错误消息过滤，避免相同错误消息频繁出现
                        !msg.is_empty()
                    }
                    _ => true
                }
            });
        }
    }
    
    /// 处理事件循环 - 同步处理保证顺序性
    pub fn process_events(&self) {
        // 使用超时锁防止阻塞
        match self.event_bus.try_lock() {
            Ok(mut bus) => {
                bus.process_all_events();
            }
            Err(_) => {
                // 如果无法获取锁，记录警告但不阻塞
                log::warn!("EventBus锁被占用，跳过本次事件处理");
            }
        }
    }

    /// 处理指定数量的事件 - 非阻塞版本
    pub fn process_events_batch(&self, max_events: usize) -> usize {
        // 使用try_lock避免阻塞
        match self.event_bus.try_lock() {
            Ok(mut bus) => {
                bus.process_events(max_events)
            }
            Err(_) => {
                // 如果无法获取锁，记录警告并返回0
                log::warn!("EventBus锁被占用，跳过本次批量事件处理");
                0
            }
        }
    }

    /// 发布事件到总线 - 非阻塞版本
    pub fn publish(&self, event: Event) {
        match self.event_bus.try_lock() {
            Ok(mut bus) => {
                bus.publish(event);
            }
            Err(_) => {
                // 如果无法获取锁，记录警告并丢弃事件
                log::warn!("EventBus锁被占用，丢弃事件: {:?}", event.event_type.type_name());
            }
        }
    }

    /// 批量发布事件 - 非阻塞版本
    pub fn publish_batch(&self, events: Vec<Event>) {
        match self.event_bus.try_lock() {
            Ok(mut bus) => {
                bus.publish_batch(events);
            }
            Err(_) => {
                // 如果无法获取锁，记录警告并丢弃事件
                log::warn!("EventBus锁被占用，丢弃{}个批量事件", events.len());
            }
        }
    }

    /// 获取待处理事件数量 - 非阻塞版本
    pub fn pending_events(&self) -> usize {
        match self.event_bus.try_lock() {
            Ok(bus) => bus.pending_events(),
            Err(_) => {
                // 如果无法获取锁，返回0并记录警告
                log::warn!("EventBus锁被占用，无法获取待处理事件数量");
                0
            }
        }
    }

    /// 获取事件总线统计信息 - 非阻塞版本
    pub fn get_stats(&self) -> Option<crate::events::event_bus::EventBusStats> {
        match self.event_bus.try_lock() {
            Ok(bus) => Some(bus.stats().clone()),
            Err(_) => {
                // 如果无法获取锁，返回None
                log::warn!("EventBus锁被占用，无法获取统计信息");
                None
            }
        }
    }

    /// 获取缓冲区使用情况 (当前大小, 最大容量) - 非阻塞版本
    pub fn get_buffer_usage(&self) -> (usize, usize) {
        match self.event_bus.try_lock() {
            Ok(bus) => (bus.pending_events(), bus.capacity()),
            Err(_) => {
                // 如果无法获取锁，返回默认值
                log::warn!("EventBus锁被占用，无法获取缓冲区使用情况");
                (0, 0)
            }
        }
    }

    /// 清空所有待处理事件
    pub fn clear_events(&self) {
        if let Ok(mut bus) = self.event_bus.lock() {
            bus.clear_events();
        }
    }
}

impl Clone for EventDispatcher {
    fn clone(&self) -> Self {
        Self {
            event_bus: Arc::clone(&self.event_bus),
        }
    }
}
