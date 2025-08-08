// Binance Provider - 币安交易所数据提供者
//
// 本文件实现了币安交易所的统一数据Provider，负责：
// - 整合现有WebSocketManager功能
// - 提供统一的币安数据接口
// - 支持WebSocket实时数据
// - 支持REST API查询（未来扩展）
// - 自动重连和错误恢复
//
// 设计原则：
// 1. 向后兼容：完全兼容现有WebSocket功能
// 2. 单一职责：专注于币安交易所数据处理
// 3. 可扩展性：为REST API和其他功能预留接口
// 4. 错误隔离：完善的错误处理和状态管理

use super::{
    DataProvider, ProviderType, ProviderStatus, EventKind, PerformanceMetrics,
    BinanceConnectionMode,
    error::{ProviderError, ProviderResult},
};
use crate::config::ProviderIdentity;
use crate::events::EventType;
use crate::websocket::{WebSocketManager, WebSocketConfig};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Instant;

/// 币安Provider配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceProviderConfig {
    /// 交易对符号列表 (支持多个symbol)
    pub symbols: Vec<String>,
    
    /// 向后兼容的单个symbol (优先级低于symbols)
    #[serde(skip_serializing_if = "String::is_empty")]
    pub symbol: String,
    
    /// 连接模式
    pub connection_mode: BinanceConnectionMode,
    
    /// WebSocket配置（当连接模式为WebSocket或Hybrid时使用）
    pub websocket_config: Option<BinanceWebSocketConfig>,
    
    /// REST API配置（当连接模式为RestAPI或Hybrid时使用）
    pub rest_api_config: Option<BinanceRestAPIConfig>,
    
    /// 自动故障转移配置
    pub failover_config: FailoverConfig,
}

impl Default for BinanceProviderConfig {
    fn default() -> Self {
        Self {
            symbols: vec!["BTCFDUSD".to_string()],
            symbol: String::new(), // 向后兼容，默认为空
            connection_mode: BinanceConnectionMode::WebSocket,
            websocket_config: Some(BinanceWebSocketConfig::default()),
            rest_api_config: None,
            failover_config: FailoverConfig::default(),
        }
    }
}

impl BinanceProviderConfig {
    /// 获取有效的交易对符号列表（优先使用symbols，否则使用单个symbol）
    pub fn get_symbols(&self) -> Vec<String> {
        if !self.symbols.is_empty() {
            self.symbols.clone()
        } else if !self.symbol.is_empty() {
            vec![self.symbol.clone()]
        } else {
            vec!["BTCFDUSD".to_string()] // 默认值
        }
    }
    
    /// 获取主要交易对符号（第一个symbol）
    pub fn get_primary_symbol(&self) -> String {
        let symbols = self.get_symbols();
        symbols.first().cloned().unwrap_or_else(|| "BTCFDUSD".to_string())
    }
}

/// 币安WebSocket配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceWebSocketConfig {
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
            endpoint_url: None,
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

/// 币安REST API配置（未来扩展）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceRestAPIConfig {
    /// API端点URL
    pub endpoint_url: String,
    
    /// API密钥（可选）
    pub api_key: Option<String>,
    
    /// API密钥Secret（可选）
    pub api_secret: Option<String>,
    
    /// 请求超时（毫秒）
    pub timeout_ms: u64,
    
    /// 请求频率限制（每秒请求数）
    pub rate_limit: u32,
}

impl Default for BinanceRestAPIConfig {
    fn default() -> Self {
        Self {
            endpoint_url: "https://fapi.binance.com".to_string(),
            api_key: None,
            api_secret: None,
            timeout_ms: 5000,
            rate_limit: 10,
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
    /// 从字符串创建 StreamType
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "depth" => Some(StreamType::Depth { levels: 20, update_speed: UpdateSpeed::Ms100 }),
            "trade" => Some(StreamType::Trade),
            "bookticker" => Some(StreamType::BookTicker),
            "ticker" => Some(StreamType::MiniTicker),
            _ => None,
        }
    }
    
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

/// 故障转移配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    /// 是否启用自动故障转移
    pub enabled: bool,
    
