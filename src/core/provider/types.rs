// Provider类型定义 - 核心数据结构
//
// 本文件定义了Provider系统使用的所有核心类型，包括：
// - 枚举类型：ProviderType、EventKind等
// - 状态结构：ProviderStatus、ProviderMetrics等
// - 配置结构：PlaybackInfo、PerformanceMetrics等
//
// 设计原则：
// 1. 强类型：使用newtype模式和枚举确保类型安全
// 2. 序列化：支持配置文件和网络传输
// 3. 调试友好：实现Debug trait便于调试
// 4. 扩展性：使用枚举和trait支持未来扩展

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 数据提供者类型分类 - 按交易所分类
/// 
/// 重构后的Provider类型，按交易所来分类而不是按数据源类型：
/// - Binance: 币安交易所Provider（支持WebSocket、REST API等）
/// - HistoricalData: 历史文件数据Provider
/// - OKX: OKX交易所Provider（未来扩展）
/// - Bybit: Bybit交易所Provider（未来扩展）
/// - Custom: 自定义Provider
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderType {
    /// 币安交易所Provider
    /// 
    /// 特征：
    /// - 支持WebSocket实时数据
    /// - 支持REST API查询
    /// - 自动重连和错误恢复
    /// - 完整的期货市场数据
    Binance {
        /// 连接模式
        mode: BinanceConnectionMode,
    },

    /// 历史数据文件Provider
    /// 
    /// 特征：
    /// - 读取本地历史数据文件
    /// - 支持多种格式（CSV、JSON、二进制）
    /// - 可控播放速度
    /// - 支持回测模式
    HistoricalData {
        /// 数据格式
        format: HistoricalDataFormat,
    },

    /// OKX交易所Provider（未来扩展）
    #[cfg(feature = "okx")]
    OKX {
        /// 连接模式
        mode: OKXConnectionMode,
    },

    /// Bybit交易所Provider（未来扩展）
    #[cfg(feature = "bybit")]
    Bybit {
        /// 连接模式
        mode: BybitConnectionMode,
    },

    /// 自定义Provider
    /// 
    /// 用于用户自定义的数据源
    Custom {
        /// Provider标识符
        identifier: String,
        /// Provider描述
        description: String,
    },
}

/// 币安连接模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BinanceConnectionMode {
    /// WebSocket实时数据流
    WebSocket,
    /// REST API查询模式
    RestAPI,
    /// 混合模式（WebSocket + REST API）
    Hybrid,
}

/// 历史数据格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HistoricalDataFormat {
    /// CSV格式
    CSV,
    /// JSON格式
    JSON,
    /// 二进制格式
    Binary,
    /// 压缩格式
    Compressed,
}

/// OKX连接模式（未来扩展）
#[cfg(feature = "okx")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OKXConnectionMode {
    WebSocket,
    RestAPI,
    Hybrid,
}

/// Bybit连接模式（未来扩展）
#[cfg(feature = "bybit")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BybitConnectionMode {
    WebSocket,
    RestAPI,
    Hybrid,
}

impl ProviderType {
    /// 获取类型的字符串描述
    pub fn as_str(&self) -> &str {
        match self {
            ProviderType::Binance { .. } => "Binance",
            ProviderType::HistoricalData { .. } => "HistoricalData",
            #[cfg(feature = "okx")]
            ProviderType::OKX { .. } => "OKX",
            #[cfg(feature = "bybit")]
            ProviderType::Bybit { .. } => "Bybit",
            ProviderType::Custom { identifier, .. } => identifier,
        }
    }

