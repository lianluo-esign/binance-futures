// Binance WebSocket数据提供者
//
// 本文件实现了基于Binance WebSocket API的数据Provider，负责：
// - 重构现有WebSocketManager为Provider接口
// - 实时市场数据获取和解析
// - 连接管理和自动重连
// - 事件映射和类型转换
//
// 设计原则：
// 1. 完全兼容：保持现有WebSocket功能完全不变
// 2. 无缝集成：直接使用现有WebSocketManager
// 3. 状态透明：提供详细的状态监控
// 4. 错误恢复：完善的错误处理和重连机制

use super::{
    DataProvider, ProviderType, ProviderStatus, EventKind, PerformanceMetrics,
    error::{ProviderError, ProviderResult, TimeoutType},
};
use crate::events::EventType;
use crate::websocket::{WebSocketManager, WebSocketConfig};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};

/// Binance WebSocket配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceWebSocketConfig {
    /// 交易对符号
    pub symbol: String,
    
    /// WebSocket端点URL
    pub endpoint_url: Option<String>,
    
    /// 订阅的数据流类型
    pub streams: Vec<StreamType>,
    
    /// 重连配置
    pub reconnect_config: ReconnectConfig,
    
    /// 心跳间隔（秒）
    pub heartbeat_interval_secs: u64,
    
    /// 最大消息缓冲区大小
    pub max_buffer_size: usize,
    
    /// 是否启用压缩
    pub compression_enabled: bool,
}

impl Default for BinanceWebSocketConfig {
    fn default() -> Self {
        Self {
            symbol: "BTCUSDT".to_string(),
            endpoint_url: None, // 使用默认端点
            streams: vec![
                StreamType::BookTicker,
                StreamType::Depth { levels: 20, update_speed: UpdateSpeed::Ms100 },
                StreamType::Trade,
            ],
            reconnect_config: ReconnectConfig::default(),
            heartbeat_interval_secs: 30,
            max_buffer_size: 1000,
            compression_enabled: false,
        }
    }
}

/// WebSocket数据流类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StreamType {
    /// 最佳买卖价数据流
    BookTicker,
    
    /// 深度数据流
    Depth {
        levels: u16,
        update_speed: UpdateSpeed,
    },
    
    /// 成交数据流
    Trade,
    
    /// K线数据流
    Kline {
        interval: String, // "1m", "5m", "1h" etc.
    },
    
    /// 24小时统计数据流
    Ticker24hr,
    
    /// 迷你统计数据流
    MiniTicker,
}

impl StreamType {
    /// 生成Binance WebSocket流名称
    pub fn to_binance_stream(&self, symbol: &str) -> String {
        let symbol_lower = symbol.to_lowercase();
        match self {
            StreamType::BookTicker => format!("{}@bookTicker", symbol_lower),
            StreamType::Depth { levels, update_speed } => {
                format!("{}@depth{}@{}", symbol_lower, levels, update_speed.as_str())
            }
            StreamType::Trade => format!("{}@trade", symbol_lower),
            StreamType::Kline { interval } => format!("{}@kline_{}", symbol_lower, interval),
            StreamType::Ticker24hr => format!("{}@ticker", symbol_lower),
            StreamType::MiniTicker => format!("{}@miniTicker", symbol_lower),
        }
    }

    /// 获取对应的EventKind
    pub fn to_event_kind(&self) -> EventKind {
        match self {
            StreamType::BookTicker => EventKind::BookTicker,
            StreamType::Depth { .. } => EventKind::DepthUpdate,
            StreamType::Trade => EventKind::Trade,
            StreamType::Kline { .. } => EventKind::TickPrice,
            StreamType::Ticker24hr => EventKind::TickPrice,
            StreamType::MiniTicker => EventKind::TickPrice,
        }
    }
}

/// 更新速度配置
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum UpdateSpeed {
    /// 1000ms间隔
    Ms1000,
    /// 100ms间隔
    Ms100,
}

impl UpdateSpeed {
    pub fn as_str(&self) -> &'static str {
        match self {
            UpdateSpeed::Ms1000 => "1000ms",
            UpdateSpeed::Ms100 => "100ms",
        }
    }
}

