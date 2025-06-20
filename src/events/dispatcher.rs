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
        if let Ok(mut bus) = self.event_bus.lock() {
            bus.process_all_events();
        }
    }

    /// 处理指定数量的事件
    pub fn process_events_batch(&self, max_events: usize) -> usize {
        if let Ok(mut bus) = self.event_bus.lock() {
            bus.process_events(max_events)
        } else {
            0
        }
    }

    /// 发布事件到总线
    pub fn publish(&self, event: Event) {
        if let Ok(mut bus) = self.event_bus.lock() {
            bus.publish(event);
        }
    }

    /// 批量发布事件
    pub fn publish_batch(&self, events: Vec<Event>) {
        if let Ok(mut bus) = self.event_bus.lock() {
            bus.publish_batch(events);
        }
    }

    /// 获取待处理事件数量
    pub fn pending_events(&self) -> usize {
        if let Ok(bus) = self.event_bus.lock() {
            bus.pending_events()
        } else {
            0
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