    /// 获取详细描述
    pub fn detailed_description(&self) -> String {
        match self {
            ProviderType::Binance { mode } => {
                format!("Binance Exchange Provider ({:?})", mode)
            },
            ProviderType::HistoricalData { format } => {
                format!("Historical Data Provider ({:?})", format)
            },
            #[cfg(feature = "okx")]
            ProviderType::OKX { mode } => {
                format!("OKX Exchange Provider ({:?})", mode)
            },
            #[cfg(feature = "bybit")]
            ProviderType::Bybit { mode } => {
                format!("Bybit Exchange Provider ({:?})", mode)
            },
            ProviderType::Custom { identifier, description } => {
                format!("Custom Provider: {} - {}", identifier, description)
            },
        }
    }

    /// 从字符串解析ProviderType（简化版本，用于向后兼容）
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "binance" => Some(ProviderType::Binance { 
                mode: BinanceConnectionMode::WebSocket 
            }),
            "historical" | "history" | "backtest" => Some(ProviderType::HistoricalData { 
                format: HistoricalDataFormat::JSON 
            }),
            #[cfg(feature = "okx")]
            "okx" => Some(ProviderType::OKX { 
                mode: OKXConnectionMode::WebSocket 
            }),
            #[cfg(feature = "bybit")]
            "bybit" => Some(ProviderType::Bybit { 
                mode: BybitConnectionMode::WebSocket 
            }),
            _ => None,
        }
    }

    /// 检查是否支持播放控制
    pub fn supports_playback_control(&self) -> bool {
        matches!(self, ProviderType::HistoricalData { .. })
    }

    /// 检查是否为实时数据
    pub fn is_realtime(&self) -> bool {
        match self {
            ProviderType::Binance { mode } => {
                matches!(mode, BinanceConnectionMode::WebSocket | BinanceConnectionMode::Hybrid)
            },
            ProviderType::HistoricalData { .. } => false,
            #[cfg(feature = "okx")]
            ProviderType::OKX { mode } => {
                matches!(mode, OKXConnectionMode::WebSocket | OKXConnectionMode::Hybrid)
            },
            #[cfg(feature = "bybit")]
            ProviderType::Bybit { mode } => {
                matches!(mode, BybitConnectionMode::WebSocket | BybitConnectionMode::Hybrid)
            },
            ProviderType::Custom { .. } => false, // 默认不是实时
        }
    }

    /// 检查是否为交易所Provider
    pub fn is_exchange(&self) -> bool {
        match self {
            ProviderType::Binance { .. } => true,
            #[cfg(feature = "okx")]
            ProviderType::OKX { .. } => true,
            #[cfg(feature = "bybit")]
            ProviderType::Bybit { .. } => true,
            _ => false,
        }
    }

    /// 获取默认的事件类型
    pub fn default_supported_events(&self) -> Vec<EventKind> {
        match self {
            ProviderType::Binance { .. } => vec![
                EventKind::TickPrice,
                EventKind::DepthUpdate,
                EventKind::Trade,
                EventKind::BookTicker,
            ],
            ProviderType::HistoricalData { .. } => vec![
                EventKind::TickPrice,
                EventKind::Trade,
                EventKind::DepthUpdate,
            ],
            #[cfg(feature = "okx")]
            ProviderType::OKX { .. } => vec![
                EventKind::TickPrice,
                EventKind::DepthUpdate,
                EventKind::Trade,
                EventKind::BookTicker,
            ],
            #[cfg(feature = "bybit")]
            ProviderType::Bybit { .. } => vec![
                EventKind::TickPrice,
                EventKind::DepthUpdate,
                EventKind::Trade,
                EventKind::BookTicker,
            ],
            ProviderType::Custom { .. } => vec![
                EventKind::TickPrice,
            ],
        }
    }
}

/// 支持的事件类型
/// 
/// 定义Provider能够生成的事件类型，与现有EventType保持兼容
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventKind {
    /// 价格tick事件
    TickPrice,
    
    /// 深度更新事件
    DepthUpdate,
    
    /// 成交事件
    Trade,
    
    /// 最佳买卖价事件
    BookTicker,
    
    /// 信号事件
    Signal,
    
    /// 交易请求事件
    OrderRequest,
    
    /// 持仓更新事件
    PositionUpdate,
    
    /// 订单取消事件
    OrderCancel,
    
    /// 止损事件
    OrderStopLoss,
    
    /// 止盈事件
    OrderTakeProfit,
    
    /// 风险事件
    RiskEvent,
    
    /// WebSocket错误事件
    WebSocketError,
}

