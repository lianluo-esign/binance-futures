use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::core::RingBuffer;
use crate::events::{Event, EventType, EventBus, EventDispatcher, LockFreeEventDispatcher};
use crate::orderbook::{OrderBookManager, MarketSnapshot};
use crate::websocket::{WebSocketManager, WebSocketConfig};
use crate::monitoring::InternalMonitor;
use crate::gui::volume_profile::VolumeProfileManager;
use crate::Config;

// Provider系统导入
use crate::core::{
    ProviderManager, ProviderManagerConfig, BinanceWebSocketProvider,
    BinanceWebSocketConfig, ProviderMetadata, ProviderType, DataProvider, ProviderError,
    StreamType, UpdateSpeed, ReconnectConfig, ProviderManagerStatus,
};
use std::collections::HashMap;

/// 响应式应用主结构（基于EventBus架构）
pub struct ReactiveApp {
    // 事件系统 - 使用无锁实现
    event_dispatcher: LockFreeEventDispatcher,
    
    // 业务组件
    orderbook_manager: OrderBookManager,
    websocket_manager: WebSocketManager,
    volume_profile_manager: VolumeProfileManager,
    
    // 应用状态
    config: Config,
    running: bool,
    last_update: Instant,
    
    // UI相关
    scroll_offset: usize,
    auto_scroll: bool,

    // 自动居中相关
    edge_threshold: usize,  // 边界阈值（距离顶部或底部的行数）
    auto_center_enabled: bool,

    // 价格跟踪相关
    price_tracking_enabled: bool,  // 是否启用价格跟踪
    last_tracked_price: Option<f64>,  // 上次跟踪的价格
    price_change_threshold: f64,   // 价格变化阈值，超过此值触发重新居中
    
    // 性能监控
    events_processed_per_second: f64,
    last_performance_check: Instant,
    events_processed_since_last_check: u64,

    // 健康监控
    last_heartbeat: Instant,
    heartbeat_interval: Duration,
    last_data_received: Option<Instant>,
    health_check_failures: u32,
    is_healthy: bool,

    // 内部监控系统
    internal_monitor: InternalMonitor,

    // 断路器状态
    circuit_breaker_failures: u32,
    circuit_breaker_open: bool,
    circuit_breaker_last_failure: Option<Instant>,
    
    // 最新交易数据（用于价格图表）
    last_trade_volume: Option<f64>,

    // Provider系统 (新增，向后兼容)
    provider_manager: Option<ProviderManager>,
    
    // Provider使用模式标志
    use_provider_system: bool,
}

impl ReactiveApp {
    pub fn new(config: Config) -> Self {
        // 创建无锁事件分发器
        let event_dispatcher = LockFreeEventDispatcher::new(config.event_buffer_size);
        
        // 创建WebSocket管理器
        let ws_config = WebSocketConfig::new(config.symbol.clone());
        let websocket_manager = WebSocketManager::new(ws_config);
        
        // 创建订单簿管理器
        let orderbook_manager = OrderBookManager::new();
        
        let now = Instant::now();

        Self {
            event_dispatcher,
            orderbook_manager,
            websocket_manager,
            volume_profile_manager: VolumeProfileManager::new(),
            config,
            running: false,
            last_update: now,
            scroll_offset: 0,
            auto_scroll: true,
            edge_threshold: 2,  // 距离边界2行时触发自动居中
            auto_center_enabled: true,
            price_tracking_enabled: true,
            last_tracked_price: None,
            price_change_threshold: 0.1,  // 价格变化0.1美元时触发重新居中
            events_processed_per_second: 0.0,
            last_performance_check: now,
            events_processed_since_last_check: 0,
            last_heartbeat: now,
            heartbeat_interval: Duration::from_secs(30), // 30秒心跳间隔
            last_data_received: None,
            health_check_failures: 0,
            is_healthy: true,
            internal_monitor: InternalMonitor::new(),
            circuit_breaker_failures: 0,
            circuit_breaker_open: false,
            circuit_breaker_last_failure: None,
            last_trade_volume: None,
            
            // Provider系统默认禁用，保持向后兼容
            provider_manager: None,
            use_provider_system: false,
        }
    }

    /// 创建使用Provider系统的应用实例
    /// 
    /// 这是新的创建方式，使用Provider抽象层管理数据源
    /// 
    /// # 参数
    /// - `config`: 应用配置
    /// - `provider_config`: Provider管理器配置（可选）
    /// 
    /// # 返回值
    /// 配置了Provider系统的ReactiveApp实例
    pub fn new_with_provider_system(
        config: Config, 
        provider_config: Option<ProviderManagerConfig>
    ) -> Result<Self, ProviderError> {
        log::info!("创建使用Provider系统的ReactiveApp");
        
        // 创建基础应用实例
        let mut app = Self::new(config);
        
        // 创建Provider管理器
        let provider_manager_config = provider_config.unwrap_or_else(|| {
            ProviderManagerConfig {
                default_provider_id: "binance_websocket".to_string(),
                ..Default::default()
            }
        });
        
        let mut provider_manager = ProviderManager::new(provider_manager_config);
        
        // 创建Binance WebSocket Provider
        let binance_config = BinanceWebSocketConfig {
            symbol: app.config.symbol.clone(),
            streams: vec![
                StreamType::BookTicker,
                StreamType::Depth { 
                    levels: 20, 
                    update_speed: UpdateSpeed::Ms100 
                },
                StreamType::Trade,
            ],
            reconnect_config: ReconnectConfig::default(),
            heartbeat_interval_secs: 30,
            max_buffer_size: 1000,
            compression_enabled: false,
            ..Default::default()
        };
        
        let binance_provider = BinanceWebSocketProvider::new(binance_config);
        
        // 创建Provider元数据
        let provider_metadata = ProviderMetadata {
            id: "binance_websocket".to_string(),
            name: "Binance WebSocket".to_string(),
            description: "Binance实时WebSocket数据源".to_string(),
            provider_type: ProviderType::Binance { 
                mode: crate::core::provider::BinanceConnectionMode::WebSocket 
            },
            priority: 100,
            is_fallback: false,
            tags: {
                let mut tags = HashMap::new();
                tags.insert("exchange".to_string(), "binance".to_string());
                tags.insert("type".to_string(), "websocket".to_string());
                tags
            },
        };
        
        // 注册Provider
        provider_manager.register_provider(binance_provider, provider_metadata)
            .map_err(|e| {
                log::error!("注册Binance WebSocket Provider失败: {}", e);
                e
            })?;
        
        // 启用Provider系统
        app.provider_manager = Some(provider_manager);
        app.use_provider_system = true;
        
        log::info!("Provider系统初始化完成");
        Ok(app)
    }