/// 重连配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectConfig {
    /// 是否启用自动重连
    pub enabled: bool,
    
    /// 最大重连次数
    pub max_attempts: u32,
    
    /// 初始重连延迟（毫秒）
    pub initial_delay_ms: u64,
    
    /// 最大重连延迟（毫秒）
    pub max_delay_ms: u64,
    
    /// 延迟倍增因子
    pub backoff_multiplier: f64,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Binance WebSocket Provider实现
/// 
/// 基于现有WebSocketManager的Provider包装器，保持完全兼容
pub struct BinanceWebSocketProvider {
    /// 内部WebSocket管理器（复用现有实现）
    websocket_manager: WebSocketManager,
    
    /// Provider配置
    config: BinanceWebSocketConfig,
    
    /// Provider状态
    status: ProviderStatus,
    
    /// 支持的事件类型
    supported_events: Vec<EventKind>,
    
    /// 启动时间
    start_time: Option<Instant>,
    
    /// 统计信息
    events_received_total: u64,
    last_event_time: Option<Instant>,
    error_count: u32,
    
    /// 性能监控
    performance_window_start: Instant,
    performance_events_count: u64,
    performance_bytes_count: u64,
}

impl std::fmt::Debug for BinanceWebSocketProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinanceWebSocketProvider")
            .field("config", &self.config)
            .field("status", &self.status)
            .field("supported_events", &self.supported_events)
            .field("events_received_total", &self.events_received_total)
            .finish()
    }
}

impl BinanceWebSocketProvider {
    /// 创建新的Binance WebSocket Provider
    pub fn new(config: BinanceWebSocketConfig) -> Self {
        // 转换为现有WebSocketConfig格式
        let ws_config = WebSocketConfig::new(config.symbol.clone());
        let websocket_manager = WebSocketManager::new(ws_config);
        
        // 计算支持的事件类型
        let supported_events = config.streams
            .iter()
            .map(|stream| stream.to_event_kind())
            .collect::<Vec<_>>();
        
        // 初始化状态
        let mut status = ProviderStatus::new(ProviderType::Binance { mode: super::BinanceConnectionMode::WebSocket });
        status.provider_metrics = super::types::ProviderMetrics::WebSocket {
            reconnect_count: 0,
            ping_latency_ms: None,
            messages_per_second: 0.0,
            connection_duration: None,
            websocket_state: "Disconnected".to_string(),
        };
        
        let now = Instant::now();
        
        Self {
            websocket_manager,
            config,
            status,
            supported_events,
            start_time: None,
            events_received_total: 0,
            last_event_time: None,
            error_count: 0,
            performance_window_start: now,
            performance_events_count: 0,
            performance_bytes_count: 0,
        }
    }

    /// 从现有WebSocketManager创建Provider（用于现有代码兼容）
    pub fn from_websocket_manager(
        websocket_manager: WebSocketManager,
        config: BinanceWebSocketConfig,
    ) -> Self {
        let supported_events = config.streams
            .iter()
            .map(|stream| stream.to_event_kind())
            .collect::<Vec<_>>();
        
        let mut status = ProviderStatus::new(ProviderType::Binance { mode: super::BinanceConnectionMode::WebSocket });
        let is_connected = websocket_manager.is_connected();
        status.is_connected = is_connected;
        
        let now = Instant::now();
        
        Self {
            websocket_manager,
            config,
            status,
            supported_events,
            start_time: if is_connected { Some(now) } else { None },
            events_received_total: 0,
            last_event_time: None,
            error_count: 0,
            performance_window_start: now,
            performance_events_count: 0,
            performance_bytes_count: 0,
        }
    }

    /// 获取底层WebSocketManager的引用（用于现有代码兼容）
    pub fn websocket_manager(&self) -> &WebSocketManager {
        &self.websocket_manager
    }

    /// 获取底层WebSocketManager的可变引用（用于现有代码兼容）
    pub fn websocket_manager_mut(&mut self) -> &mut WebSocketManager {
        &mut self.websocket_manager
    }