impl EventKind {
    /// 获取事件类型的字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            EventKind::TickPrice => "TickPrice",
            EventKind::DepthUpdate => "DepthUpdate",
            EventKind::Trade => "Trade",
            EventKind::BookTicker => "BookTicker",
            EventKind::Signal => "Signal",
            EventKind::OrderRequest => "OrderRequest",
            EventKind::PositionUpdate => "PositionUpdate",
            EventKind::OrderCancel => "OrderCancel",
            EventKind::OrderStopLoss => "OrderStopLoss",
            EventKind::OrderTakeProfit => "OrderTakeProfit",
            EventKind::RiskEvent => "RiskEvent",
            EventKind::WebSocketError => "WebSocketError",
        }
    }

    /// 检查是否为市场数据事件
    pub fn is_market_data(&self) -> bool {
        matches!(
            self,
            EventKind::TickPrice
                | EventKind::DepthUpdate
                | EventKind::Trade
                | EventKind::BookTicker
        )
    }

    /// 检查是否为交易事件
    pub fn is_trading_event(&self) -> bool {
        matches!(
            self,
            EventKind::OrderRequest
                | EventKind::PositionUpdate
                | EventKind::OrderCancel
                | EventKind::OrderStopLoss
                | EventKind::OrderTakeProfit
        )
    }

    /// 获取事件优先级
    pub fn default_priority(&self) -> EventPriority {
        match self {
            EventKind::WebSocketError | EventKind::RiskEvent => EventPriority::Critical,
            EventKind::OrderRequest
            | EventKind::OrderCancel
            | EventKind::OrderStopLoss
            | EventKind::OrderTakeProfit
            | EventKind::Signal => EventPriority::High,
            EventKind::TickPrice | EventKind::Trade => EventPriority::Normal,
            EventKind::DepthUpdate | EventKind::BookTicker => EventPriority::Normal,
            EventKind::PositionUpdate => EventPriority::Normal,
        }
    }
}

/// 事件优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Provider状态信息
/// 
/// 包含Provider的运行状态、统计信息和健康指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    /// Provider类型
    pub provider_type: ProviderType,
    
    /// 是否已连接/就绪
    pub is_connected: bool,
    
    /// 是否正在运行
    pub is_running: bool,
    
    /// 已接收事件总数
    pub events_received: u64,
    
    /// 最后一次事件时间戳
    pub last_event_time: Option<u64>,
    
    /// 错误计数
    pub error_count: u32,
    
    /// 连续错误次数
    pub consecutive_errors: u32,
    
    /// 最后错误信息
    pub last_error: Option<String>,
    
    /// Provider特定指标
    pub provider_metrics: ProviderMetrics,
    
    /// 状态更新时间
    pub status_timestamp: u64,
    
    /// 健康状态
    pub is_healthy: bool,
    
    /// Provider特定指标（可选）
    pub metrics: Option<ProviderMetrics>,
    
    /// 自定义元数据（用于扩展）
    pub custom_metadata: Option<HashMap<String, serde_json::Value>>,
}

impl ProviderStatus {
    /// 创建新的状态实例
    pub fn new(provider_type: ProviderType) -> Self {
        let metrics = ProviderMetrics::new(provider_type.clone());
        Self {
            provider_type: provider_type.clone(),
            is_connected: false,
            is_running: false,
            events_received: 0,
            last_event_time: None,
            error_count: 0,
            consecutive_errors: 0,
            last_error: None,
            provider_metrics: metrics.clone(),
            status_timestamp: Self::current_timestamp(),
            is_healthy: false,
            metrics: Some(metrics),
            custom_metadata: None,
        }
    }