    /// WebSocket失败后是否切换到REST API
    pub websocket_to_rest_fallback: bool,
    
    /// 故障检测阈值（连续失败次数）
    pub failure_threshold: u32,
    
    /// 健康检查间隔（毫秒）
    pub health_check_interval_ms: u64,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            websocket_to_rest_fallback: false, // 暂时禁用，等REST API实现
            failure_threshold: 3,
            health_check_interval_ms: 5000,
        }
    }
}

/// 币安Provider实现
pub struct BinanceProvider {
    /// Provider配置
    config: BinanceProviderConfig,
    
    /// WebSocket管理器（主要数据源）
    websocket_manager: Option<WebSocketManager>,
    
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
    consecutive_failures: u32,
    
    /// 性能监控
    performance_window_start: Instant,
    performance_events_count: u64,
    performance_bytes_count: u64,
}

impl std::fmt::Debug for BinanceProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinanceProvider")
            .field("config", &self.config)
            .field("status", &self.status)
            .field("supported_events", &self.supported_events)
            .field("events_received_total", &self.events_received_total)
            .field("has_websocket_manager", &self.websocket_manager.is_some())
            .finish()
    }
}

impl BinanceProvider {
    /// 从配置创建新的币安Provider
    pub fn from_config(config: crate::config::BinanceWebSocketConfig) -> Result<Self, String> {
        // 简化的配置转换，与src/websocket保持一致
        let provider_config = BinanceProviderConfig {
            symbols: config.subscription.symbols.clone(),
            symbol: String::new(), // 不使用单个symbol
            connection_mode: BinanceConnectionMode::WebSocket,
            websocket_config: Some(BinanceWebSocketConfig {
                endpoint_url: Some(config.connection.base_url.clone()),
                streams: config.subscription.streams
                    .iter()
                    .filter_map(|s| StreamType::from_string(s))
                    .collect(), // 使用配置中的streams并转换为StreamType
                reconnect_config: ReconnectConfig {
                    enabled: true,
                    initial_delay_ms: config.connection.reconnect_delay_ms,
                    max_delay_ms: config.connection.reconnect_delay_ms * 8, // 指数退避上限
                    backoff_multiplier: 2.0, // 默认指数退避倍数
                    max_attempts: config.connection.max_reconnect_attempts,
                },
                heartbeat_interval_secs: 30,
                max_buffer_size: 1000,
                compression_enabled: false,
            }),
            rest_api_config: None,
            failover_config: FailoverConfig::default(),
        };
        
        let mut provider = Self::new(provider_config);
        
        log::info!("创建BinanceProvider，symbols: {:?}，streams: {:?}", 
                  provider.config.get_symbols(), 
                  config.subscription.streams);
        
        Ok(provider)
    }

    /// 创建新的币安Provider
    pub fn new(config: BinanceProviderConfig) -> Self {
        // 计算支持的事件类型
        let supported_events = if let Some(ws_config) = &config.websocket_config {
            ws_config.streams
                .iter()
                .map(|stream| stream.to_event_kind())
                .collect::<Vec<_>>()
        } else {
            // 默认支持的事件类型
            vec![
                EventKind::TickPrice,
                EventKind::DepthUpdate,
                EventKind::Trade,
                EventKind::BookTicker,
            ]
        };

        // 初始化状态
        let provider_type = ProviderType::Binance { 
            mode: config.connection_mode 
        };
        let mut status = ProviderStatus::new(provider_type);
        
        // 设置WebSocket指标
        status.provider_metrics = super::types::ProviderMetrics::WebSocket {
            reconnect_count: 0,
            ping_latency_ms: None,
            messages_per_second: 0.0,
            connection_duration: None,
            websocket_state: "Disconnected".to_string(),
        };

        let now = Instant::now();

        Self {
            config,
            websocket_manager: None,
            status,
            supported_events,
            start_time: None,
            events_received_total: 0,
            last_event_time: None,
            error_count: 0,
            consecutive_failures: 0,
            performance_window_start: now,
            performance_events_count: 0,
            performance_bytes_count: 0,
        }
    }