    /// 更新状态信息
    fn update_status(&mut self) {
        let ws_connected = self.websocket_manager.is_connected();
        let was_connected = self.status.is_connected;
        
        self.status.is_connected = ws_connected;
        self.status.update_timestamp();
        
        // 如果连接状态发生变化，更新相关状态
        if ws_connected && !was_connected {
            self.start_time = Some(Instant::now());
        } else if !ws_connected && was_connected {
            self.start_time = None;
        }
        
        // 更新WebSocket特定指标
        if let super::types::ProviderMetrics::WebSocket {
            ref mut reconnect_count,
            ref mut ping_latency_ms,
            ref mut messages_per_second,
            ref mut connection_duration,
            ref mut websocket_state,
        } = self.status.provider_metrics {
            
            // 获取WebSocket管理器的统计信息
            if let Some(ws_stats) = self.websocket_manager.get_stats() {
                *reconnect_count = ws_stats.reconnect_attempts;
                *connection_duration = ws_stats.connection_duration;
            }
            
            // 更新连接状态
            *websocket_state = if ws_connected {
                "Connected".to_string()
            } else if self.websocket_manager.should_reconnect() {
                "Reconnecting".to_string()
            } else {
                "Disconnected".to_string()
            };
            
            // 计算消息频率
            if let Some(start_time) = self.start_time {
                let duration = start_time.elapsed();
                if duration.as_secs() > 0 {
                    *messages_per_second = self.events_received_total as f64 / duration.as_secs_f64();
                }
            }
        }
        
        // 更新健康状态
        self.status.is_healthy = self.check_health_internal();
    }

    /// 内部健康检查
    fn check_health_internal(&self) -> bool {
        let base_healthy = self.status.is_connected && self.status.consecutive_errors < 5;
        
        // 检查最近是否有数据
        let recent_data = self.last_event_time
            .map(|t| t.elapsed().as_secs() < 60)
            .unwrap_or(false);
        
        base_healthy && (recent_data || self.start_time.map(|t| t.elapsed().as_secs() < 30).unwrap_or(true))
    }