    /// 更新状态时间戳
    pub fn update_timestamp(&mut self) {
        self.status_timestamp = Self::current_timestamp();
    }

    /// 获取当前时间戳
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// 记录事件接收
    pub fn record_event(&mut self) {
        self.events_received += 1;
        self.last_event_time = Some(Self::current_timestamp());
        self.consecutive_errors = 0; // 重置错误计数
    }

    /// 记录错误
    pub fn record_error(&mut self, error: &str) {
        self.error_count += 1;
        self.consecutive_errors += 1;
        self.last_error = Some(error.to_string());
        self.update_timestamp();
    }

    /// 检查是否健康
    pub fn check_health(&self) -> bool {
        // 基础健康检查逻辑
        self.is_connected && 
        self.consecutive_errors < 5 && 
        self.last_event_time.map_or(false, |t| {
            Self::current_timestamp() - t < 60000 // 1分钟内有事件
        })
    }
}

/// Provider特定指标
/// 
/// 根据不同Provider类型包含特定的性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderMetrics {
    /// WebSocket Provider指标
    WebSocket {
        /// 重连次数
        reconnect_count: u32,
        /// ping延迟（毫秒）
        ping_latency_ms: Option<f64>,
        /// 每秒消息数
        messages_per_second: f64,
        /// 连接持续时间
        connection_duration: Option<Duration>,
        /// WebSocket状态
        websocket_state: String,
    },

    /// 历史文件Provider指标
    Historical {
        /// 文件读取进度 (0.0-1.0)
        file_progress: f64,
        /// 播放速度倍数
        playback_speed: f64,
        /// 当前时间戳
        current_timestamp: u64,
        /// 总事件数
        total_events: u64,
        /// 已处理事件数
        processed_events: u64,
        /// 文件路径
        file_path: String,
    },

    /// 混合Provider指标
    Hybrid {
        /// 实时数据指标
        realtime_metrics: Box<ProviderMetrics>,
        /// 历史数据指标
        historical_metrics: Box<ProviderMetrics>,
        /// 当前活跃数据源
        active_source: String,
    },

    /// 通用指标（用于其他Provider类型）
    Generic {
        /// 自定义指标
        metrics: HashMap<String, serde_json::Value>,
    },
}

impl ProviderMetrics {
    /// 创建新的指标实例
    pub fn new(provider_type: ProviderType) -> Self {
        match provider_type {
            ProviderType::Binance { mode } => {
                match mode {
                    BinanceConnectionMode::WebSocket | BinanceConnectionMode::Hybrid => {
                        ProviderMetrics::WebSocket {
                            reconnect_count: 0,
                            ping_latency_ms: None,
                            messages_per_second: 0.0,
                            connection_duration: None,
                            websocket_state: "Disconnected".to_string(),
                        }
                    },
                    BinanceConnectionMode::RestAPI => {
                        ProviderMetrics::Generic {
                            metrics: HashMap::new(),
                        }
                    },
                }
            },
            ProviderType::HistoricalData { .. } => ProviderMetrics::Historical {
                file_progress: 0.0,
                playback_speed: 1.0,
                current_timestamp: 0,
                total_events: 0,
                processed_events: 0,
                file_path: String::new(),
            },
            #[cfg(feature = "okx")]
            ProviderType::OKX { mode } => {
                match mode {
                    OKXConnectionMode::WebSocket | OKXConnectionMode::Hybrid => {
                        ProviderMetrics::WebSocket {
                            reconnect_count: 0,
                            ping_latency_ms: None,
                            messages_per_second: 0.0,
                            connection_duration: None,
                            websocket_state: "Disconnected".to_string(),
                        }
                    },
                    OKXConnectionMode::RestAPI => {
                        ProviderMetrics::Generic {
                            metrics: HashMap::new(),
                        }
                    },
                }
            },
            #[cfg(feature = "bybit")]
            ProviderType::Bybit { mode } => {
                match mode {
                    BybitConnectionMode::WebSocket | BybitConnectionMode::Hybrid => {
                        ProviderMetrics::WebSocket {
                            reconnect_count: 0,
                            ping_latency_ms: None,
                            messages_per_second: 0.0,
                            connection_duration: None,
                            websocket_state: "Disconnected".to_string(),
                        }
                    },
                    BybitConnectionMode::RestAPI => {
                        ProviderMetrics::Generic {
                            metrics: HashMap::new(),
                        }
                    },
                }
            },
            ProviderType::Custom { .. } => ProviderMetrics::Generic {
                metrics: HashMap::new(),
            },
        }
    }