    /// 从现有WebSocketManager创建Provider（向后兼容）
    pub fn from_websocket_manager(
        websocket_manager: WebSocketManager,
        symbol: String,
    ) -> Self {
        let config = BinanceProviderConfig {
            symbols: vec![symbol.clone()],
            symbol,
            connection_mode: BinanceConnectionMode::WebSocket,
            websocket_config: Some(BinanceWebSocketConfig::default()),
            rest_api_config: None,
            failover_config: FailoverConfig::default(),
        };

        let supported_events = vec![
            EventKind::TickPrice,
            EventKind::DepthUpdate,
            EventKind::Trade,
            EventKind::BookTicker,
        ];

        let provider_type = ProviderType::Binance { 
            mode: BinanceConnectionMode::WebSocket 
        };
        let mut status = ProviderStatus::new(provider_type);
        let is_connected = websocket_manager.is_connected();
        status.is_connected = is_connected;

        let now = Instant::now();

        Self {
            config,
            websocket_manager: Some(websocket_manager),
            status,
            supported_events,
            start_time: if is_connected { Some(now) } else { None },
            events_received_total: 0,
            last_event_time: None,
            error_count: 0,
            consecutive_failures: 0,
            performance_window_start: now,
            performance_events_count: 0,
            performance_bytes_count: 0,
        }
    }

    /// 获取底层WebSocketManager的引用（向后兼容）
    pub fn websocket_manager(&self) -> Option<&WebSocketManager> {
        self.websocket_manager.as_ref()
    }

    /// 获取底层WebSocketManager的可变引用（向后兼容）
    pub fn websocket_manager_mut(&mut self) -> Option<&mut WebSocketManager> {
        self.websocket_manager.as_mut()
    }
    
    /// 为多个symbols构建WebSocket流订阅URL（与src/websocket保持一致）
    fn build_websocket_streams(&self, symbols: &[String], stream_types: &[StreamType]) -> Vec<String> {
        let mut all_streams = Vec::new();
        
        for symbol in symbols {
            for stream_type in stream_types {
                // 转换StreamType到字符串格式
                let stream_str = self.stream_type_to_string(stream_type);
                let stream_name = format!("{}@{}", symbol.to_lowercase(), stream_str);
                all_streams.push(stream_name);
            }
        }
        
        log::info!("构建WebSocket流订阅: {:?}", all_streams);
        all_streams
    }
    
    /// 将StreamType转换为字符串格式
    fn stream_type_to_string(&self, stream_type: &StreamType) -> String {
        match stream_type {
            StreamType::BookTicker => "bookTicker".to_string(),
            StreamType::Depth { .. } => "depth".to_string(),
            StreamType::Trade => "trade".to_string(),
            StreamType::Kline { interval } => {
                format!("kline_{}", interval)
            },
            StreamType::MiniTicker => "miniTicker".to_string(),
            StreamType::Ticker24hr => "ticker".to_string(),
        }
    }