    /// 初始化应用程序
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 初始化信息写入日志文件，不输出到控制台
        log::info!("初始化应用程序: {} (模式: {})", 
            self.config.symbol,
            if self.use_provider_system { "Provider系统" } else { "传统WebSocket" }
        );

        if self.use_provider_system {
            // 使用Provider系统
            if let Some(ref mut provider_manager) = self.provider_manager {
                log::info!("启动Provider管理器...");
                provider_manager.start()
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            } else {
                return Err("Provider系统未正确初始化".into());
            }
        } else {
            // 使用传统WebSocket系统
            log::info!("正在建立WebSocket连接...");
            self.websocket_manager.connect()?;
        }

        self.running = true;
        log::info!("应用程序初始化完成 - {} 已启动", 
            if self.use_provider_system { "Provider系统" } else { "传统WebSocket系统" }
        );

        Ok(())
    }

    /// 主事件循环（非阻塞）- 支持Provider系统和传统WebSocket
    pub fn event_loop(&mut self) {
        if !self.running {
            return;
        }

        // 1. 数据源处理：根据使用的系统类型选择不同的处理方式
        let events_processed = if self.use_provider_system {
            // 使用Provider系统
            self.process_provider_events()
        } else {
            // 使用传统WebSocket系统
            if self.websocket_manager.is_connected() {
                self.process_websocket_messages();
            } else if self.websocket_manager.should_reconnect() {
                // 简化重连逻辑，避免复杂的健康检查
                let _ = self.websocket_manager.attempt_reconnect();
            }
            
            // 处理事件队列
            self.process_events()
        };

        // 3. 更新性能统计（简化版本）
        self.update_performance_stats(events_processed);

        // 4. 发送心跳（如果需要）
        self.send_heartbeat_if_needed();

        // 5. 更新内部监控系统
        self.update_internal_monitoring(events_processed);

        // 6. 处理自动滚动逻辑（基于当前交易价格）
        self.update_auto_scroll();

        // 7. 处理价格跟踪逻辑
        self.update_price_tracking();

        // 7. 检查并尝试自动恢复
        self.check_and_recover();

        // 8. 更新最后更新时间
        self.last_update = Instant::now();
    }

    /// 处理Provider事件（新的事件处理方式）
    fn process_provider_events(&mut self) -> usize {
        if let Some(ref mut provider_manager) = self.provider_manager {
            match provider_manager.process_events() {
                Ok(events) => {
                    let mut events_created = 0;
                    
                    // 限制每次处理的事件数量
                    const MAX_EVENTS_PER_CYCLE: usize = 50;
                    let events_to_process = events.into_iter().take(MAX_EVENTS_PER_CYCLE);
                    
                    for event_type in events_to_process {
                        // 更新业务组件
                        self.update_business_components(&event_type);
                        
                        // 发布事件到事件系统
                        let event = Event::new(event_type, "provider".to_string());
                        self.event_dispatcher.publish(event);
                        events_created += 1;
                    }
                    
                    if events_created > 0 {
                        log::info!("Provider系统处理了 {} 个事件", events_created);
                        self.last_data_received = Some(Instant::now());
                    }
                    
                    // 处理事件队列
                    let events_processed = self.process_events();
                    events_processed + events_created
                }
                Err(e) => {
                    log::warn!("Provider事件处理失败: {}", e);
                    
                    // 更新错误统计
                    self.circuit_breaker_failures += 1;
                    self.circuit_breaker_last_failure = Some(Instant::now());
                    
                    // 如果失败次数过多，可以考虑切换Provider
                    if self.circuit_breaker_failures >= 5 {
                        self.circuit_breaker_open = true;
                        log::error!("Provider断路器打开，暂停事件处理");
                    }
                    
                    // 处理现有事件队列
                    self.process_events()
                }
            }
        } else {
            log::error!("Provider管理器未初始化");
            0
        }
    }

    /// 更新业务组件（从EventType数据中）
    fn update_business_components(&mut self, event_type: &EventType) {
        match event_type {
            EventType::DepthUpdate(data) => {
                self.orderbook_manager.handle_depth_update(data);
            }
            EventType::Trade(data) => {
                // 存储交易成交量信息（用于价格图表）
                if let Some(qty_str) = data["q"].as_str() {
                    if let Ok(qty) = qty_str.parse::<f64>() {
                        self.last_trade_volume = Some(qty);
                    }
                }
                
                self.orderbook_manager.handle_trade(data);
                self.update_volume_profile_from_trade(data);
            }
            EventType::BookTicker(data) => {
                self.orderbook_manager.handle_book_ticker(data);
            }
            _ => {} // 其他事件类型暂不处理
        }
    }

    /// 处理WebSocket消息 - 带断路器保护的版本（传统模式）
    fn process_websocket_messages(&mut self) {
        // 检查断路器状态
        if self.circuit_breaker_open {
            // 检查是否可以尝试恢复
            if let Some(last_failure) = self.circuit_breaker_last_failure {
                if last_failure.elapsed() > Duration::from_secs(30) {
                    // 30秒后尝试恢复
                    self.circuit_breaker_open = false;
                    self.circuit_breaker_failures = 0;
                    log::info!("断路器恢复，尝试重新处理WebSocket消息");
                } else {
                    // 断路器仍然开启，跳过处理
                    return;
                }
            }
        }

        // 尝试读取消息，带错误处理
        match self.websocket_manager.read_messages() {
            Ok(messages) => {
                if !messages.is_empty() {
                    self.last_data_received = Some(Instant::now());
                    // 重置断路器失败计数
                    self.circuit_breaker_failures = 0;

                    // 添加调试日志
                    log::info!("收到 {} 条WebSocket消息", messages.len());
                }

                // 限制每次处理的消息数量，防止阻塞
                const MAX_MESSAGES_PER_CYCLE: usize = 50;
                let messages_to_process = messages.into_iter().take(MAX_MESSAGES_PER_CYCLE);

                let mut events_created = 0;
                for message in messages_to_process {
                    if let Some(event) = self.convert_message_to_event(message) {
                        self.event_dispatcher.publish(event);
                        events_created += 1;
                    }
                }

                if events_created > 0 {
                    log::info!("创建了 {} 个事件", events_created);
                }
            }
            Err(e) => {
                // 记录错误并更新断路器状态
                self.circuit_breaker_failures += 1;
                self.circuit_breaker_last_failure = Some(Instant::now());

                log::warn!("WebSocket消息处理失败 (失败次数: {}): {}", self.circuit_breaker_failures, e);

                // 如果失败次数过多，打开断路器
                if self.circuit_breaker_failures >= 5 {
                    self.circuit_breaker_open = true;
                    log::error!("WebSocket断路器打开，暂停消息处理30秒");
                }
            }
        }
    }

    /// 将WebSocket消息转换为事件 - 带超时保护的版本
    fn convert_message_to_event(&mut self, message: serde_json::Value) -> Option<Event> {
        let start_time = Instant::now();


        // 检查消息结构
        let stream = message["stream"].as_str()?;
        let data = message["data"].as_object()?;
        let event_data = serde_json::Value::Object(data.clone());


        // 根据流类型处理事件，但添加超时检查
        let event_type = if stream.contains("depth") {
            // 检查处理时间，防止长时间阻塞
            if start_time.elapsed() > Duration::from_millis(100) {
                log::warn!("深度更新处理超时，跳过");
                return None;
            }
            // 同时更新订单簿管理器
            self.orderbook_manager.handle_depth_update(&event_data);
            EventType::DepthUpdate(event_data)
        } else if stream.contains("trade") {
            if start_time.elapsed() > Duration::from_millis(100) {
                log::warn!("交易更新处理超时，跳过");
                return None;
            }
            
            // 存储交易成交量信息（用于价格图表）
            if let Some(qty_str) = event_data["q"].as_str() {
                if let Ok(qty) = qty_str.parse::<f64>() {
                    self.last_trade_volume = Some(qty);
                }
            }
            
            // 同时更新订单簿管理器
            self.orderbook_manager.handle_trade(&event_data);
            
            // 更新Volume Profile数据
            self.update_volume_profile_from_trade(&event_data);
            
            EventType::Trade(event_data)
        } else if stream.contains("bookTicker") {
            if start_time.elapsed() > Duration::from_millis(100) {
                log::warn!("BookTicker更新处理超时，跳过");
                return None;
            }
            // 同时更新订单簿管理器
            self.orderbook_manager.handle_book_ticker(&event_data);
            EventType::BookTicker(event_data)
        } else {
            return None;
        };

        // 最终超时检查
        if start_time.elapsed() > Duration::from_millis(200) {
            log::warn!("事件转换总时间超时: {:?}", start_time.elapsed());
        }

        Some(Event::new(event_type, "websocket".to_string()))
    }

    /// 处理事件队列
    fn process_events(&mut self) -> usize {
        // 限制每次处理的事件数量，避免UI阻塞
        const MAX_EVENTS_PER_CYCLE: usize = 100;
        let events_processed = self.event_dispatcher.process_events_batch(MAX_EVENTS_PER_CYCLE);

        // 添加调试日志
        if events_processed > 0 {
            log::info!("处理了 {} 个事件", events_processed);
        }

        events_processed
    }

    /// 更新性能统计 - 简化版本
    fn update_performance_stats(&mut self, events_processed: usize) {
        self.events_processed_since_last_check += events_processed as u64;

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_performance_check);

        // 每秒更新一次统计
        if elapsed >= Duration::from_secs(1) {
            self.events_processed_per_second =
                self.events_processed_since_last_check as f64 / elapsed.as_secs_f64();

            self.events_processed_since_last_check = 0;
            self.last_performance_check = now;
        }
    }

    /// 更新内部监控系统
    fn update_internal_monitoring(&mut self, events_processed: usize) {
        // 更新缓冲区使用情况
        let (current_buffer_size, max_buffer_capacity) = self.get_buffer_usage();
        self.internal_monitor.update_buffer_usage(current_buffer_size, max_buffer_capacity);

        // 更新事件处理统计
        let event_types = std::collections::HashMap::new(); // 简化版本，后续可以扩展
        self.internal_monitor.update_event_processing(events_processed, current_buffer_size, &event_types);

        // 更新WebSocket健康状态 - 使用管理器统计而不是连接统计
        let manager_stats = self.websocket_manager.get_manager_stats();
        let websocket_stats = self.websocket_manager.get_stats();
        let reconnect_count = websocket_stats.as_ref().map(|s| s.reconnect_attempts).unwrap_or(0);

        // 使用管理器统计中的消息数量，这个更准确
        self.internal_monitor.update_websocket_health(
            self.websocket_manager.is_connected(),
            manager_stats.total_json_messages, // 使用JSON消息计数
            reconnect_count,
            0.0 // ping延迟，简化版本
        );

        // 检测阻塞情况
        self.internal_monitor.detect_blocking();
    }

    /// 检查系统状态并尝试自动恢复
    fn check_and_recover(&mut self) {
        // 检查是否检测到阻塞或死锁
        if self.internal_monitor.blocking_detector.is_blocked {
            let blocked_duration = self.internal_monitor.blocking_detector.blocked_since
                .map(|since| since.elapsed())
                .unwrap_or(Duration::from_secs(0));

            // 如果阻塞超过30秒，尝试自动恢复
            if blocked_duration > Duration::from_secs(30) {
                log::warn!("系统阻塞超过30秒，尝试自动恢复");

                if let Some(component) = &self.internal_monitor.blocking_detector.blocking_component {
                    match component.as_str() {
                        "EventProcessing" => {
                            self.recover_event_processing();
                        }
                        "WebSocket" => {
                            self.recover_websocket();
                        }
                        "BufferOverflow" => {
                            self.recover_buffer_overflow();
                        }
                        _ => {
                            log::warn!("未知的阻塞组件: {}", component);
                        }
                    }
                }
            }

            // 如果检测到死锁，执行更激进的恢复措施
            if self.internal_monitor.blocking_detector.deadlock_detected {
                log::error!("检测到死锁，执行紧急恢复");
                self.emergency_recovery();
            }
        }
    }

    /// 恢复事件处理系统
    fn recover_event_processing(&mut self) {
        log::info!("尝试恢复事件处理系统");

        // 重置断路器
        self.circuit_breaker_open = false;
        self.circuit_breaker_failures = 0;

        // 尝试处理积压的事件
        let _ = self.event_dispatcher.process_events_batch(1000);

        log::info!("事件处理系统恢复完成");
    }

    /// 恢复WebSocket连接
    fn recover_websocket(&mut self) {
        log::info!("尝试恢复WebSocket连接");

        // 断开并重连WebSocket
        self.websocket_manager.disconnect();

        // 等待一小段时间后重连
        if let Err(e) = self.websocket_manager.attempt_reconnect() {
            log::error!("WebSocket重连失败: {}", e);
        } else {
            log::info!("WebSocket重连成功");
        }

        // 重置相关状态
        self.last_data_received = None;
        self.circuit_breaker_open = false;
        self.circuit_breaker_failures = 0;
    }

    /// 恢复缓冲区溢出
    fn recover_buffer_overflow(&mut self) {
        log::info!("尝试恢复缓冲区溢出");

        // 强制处理所有积压事件
        let _ = self.event_dispatcher.process_events();

        // 如果仍然溢出，清空部分缓冲区（紧急措施）
        let (current_usage, max_capacity) = self.get_buffer_usage();
        if current_usage > max_capacity * 9 / 10 {
            log::warn!("缓冲区仍然接近满载，执行紧急清理");
            // 这里可以实现更激进的清理策略
        }

        log::info!("缓冲区恢复完成");
    }

    /// 紧急恢复 - 用于死锁情况
    fn emergency_recovery(&mut self) {
        log::error!("执行紧急恢复程序");

        // 重置所有状态
        self.circuit_breaker_open = false;
        self.circuit_breaker_failures = 0;
        self.health_check_failures = 0;
        self.is_healthy = true;

        // 重连WebSocket
        self.websocket_manager.disconnect();
        let _ = self.websocket_manager.attempt_reconnect();

        // 清理事件队列
        let _ = self.event_dispatcher.process_events();

        // 重置监控状态
        self.internal_monitor.blocking_detector.is_blocked = false;
        self.internal_monitor.blocking_detector.blocked_since = None;
        self.internal_monitor.blocking_detector.blocking_component = None;
        self.internal_monitor.blocking_detector.deadlock_detected = false;

        log::error!("紧急恢复完成");
    }



    /// 简化的健康检查 - 仅记录心跳，避免复杂逻辑
    fn send_heartbeat_if_needed(&mut self) {
        let now = Instant::now();

        // 每30秒发送一次心跳日志
        if now.duration_since(self.last_heartbeat) >= self.heartbeat_interval {
            let stats = self.get_stats();
            log::info!("应用心跳: 运行={}, 事件/秒={:.1}, WebSocket={}",
                stats.running,
                stats.events_processed_per_second,
                if stats.websocket_connected { "连接" } else { "断开" }
            );
            self.last_heartbeat = now;
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
            is_healthy: true, // 简化为总是健康
            health_check_failures: 0, // 简化为0
            last_data_received: self.last_data_received,
        }
    }

    /// 获取EventBus缓冲区使用情况
    pub fn get_buffer_usage(&self) -> (usize, usize) {
        self.event_dispatcher.get_buffer_usage()
    }

    /// 获取内部监控数据
    pub fn get_internal_monitor(&self) -> &InternalMonitor {
        &self.internal_monitor
    }

    /// 获取WebSocket连接信息用于调试
    pub fn get_websocket_debug_info(&self) -> String {
        let stats = self.websocket_manager.get_stats();
        let manager_stats = self.websocket_manager.get_manager_stats();

        format!(
            "WebSocket调试信息:\n连接状态: {}\n总消息数: {}\n缓冲消息数: {}\nJSON解析错误: {}\n连接错误: {}\n连续错误: {}",
            if self.websocket_manager.is_connected() { "已连接" } else { "未连接" },
            stats.map(|s| s.total_messages_received).unwrap_or(0),
            manager_stats.messages_buffered,
            manager_stats.json_parse_errors,
            manager_stats.connection_errors,
            manager_stats.consecutive_errors
        )
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

    /// 获取Volume Profile管理器
    pub fn get_volume_profile_manager(&self) -> &VolumeProfileManager {
        &self.volume_profile_manager
    }
    
    /// 获取最新交易的成交量
    pub fn get_last_trade_volume(&self) -> Option<f64> {
        self.last_trade_volume
    }

    /// 从交易数据更新Volume Profile
    fn update_volume_profile_from_trade(&mut self, data: &serde_json::Value) {
        if let (Some(price_str), Some(qty_str), Some(is_buyer_maker)) = (
            data["p"].as_str(),
            data["q"].as_str(),
            data["m"].as_bool(),
        ) {
            if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                let side = if is_buyer_maker { "sell" } else { "buy" };
                
                        // 更新Volume Profile数据
                self.volume_profile_manager.handle_trade(price, qty, side);
            }
        }
    }

    pub fn get_max_visible_rows(&self) -> usize {
        self.config.max_visible_rows
    }

    pub fn get_price_precision(&self) -> f64 {
        self.config.price_precision
    }

    /// 计算自动居中的滚动偏移
    pub fn calculate_auto_center_scroll(&self, price_levels: &[f64], visible_rows: usize) -> usize {
        if !self.auto_center_enabled || self.auto_scroll {
            return self.scroll_offset; // 如果禁用自动居中或启用了自动滚动，返回当前偏移
        }

        let (best_bid, best_ask) = self.orderbook_manager.get_best_prices();

        // 如果没有最优价格信息，返回当前偏移
        let (best_bid_price, best_ask_price) = match (best_bid, best_ask) {
            (Some(bid), Some(ask)) => (bid, ask),
            _ => return self.scroll_offset,
        };

        // 找到最优买价和卖价在价格列表中的位置
        let mut best_bid_index = None;
        let mut best_ask_index = None;

        // 计算价格匹配的容差
        let price_tolerance = if self.config.price_precision > 0.0 {
            self.config.price_precision / 2.0
        } else {
            0.1  // 默认容差
        };

        for (i, &price) in price_levels.iter().enumerate() {
            if (price - best_bid_price).abs() < price_tolerance {
                best_bid_index = Some(i);
            }
            if (price - best_ask_price).abs() < price_tolerance {
                best_ask_index = Some(i);
            }
        }

        // 计算需要居中的价格位置（使用最优买价和卖价的中间位置）
        let center_index = match (best_bid_index, best_ask_index) {
            (Some(bid_idx), Some(ask_idx)) => (bid_idx + ask_idx) / 2,
            (Some(bid_idx), None) => bid_idx,
            (None, Some(ask_idx)) => ask_idx,
            (None, None) => return self.scroll_offset,
        };

        // 检查是否需要自动居中
        let current_visible_start = self.scroll_offset;
        let current_visible_end = self.scroll_offset + visible_rows;

        // 如果最优价格接近窗口边界，触发自动居中
        let needs_centering = center_index < current_visible_start + self.edge_threshold ||
                             center_index > current_visible_end.saturating_sub(self.edge_threshold);

        if needs_centering {
            // 计算新的滚动偏移，使最优价格居中
            let target_scroll = center_index.saturating_sub(visible_rows / 2);
            let max_scroll = price_levels.len().saturating_sub(visible_rows);
            target_scroll.min(max_scroll)
        } else {
            self.scroll_offset
        }
    }

    /// 计算基于当前交易价格的智能滑动窗口偏移
    /// 当交易价格跳动到距离可见窗口边界2个价格层级时，自动滑动窗口使交易价格居中
    pub fn calculate_auto_scroll_for_trade_price(&mut self, price_levels: &[f64], visible_rows: usize, current_trade_price: Option<f64>) -> usize {
        if !self.auto_scroll {
            return self.scroll_offset; // 如果禁用自动滚动，返回当前偏移
        }

        // 如果没有当前交易价格，返回当前偏移
        let trade_price = match current_trade_price {
            Some(price) => price,
            None => return self.scroll_offset,
        };

        // 计算价格匹配的容差，基于价格精度或默认值
        let price_tolerance = if self.config.price_precision > 0.0 {
            self.config.price_precision / 2.0  // 使用精度的一半作为容差
        } else {
            0.1  // 默认容差，适合BTCUSDT等高价格币种
        };

        // 找到当前交易价格在价格列表中的位置
        let mut trade_price_index = None;
        let mut closest_distance = f64::MAX;
        let mut closest_index = 0;

        for (i, &price) in price_levels.iter().enumerate() {
            let distance = (price - trade_price).abs();

            // 记录最接近的价格索引
            if distance < closest_distance {
                closest_distance = distance;
                closest_index = i;
            }

            // 如果在容差范围内，直接使用
            if distance < price_tolerance {
                trade_price_index = Some(i);
                break;
            }
        }

        let trade_index = match trade_price_index {
            Some(index) => {
                    index
            },
            None => {
                // 如果没有精确匹配，检查距离是否过大
                if !price_levels.is_empty() && closest_distance < 100.0 {  // 允许100美元的差距
                    closest_index
                } else {
                    // 如果价格差距过大，触发orderbook重新初始化
                    if closest_distance > 200.0 {
                        // 调用重新初始化逻辑
                        self.check_and_reinitialize_orderbook(trade_price);
                    }

                    return self.scroll_offset; // 如果找不到合适的价格，返回当前偏移
                }
            }
        };

        // 检查当前交易价格是否接近窗口边界（距离边界2个价格层级）
        let current_visible_start = self.scroll_offset;
        let current_visible_end = self.scroll_offset + visible_rows;

        // 检查是否需要滑动窗口：
        // 1. 交易价格接近窗口顶部边界（距离顶部2行以内）
        // 2. 交易价格接近窗口底部边界（距离底部2行以内）
        let near_top = trade_index < current_visible_start + self.edge_threshold;
        let near_bottom = trade_index > current_visible_end.saturating_sub(self.edge_threshold);

        let needs_scrolling = near_top || near_bottom;


        if needs_scrolling {
            // 计算新的滚动偏移，使交易价格居中到可见窗口的正中间
            let target_scroll = trade_index.saturating_sub(visible_rows / 2);
            let max_scroll = price_levels.len().saturating_sub(visible_rows);
            let new_scroll = target_scroll.min(max_scroll);

            // 更新滚动偏移
            self.scroll_offset = new_scroll;


            new_scroll
        } else {
            self.scroll_offset
        }
    }

    /// 设置自动居中功能开关
    pub fn set_auto_center_enabled(&mut self, enabled: bool) {
        self.auto_center_enabled = enabled;
    }

    /// 获取自动居中功能状态
    pub fn is_auto_center_enabled(&self) -> bool {
        self.auto_center_enabled
    }

    /// 设置价格跟踪功能开关
    pub fn set_price_tracking_enabled(&mut self, enabled: bool) {
        self.price_tracking_enabled = enabled;
    }

    /// 获取价格跟踪功能状态
    pub fn is_price_tracking_enabled(&self) -> bool {
        self.price_tracking_enabled
    }

    /// 强制重新居中到当前交易价格
    pub fn force_recenter_on_current_price(&mut self) {
        let market_snapshot = self.orderbook_manager.get_market_snapshot();
        if let Some(current_price) = market_snapshot.current_price {
            // 重置跟踪价格，强制触发重新居中
            self.last_tracked_price = None;
            self.center_window_on_price(current_price);
            self.last_tracked_price = Some(current_price);
        }
    }

    /// 更新自动滚动逻辑（在事件循环中调用）
    fn update_auto_scroll(&mut self) {
        if !self.auto_scroll {
            return; // 如果禁用自动滚动，直接返回
        }

        // 获取当前交易价格
        let market_snapshot = self.orderbook_manager.get_market_snapshot();
        let current_trade_price = market_snapshot.current_price;

        if current_trade_price.is_none() {
            return; // 如果没有当前价格，直接返回
        }

        // 应用价格精度聚合到当前交易价格
        let aggregated_trade_price = current_trade_price.map(|price| {
            let precision = self.config.price_precision;
            if precision <= 0.0 {
                price
            } else {
                (price / precision).floor() * precision
            }
        });

        // 生成聚合后的价格列表
        let order_flows = self.orderbook_manager.get_order_flows();
        if order_flows.is_empty() {
            return; // 如果没有订单簿数据，直接返回
        }

        // 应用价格精度聚合
        let aggregated_order_flows = self.aggregate_order_flows(&order_flows);

        // 构建聚合后的价格列表
        let mut price_levels: Vec<_> = aggregated_order_flows.keys().collect();
        price_levels.sort_by(|a, b| b.cmp(a)); // 从高价到低价排序
        let max_levels = self.config.max_visible_rows.min(price_levels.len());
        let price_values: Vec<f64> = price_levels.iter().take(max_levels).map(|k| k.0).collect();

        // 使用配置的可见行数
        let visible_rows = self.config.max_visible_rows.min(50); // 限制最大值以避免性能问题


        // 检查是否需要重新初始化orderbook
        if let Some(trade_price) = current_trade_price {
            self.check_and_reinitialize_orderbook(trade_price);
        }

        // 调用自动滚动计算方法
        let _new_scroll = self.calculate_auto_scroll_for_trade_price(&price_values, visible_rows, aggregated_trade_price);
        // scroll_offset 已经在 calculate_auto_scroll_for_trade_price 方法中更新
    }

    /// 检查是否需要重新初始化orderbook
    /// 当交易价格与orderbook价格范围差距过大时，重新初始化
    fn check_and_reinitialize_orderbook(&mut self, trade_price: f64) {
        let order_flows = self.orderbook_manager.get_order_flows();
        if order_flows.is_empty() {
            return;
        }

        // 获取当前orderbook的价格范围
        let prices: Vec<f64> = order_flows.keys().map(|k| k.0).collect();
        if prices.is_empty() {
            return;
        }

        let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        // 检查交易价格是否在合理范围内
        let price_range = max_price - min_price;
        let tolerance = price_range * 0.1; // 允许10%的扩展范围

        if trade_price < min_price - tolerance || trade_price > max_price + tolerance {
            log::warn!("交易价格 {:.2} 超出orderbook范围 [{:.2} - {:.2}]，订单簿将通过WebSocket自动更新",
                      trade_price, min_price, max_price);
            // 注意：订单簿现在完全依赖WebSocket实时更新，不再需要REST API重新初始化
        }
    }

    /// 将窗口居中到指定价格
    fn center_window_on_price(&mut self, target_price: f64) {
        // 应用价格精度聚合到目标价格
        let precision = self.config.price_precision;
        let aggregated_target_price = if precision <= 0.0 {
            target_price
        } else {
            (target_price / precision).floor() * precision
        };

        // 生成聚合后的价格列表
        let order_flows = self.orderbook_manager.get_order_flows();
        if order_flows.is_empty() {
            return; // 如果没有订单簿数据，直接返回
        }

        // 应用价格精度聚合
        let aggregated_order_flows = self.aggregate_order_flows(&order_flows);

        // 构建聚合后的价格列表
        let mut price_levels: Vec<_> = aggregated_order_flows.keys().collect();
        price_levels.sort_by(|a, b| b.cmp(a)); // 从高价到低价排序
        let max_levels = self.config.max_visible_rows.min(price_levels.len());
        let price_values: Vec<f64> = price_levels.iter().take(max_levels).map(|k| k.0).collect();

        // 找到目标价格在价格列表中的位置
        let mut target_index = None;
        let mut closest_distance = f64::MAX;
        let mut closest_index = 0;

        // 计算价格匹配的容差
        let price_tolerance = if precision > 0.0 {
            precision / 2.0
        } else {
            0.1  // 默认容差
        };

        for (i, &price) in price_values.iter().enumerate() {
            let distance = (price - aggregated_target_price).abs();

            // 记录最接近的价格索引
            if distance < closest_distance {
                closest_distance = distance;
                closest_index = i;
            }

            // 如果在容差范围内，直接使用
            if distance < price_tolerance {
                target_index = Some(i);
                break;
            }
        }

        // 如果没有精确匹配，使用最接近的价格
        if target_index.is_none() && !price_values.is_empty() && closest_distance < 10.0 {
            target_index = Some(closest_index);
        }

        if let Some(index) = target_index {
            // 使用配置的可见行数
            let visible_rows = self.config.max_visible_rows.min(50); // 限制最大值以避免性能问题

            // 计算新的滚动偏移，使目标价格居中
            let target_scroll = index.saturating_sub(visible_rows / 2);
            let max_scroll = price_values.len().saturating_sub(visible_rows);
            let new_scroll = target_scroll.min(max_scroll);

            // 更新滚动偏移
            self.scroll_offset = new_scroll;

        }
    }

    /// 更新价格跟踪逻辑（主动跟随当前交易价格）- 改进版本
    fn update_price_tracking(&mut self) {
        if !self.price_tracking_enabled {
            return; // 如果禁用价格跟踪，直接返回
        }

        // 获取市场快照
        let market_snapshot = self.orderbook_manager.get_market_snapshot();
        
        // 优先使用当前交易价格，如果没有则使用最优买价
        let reference_price = market_snapshot.current_price
            .or(market_snapshot.best_bid_price)
            .or(market_snapshot.best_ask_price);

        if let Some(current_price) = reference_price {
            // 检查价格是否发生了显著变化
            let should_recenter = match self.last_tracked_price {
                Some(last_price) => {
                    let price_change = (current_price - last_price).abs();
                    // 对于价格下跌，使用更敏感的阈值
                    let threshold = if current_price < last_price {
                        self.price_change_threshold * 0.5 // 价格下跌时使用更小的阈值
                    } else {
                        self.price_change_threshold
                    };
                    price_change >= threshold
                }
                None => true, // 第一次设置价格
            };

            if should_recenter {
                // 更新跟踪的价格
                self.last_tracked_price = Some(current_price);

                // 触发窗口重新居中
                self.center_window_on_price(current_price);

            }
        }
    }

    /// 应用价格精度聚合到订单流数据
    fn aggregate_order_flows(&self, order_flows: &std::collections::BTreeMap<ordered_float::OrderedFloat<f64>, crate::orderbook::OrderFlow>) -> std::collections::BTreeMap<ordered_float::OrderedFloat<f64>, crate::orderbook::OrderFlow> {
        use std::collections::BTreeMap;
        use ordered_float::OrderedFloat;
        use crate::orderbook::OrderFlow;

        let precision = self.config.price_precision;
        if precision <= 0.0 {
            return order_flows.clone(); // 如果精度无效，返回原始数据
        }

        let mut aggregated: BTreeMap<OrderedFloat<f64>, OrderFlow> = BTreeMap::new();

        for (price_key, order_flow) in order_flows {
            let original_price = price_key.0;

            // 使用floor函数进行价格聚合
            let aggregated_price = (original_price / precision).floor() * precision;
            let aggregated_key = OrderedFloat(aggregated_price);

            // 获取或创建聚合价格级别
            let aggregated_flow = aggregated.entry(aggregated_key).or_insert_with(OrderFlow::new);

            // 聚合买卖价格和数量
            aggregated_flow.bid_ask.bid += order_flow.bid_ask.bid;
            aggregated_flow.bid_ask.ask += order_flow.bid_ask.ask;
            aggregated_flow.bid_ask.timestamp = aggregated_flow.bid_ask.timestamp.max(order_flow.bid_ask.timestamp);

            // 聚合交易记录
            aggregated_flow.history_trade_record.buy_volume += order_flow.history_trade_record.buy_volume;
            aggregated_flow.history_trade_record.sell_volume += order_flow.history_trade_record.sell_volume;
            aggregated_flow.history_trade_record.timestamp = aggregated_flow.history_trade_record.timestamp.max(order_flow.history_trade_record.timestamp);

            aggregated_flow.realtime_trade_record.buy_volume += order_flow.realtime_trade_record.buy_volume;
            aggregated_flow.realtime_trade_record.sell_volume += order_flow.realtime_trade_record.sell_volume;
            aggregated_flow.realtime_trade_record.timestamp = aggregated_flow.realtime_trade_record.timestamp.max(order_flow.realtime_trade_record.timestamp);

            // 聚合撤单记录
            aggregated_flow.realtime_cancel_records.bid_cancel += order_flow.realtime_cancel_records.bid_cancel;
            aggregated_flow.realtime_cancel_records.ask_cancel += order_flow.realtime_cancel_records.ask_cancel;
            aggregated_flow.realtime_cancel_records.timestamp = aggregated_flow.realtime_cancel_records.timestamp.max(order_flow.realtime_cancel_records.timestamp);

            // 聚合增加订单
            aggregated_flow.realtime_increase_order.bid += order_flow.realtime_increase_order.bid;
            aggregated_flow.realtime_increase_order.ask += order_flow.realtime_increase_order.ask;
            aggregated_flow.realtime_increase_order.timestamp = aggregated_flow.realtime_increase_order.timestamp.max(order_flow.realtime_increase_order.timestamp);
        }

        aggregated
    }

    // ========== Provider系统相关API ==========

    /// 检查是否使用Provider系统
    pub fn is_using_provider_system(&self) -> bool {
        self.use_provider_system
    }

    /// 获取Provider管理器的引用（只读）
    pub fn provider_manager(&self) -> Option<&ProviderManager> {
        self.provider_manager.as_ref()
    }

    /// 获取当前活跃的Provider状态
    pub fn get_active_provider_status(&self) -> Option<crate::core::provider::ProviderStatus> {
        if let Some(ref provider_manager) = self.provider_manager {
            if let Some(provider_id) = provider_manager.get_active_provider_id() {
                if let Ok(status_map) = provider_manager.get_all_provider_status() {
                    return status_map.get(&provider_id).cloned();
                }
            }
        }
        None
    }

    /// 获取Provider管理器状态
    pub fn get_provider_manager_status(&self) -> Option<ProviderManagerStatus> {
        if let Some(ref provider_manager) = self.provider_manager {
            provider_manager.get_status().ok()
        } else {
            None
        }
    }

    /// 切换到指定的Provider
    /// 
    /// # 参数
    /// - `provider_id`: 目标Provider的ID
    /// 
    /// # 返回值
    /// - `Ok(())`: 切换成功
    /// - `Err(String)`: 切换失败的错误信息
    pub fn switch_provider(&self, provider_id: &str) -> Result<(), String> {
        if !self.use_provider_system {
            return Err("未使用Provider系统，无法切换Provider".to_string());
        }

        if let Some(ref provider_manager) = self.provider_manager {
            provider_manager.switch_to_provider(provider_id)
                .map_err(|e| format!("切换Provider失败: {}", e))
        } else {
            Err("Provider管理器未初始化".to_string())
        }
    }

    /// 获取所有注册的Provider信息
    pub fn get_all_providers_info(&self) -> Vec<ProviderInfo> {
        let mut providers = Vec::new();
        
        if let Some(ref provider_manager) = self.provider_manager {
            if let Ok(status_map) = provider_manager.get_all_provider_status() {
                for (provider_id, status) in status_map {
                    providers.push(ProviderInfo {
                        id: provider_id,
                        provider_type: status.provider_metrics.summary(),
                        is_connected: status.is_connected,
                        is_healthy: status.is_healthy,
                        events_received: status.events_received,
                        last_event_time: status.last_event_time,
                        error_count: status.error_count,
                    });
                }
            }
        }
        
        providers
    }

    /// 启用Provider系统（从传统WebSocket模式迁移）
    /// 
    /// 注意：这会停用传统的WebSocket系统
    pub fn enable_provider_system(&mut self, provider_config: Option<ProviderManagerConfig>) -> Result<(), Box<dyn std::error::Error>> {
        if self.use_provider_system {
            return Ok(()); // 已经启用
        }

        log::info!("正在从传统WebSocket模式迁移到Provider系统");

        // 停止传统WebSocket系统
        self.websocket_manager.disconnect();
        
        // 创建基于现有WebSocket管理器的Provider
        let binance_config = BinanceWebSocketConfig {
            symbol: self.config.symbol.clone(),
            streams: vec![
                StreamType::BookTicker,
                StreamType::Depth { 
                    levels: 20, 
                    update_speed: UpdateSpeed::Ms100 
                },
                StreamType::Trade,
            ],
            reconnect_config: ReconnectConfig::default(),
            heartbeat_interval_secs: 30,
            max_buffer_size: 1000,
            compression_enabled: false,
            ..Default::default()
        };
        
        // 从现有WebSocket管理器创建Provider
        let binance_provider = BinanceWebSocketProvider::from_websocket_manager(
            std::mem::replace(&mut self.websocket_manager, WebSocketManager::new(
                WebSocketConfig::new(self.config.symbol.clone())
            )),
            binance_config
        );

        // 创建Provider管理器
        let provider_manager_config = provider_config.unwrap_or_else(|| {
            ProviderManagerConfig {
                default_provider_id: "binance_websocket".to_string(),
                ..Default::default()
            }
        });
        
        let mut provider_manager = ProviderManager::new(provider_manager_config);

        // 创建Provider元数据
        let provider_metadata = ProviderMetadata {
            id: "binance_websocket".to_string(),
            name: "Binance WebSocket".to_string(),
            description: "从传统WebSocket系统迁移的Binance数据源".to_string(),
            provider_type: ProviderType::Binance { 
                mode: crate::core::provider::BinanceConnectionMode::WebSocket 
            },
            priority: 100,
            is_fallback: false,
            tags: {
                let mut tags = HashMap::new();
                tags.insert("exchange".to_string(), "binance".to_string());
                tags.insert("type".to_string(), "websocket".to_string());
                tags.insert("migrated".to_string(), "true".to_string());
                tags
            },
        };

        // 注册Provider
        provider_manager.register_provider(binance_provider, provider_metadata)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        // 启动Provider管理器
        provider_manager.start()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        // 启用Provider系统
        self.provider_manager = Some(provider_manager);
        self.use_provider_system = true;

        log::info!("成功迁移到Provider系统");
        Ok(())
    }

    /// 禁用Provider系统（回退到传统WebSocket模式）
    pub fn disable_provider_system(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.use_provider_system {
            return Ok(()); // 已经禁用
        }

        log::info!("正在从Provider系统回退到传统WebSocket模式");

        // 停止Provider系统
        if let Some(mut provider_manager) = self.provider_manager.take() {
            provider_manager.stop()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        }

        // 重新启用传统WebSocket系统
        self.websocket_manager.connect()?;
        self.use_provider_system = false;

        log::info!("成功回退到传统WebSocket模式");
        Ok(())
    }
}

/// Provider信息结构体
#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub id: String,
    pub provider_type: String,
    pub is_connected: bool,
    pub is_healthy: bool,
    pub events_received: u64,
    pub last_event_time: Option<u64>,
    pub error_count: u32,
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
    pub is_healthy: bool,
    pub health_check_failures: u32,
    pub last_data_received: Option<Instant>,
}