    /// 获取指标的字符串表示
    pub fn summary(&self) -> String {
        match self {
            ProviderMetrics::WebSocket {
                reconnect_count,
                ping_latency_ms,
                messages_per_second,
                connection_duration,
                websocket_state,
            } => {
                format!(
                    "WebSocket: {} | Reconnects: {} | Ping: {:.1}ms | MPS: {:.1} | Uptime: {:?}",
                    websocket_state,
                    reconnect_count,
                    ping_latency_ms.unwrap_or(0.0),
                    messages_per_second,
                    connection_duration.unwrap_or_default()
                )
            }
            ProviderMetrics::Historical {
                file_progress,
                playback_speed,
                processed_events,
                total_events,
                ..
            } => {
                format!(
                    "Historical: {:.1}% | Speed: {:.1}x | Events: {}/{}",
                    file_progress * 100.0,
                    playback_speed,
                    processed_events,
                    total_events
                )
            }
            ProviderMetrics::Hybrid { active_source, .. } => {
                format!("Hybrid: Active={}", active_source)
            }
            ProviderMetrics::Generic { metrics } => {
                format!("Generic: {} metrics", metrics.len())
            }
        }
    }
}

/// 播放控制信息
/// 
/// 用于历史数据Provider的播放状态管理
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackInfo {
    /// 是否正在播放
    pub is_playing: bool,
    
    /// 是否已暂停
    pub is_paused: bool,
    
    /// 播放速度倍数
    pub playback_speed: f64,
    
    /// 当前时间戳
    pub current_timestamp: u64,
    
    /// 开始时间戳
    pub start_timestamp: u64,
    
    /// 结束时间戳
    pub end_timestamp: u64,
    
    /// 播放进度 (0.0-1.0)
    pub progress: f64,
    
    /// 是否支持跳转
    pub can_seek: bool,
    
    /// 是否循环播放
    pub loop_enabled: bool,
}

impl PlaybackInfo {
    /// 创建新的播放信息
    pub fn new(start_timestamp: u64, end_timestamp: u64) -> Self {
        Self {
            is_playing: false,
            is_paused: false,
            playback_speed: 1.0,
            current_timestamp: start_timestamp,
            start_timestamp,
            end_timestamp,
            progress: 0.0,
            can_seek: true,
            loop_enabled: false,
        }
    }

    /// 更新当前时间戳和进度
    pub fn update_timestamp(&mut self, timestamp: u64) {
        self.current_timestamp = timestamp.max(self.start_timestamp).min(self.end_timestamp);
        
        if self.end_timestamp > self.start_timestamp {
            self.progress = (self.current_timestamp - self.start_timestamp) as f64 /
                           (self.end_timestamp - self.start_timestamp) as f64;
            self.progress = self.progress.max(0.0).min(1.0);
        }
    }

    /// 检查是否已到达结尾
    pub fn is_at_end(&self) -> bool {
        self.current_timestamp >= self.end_timestamp
    }

    /// 获取剩余时间（毫秒）
    pub fn remaining_time(&self) -> u64 {
        self.end_timestamp.saturating_sub(self.current_timestamp)
    }