    /// 初始化WebSocket连接（与src/websocket保持一致）
    fn initialize_websocket(&mut self) -> ProviderResult<()> {
        if let Some(ws_config) = &self.config.websocket_config {
            let symbols = self.config.get_symbols();
            if symbols.is_empty() {
                return Err(ProviderError::configuration_field(
                    "没有配置任何交易对符号",
                    "symbols",
                    Some("至少一个有效的符号".to_string()),
                    Some("empty vec".to_string()),
                ));
            }
            
            // 使用第一个symbol创建WebSocketConfig，但需要支持多个symbol
            // 这与src/websocket/connection.rs的实现保持一致
            let primary_symbol = symbols[0].clone();
            let mut ws_manager_config = WebSocketConfig::new(primary_symbol.clone());
            
            // 如果配置了streams，为所有symbol构建streams
            if !ws_config.streams.is_empty() {
                ws_manager_config.streams = self.build_websocket_streams(&symbols, &ws_config.streams);
            } else if symbols.len() > 1 {
                // 如果没有配置streams但有多个symbol，使用默认streams
                let stream_types = vec![
                    StreamType::Depth { levels: 20, update_speed: UpdateSpeed::Ms100 },
                    StreamType::Trade,
                    StreamType::BookTicker,
                ];
                ws_manager_config.streams = self.build_websocket_streams(&symbols, &stream_types);
            }
            // 如果只有一个symbol且没有配置streams，使用WebSocketConfig::new的默认streams
            
            // 创建WebSocket管理器
            let websocket_manager = WebSocketManager::new(ws_manager_config);
            self.websocket_manager = Some(websocket_manager);
            
            log::info!("WebSocket管理器初始化完成，symbols: {:?}, streams: {:?}", 
                      symbols, ws_config.streams);
            Ok(())
        } else {
            Err(ProviderError::configuration_field(
                "WebSocket模式需要WebSocket配置",
                "websocket_config",
                Some("BinanceWebSocketConfig".to_string()),
                Some("None".to_string()),
            ))
        }
    }

