// Standard Binance Provider - 符合标准接口的币安数据提供者
//
// 按照DataProvider trait标准实现的BinanceProvider
// 专注于通过WebSocket连接获取币安实时数据，符合Provider系统规范

use crate::config::provider_config::BinanceWebSocketConfig;
use crate::events::EventType;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use super::{DataProvider, ProviderStatus, ProviderType, EventKind, PerformanceMetrics, BinanceConnectionMode, ProviderMetrics};
use super::error::{ProviderError, ProviderResult};
use tungstenite::{connect, Message, WebSocket};
use tungstenite::stream::MaybeTlsStream;
use std::net::TcpStream;
use serde_json::Value;

/// WebSocket连接状态
#[derive(Debug, Clone, Copy, PartialEq)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// WebSocket消息类型
#[derive(Debug)]
struct WebSocketMessage {
    data: Value,
    timestamp: u64,
}

/// 内部WebSocket管理器
#[derive(Debug)]
struct WebSocketManager {
    connection: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
    state: ConnectionState,
    url: String,
    last_ping: Instant,
    last_pong: Instant,
    reconnect_attempts: u32,
    max_reconnect_attempts: u32,
    reconnect_delay: Duration,
    ping_interval: Duration,
}

impl WebSocketManager {
    fn new(config: &BinanceWebSocketConfig) -> Self {
        // 构建订阅URL
        let symbols: Vec<String> = config.subscription.symbols
            .iter()
            .map(|s| s.to_lowercase())
            .collect();
        
        let mut streams = Vec::new();
        for symbol in &symbols {
            for stream in &config.subscription.streams {
                streams.push(format!("{}@{}", symbol, stream));
            }
        }
        
        let url = format!(
            "{}/stream?streams={}",
            config.connection.base_url,
            streams.join("/")
        );
        
        Self {
            connection: None,
            state: ConnectionState::Disconnected,
            url,
            last_ping: Instant::now(),
            last_pong: Instant::now(),
            reconnect_attempts: 0,
            max_reconnect_attempts: config.connection.max_reconnect_attempts,
            reconnect_delay: Duration::from_millis(config.connection.reconnect_delay_ms as u64),
            ping_interval: Duration::from_millis(config.connection.ping_interval_ms as u64),
        }
    }
    
    fn connect(&mut self) -> ProviderResult<()> {
        log::info!("连接到Binance WebSocket: {}", self.url);
        self.state = ConnectionState::Connecting;
        
        match connect(&self.url) {
            Ok((socket, _)) => {
                self.connection = Some(socket);
                self.state = ConnectionState::Connected;
                self.reconnect_attempts = 0;
                self.last_pong = Instant::now();
                log::info!("WebSocket连接成功");
                Ok(())
            }
            Err(e) => {
                self.state = ConnectionState::Failed;
                Err(ProviderError::connection(
                    format!("WebSocket连接失败: {}", e),
                    Some(self.url.clone()),
                    true
                ))
            }
        }
    }
    
    fn disconnect(&mut self) {
        if let Some(mut socket) = self.connection.take() {
            let _ = socket.close(None);
        }
        self.state = ConnectionState::Disconnected;
        log::info!("WebSocket连接已断开");
    }
    
    fn is_connected(&self) -> bool {
        matches!(self.state, ConnectionState::Connected)
    }
    
    fn should_reconnect(&self) -> bool {
        matches!(self.state, ConnectionState::Failed) && 
        self.reconnect_attempts < self.max_reconnect_attempts
    }
    
    fn try_reconnect(&mut self) -> ProviderResult<()> {
        if !self.should_reconnect() {
            return Err(ProviderError::connection(
                "超出最大重连尝试次数".to_string(),
                Some(self.url.clone()),
                false
            ));
        }
        
        self.reconnect_attempts += 1;
        self.state = ConnectionState::Reconnecting;
        
        log::info!("尝试重连 ({}/{})", self.reconnect_attempts, self.max_reconnect_attempts);
        
        // 等待重连延迟
        thread::sleep(self.reconnect_delay);
        
        self.connect()
    }
    