    /// 获取播放时长（毫秒）
    pub fn total_duration(&self) -> u64 {
        self.end_timestamp.saturating_sub(self.start_timestamp)
    }
}

/// 性能指标
/// 
/// Provider的性能统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// 已接收事件总数
    pub events_received: u64,
    
    /// 最后一次事件时间戳
    pub last_event_time: Option<u64>,
    
    /// 错误计数
    pub error_count: u32,
    
    /// 每秒事件数
    pub events_per_second: f64,
    
    /// 每秒字节数
    pub bytes_per_second: f64,
    
    /// 平均延迟（毫秒）
    pub average_latency_ms: f64,
    
    /// 最大延迟（毫秒）
    pub max_latency_ms: f64,
    
    /// CPU使用率
    pub cpu_usage_percent: f64,
    
    /// 内存使用量（MB）
    pub memory_usage_mb: f64,
    
    /// 错误率
    pub error_rate: f64,
    
    /// 统计时间窗口（秒）
    pub window_seconds: u64,
}

impl PerformanceMetrics {
    /// 创建空的性能指标
    pub fn new() -> Self {
        Self {
            events_received: 0,
            last_event_time: None,
            error_count: 0,
            events_per_second: 0.0,
            bytes_per_second: 0.0,
            average_latency_ms: 0.0,
            max_latency_ms: 0.0,
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0.0,
            error_rate: 0.0,
            window_seconds: 60, // 默认1分钟窗口
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// 交易所标识
/// 
/// 支持的交易所列表，用于多交易所Provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExchangeId {
    Binance,
    Bybit,
    OKX,
    FTX,
    Kraken,
    Coinbase,
    Huobi,
    KuCoin,
    Custom(u32), // 自定义交易所ID
}

impl ExchangeId {
    /// 获取交易所名称
    pub fn name(&self) -> &'static str {
        match self {
            ExchangeId::Binance => "Binance",
            ExchangeId::Bybit => "Bybit",
            ExchangeId::OKX => "OKX",
            ExchangeId::FTX => "FTX",
            ExchangeId::Kraken => "Kraken",
            ExchangeId::Coinbase => "Coinbase",
            ExchangeId::Huobi => "Huobi",
            ExchangeId::KuCoin => "KuCoin",
            ExchangeId::Custom(_) => "Custom",
        }
    }

    /// 从字符串解析交易所ID
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "binance" => Some(ExchangeId::Binance),
            "bybit" => Some(ExchangeId::Bybit),
            "okx" => Some(ExchangeId::OKX),
            "ftx" => Some(ExchangeId::FTX),
            "kraken" => Some(ExchangeId::Kraken),
            "coinbase" => Some(ExchangeId::Coinbase),
            "huobi" => Some(ExchangeId::Huobi),
            "kucoin" => Some(ExchangeId::KuCoin),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_serialization() {
        let provider_type = ProviderType::Binance { mode: BinanceConnectionMode::WebSocket };
        let serialized = serde_json::to_string(&provider_type).unwrap();
        let deserialized: ProviderType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(provider_type, deserialized);
    }

    #[test]
    fn test_provider_status_creation() {
        let status = ProviderStatus::new(ProviderType::Binance { mode: BinanceConnectionMode::WebSocket });
        assert!(!status.is_connected);
        assert!(!status.is_running);
        assert_eq!(status.events_received, 0);
    }

    #[test]
    fn test_playback_info_progress() {
        let mut info = PlaybackInfo::new(1000, 2000);
        info.update_timestamp(1500);
        assert_eq!(info.progress, 0.5);
    }

    #[test]
    fn test_event_kind_categorization() {
        assert!(EventKind::TickPrice.is_market_data());
        assert!(EventKind::OrderRequest.is_trading_event());
        assert!(!EventKind::TickPrice.is_trading_event());
    }
}