    /// 更新状态信息
    fn update_status(&mut self) {
        let ws_connected = self.websocket_manager
            .as_ref()
            .map(|ws| ws.is_connected())
            .unwrap_or(false);
            
        let was_connected = self.status.is_connected;
        
        self.status.is_connected = ws_connected;
        self.status.update_timestamp();
        
        // 如果连接状态发生变化，更新相关状态
        if ws_connected && !was_connected {
            self.start_time = Some(Instant::now());
            self.consecutive_failures = 0;
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
            if let Some(ws_manager) = &self.websocket_manager {
                if let Some(ws_stats) = ws_manager.get_stats() {
                    *reconnect_count = ws_stats.reconnect_attempts;
                    *connection_duration = ws_stats.connection_duration;
                }
                
                // 更新连接状态
                *websocket_state = if ws_connected {
                    "Connected".to_string()
                } else if ws_manager.should_reconnect() {
                    "Reconnecting".to_string()
                } else {
                    "Disconnected".to_string()
                };
            }
            
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
        let base_healthy = self.status.is_connected && 
                          self.consecutive_failures < self.config.failover_config.failure_threshold;
        
        // 检查最近是否有数据
        let recent_data = self.last_event_time
            .map(|t| t.elapsed().as_secs() < 60)
            .unwrap_or(false);
        
        base_healthy && (recent_data || self.start_time.map(|t| t.elapsed().as_secs() < 30).unwrap_or(true))
    }

    /// 将WebSocket消息转换为EventType（静态版本）
    fn convert_message_to_event_static(message: &Value) -> Option<EventType> {
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

    /// 处理WebSocket数据
    fn read_websocket_events(&mut self) -> ProviderResult<Vec<EventType>> {
        // 先检查WebSocket管理器是否存在
        let ws_manager_available = self.websocket_manager.is_some();
        if !ws_manager_available {
            return Err(ProviderError::state(
                "WebSocket管理器未初始化",
                "uninitialized",
                "initialized",
                "read_events"
            ));
        }

        // 检查连接状态和重连需求
        let needs_reconnect = {
            let ws_manager = self.websocket_manager.as_ref().unwrap();
            !ws_manager.is_connected() && ws_manager.should_reconnect()
        };

        // 如果需要重连，处理重连
        if needs_reconnect {
            let reconnect_result = {
                let ws_manager = self.websocket_manager.as_mut().unwrap();
                ws_manager.attempt_reconnect()
            };
            
            match reconnect_result {
                Ok(()) => {
                    log::info!("WebSocket重连成功");
                    self.consecutive_failures = 0;
                    self.update_status();
                }
                Err(e) => {
                    self.error_count += 1;
                    self.consecutive_failures += 1;
                    self.status.record_error(&format!("重连失败: {}", e));
                    self.update_status();
                    
                    return Err(ProviderError::connection(
                        format!("重连失败: {}", e),
                        None,
                        true,
                    ));
                }
            }
        }

        // 读取WebSocket消息
        let read_result = {
            let ws_manager = self.websocket_manager.as_mut().unwrap();
            ws_manager.read_messages()
        };

        match read_result {
            Ok(messages) => {
                let mut events = Vec::new();
                let mut estimated_bytes = 0;

                let mut events_count = 0;
                let now = Instant::now();
                
                for message in messages {
                    // 估算消息大小（用于性能统计）
                    estimated_bytes += message.to_string().len();

                    // 转换为EventType（分离函数调用避免借用冲突）
                    if let Some(event) = Self::convert_message_to_event_static(&message) {
                        events.push(event);
                        events_count += 1;
                    }
                }
                
                // 批量更新统计
                if events_count > 0 {
                    self.events_received_total += events_count;
                    self.last_event_time = Some(now);
                    for _ in 0..events_count {
                        self.status.record_event();
                    }
                }

                // 更新性能统计
                self.update_performance_metrics(events.len(), estimated_bytes);
                self.consecutive_failures = 0; // 重置失败计数
                self.update_status();

                Ok(events)
            }
            Err(e) => {
                self.error_count += 1;
                self.consecutive_failures += 1;
                let error_msg = format!("读取WebSocket消息失败: {}", e);
                self.status.record_error(&error_msg);
                self.update_status();

                Err(ProviderError::connection(error_msg, None, true))
            }
        }
    }
}

impl DataProvider for BinanceProvider {
    type Error = ProviderError;

    fn initialize(&mut self) -> ProviderResult<()> {
        log::info!("初始化Binance Provider ({:?}), symbols: {:?}", 
                  self.config.connection_mode, 
                  self.config.get_symbols());

        // 验证配置
        let symbols = self.config.get_symbols();
        if symbols.is_empty() {
            return Err(ProviderError::configuration_field(
                "没有配置任何交易对符号",
                "symbols",
                Some("至少一个有效的符号".to_string()),
                Some("empty symbols list".to_string()),
            ));
        }
        
        log::info!("验证配置完成，交易对符号: {:?}", symbols);

        // 根据连接模式初始化相应组件
        match self.config.connection_mode {
            BinanceConnectionMode::WebSocket | BinanceConnectionMode::Hybrid => {
                self.initialize_websocket()?;
            }
            BinanceConnectionMode::RestAPI => {
                // TODO: 实现REST API初始化
                return Err(ProviderError::configuration(
                    "REST API模式暂未实现，请使用WebSocket模式".to_string()
                ));
            }
        }

        self.status.is_running = false; // 初始化完成但未启动
        self.update_status();

        log::info!("Binance Provider初始化完成");
        Ok(())
    }

    fn start(&mut self) -> ProviderResult<()> {
        log::info!("启动Binance Provider");

        match self.config.connection_mode {
            BinanceConnectionMode::WebSocket | BinanceConnectionMode::Hybrid => {
                if let Some(ws_manager) = &mut self.websocket_manager {
                    // 建立WebSocket连接
                    ws_manager.connect()
                        .map_err(|e| ProviderError::connection(
                            format!("WebSocket连接失败: {}", e),
                            Some("wss://stream.binance.com:9443".to_string()),
                            true,
                        ))?;

                    // 订阅数据流
                    if let Some(ws_config) = &self.config.websocket_config {
                        let primary_symbol = self.config.get_primary_symbol();
                        let streams: Vec<String> = ws_config.streams
                            .iter()
                            .map(|stream| stream.to_binance_stream(&primary_symbol))
                            .collect();

                        ws_manager.subscribe(streams)
                            .map_err(|e| ProviderError::connection(
                                format!("订阅数据流失败: {}", e),
                                None,
                                true,
                            ))?;
                    }
                } else {
                    return Err(ProviderError::state(
                        "WebSocket管理器未初始化",
                        "uninitialized",
                        "initialized",
                        "start"
                    ));
                }
            }
            BinanceConnectionMode::RestAPI => {
                // TODO: 实现REST API启动
                return Err(ProviderError::configuration(
                    "REST API模式暂未实现".to_string()
                ));
            }
        }

        self.status.is_running = true;
        self.start_time = Some(Instant::now());
        self.performance_window_start = Instant::now();
        self.consecutive_failures = 0;
        self.update_status();

        log::info!("Binance Provider启动完成");
        Ok(())
    }

    fn stop(&mut self) -> ProviderResult<()> {
        log::info!("停止Binance Provider");

        // 停止WebSocket连接
        if let Some(ws_manager) = &mut self.websocket_manager {
            ws_manager.disconnect();
        }

        self.status.is_running = false;
        self.start_time = None;
        self.update_status();

        log::info!("Binance Provider已停止");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        match self.config.connection_mode {
            BinanceConnectionMode::WebSocket | BinanceConnectionMode::Hybrid => {
                self.websocket_manager
                    .as_ref()
                    .map(|ws| ws.is_connected())
                    .unwrap_or(false)
            }
            BinanceConnectionMode::RestAPI => {
                // TODO: 实现REST API连接检查
                false
            }
        }
    }

    fn read_events(&mut self) -> ProviderResult<Vec<EventType>> {
        match self.config.connection_mode {
            BinanceConnectionMode::WebSocket | BinanceConnectionMode::Hybrid => {
                self.read_websocket_events()
            }
            BinanceConnectionMode::RestAPI => {
                // TODO: 实现REST API事件读取
                Err(ProviderError::configuration(
                    "REST API模式暂未实现".to_string()
                ))
            }
        }
    }

    fn get_status(&self) -> ProviderStatus {
        self.status.clone()
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Binance { mode: self.config.connection_mode }
    }

    fn supported_events(&self) -> &[EventKind] {
        &self.supported_events
    }

    fn get_config_info(&self) -> Option<String> {
        Some(format!(
            "Symbols: {:?}, Mode: {:?}, Streams: {}",
            self.config.get_symbols(),
            self.config.connection_mode,
            if let Some(ws_config) = &self.config.websocket_config {
                format!("{:?}", ws_config.streams)
            } else {
                "None".to_string()
            }
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

/// ProviderIdentity implementation - defines the canonical name and type
impl ProviderIdentity for BinanceProvider {
    /// Canonical name that MUST be used in configuration files
    const CANONICAL_NAME: &'static str = "binance_market_provider";
    
    /// Canonical type identifier 
    const CANONICAL_TYPE: &'static str = "BinanceWebSocket";
    
    /// Human-readable display name
    const DISPLAY_NAME: &'static str = "Binance Market Data Provider";
    
    /// Provider version
    const VERSION: &'static str = "1.0.0";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_provider_config_default() {
        let config = BinanceProviderConfig::default();
        assert_eq!(config.symbol, ""); // 向后兼容，默认为空
        assert_eq!(config.symbols, vec!["BTCFDUSD".to_string()]);
        assert_eq!(config.connection_mode, BinanceConnectionMode::WebSocket);
        assert!(config.websocket_config.is_some());
    }

    #[test]
    fn test_stream_type_to_binance_stream() {
        let book_ticker = StreamType::BookTicker;
        assert_eq!(book_ticker.to_binance_stream("BTCUSDT"), "btcusdt@bookTicker");

        let depth = StreamType::Depth { levels: 20, update_speed: UpdateSpeed::Ms100 };
        assert_eq!(depth.to_binance_stream("ETHUSDT"), "ethusdt@depth20@100ms");
    }

    #[test]
    fn test_provider_initialization() {
        let config = BinanceProviderConfig::default();
        let mut provider = BinanceProvider::new(config);
        
        assert!(provider.initialize().is_ok());
        assert_eq!(
            provider.provider_type(), 
            ProviderType::Binance { mode: BinanceConnectionMode::WebSocket }
        );
        assert!(!provider.supported_events().is_empty());
    }

    #[test]
    fn test_empty_symbol_config_error() {
        let mut config = BinanceProviderConfig::default();
        config.symbol = String::new();
        
        let mut provider = BinanceProvider::new(config);
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