    /// 将WebSocket消息转换为EventType
    fn convert_message_to_event(&self, message: &Value) -> Option<EventType> {
        // 检查消息结构
        if let Some(stream) = message.get("stream").and_then(|s| s.as_str()) {
            // 根据流类型转换事件
            if stream.contains("@bookTicker") {
                Some(EventType::BookTicker(message.clone()))
            } else if stream.contains("@depth") {
                Some(EventType::DepthUpdate(message.clone()))
            } else if stream.contains("@trade") {
                Some(EventType::Trade(message.clone()))
            } else if stream.contains("@kline") || stream.contains("@ticker") {
                Some(EventType::TickPrice(message.clone()))
            } else {
                // 未知流类型，记录警告
                log::warn!("未知的WebSocket流类型: {}", stream);
                None
            }
        } else {
            // 可能是旧格式消息或错误消息
            if message.get("e").is_some() {
                // 处理事件类型字段
                if let Some(event_type) = message.get("e").and_then(|e| e.as_str()) {
                    match event_type {
                        "bookTicker" => Some(EventType::BookTicker(message.clone())),
                        "depthUpdate" => Some(EventType::DepthUpdate(message.clone())),
                        "trade" => Some(EventType::Trade(message.clone())),
                        "kline" => Some(EventType::TickPrice(message.clone())),
                        "24hrTicker" => Some(EventType::TickPrice(message.clone())),
                        _ => {
                            log::warn!("未知的事件类型: {}", event_type);
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                // 可能是错误消息或其他类型
                log::debug!("无法识别的WebSocket消息格式: {}", message);
                None
            }
        }
    }

    /// 更新性能统计
    fn update_performance_metrics(&mut self, events_count: usize, estimated_bytes: usize) {
        self.performance_events_count += events_count as u64;
        self.performance_bytes_count += estimated_bytes as u64;
    }
}

impl DataProvider for BinanceWebSocketProvider {
    type Error = ProviderError;

    fn initialize(&mut self) -> ProviderResult<()> {
        log::info!("初始化Binance WebSocket Provider: {}", self.config.symbol);
        
        // 验证配置
        if self.config.symbol.is_empty() {
            return Err(ProviderError::configuration_field(
                "交易对符号不能为空",
                "symbol",
                Some("非空字符串".to_string()),
                Some("空字符串".to_string()),
            ));
        }
        
        if self.config.streams.is_empty() {
            return Err(ProviderError::configuration_field(
                "至少需要订阅一个数据流",
                "streams",
                Some("非空数组".to_string()),
                Some("空数组".to_string()),
            ));
        }
        
        // TODO: 在这里可以添加更多的初始化逻辑，如：
        // - 验证交易对是否有效
        // - 检查网络连通性
        // - 预热连接池等
        
        self.status.is_running = false; // 初始化完成但未启动
        self.update_status();
        
        log::info!("Binance WebSocket Provider初始化完成");
        Ok(())
    }

    fn start(&mut self) -> ProviderResult<()> {
        log::info!("启动Binance WebSocket Provider");
        
        // 建立WebSocket连接
        self.websocket_manager.connect()
            .map_err(|e| ProviderError::connection(
                format!("WebSocket连接失败: {}", e),
                Some("wss://stream.binance.com:9443".to_string()),
                true,
            ))?;
        
        // 订阅数据流
        let streams: Vec<String> = self.config.streams
            .iter()
            .map(|stream| stream.to_binance_stream(&self.config.symbol))
            .collect();
        
        self.websocket_manager.subscribe(streams)
            .map_err(|e| ProviderError::connection(
                format!("订阅数据流失败: {}", e),
                None,
                true,
            ))?;
        
        self.status.is_running = true;
        self.start_time = Some(Instant::now());
        self.performance_window_start = Instant::now();
        self.update_status();
        
        log::info!("Binance WebSocket Provider启动完成，已订阅 {} 个数据流", self.config.streams.len());
        Ok(())
    }

    fn stop(&mut self) -> ProviderResult<()> {
        log::info!("停止Binance WebSocket Provider");
        
        self.websocket_manager.disconnect();
        self.status.is_running = false;
        self.start_time = None;
        self.update_status();
        
        log::info!("Binance WebSocket Provider已停止");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.websocket_manager.is_connected()
    }

    fn read_events(&mut self) -> ProviderResult<Vec<EventType>> {
        // 检查是否需要重连
        if !self.is_connected() && self.websocket_manager.should_reconnect() {
            match self.websocket_manager.attempt_reconnect() {
                Ok(()) => {
                    log::info!("WebSocket重连成功");
                    self.update_status();
                }
                Err(e) => {
                    self.error_count += 1;
                    self.status.record_error(&format!("重连失败: {}", e));
                    return Err(ProviderError::connection(
                        format!("重连失败: {}", e),
                        None,
                        true,
                    ));
                }
            }
        }
        
        // 读取WebSocket消息
        match self.websocket_manager.read_messages() {
            Ok(messages) => {
                let mut events = Vec::new();
                let mut estimated_bytes = 0;
                
                for message in messages {
                    // 估算消息大小（用于性能统计）
                    estimated_bytes += message.to_string().len();
                    
                    // 转换为EventType
                    if let Some(event) = self.convert_message_to_event(&message) {
                        events.push(event);
                        
                        // 更新统计
                        self.events_received_total += 1;
                        self.last_event_time = Some(Instant::now());
                        self.status.record_event();
                    }
                }
                
                // 更新性能统计
                self.update_performance_metrics(events.len(), estimated_bytes);
                self.update_status();
                
                Ok(events)
            }
            Err(e) => {
                self.error_count += 1;
                let error_msg = format!("读取WebSocket消息失败: {}", e);
                self.status.record_error(&error_msg);
                self.update_status();
                
                Err(ProviderError::connection(error_msg, None, true))
            }
        }
    }

    fn get_status(&self) -> ProviderStatus {
        self.status.clone()
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Binance { mode: super::BinanceConnectionMode::WebSocket }
    }

    fn supported_events(&self) -> &[EventKind] {
        &self.supported_events
    }

    fn get_config_info(&self) -> Option<String> {
        Some(format!(
            "Symbol: {}, Streams: {:?}, Endpoint: {}",
            self.config.symbol,
            self.config.streams,
            self.config.endpoint_url.as_deref().unwrap_or("default")
        ))
    }

    fn health_check(&self) -> bool {
        self.check_health_internal()
    }

    fn get_performance_metrics(&self) -> Option<PerformanceMetrics> {
        let window_duration = self.performance_window_start.elapsed();
        if window_duration.as_secs() == 0 {
            return None;
        }
        
        let events_per_second = self.performance_events_count as f64 / window_duration.as_secs_f64();
        let bytes_per_second = self.performance_bytes_count as f64 / window_duration.as_secs_f64();
        
        Some(PerformanceMetrics {
            events_per_second,
            bytes_per_second,
            average_latency_ms: 0.0, // TODO: 实现延迟测量
            max_latency_ms: 0.0,
            cpu_usage_percent: 0.0, // TODO: 实现CPU监控
            memory_usage_mb: 0.0, // TODO: 实现内存监控
            error_rate: if self.events_received_total > 0 {
                self.error_count as f64 / self.events_received_total as f64
            } else {
                0.0
            },
            window_seconds: window_duration.as_secs(),
        })
    }
}

// 为了兼容现有代码，提供一些便利方法
impl BinanceWebSocketProvider {
    /// 获取WebSocket健康状态（兼容现有代码）
    pub fn get_websocket_health_status(&self) -> crate::websocket::manager::WebSocketHealthStatus {
        self.websocket_manager.get_health_status()
    }

    /// 获取WebSocket性能信息（兼容现有代码）
    pub fn get_websocket_performance(&self) -> crate::websocket::manager::PerformanceInfo {
        self.websocket_manager.check_performance()
    }

    /// 发送ping（兼容现有代码）
    pub fn send_ping(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 这个功能在read_events中自动处理，这里提供兼容接口
        Ok(())
    }

    /// 获取连接状态描述（兼容现有代码）
    pub fn get_status_description(&self) -> String {
        format!(
            "Provider状态: {} | 连接: {} | 事件: {} | 错误: {}",
            if self.status.is_running { "运行中" } else { "已停止" },
            if self.status.is_connected { "已连接" } else { "未连接" },
            self.events_received_total,
            self.error_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_websocket_config_default() {
        let config = BinanceWebSocketConfig::default();
        assert_eq!(config.symbol, "BTCUSDT");
        assert!(!config.streams.is_empty());
        assert!(config.reconnect_config.enabled);
    }

    #[test]
    fn test_stream_type_to_binance_stream() {
        let book_ticker = StreamType::BookTicker;
        assert_eq!(book_ticker.to_binance_stream("BTCUSDT"), "btcusdt@bookTicker");

        let depth = StreamType::Depth { levels: 20, update_speed: UpdateSpeed::Ms100 };
        assert_eq!(depth.to_binance_stream("ETHUSDT"), "ethusdt@depth20@100ms");
    }

    #[test]
    fn test_stream_type_to_event_kind() {
        assert_eq!(StreamType::BookTicker.to_event_kind(), EventKind::BookTicker);
        assert_eq!(StreamType::Trade.to_event_kind(), EventKind::Trade);
        assert_eq!(
            StreamType::Depth { levels: 20, update_speed: UpdateSpeed::Ms100 }.to_event_kind(),
            EventKind::DepthUpdate
        );
    }

    #[test]
    fn test_provider_initialization() {
        let config = BinanceWebSocketConfig::default();
        let mut provider = BinanceWebSocketProvider::new(config);
        
        assert!(provider.initialize().is_ok());
        assert_eq!(provider.provider_type(), ProviderType::Binance { mode: super::BinanceConnectionMode::WebSocket });
        assert!(!provider.supported_events().is_empty());
    }

    #[test]
    fn test_empty_symbol_config_error() {
        let mut config = BinanceWebSocketConfig::default();
        config.symbol = String::new();
        
        let mut provider = BinanceWebSocketProvider::new(config);
        let result = provider.initialize();
        
        assert!(result.is_err());
        match result.unwrap_err() {
            ProviderError::ConfigurationError { field, .. } => {
                assert_eq!(field, Some("symbol".to_string()));
            }
            _ => panic!("Expected ConfigurationError"),
        }
    }
}