// Provider-specific configuration structures
// Following OOP principles with polymorphic provider configs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Polymorphic provider configuration enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderConfig {
    BinanceWebSocket(BinanceWebSocketConfig),
    Gzip(GzipProviderConfig),
    BinanceRest(BinanceRestConfig),
    Mock(MockProviderConfig),
}

// Base trait for all provider configurations (OOP interface)
pub trait ProviderConfigTrait {
    fn name(&self) -> &str;
    fn provider_type(&self) -> &str;
    fn version(&self) -> &str;
    fn is_enabled(&self) -> bool;
    fn validate(&self) -> Result<(), String>;
}

/// Binance WebSocket provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceWebSocketConfig {
    pub provider: ProviderInfo,
    pub connection: WebSocketConnection,
    pub subscription: WebSocketSubscription,
    pub authentication: Authentication,
    pub performance: WebSocketPerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConnection {
    pub base_url: String,
    pub max_reconnect_attempts: u32,
    pub reconnect_delay_ms: u64,
    pub ping_interval_ms: u64,
    pub connection_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authentication {
    pub api_key: String,
    pub api_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketSubscription {
    pub symbols: Vec<String>,
    pub streams: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketPerformanceConfig {
    pub buffer_size: usize,
    pub batch_processing: bool,
    pub batch_size: usize,
}


/// Gzip provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GzipProviderConfig {
    pub provider: ProviderInfo,
    pub data_source: DataSourceConfig,
    pub files: FilesConfig,
    pub playback: PlaybackConfig,
    pub parsing: ParsingConfig,
    pub buffering: BufferingConfig,
    pub filtering: GzipFilteringConfig,
    pub performance: GzipPerformanceConfig,
    pub error_handling: ErrorHandlingConfig,
    pub logging: GzipLoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceConfig {
    pub data_directory: String,
    pub file_pattern: String,
    pub symbols: Vec<String>,
    pub date_range_start: Option<String>,
    pub date_range_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesConfig {
    pub specific_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackConfig {
    pub initial_speed: f64,
    pub auto_start: bool,
    pub loop_enabled: bool,
    pub start_paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsingConfig {
    pub timestamp_format: String,
    pub line_delimiter: String,
    pub field_separator: String,
    pub skip_invalid_lines: bool,
    pub max_parse_errors: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferingConfig {
    pub read_buffer_size: usize,
    pub decompress_buffer_size: usize,
    pub event_buffer_size: usize,
    pub prefetch_enabled: bool,
    pub prefetch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GzipFilteringConfig {
    pub filter_by_event_type: bool,
    pub allowed_event_types: Vec<String>,
    pub filter_by_time: bool,
    pub time_start: u64,
    pub time_end: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GzipPerformanceConfig {
    pub use_parallel_decompression: bool,
    pub thread_count: usize,
    pub cache_decompressed_data: bool,
    pub cache_size_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandlingConfig {
    pub on_parse_error: String,
    pub on_file_error: String,
    pub max_consecutive_errors: usize,
    pub retry_on_error: bool,
    pub retry_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GzipLoggingConfig {
    pub log_level: String,
    pub log_parsing_errors: bool,
    pub log_performance_metrics: bool,
    pub metrics_interval: u64,
    pub show_progress: bool,
}

/// Binance REST provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceRestConfig {
    pub provider: ProviderInfo,
    pub connection: RestConnection,
    pub authentication: Authentication,
    pub rate_limiting: RateLimitingConfig,
    pub endpoints: EndpointsConfig,
    pub market_data: MarketDataConfig,
    pub caching: CachingConfig,
    pub error_handling: RestErrorHandling,
    pub logging: RestLoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestConnection {
    pub base_url: String,
    pub testnet_url: String,
    pub use_testnet: bool,
    pub timeout: u64,
    pub max_retries: u32,
    pub retry_delay: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    pub enabled: bool,
    pub requests_per_minute: u32,
    pub requests_per_second: u32,
    pub order_requests_per_second: u32,
    pub weight_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointsConfig {
    pub klines_endpoint: String,
    pub ticker_endpoint: String,
    pub depth_endpoint: String,
    pub trades_endpoint: String,
    pub account_endpoint: String,
    pub order_endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDataConfig {
    pub symbols: Vec<String>,
    pub default_limit: u32,
    pub kline_interval: String,
    pub depth_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachingConfig {
    pub enabled: bool,
    pub cache_duration: u64,
    pub cache_size: usize,
    pub cache_market_data: bool,
    pub cache_account_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestErrorHandling {
    pub on_rate_limit: String,
    pub on_network_error: String,
    pub log_errors: bool,
    pub propagate_errors: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestLoggingConfig {
    pub log_level: String,
    pub log_requests: bool,
    pub log_responses: bool,
    pub log_errors: bool,
}

/// Mock provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockProviderConfig {
    pub provider: ProviderInfo,
    pub data_generation: DataGenerationConfig,
    pub event_generation: EventGenerationConfig,
    pub replay: ReplayConfig,
    pub order_book: OrderBookConfig,
    pub trade_simulation: TradeSimulationConfig,
    pub market_conditions: MarketConditionsConfig,
    pub anomalies: AnomaliesConfig,
    pub performance: MockPerformanceConfig,
    pub logging: MockLoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataGenerationConfig {
    pub mode: String,
    pub symbols: Vec<String>,
    pub base_price: f64,
    pub price_volatility: f64,
    pub volume_range: [f64; 2],
    pub spread: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventGenerationConfig {
    pub events_per_second: u32,
    pub event_types: Vec<String>,
    pub event_distribution: HashMap<String, u32>,
    pub randomize_timing: bool,
    pub burst_mode: bool,
    pub burst_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayConfig {
    pub replay_file: String,
    pub replay_speed: f64,
    pub loop_on_end: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookConfig {
    pub depth_levels: u32,
    pub initial_liquidity: f64,
    pub liquidity_distribution: String,
    pub update_frequency: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSimulationConfig {
    pub trade_size_distribution: String,
    pub min_trade_size: f64,
    pub max_trade_size: f64,
    pub aggressive_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConditionsConfig {
    pub condition: String,
    pub trend_strength: f64,
    pub volatility_cycles: bool,
    pub cycle_period: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomaliesConfig {
    pub enabled: bool,
    pub types: Vec<String>,
    pub frequency: f64,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockPerformanceConfig {
    pub use_cache: bool,
    pub cache_size: usize,
    pub batch_events: bool,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockLoggingConfig {
    pub log_level: String,
    pub log_generated_data: bool,
    pub save_generated_data: bool,
    pub output_file: String,
}

// Default implementations for backward compatibility

fn default_buffer_size() -> usize {
    1000
}

impl Default for BinanceWebSocketConfig {
    fn default() -> Self {
        Self {
            provider: ProviderInfo {
                name: "binance_market_provider".to_string(),
                provider_type: "BinanceWebSocket".to_string(),
                version: "1.0.0".to_string(),
            },
            connection: WebSocketConnection {
                base_url: "wss://stream.binance.com:9443".to_string(),
                max_reconnect_attempts: 5,
                reconnect_delay_ms: 1000,
                ping_interval_ms: 30000,
                connection_timeout_ms: 10000,
            },
            subscription: WebSocketSubscription {
                symbols: vec!["BTCFDUSD".to_string()],
                streams: vec!["depth".to_string(), "trade".to_string(), "bookTicker".to_string()],
            },
            authentication: Authentication {
                api_key: String::new(),
                api_secret: String::new(),
            },
            performance: WebSocketPerformanceConfig {
                buffer_size: 1000,
                batch_processing: false,
                batch_size: 100,
            },
        }
    }
}

impl Default for GzipProviderConfig {
    fn default() -> Self {
        Self {
            provider: ProviderInfo {
                name: "Gzip Historical Provider".to_string(),
                provider_type: "GzipProvider".to_string(),
                version: "1.0.0".to_string(),
            },
            data_source: DataSourceConfig {
                data_directory: "./data".to_string(),
                file_pattern: "*.gz".to_string(),
                symbols: vec!["BTCFDUSD".to_string()],
                date_range_start: None,
                date_range_end: None,
            },
            files: FilesConfig {
                specific_files: vec![],
            },
            playback: PlaybackConfig {
                initial_speed: 1.0,
                auto_start: true,
                loop_enabled: false,
                start_paused: false,
            },
            parsing: ParsingConfig {
                timestamp_format: "nanoseconds".to_string(),
                line_delimiter: "\n".to_string(),
                field_separator: " ".to_string(),
                skip_invalid_lines: true,
                max_parse_errors: 100,
            },
            buffering: BufferingConfig {
                read_buffer_size: 65536,
                decompress_buffer_size: 131072,
                event_buffer_size: 10000,
                prefetch_enabled: true,
                prefetch_size: 1000,
            },
            filtering: GzipFilteringConfig {
                filter_by_event_type: false,
                allowed_event_types: vec!["bookTicker".to_string(), "trade".to_string()],
                filter_by_time: false,
                time_start: 0,
                time_end: 0,
            },
            performance: GzipPerformanceConfig {
                use_parallel_decompression: false,
                thread_count: 2,
                cache_decompressed_data: false,
                cache_size_mb: 100,
            },
            error_handling: ErrorHandlingConfig {
                on_parse_error: "skip".to_string(),
                on_file_error: "next".to_string(),
                max_consecutive_errors: 10,
                retry_on_error: true,
                retry_count: 3,
            },
            logging: GzipLoggingConfig {
                log_level: "info".to_string(),
                log_parsing_errors: true,
                log_performance_metrics: true,
                metrics_interval: 10000,
                show_progress: true,
            },
        }
    }
}

// Implement trait for each provider config type
impl ProviderConfigTrait for BinanceWebSocketConfig {
    fn name(&self) -> &str {
        &self.provider.name
    }
    
    fn provider_type(&self) -> &str {
        &self.provider.provider_type
    }
    
    fn version(&self) -> &str {
        &self.provider.version
    }
    
    fn is_enabled(&self) -> bool {
        true // Can be extended based on config
    }
    
    fn validate(&self) -> Result<(), String> {
        if self.connection.base_url.is_empty() {
            return Err("WebSocket base URL cannot be empty".to_string());
        }
        if self.subscription.symbols.is_empty() {
            return Err("At least one symbol must be configured".to_string());
        }
        Ok(())
    }
}

impl ProviderConfigTrait for GzipProviderConfig {
    fn name(&self) -> &str {
        &self.provider.name
    }
    
    fn provider_type(&self) -> &str {
        &self.provider.provider_type
    }
    
    fn version(&self) -> &str {
        &self.provider.version
    }
    
    fn is_enabled(&self) -> bool {
        true
    }
    
    fn validate(&self) -> Result<(), String> {
        if self.data_source.data_directory.is_empty() {
            return Err("Data directory cannot be empty".to_string());
        }
        Ok(())
    }
}