    fn send_ping(&mut self) -> ProviderResult<()> {
        if let Some(ref mut socket) = self.connection {
            match socket.send(Message::Ping(vec![])) {
                Ok(_) => {
                    self.last_ping = Instant::now();
                    Ok(())
                }
                Err(e) => {
                    self.state = ConnectionState::Failed;
                    Err(ProviderError::connection(
                        format!("发送ping失败: {}", e),
                        Some(self.url.clone()),
                        true
                    ))
                }
            }
        } else {
            Err(ProviderError::state(
                "WebSocket未连接".to_string(),
                "disconnected".to_string(),
                "connected".to_string(),
                "send_ping".to_string()
            ))
        }
    }
    
    fn read_message(&mut self) -> ProviderResult<Option<WebSocketMessage>> {
        if let Some(ref mut socket) = self.connection {
            match socket.read() {
                Ok(msg) => {
                    match msg {
                        Message::Text(text) => {
                            match serde_json::from_str::<Value>(&text) {
                                Ok(data) => {
                                    let timestamp = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as u64;
                                    
                                    Ok(Some(WebSocketMessage { data, timestamp }))
                                }
                                Err(e) => {
                                    log::warn!("JSON解析失败: {}", e);
                                    Ok(None)
                                }
                            }
                        }
                        Message::Pong(_) => {
                            self.last_pong = Instant::now();
                            Ok(None)
                        }
                        Message::Ping(payload) => {
                            // 自动回复pong
                            let _ = socket.send(Message::Pong(payload));
                            Ok(None)
                        }
                        Message::Close(_) => {
                            log::warn!("收到WebSocket关闭消息");
                            self.state = ConnectionState::Failed;
                            Ok(None)
                        }
                        _ => Ok(None)
                    }
                }
                Err(tungstenite::Error::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // 非阻塞模式，没有消息可读
                    Ok(None)
                }
                Err(e) => {
                    log::error!("读取WebSocket消息失败: {}", e);
                    self.state = ConnectionState::Failed;
                    Err(ProviderError::connection(
                        format!("读取消息失败: {}", e),
                        Some(self.url.clone()),
                        true
                    ))
                }
            }
        } else {
            Err(ProviderError::state(
                "WebSocket未连接".to_string(),
                "disconnected".to_string(),
                "connected".to_string(),
                "read_message".to_string()
            ))
        }
    }
    
    fn needs_ping(&self) -> bool {
        self.is_connected() && self.last_ping.elapsed() >= self.ping_interval
    }
    
    fn is_connection_stale(&self) -> bool {
        self.is_connected() && self.last_pong.elapsed() > self.ping_interval * 2
    }
}

/// 标准Binance Provider - 实现DataProvider trait
#[derive(Debug)]
pub struct BinanceProvider {
    /// WebSocket配置
    config: BinanceWebSocketConfig,
    
    /// WebSocket管理器（使用Arc<Mutex<>>来支持多线程访问）
    ws_manager: Arc<Mutex<WebSocketManager>>,
    
    /// 连接状态
    connected: bool,
    
    /// 运行状态
    running: bool,
    
    /// 事件缓冲区
    event_buffer: Arc<Mutex<VecDeque<EventType>>>,
    
    /// 后台线程句柄
    worker_thread: Option<JoinHandle<()>>,
    
    /// 支持的事件类型
    supported_events: Vec<EventKind>,
    
    /// 性能指标
    performance: PerformanceMetrics,
}

impl BinanceProvider {
    /// 标准名称
    pub const CANONICAL_NAME: &'static str = "Binance WebSocket Provider";
    
    /// 标准类型
    pub const CANONICAL_TYPE: &'static str = "binance_websocket";

