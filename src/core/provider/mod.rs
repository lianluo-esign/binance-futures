// Provider抽象层 - 按交易所分类的统一数据源接口
//
// 重构后的Provider系统，支持：
// - 按交易所分类的Provider（Binance、OKX、Bybit等）
// - 历史数据Provider（本地文件）
// - 统一的事件接口和状态管理
// - 向后兼容现有WebSocket功能
//
// 设计原则：
// 1. 交易所导向：每个交易所都是独立的Provider
// 2. 内部抽象：交易所内部处理WebSocket、REST API等连接方式
// 3. 统一接口：对外提供一致的事件接口
// 4. 向后兼容：保持现有代码的兼容性
// 5. 可扩展性：便于添加新的交易所支持

pub mod types;
pub mod binance_market_provider;
pub mod gzip_historical_provider;
pub mod gzip_provider;
pub mod manager;
pub mod error;
pub mod config_adapter;
pub mod provider_selector;
pub mod provider_launcher;

// 重构后的核心导出
pub use types::*;
pub use binance_market_provider::BinanceProvider;
pub use gzip_historical_provider::{HistoricalDataProvider, HistoricalDataConfig};
pub use gzip_provider::{GzipProvider, GzipProviderConfig};
pub use manager::ProviderManager;
pub use error::{ProviderError, ProviderResult};
pub use config_adapter::{ConfigurableProvider as Configurable, ProviderConfigAdapter, ConfiguredProviderFactory};
pub use provider_selector::{ProviderSelector, ProviderOption, ProviderOptionStatus};
pub use provider_launcher::{ProviderLauncher, LaunchConfig, LaunchProgress, LaunchResult};

use crate::events::EventType;
use std::fmt::Debug;

/// Provider枚举 - 运行时Provider包装
/// 
/// 这个枚举包装了所有具体的Provider实现，允许运行时多态
#[derive(Debug)]
pub enum AnyProvider {
    /// Binance WebSocket Provider
    Binance(BinanceProvider),
    /// 历史数据Provider
    Historical(HistoricalDataProvider),
    /// Gzip压缩数据Provider
    Gzip(GzipProvider),
}

impl AnyProvider {
    /// 获取Provider类型
    pub fn provider_type(&self) -> ProviderType {
        match self {
            AnyProvider::Binance(p) => p.provider_type(),
            AnyProvider::Historical(p) => p.provider_type(),
            AnyProvider::Gzip(p) => p.provider_type(),
        }
    }
    
    /// 检查是否连接
    pub fn is_connected(&self) -> bool {
        match self {
            AnyProvider::Binance(p) => p.is_connected(),
            AnyProvider::Historical(p) => p.is_connected(),
            AnyProvider::Gzip(p) => p.is_connected(),
        }
    }
    
    /// 获取状态
    pub fn get_status(&self) -> ProviderStatus {
        match self {
            AnyProvider::Binance(p) => p.get_status(),
            AnyProvider::Historical(p) => p.get_status(),
            AnyProvider::Gzip(p) => p.get_status(),
        }
    }
    
    /// 初始化
    pub fn initialize(&mut self) -> ProviderResult<()> {
        match self {
            AnyProvider::Binance(p) => p.initialize().map_err(|e| e.into()),
            AnyProvider::Historical(p) => p.initialize().map_err(|e| e.into()),
            AnyProvider::Gzip(p) => p.initialize().map_err(|e| e.into()),
        }
    }
    
    /// 启动
    pub fn start(&mut self) -> ProviderResult<()> {
        match self {
            AnyProvider::Binance(p) => p.start().map_err(|e| e.into()),
            AnyProvider::Historical(p) => p.start().map_err(|e| e.into()),
            AnyProvider::Gzip(p) => p.start().map_err(|e| e.into()),
        }
    }
    
    /// 停止
    pub fn stop(&mut self) -> ProviderResult<()> {
        match self {
            AnyProvider::Binance(p) => p.stop().map_err(|e| e.into()),
            AnyProvider::Historical(p) => p.stop().map_err(|e| e.into()),
            AnyProvider::Gzip(p) => p.stop().map_err(|e| e.into()),
        }
    }
    
    /// 读取事件
    pub fn read_events(&mut self) -> ProviderResult<Vec<EventType>> {
        match self {
            AnyProvider::Binance(p) => p.read_events().map_err(|e| e.into()),
            AnyProvider::Historical(p) => p.read_events().map_err(|e| e.into()),
            AnyProvider::Gzip(p) => p.read_events().map_err(|e| e.into()),
        }
    }
    
    /// 健康检查
    pub fn health_check(&self) -> bool {
        match self {
            AnyProvider::Binance(p) => p.health_check(),
            AnyProvider::Historical(p) => p.health_check(),
            AnyProvider::Gzip(p) => p.health_check(),
        }
    }
    
