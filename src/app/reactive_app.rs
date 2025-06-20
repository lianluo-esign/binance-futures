use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::core::RingBuffer;
use crate::events::{Event, EventType, EventBus, EventDispatcher};
use crate::orderbook::{OrderBookManager, MarketSnapshot};
use crate::websocket::{WebSocketManager, WebSocketConfig};
use crate::Config;

/// 响应式应用主结构（基于EventBus架构）
pub struct ReactiveApp {
    // 事件系统
    event_bus: Arc<Mutex<EventBus>>,
    event_dispatcher: EventDispatcher,
    
    // 业务组件
    orderbook_manager: OrderBookManager,
    websocket_manager: WebSocketManager,
    
    // 应用状态
    config: Config,
    running: bool,
    last_update: Instant,
    
    // UI相关
    scroll_offset: usize,
    auto_scroll: bool,
    
    // 性能监控
    events_processed_per_second: f64,
    last_performance_check: Instant,
    events_processed_since_last_check: u64,
}

impl ReactiveApp {
    pub fn new(config: Config) -> Self {
        // 创建事件总线
        let event_bus = Arc::new(Mutex::new(EventBus::new(config.event_buffer_size)));
        
        // 创建事件分发器
        let event_dispatcher = EventDispatcher::new(Arc::clone(&event_bus));
        
        // 创建WebSocket管理器
        let ws_config = WebSocketConfig::new(config.symbol.clone());
        let websocket_manager = WebSocketManager::new(ws_config);
        
        // 创建订单簿管理器
        let orderbook_manager = OrderBookManager::new();
        
        let now = Instant::now();
        
        Self {
            event_bus,
            event_dispatcher,
            orderbook_manager,
            websocket_manager,
            config,
            running: false,
            last_update: now,
            scroll_offset: 0,
            auto_scroll: true,
            events_processed_per_second: 0.0,
            last_performance_check: now,
            events_processed_since_last_check: 0,
        }
    }

    /// 初始化应用程序
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 初始化信息写入日志文件，不输出到控制台
        log::info!("初始化应用程序: {}", self.config.symbol);
        
        // 初始化事件处理器
        self.event_dispatcher.init_handlers();
        
        // 连接WebSocket
        self.websocket_manager.connect()?;
        
        self.running = true;
        // 初始化完成信息写入日志文件，不输出到控制台
        log::info!("应用程序初始化完成");
        
