// Provider错误处理系统
//
// 本文件定义了Provider系统的统一错误处理机制，包括：
// - ProviderError：统一的错误类型
// - 具体错误变体：连接、配置、数据解析等错误
// - 错误恢复策略：重试、降级、通知等
// - 错误上下文：提供详细的调试信息
//
// 设计原则：
// 1. 统一接口：所有Provider使用相同的错误类型
// 2. 详细信息：提供足够的上下文便于调试
// 3. 可恢复性：区分可恢复和不可恢复的错误
// 4. 性能友好：避免不必要的字符串分配

use std::fmt;
use std::error::Error as StdError;
use thiserror::Error;

/// Provider统一错误类型
/// 
/// 所有Provider相关的错误都应该转换为此类型，提供统一的错误处理接口
#[derive(Error, Debug)]
pub enum ProviderError {
    /// 初始化错误
    /// 
    /// 在Provider初始化阶段发生的错误，如配置验证失败、资源分配失败等
    #[error("Provider initialization failed: {message}")]
    InitializationError {
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// 连接错误
    /// 
    /// 网络连接相关的错误，如WebSocket连接失败、DNS解析失败等
    #[error("Connection error: {message}")]
    ConnectionError {
        message: String,
        endpoint: Option<String>,
        retry_count: u32,
        is_recoverable: bool,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// 配置错误
    /// 
    /// Provider配置相关的错误，如参数验证失败、配置文件格式错误等
    #[error("Configuration error: {message}")]
    ConfigurationError {
        message: String,
        field: Option<String>,
        expected: Option<String>,
        actual: Option<String>,
    },

    /// 数据解析错误
    /// 
    /// 数据格式解析错误，如JSON解析失败、时间戳格式错误等
    #[error("Data parsing error: {message}")]
    DataParsingError {
        message: String,
        data_type: String,
        raw_data: Option<String>,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// 文件系统错误
    /// 
    /// 文件操作相关的错误，如文件不存在、权限不足等
    #[error("File system error: {message}")]
    FileSystemError {
        message: String,
        file_path: Option<String>,
        operation: String, // "read", "write", "open", "seek" etc.
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// 网络超时错误
    /// 
    /// 网络操作超时，区分连接超时和数据传输超时
    #[error("Network timeout: {message}")]
    TimeoutError {
        message: String,
        timeout_type: TimeoutType,
        timeout_duration_ms: u64,
        retry_count: u32,
    },

    /// 状态错误
    /// 
    /// Provider状态不正确导致的错误，如在未连接状态下尝试读取数据
    #[error("Invalid state: {message}")]
    StateError {
        message: String,
        current_state: String,
        expected_state: String,
        operation: String,
    },

    /// 资源不足错误
    /// 
    /// 系统资源不足，如内存不足、文件句柄用尽等
    #[error("Resource exhaustion: {message}")]
    ResourceError {
        message: String,
        resource_type: String,
        current_usage: Option<u64>,
        limit: Option<u64>,
    },

    /// 数据验证错误
    /// 
    /// 接收到的数据不符合预期格式或范围
    #[error("Data validation error: {message}")]
    ValidationError {
        message: String,
        field: Option<String>,
        constraint: String,
        value: Option<String>,
    },

    /// 认证/授权错误
    /// 
    /// API密钥错误、权限不足等认证相关错误
    #[error("Authentication error: {message}")]
    AuthenticationError {
        message: String,
        error_code: Option<i32>,
        retry_after: Option<u64>, // 重试延迟（毫秒）
    },

    /// 速率限制错误
    /// 
    /// API调用频率超出限制
    #[error("Rate limit exceeded: {message}")]
    RateLimitError {
        message: String,
        limit_type: String,
        retry_after: u64, // 重试延迟（毫秒）
        current_rate: Option<f64>,
        limit_rate: Option<f64>,
    },

    /// 协议错误
    /// 
    /// WebSocket协议、HTTP协议等协议层面的错误
    #[error("Protocol error: {message}")]
    ProtocolError {
        message: String,
        protocol: String, // "websocket", "http", "tcp" etc.
        error_code: Option<i32>,
        is_recoverable: bool,
    },

    /// 内部错误
    /// 
    /// Provider内部逻辑错误，通常表示程序bug
    #[error("Internal error: {message}")]
    InternalError {
        message: String,
        component: String,
        debug_info: Option<String>,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// 业务逻辑错误
    /// 
    /// 业务层面的错误，如市场关闭时间、不支持的交易对等
    #[error("Business logic error: {message}")]
    BusinessError {
        message: String,
        error_code: String,
        recoverable: bool,
        user_message: Option<String>, // 用户友好的错误消息
    },
}

/// 超时类型分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutType {
    /// 连接建立超时
    Connection,
    /// 数据传输超时
    DataTransfer,
    /// 心跳超时
    Heartbeat,
    /// 响应超时
    Response,
    /// 自定义超时
    Custom,
}

impl fmt::Display for TimeoutType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeoutType::Connection => write!(f, "Connection"),
            TimeoutType::DataTransfer => write!(f, "DataTransfer"),
            TimeoutType::Heartbeat => write!(f, "Heartbeat"),
            TimeoutType::Response => write!(f, "Response"),
            TimeoutType::Custom => write!(f, "Custom"),
        }
    }
}

/// Provider错误结果类型别名
pub type ProviderResult<T> = Result<T, ProviderError>;

impl ProviderError {
    /// 创建初始化错误
    pub fn initialization(message: impl Into<String>) -> Self {
        ProviderError::InitializationError {
            message: message.into(),
            source: None,
        }
    }