    /// 获取支持的事件类型
    pub fn supported_events(&self) -> &[EventKind] {
        match self {
            AnyProvider::Binance(p) => p.supported_events(),
            AnyProvider::Historical(p) => p.supported_events(),
            AnyProvider::Gzip(p) => p.supported_events(),
        }
    }
}

// 为AnyProvider实现From traits
impl From<BinanceProvider> for AnyProvider {
    fn from(provider: BinanceProvider) -> Self {
        AnyProvider::Binance(provider)
    }
}

impl From<HistoricalDataProvider> for AnyProvider {
    fn from(provider: HistoricalDataProvider) -> Self {
        AnyProvider::Historical(provider)
    }
}

impl From<GzipProvider> for AnyProvider {
    fn from(provider: GzipProvider) -> Self {
        AnyProvider::Gzip(provider)
    }
}

/// 数据提供者核心抽象接口
/// 
/// 所有数据源Provider都必须实现此trait，包括：
/// - WebSocket实时数据源
/// - 历史文件数据源
/// - 混合数据源
/// 
/// # 设计目标
/// - 统一接口：所有Provider输出标准EventType
/// - 零成本抽象：通过泛型实现编译时多态
/// - 错误安全：完善的错误处理和恢复机制
/// - 状态透明：提供详细的状态监控信息
/// 
/// # 使用示例
/// ```rust
/// let mut provider = BinanceWebSocketProvider::new(config);
/// provider.initialize()?;
/// provider.start()?;
/// 
/// loop {
///     match provider.read_events() {
///         Ok(events) => {
///             for event in events {
///                 // 处理事件...
///             }
///         }
///         Err(e) => {
///             eprintln!("Provider错误: {}", e);
///             break;
///         }
///     }
/// }
/// ```
pub trait DataProvider: Send + Sync + Debug {
    /// Provider专用错误类型
    type Error: std::error::Error + Send + Sync + 'static;

    /// 初始化Provider
    /// 
    /// 执行必要的初始化工作，如：
    /// - 验证配置参数
    /// - 建立连接
    /// - 分配资源
    /// 
    /// 此方法应该是幂等的，多次调用不应产生副作用
    fn initialize(&mut self) -> Result<(), Self::Error>;

    /// 启动数据流
    /// 
    /// 开始从数据源读取数据。对于：
    /// - WebSocket Provider：建立WebSocket连接
    /// - 文件Provider：开始读取文件
    /// - 混合Provider：启动所有数据源
    fn start(&mut self) -> Result<(), Self::Error>;

    /// 停止数据流
    /// 
    /// 优雅地停止数据读取，释放资源
    fn stop(&mut self) -> Result<(), Self::Error>;

    /// 检查连接/数据源状态
    /// 
    /// 返回Provider是否处于活跃状态：
    /// - WebSocket：连接是否建立且健康
    /// - 文件：文件是否打开且可读
    /// - 混合：至少一个数据源活跃
    fn is_connected(&self) -> bool;

    /// 非阻塞读取事件
    /// 
    /// 从数据源读取可用的事件，转换为标准EventType。
    /// 此方法必须是非阻塞的，如果没有可用数据应立即返回空Vec。
    /// 
    /// # 返回值
    /// - `Ok(events)`: 成功读取的事件列表（可能为空）
    /// - `Err(error)`: 读取过程中的错误
    /// 
    /// # 错误处理
    /// Provider应该尽可能从错误中恢复，只有在无法恢复时才返回错误
    fn read_events(&mut self) -> Result<Vec<EventType>, Self::Error>;

    /// 获取Provider状态信息
    /// 
    /// 返回详细的状态信息，用于监控和调试
    fn get_status(&self) -> ProviderStatus;

    /// 获取Provider类型
    /// 
    /// 标识这是实时、历史还是混合数据源
    fn provider_type(&self) -> ProviderType;

    /// 获取支持的事件类型
    /// 
    /// 返回此Provider能够生成的事件类型列表
    fn supported_events(&self) -> &[EventKind];

    /// 获取Provider配置信息（可选）
    /// 
    /// 返回当前的配置信息，用于UI显示和调试
    fn get_config_info(&self) -> Option<String> {
        None
    }

    /// 检查Provider健康状态（可选）
    /// 
    /// 执行健康检查，判断Provider是否正常工作
    /// 默认实现基于连接状态，具体Provider可以覆盖此方法
    fn health_check(&self) -> bool {
        self.is_connected()
    }

    /// 获取性能指标（可选）
    /// 
    /// 返回Provider的性能统计信息
    fn get_performance_metrics(&self) -> Option<PerformanceMetrics> {
        None
    }
}