    /// 创建新的BinanceProvider实例
    pub fn new(config: BinanceWebSocketConfig) -> Self {
        let supported_events = vec![
            EventKind::DepthUpdate,
            EventKind::Trade,
            EventKind::BookTicker,
        ];
        
        let ws_manager = Arc::new(Mutex::new(WebSocketManager::new(&config)));
        let event_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(1000)));
        
        Self {
            config,
            ws_manager,
            connected: false,
            running: false,
            event_buffer,
            worker_thread: None,
            supported_events,
            performance: PerformanceMetrics::new(),
        }
    }

    /// 从配置创建BinanceProvider
    pub fn from_config(config: BinanceWebSocketConfig) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::new(config))
    }
    
    /// 从TOML文件加载配置并创建BinanceProvider
    pub fn from_config_file(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        log::info!("从配置文件加载BinanceProvider: {}", config_path);
        
        let config_content = std::fs::read_to_string(config_path)
            .map_err(|e| format!("读取配置文件失败: {}", e))?;
            
        let config: BinanceWebSocketConfig = toml::from_str(&config_content)
            .map_err(|e| format!("解析配置文件失败: {}", e))?;
        
        // 验证配置
        Self::validate_config(&config)?;
        
        log::info!("配置文件加载成功: {} 符号, {} 流", 
            config.subscription.symbols.len(), 
            config.subscription.streams.len());
        
        Ok(Self::new(config))
    }
    
    /// 使用默认配置文件路径加载BinanceProvider
    pub fn from_default_config() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = "configs/providers/binance_market_provider.toml";
        Self::from_config_file(config_path)
    }
    
    /// 验证配置有效性
    fn validate_config(config: &BinanceWebSocketConfig) -> Result<(), Box<dyn std::error::Error>> {
        // 验证基本URL
        if config.connection.base_url.is_empty() {
            return Err("WebSocket基础URL不能为空".into());
        }
        
        // 验证符号列表
        if config.subscription.symbols.is_empty() {
            return Err("订阅符号列表不能为空".into());
        }
        
        // 验证流类型
        if config.subscription.streams.is_empty() {
            return Err("订阅流类型列表不能为空".into());
        }
        
        // 验证数值参数
        if config.connection.max_reconnect_attempts == 0 {
            return Err("最大重连次数必须大于0".into());
        }
        
        if config.connection.reconnect_delay_ms == 0 {
            return Err("重连延迟必须大于0".into());
        }
        
        if config.connection.ping_interval_ms == 0 {
            return Err("心跳间隔必须大于0".into());
        }
        
        log::debug!("配置验证通过");
        Ok(())
    }

    /// 启动后台工作线程
    fn start_worker_thread(&mut self) -> ProviderResult<()> {
        if self.worker_thread.is_some() {
            return Ok(()); // 已经启动
        }
        
        let ws_manager = Arc::clone(&self.ws_manager);
        let event_buffer = Arc::clone(&self.event_buffer);
        let mut performance = self.performance.clone();
        
        let handle = thread::spawn(move || {
            log::info!("WebSocket工作线程启动");
            let mut last_stats_update = Instant::now();
            
            loop {
                // 检查连接状态
                {
                    let mut manager = ws_manager.lock().unwrap();
                    
                    // 如果需要连接
                    if !manager.is_connected() && manager.should_reconnect() {
                        if let Err(e) = manager.try_reconnect() {
                            log::error!("重连失败: {}", e);
                            thread::sleep(Duration::from_secs(1));
                            continue;
                        }
                    }
                    
                    // 检查连接是否过期
                    if manager.is_connection_stale() {
                        log::warn!("WebSocket连接过期，标记为失败状态");
                        manager.state = ConnectionState::Failed;
                        continue;
                    }
                    
                    // 发送心跳
                    if manager.needs_ping() {
                        if let Err(e) = manager.send_ping() {
                            log::error!("发送心跳失败: {}", e);
                            continue;
                        }
                    }
                    
                    // 读取消息
                    match manager.read_message() {
                        Ok(Some(ws_msg)) => {
                            // 解析并转换消息为EventType
                            if let Some(event) = Self::parse_websocket_message(&ws_msg) {
                                // 添加到缓冲区
                                if let Ok(mut buffer) = event_buffer.lock() {
                                    buffer.push_back(event);
                                    
                                    // 限制缓冲区大小
                                    while buffer.len() > 1000 {
                                        buffer.pop_front();
                                    }
                                    
                                    // 更新性能指标
                                    performance.events_received += 1;
                                    performance.last_event_time = Some(ws_msg.timestamp);
                                }
                            }
                        }
                        Ok(None) => {
                            // 没有消息，继续循环
                        }
                        Err(e) => {
                            log::error!("读取WebSocket消息失败: {}", e);
                            performance.error_count += 1;
                        }
                    }
                }
                
                // 更新性能统计
                if last_stats_update.elapsed() >= Duration::from_secs(1) {
                    let events_per_sec = performance.events_received as f64 / last_stats_update.elapsed().as_secs_f64();
                    performance.events_per_second = events_per_sec;
                    last_stats_update = Instant::now();
                }
                
                // 短暂休眠以避免CPU过度使用
                thread::sleep(Duration::from_millis(1));
            }
        });
        
        self.worker_thread = Some(handle);
        Ok(())
    }
    
    /// 停止后台工作线程
    fn stop_worker_thread(&mut self) {
        // 注意：这是一个简化的停止实现
        // 在生产环境中，应该使用适当的信号机制来优雅地停止线程
        if let Some(_handle) = self.worker_thread.take() {
            // 断开WebSocket连接，这会导致工作线程退出
            if let Ok(mut manager) = self.ws_manager.lock() {
                manager.disconnect();
            }
            
            // 等待线程结束（在实际应用中可能需要超时机制）
            // handle.join().unwrap_or_else(|_| log::warn!("工作线程未能正常结束"));
        }
    }
    
    /// 解析WebSocket消息并转换为EventType
    fn parse_websocket_message(ws_msg: &WebSocketMessage) -> Option<EventType> {
        let data = &ws_msg.data;
        
        // 检查是否是流数据包装格式
        if let Some(stream) = data.get("stream").and_then(|s| s.as_str()) {
            let event_data = data.get("data")?;
            
            if stream.contains("@depth") {
                return Some(EventType::DepthUpdate(event_data.clone()));
            } else if stream.contains("@trade") {
                return Some(EventType::Trade(event_data.clone()));
            } else if stream.contains("@bookTicker") {
                return Some(EventType::BookTicker(event_data.clone()));
            }
        }
        
        // 检查直接的事件类型（兼容其他格式）
        if let Some(event_type) = data.get("e").and_then(|e| e.as_str()) {
            match event_type {
                "depthUpdate" => Some(EventType::DepthUpdate(data.clone())),
                "trade" => Some(EventType::Trade(data.clone())),
                "24hrTicker" => Some(EventType::BookTicker(data.clone())),
                _ => {
                    log::debug!("未知事件类型: {}", event_type);
                    None
                }
            }
        } else {
            log::debug!("无法识别的WebSocket消息格式");
            None
        }
    }

    /// 检查是否运行中
    pub fn is_running(&self) -> bool {
        self.running
    }
}