    /// 创建带源错误的初始化错误
    pub fn initialization_with_source(
        message: impl Into<String>,
        source: Box<dyn StdError + Send + Sync>,
    ) -> Self {
        ProviderError::InitializationError {
            message: message.into(),
            source: Some(source),
        }
    }

    /// 创建连接错误
    pub fn connection(
        message: impl Into<String>,
        endpoint: Option<String>,
        is_recoverable: bool,
    ) -> Self {
        ProviderError::ConnectionError {
            message: message.into(),
            endpoint,
            retry_count: 0,
            is_recoverable,
            source: None,
        }
    }

    /// 创建配置错误
    pub fn configuration(message: impl Into<String>) -> Self {
        ProviderError::ConfigurationError {
            message: message.into(),
            field: None,
            expected: None,
            actual: None,
        }
    }

    /// 创建配置字段错误
    pub fn configuration_field(
        message: impl Into<String>,
        field: impl Into<String>,
        expected: Option<String>,
        actual: Option<String>,
    ) -> Self {
        ProviderError::ConfigurationError {
            message: message.into(),
            field: Some(field.into()),
            expected,
            actual,
        }
    }

    /// 创建数据解析错误
    pub fn data_parsing(
        message: impl Into<String>,
        data_type: impl Into<String>,
    ) -> Self {
        ProviderError::DataParsingError {
            message: message.into(),
            data_type: data_type.into(),
            raw_data: None,
            source: None,
        }
    }

    /// 创建文件系统错误
    pub fn file_system(
        message: impl Into<String>,
        operation: impl Into<String>,
        file_path: Option<String>,
    ) -> Self {
        ProviderError::FileSystemError {
            message: message.into(),
            file_path,
            operation: operation.into(),
            source: None,
        }
    }

    /// 创建超时错误
    pub fn timeout(
        message: impl Into<String>,
        timeout_type: TimeoutType,
        timeout_duration_ms: u64,
    ) -> Self {
        ProviderError::TimeoutError {
            message: message.into(),
            timeout_type,
            timeout_duration_ms,
            retry_count: 0,
        }
    }

    /// 创建状态错误
    pub fn state(
        message: impl Into<String>,
        current_state: impl Into<String>,
        expected_state: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        ProviderError::StateError {
            message: message.into(),
            current_state: current_state.into(),
            expected_state: expected_state.into(),
            operation: operation.into(),
        }
    }

    /// 创建验证错误
    pub fn validation(
        message: impl Into<String>,
        constraint: impl Into<String>,
    ) -> Self {
        ProviderError::ValidationError {
            message: message.into(),
            field: None,
            constraint: constraint.into(),
            value: None,
        }
    }

    /// 创建内部错误
    pub fn internal(
        message: impl Into<String>,
        component: impl Into<String>,
    ) -> Self {
        ProviderError::InternalError {
            message: message.into(),
            component: component.into(),
            debug_info: None,
            source: None,
        }
    }