/// 可控制的Provider接口
/// 
/// 为历史数据Provider等提供播放控制功能
pub trait ControllableProvider: DataProvider {
    /// 暂停数据流
    fn pause(&mut self) -> Result<(), Self::Error>;

    /// 恢复数据流
    fn resume(&mut self) -> Result<(), Self::Error>;

    /// 设置播放速度
    /// 
    /// # 参数
    /// - `speed`: 播放速度倍数，1.0为正常速度
    fn set_playback_speed(&mut self, speed: f64) -> Result<(), Self::Error>;

    /// 跳转到指定时间点
    /// 
    /// # 参数
    /// - `timestamp`: 目标时间戳（毫秒）
    fn seek_to(&mut self, timestamp: u64) -> Result<(), Self::Error>;

    /// 获取当前播放进度
    /// 
    /// 返回当前播放进度信息
    fn get_playback_info(&self) -> Option<PlaybackInfo>;
}

/// 配置Provider接口
/// 
/// 为支持运行时配置的Provider提供配置管理功能
pub trait ConfigurableProvider: DataProvider {
    /// 配置类型
    type Config: Clone + Send + Sync;

    /// 更新配置
    fn update_config(&mut self, config: Self::Config) -> Result<(), Self::Error>;

    /// 获取当前配置
    fn get_config(&self) -> Self::Config;

    /// 验证配置有效性
    fn validate_config(config: &Self::Config) -> Result<(), Self::Error>;
}

/// Provider工厂接口
/// 
/// 用于动态创建Provider实例
pub trait ProviderFactory: Send + Sync {
    /// 创建的Provider类型
    type Provider: DataProvider;
    
    /// 配置类型
    type Config: Clone + Send + Sync;

    /// 创建Provider实例
    fn create_provider(&self, config: Self::Config) -> Result<Self::Provider, ProviderError>;

    /// 获取支持的Provider类型
    fn provider_type(&self) -> ProviderType;

    /// 获取工厂名称
    fn name(&self) -> &str;

    /// 验证配置
    fn validate_config(&self, config: &Self::Config) -> Result<(), ProviderError>;
}

/// Provider创建工厂
/// 
/// 用于根据ProviderType创建相应的Provider实例
pub struct ProviderCreator;

impl ProviderCreator {
    /// 创建Binance Provider
    pub fn create_binance(config: crate::config::provider_config::BinanceWebSocketConfig) -> Result<BinanceProvider, ProviderError> {
        Ok(BinanceProvider::new(config))
    }
    
    /// 创建Historical Data Provider
    pub fn create_historical(config: gzip_historical_provider::HistoricalDataConfig) -> Result<HistoricalDataProvider, ProviderError> {
        Ok(HistoricalDataProvider::new(config))
    }
    
    /// 创建Gzip Data Provider
    pub fn create_gzip(config: gzip_provider::GzipProviderConfig) -> Result<GzipProvider, ProviderError> {
        Ok(GzipProvider::new(config))
    }
    
    /// 创建AnyProvider（动态Provider）
    pub fn create_any_provider(provider_type: ProviderType, config_json: serde_json::Value) -> ProviderResult<AnyProvider> {
        match provider_type {
            ProviderType::Binance { .. } => {
                let config: crate::config::provider_config::BinanceWebSocketConfig = serde_json::from_value(config_json)
                    .map_err(|e| ProviderError::configuration(
                        format!("Binance配置解析失败: {}", e)
                    ))?;
                Ok(AnyProvider::Binance(Self::create_binance(config)?))
            },
            ProviderType::HistoricalData { format } => {
                match format {
                    HistoricalDataFormat::Compressed => {
                        let config: gzip_provider::GzipProviderConfig = serde_json::from_value(config_json)
                            .map_err(|e| ProviderError::configuration(
                                format!("Gzip配置解析失败: {}", e)
                            ))?;
                        Ok(AnyProvider::Gzip(Self::create_gzip(config)?))
                    },
                    _ => {
                        let config: gzip_historical_provider::HistoricalDataConfig = serde_json::from_value(config_json)
                            .map_err(|e| ProviderError::configuration(
                                format!("历史数据配置解析失败: {}", e)
                            ))?;
                        Ok(AnyProvider::Historical(Self::create_historical(config)?))
                    }
                }
            },
            #[cfg(feature = "okx")]
            ProviderType::OKX { .. } => {
                Err(ProviderError::configuration("OKX Provider暂未实现".to_string()))
            },
            #[cfg(feature = "bybit")]
            ProviderType::Bybit { .. } => {
                Err(ProviderError::configuration("Bybit Provider暂未实现".to_string()))
            },
            ProviderType::Custom { .. } => {
                Err(ProviderError::configuration("自定义Provider暂未实现".to_string()))
            },
        }
    }
}