        Ok(())
    }

    /// 主事件循环（非阻塞）
    pub fn event_loop(&mut self) {
        if !self.running {
            return;
        }

        let loop_start = Instant::now();
        
        // 1. 读取WebSocket消息并转换为事件
        self.process_websocket_messages();
        
        // 2. 处理事件队列
        let events_processed = self.process_events();
        
        // 3. 更新性能统计
        self.update_performance_stats(events_processed);
        
        // 4. 检查WebSocket连接状态
        self.check_websocket_health();
        
        // 5. 更新最后更新时间
        self.last_update = loop_start;
    }

    /// 处理WebSocket消息
    fn process_websocket_messages(&mut self) {
        if let Ok(messages) = self.websocket_manager.read_messages() {
            for message in messages {
                if let Some(event) = self.convert_message_to_event(message) {
                    self.event_dispatcher.publish(event);
                }
            }
        }
    }

    /// 将WebSocket消息转换为事件
    fn convert_message_to_event(&mut self, message: serde_json::Value) -> Option<Event> {
        if let Some(stream) = message["stream"].as_str() {
            if let Some(data) = message["data"].as_object() {
                let event_data = serde_json::Value::Object(data.clone());

                let event_type = if stream.contains("depth") {
                    // 同时更新订单簿管理器
                    self.orderbook_manager.handle_depth_update(&event_data);
                    EventType::DepthUpdate(event_data)
                } else if stream.contains("trade") {
                    // 同时更新订单簿管理器
                    self.orderbook_manager.handle_trade(&event_data);
                    EventType::Trade(event_data)
                } else if stream.contains("bookTicker") {
                    // 同时更新订单簿管理器
                    self.orderbook_manager.handle_book_ticker(&event_data);
                    EventType::BookTicker(event_data)
                } else {
                    return None;
                };

                return Some(Event::new(event_type, "websocket".to_string()));
            }
        }
        None
    }

    /// 处理事件队列
    fn process_events(&mut self) -> usize {
        // 限制每次处理的事件数量，避免UI阻塞
        const MAX_EVENTS_PER_CYCLE: usize = 100;
        self.event_dispatcher.process_events_batch(MAX_EVENTS_PER_CYCLE)
    }

    /// 更新性能统计
    fn update_performance_stats(&mut self, events_processed: usize) {
        self.events_processed_since_last_check += events_processed as u64;
        
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_performance_check);
        
        if elapsed >= Duration::from_secs(1) {
            self.events_processed_per_second = 
                self.events_processed_since_last_check as f64 / elapsed.as_secs_f64();
            
            self.events_processed_since_last_check = 0;
            self.last_performance_check = now;
            
            // 记录性能日志
            // 性能统计不输出到控制台，避免干扰UI
            if self.events_processed_per_second > 0.0 {
                // 可以在这里记录到内部统计而不是日志
            }
        }
    }

    /// 检查WebSocket连接健康状态
    fn check_websocket_health(&mut self) {
        if !self.websocket_manager.is_connected() {
            if self.websocket_manager.should_reconnect() {
                // WebSocket重连信息写入日志文件，不输出到控制台
                log::warn!("WebSocket连接断开，尝试重连...");
                if let Err(e) = self.websocket_manager.attempt_reconnect() {
                    log::error!("WebSocket重连失败: {}", e);
                    
                    // 发布错误事件
                    let error_event = Event::new(
                        EventType::WebSocketError(format!("重连失败: {}", e)),
                        "websocket_manager".to_string()
                    );
                    self.event_dispatcher.publish(error_event);
                }
            }
        }
    }

    /// 停止应用程序
    pub fn stop(&mut self) {
        // 停止信息写入日志文件，不输出到控制台
        log::info!("正在停止应用程序...");
        self.running = false;
        self.websocket_manager.disconnect();
        
        // 处理剩余的事件
        self.event_dispatcher.process_events();
        
        // 停止完成信息写入日志文件，不输出到控制台
        log::info!("应用程序已停止");
    }

    /// 获取市场快照
    pub fn get_market_snapshot(&self) -> MarketSnapshot {
        self.orderbook_manager.get_market_snapshot()
    }

    /// 获取应用程序统计信息
    pub fn get_stats(&self) -> AppStats {
        let event_bus_stats = self.event_dispatcher.get_stats();
        let websocket_stats = self.websocket_manager.get_stats();
        let orderbook_stats = self.orderbook_manager.get_stats();
        
        AppStats {
            running: self.running,
            events_processed_per_second: self.events_processed_per_second,
            pending_events: self.event_dispatcher.pending_events(),
            websocket_connected: self.websocket_manager.is_connected(),
            total_events_published: event_bus_stats.as_ref().map(|s| s.total_events_published).unwrap_or(0),
            total_events_processed: event_bus_stats.as_ref().map(|s| s.total_events_processed).unwrap_or(0),
            websocket_messages_received: websocket_stats.map(|s| s.total_messages_received).unwrap_or(0),
            orderbook_updates: orderbook_stats.total_depth_updates,
            trades_processed: orderbook_stats.total_trades,
        }
    }

    // Getter方法
    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn get_symbol(&self) -> &str {
        &self.config.symbol
    }

    pub fn get_scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }

    pub fn is_auto_scroll(&self) -> bool {
        self.auto_scroll
    }

    pub fn set_auto_scroll(&mut self, auto_scroll: bool) {
        self.auto_scroll = auto_scroll;
    }

    pub fn get_orderbook_manager(&self) -> &OrderBookManager {
        &self.orderbook_manager
    }
}

/// 应用程序统计信息
#[derive(Debug, Clone)]
pub struct AppStats {
    pub running: bool,
    pub events_processed_per_second: f64,
    pub pending_events: usize,
    pub websocket_connected: bool,
    pub total_events_published: u64,
    pub total_events_processed: u64,
    pub websocket_messages_received: u64,
    pub orderbook_updates: u64,
    pub trades_processed: u64,
}