    /// 创建业务错误
    pub fn business(
        message: impl Into<String>,
        error_code: impl Into<String>,
        recoverable: bool,
    ) -> Self {
        ProviderError::BusinessError {
            message: message.into(),
            error_code: error_code.into(),
            recoverable,
            user_message: None,
        }
    }

    /// 检查错误是否可恢复
    /// 
    /// 可恢复的错误通常可以通过重试、重连等方式解决
    pub fn is_recoverable(&self) -> bool {
        match self {
            ProviderError::ConnectionError { is_recoverable, .. } => *is_recoverable,
            ProviderError::TimeoutError { .. } => true,
            ProviderError::RateLimitError { .. } => true,
            ProviderError::ProtocolError { is_recoverable, .. } => *is_recoverable,
            ProviderError::BusinessError { recoverable, .. } => *recoverable,
            ProviderError::ConfigurationError { .. } => false,
            ProviderError::AuthenticationError { .. } => false,
            ProviderError::ValidationError { .. } => false,
            ProviderError::InternalError { .. } => false,
            _ => true, // 默认认为是可恢复的
        }
    }

    /// 检查是否需要重试
    /// 
    /// 某些错误（如网络错误）可以立即重试，某些错误需要延迟重试
    pub fn should_retry(&self) -> bool {
        match self {
            ProviderError::ConnectionError { .. } => true,
            ProviderError::TimeoutError { .. } => true,
            ProviderError::DataParsingError { .. } => false, // 数据解析错误重试也会失败
            ProviderError::FileSystemError { .. } => true,
            ProviderError::ProtocolError { is_recoverable, .. } => *is_recoverable,
            ProviderError::RateLimitError { .. } => true, // 但需要延迟
            ProviderError::ConfigurationError { .. } => false,
            ProviderError::AuthenticationError { .. } => false,
            ProviderError::ValidationError { .. } => false,
            ProviderError::InternalError { .. } => false,
            _ => false,
        }
    }