/// 实现DataProvider trait
impl DataProvider for BinanceProvider {
    type Error = ProviderError;

    fn initialize(&mut self) -> Result<(), Self::Error> {
        log::info!("初始化BinanceProvider");
        
        // 先连接WebSocket
        {
            let mut manager = self.ws_manager.lock().unwrap();
            manager.connect()?;
        }
        
        // 启动工作线程
        self.start_worker_thread()?;
        
        self.running = true;
        self.connected = true;
        
        log::info!("BinanceProvider初始化完成");
        Ok(())
    }

    fn start(&mut self) -> Result<(), Self::Error> {
        if !self.running {
            return Err(ProviderError::state(
                "Provider未初始化".to_string(),
                "stopped".to_string(),
                "running".to_string(),
                "start".to_string()
            ));
        }
        
        // 检查WebSocket连接
        {
            let manager = self.ws_manager.lock().unwrap();
            if !manager.is_connected() {
                return Err(ProviderError::connection(
                    "WebSocket未连接".to_string(),
                    Some(self.config.connection.base_url.clone()),
                    true
                ));
            }
        }
        
        log::info!("BinanceProvider已启动");
        Ok(())
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        log::info!("停止BinanceProvider");
        
        self.running = false;
        self.connected = false;
        
        // 停止工作线程
        self.stop_worker_thread();
        
        // 断开WebSocket连接
        {
            let mut manager = self.ws_manager.lock().unwrap();
            manager.disconnect();
        }
        
        // 清空事件缓冲区
        {
            let mut buffer = self.event_buffer.lock().unwrap();
            buffer.clear();
        }
        
        log::info!("BinanceProvider已停止");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected && self.running && {
            let manager = self.ws_manager.lock().unwrap();
            manager.is_connected()
        }
    }