    /// 获取建议的重试延迟（毫秒）
    pub fn retry_delay_ms(&self) -> Option<u64> {
        match self {
            ProviderError::ConnectionError { retry_count, .. } => {
                // 指数退避策略
                Some(std::cmp::min(1000 * (2u64.pow(*retry_count)), 30000))
            }
            ProviderError::TimeoutError { retry_count, .. } => {
                Some(std::cmp::min(500 * (2u64.pow(*retry_count)), 10000))
            }
            ProviderError::RateLimitError { retry_after, .. } => Some(*retry_after),
            ProviderError::AuthenticationError { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// 获取错误类型字符串
    pub fn error_type(&self) -> &'static str {
        match self {
            ProviderError::InitializationError { .. } => "InitializationError",
            ProviderError::ConnectionError { .. } => "ConnectionError",
            ProviderError::ConfigurationError { .. } => "ConfigurationError",
            ProviderError::DataParsingError { .. } => "DataParsingError",
            ProviderError::FileSystemError { .. } => "FileSystemError",
            ProviderError::TimeoutError { .. } => "TimeoutError",
            ProviderError::StateError { .. } => "StateError",
            ProviderError::ResourceError { .. } => "ResourceError",
            ProviderError::ValidationError { .. } => "ValidationError",
            ProviderError::AuthenticationError { .. } => "AuthenticationError",
            ProviderError::RateLimitError { .. } => "RateLimitError",
            ProviderError::ProtocolError { .. } => "ProtocolError",
            ProviderError::InternalError { .. } => "InternalError",
            ProviderError::BusinessError { .. } => "BusinessError",
        }
    }

    /// 增加重试计数
    pub fn increment_retry_count(&mut self) {
        match self {
            ProviderError::ConnectionError { retry_count, .. } => {
                *retry_count += 1;
            }
            ProviderError::TimeoutError { retry_count, .. } => {
                *retry_count += 1;
            }
            _ => {}
        }
    }

    /// 获取用户友好的错误消息
    pub fn user_message(&self) -> String {
        match self {
            ProviderError::ConnectionError { message, .. } => {
                format!("连接失败: {}", message)
            }
            ProviderError::ConfigurationError { message, .. } => {
                format!("配置错误: {}", message)
            }
            ProviderError::AuthenticationError { message, .. } => {
                format!("认证失败: {}", message)
            }
            ProviderError::RateLimitError { message, .. } => {
                format!("请求过于频繁: {}", message)
            }
            ProviderError::BusinessError { user_message, message, .. } => {
                user_message.as_ref().unwrap_or(message).clone()
            }
            _ => format!("系统错误: {}", self),
        }
    }
}

// 为常见的标准库错误类型实现转换
impl From<std::io::Error> for ProviderError {
    fn from(error: std::io::Error) -> Self {
        ProviderError::FileSystemError {
            message: error.to_string(),
            file_path: None,
            operation: "io_operation".to_string(),
            source: Some(Box::new(error)),
        }
    }
}

impl From<serde_json::Error> for ProviderError {
    fn from(error: serde_json::Error) -> Self {
        ProviderError::DataParsingError {
            message: error.to_string(),
            data_type: "json".to_string(),
            raw_data: None,
            source: Some(Box::new(error)),
        }
    }
}

impl From<std::num::ParseIntError> for ProviderError {
    fn from(error: std::num::ParseIntError) -> Self {
        ProviderError::DataParsingError {
            message: error.to_string(),
            data_type: "integer".to_string(),
            raw_data: None,
            source: Some(Box::new(error)),
        }
    }
}

impl From<std::num::ParseFloatError> for ProviderError {
    fn from(error: std::num::ParseFloatError) -> Self {
        ProviderError::DataParsingError {
            message: error.to_string(),
            data_type: "float".to_string(),
            raw_data: None,
            source: Some(Box::new(error)),
        }
    }
}

/// 错误恢复策略
/// 
/// 定义针对不同错误类型的恢复策略
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStrategy {
    /// 立即重试
    RetryImmediately,
    /// 延迟重试
    RetryWithDelay(u64), // 延迟毫秒数
    /// 重新连接
    Reconnect,
    /// 切换到备用数据源
    Fallback,
    /// 降级服务
    Degrade,
    /// 停止服务
    Stop,
    /// 通知用户
    NotifyUser,
}

impl ProviderError {
    /// 获取推荐的恢复策略
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            ProviderError::ConnectionError { is_recoverable: true, .. } => {
                RecoveryStrategy::Reconnect
            }
            ProviderError::TimeoutError { .. } => RecoveryStrategy::RetryWithDelay(1000),
            ProviderError::RateLimitError { retry_after, .. } => {
                RecoveryStrategy::RetryWithDelay(*retry_after)
            }
            ProviderError::ConfigurationError { .. } => RecoveryStrategy::Stop,
            ProviderError::AuthenticationError { .. } => RecoveryStrategy::NotifyUser,
            ProviderError::DataParsingError { .. } => RecoveryStrategy::RetryImmediately,
            ProviderError::FileSystemError { .. } => RecoveryStrategy::RetryWithDelay(500),
            ProviderError::InternalError { .. } => RecoveryStrategy::Stop,
            _ => RecoveryStrategy::Fallback,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recoverability() {
        let connection_error = ProviderError::connection("Test error".to_string(), None, true);
        assert!(connection_error.is_recoverable());

        let config_error = ProviderError::configuration("Invalid config".to_string());
        assert!(!config_error.is_recoverable());
    }

    #[test]
    fn test_retry_delay_calculation() {
        let mut connection_error = ProviderError::connection("Test error".to_string(), None, true);
        
        assert_eq!(connection_error.retry_delay_ms(), Some(1000)); // 首次重试延迟1秒
        
        connection_error.increment_retry_count();
        assert_eq!(connection_error.retry_delay_ms(), Some(2000)); // 第二次重试延迟2秒
    }

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let provider_error: ProviderError = io_error.into();
        
        match provider_error {
            ProviderError::FileSystemError { operation, .. } => {
                assert_eq!(operation, "io_operation");
            }
            _ => panic!("Expected FileSystemError"),
        }
    }

    #[test]
    fn test_recovery_strategy() {
        let connection_error = ProviderError::connection("Test error".to_string(), None, true);
        assert_eq!(connection_error.recovery_strategy(), RecoveryStrategy::Reconnect);

        let config_error = ProviderError::configuration("Invalid config".to_string());
        assert_eq!(config_error.recovery_strategy(), RecoveryStrategy::Stop);
    }
}