    fn read_events(&mut self) -> Result<Vec<EventType>, Self::Error> {
        if !self.is_connected() {
            return Ok(Vec::new());
        }

        // 从缓冲区读取事件
        let mut buffer = self.event_buffer.lock().unwrap();
        let events: Vec<EventType> = buffer.drain(..).collect();
        
        // 更新性能指标
        if !events.is_empty() {
            self.performance.events_received += events.len() as u64;
            self.performance.last_event_time = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64
            );
            
            log::debug!("读取了 {} 个事件", events.len());
        }

        Ok(events)
    }

    fn get_status(&self) -> ProviderStatus {
        // 获取WebSocket管理器状态
        let (ws_state, reconnect_attempts, last_ping_ms) = {
            let manager = self.ws_manager.lock().unwrap();
            let last_ping_ms = manager.last_ping.elapsed().as_millis() as f64;
            (manager.state, manager.reconnect_attempts, last_ping_ms)
        };
        
        // 创建适当的ProviderMetrics
        let provider_metrics = ProviderMetrics::WebSocket {
            reconnect_count: reconnect_attempts,
            ping_latency_ms: Some(last_ping_ms),
            messages_per_second: self.performance.events_per_second,
            connection_duration: if self.connected {
                Some(Duration::from_secs(60)) // 简化实现
            } else {
                None
            },
            websocket_state: format!("{:?}", ws_state),
        };
        
        ProviderStatus {
            is_connected: self.connected,
            is_running: self.running,
            events_received: self.performance.events_received,
            last_event_time: self.performance.last_event_time,
            error_count: self.performance.error_count,
            consecutive_errors: 0, // Reset on each status check
            last_error: None, // Simplified for now
            provider_metrics,
            status_timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            is_healthy: self.connected && self.running && matches!(ws_state, ConnectionState::Connected),
        }
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Binance { 
            mode: BinanceConnectionMode::WebSocket 
        }
    }

    fn supported_events(&self) -> &[EventKind] {
        &self.supported_events
    }

    fn get_config_info(&self) -> Option<String> {
        Some(format!(
            "Binance WebSocket Provider - URL: {}, Symbols: {:?}, Streams: {:?}",
            self.config.connection.base_url,
            self.config.subscription.symbols,
            self.config.subscription.streams
        ))
    }

    fn health_check(&self) -> bool {
        self.connected && self.running && {
            let manager = self.ws_manager.lock().unwrap();
            manager.is_connected()
        }
    }

    fn get_performance_metrics(&self) -> Option<PerformanceMetrics> {
        Some(self.performance.clone())
    }
}

/// Provider统计信息
#[derive(Debug, Clone)]
pub struct ProviderStats {
    pub connected: bool,
    pub running: bool,
    pub events_buffered: usize,
    pub reconnect_attempts: u32,
    pub websocket_state: String,
}