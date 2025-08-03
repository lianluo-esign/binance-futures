/// 服务层模块 - 实现业务逻辑的服务化架构
/// 
/// 服务层包含以下核心服务:
/// - DataProcessingService: 数据处理服务
/// - RenderingService: 渲染服务
/// - PerformanceService: 性能监控服务
/// - EventService: 事件管理服务
/// - ConfigurationService: 配置管理服务

pub mod data_processing_service;
pub mod rendering_service;
pub mod performance_service;
pub mod event_service;
pub mod configuration_service;
pub mod service_manager;

// 重新导出主要服务接口
pub use data_processing_service::{DataProcessingService, DataProcessor, ProcessingResult};
pub use rendering_service::{RenderingService, RenderingQueue, RenderCommand};
pub use performance_service::{PerformanceService, PerformanceMonitor};
pub use event_service::{EventService, EventProcessor};
pub use configuration_service::{ConfigurationService, ConfigManager};
pub use service_manager::{ServiceManager, ServiceContainer};

/// 服务基础trait - 所有服务都必须实现
pub trait Service: Send + Sync {
    /// 服务名称
    fn name(&self) -> &'static str;
    
    /// 启动服务
    fn start(&mut self) -> Result<(), ServiceError>;
    
    /// 停止服务
    fn stop(&mut self) -> Result<(), ServiceError>;
    
    /// 重启服务
    fn restart(&mut self) -> Result<(), ServiceError> {
        self.stop()?;
        self.start()
    }
    
    /// 检查服务健康状态
    fn health_check(&self) -> ServiceHealth;
    
    /// 获取服务统计信息
    fn stats(&self) -> ServiceStats;
}

/// 服务错误类型
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("服务初始化失败: {0}")]
    InitializationFailed(String),
    
    #[error("服务已经在运行")]
    AlreadyRunning,
    
    #[error("服务未运行")]
    NotRunning,
    
    #[error("服务配置错误: {0}")]
    ConfigurationError(String),
    
    #[error("服务依赖错误: {0}")]
    DependencyError(String),
    
    #[error("内部错误: {0}")]
    InternalError(String),
}

/// 服务健康状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceHealth {
    /// 健康运行
    Healthy,
    /// 警告状态
    Warning(String),
    /// 不健康状态
    Unhealthy(String),
    /// 未知状态
    Unknown,
}

/// 服务统计信息
#[derive(Debug, Clone)]
pub struct ServiceStats {
    /// 服务名称
    pub service_name: String,
    /// 运行状态
    pub is_running: bool,
    /// 启动时间
    pub start_time: Option<std::time::Instant>,
    /// 处理的请求数量
    pub requests_processed: u64,
    /// 错误数量
    pub error_count: u64,
    /// 平均响应时间 (毫秒)
    pub avg_response_time_ms: f64,
    /// 内存使用量 (字节)
    pub memory_usage_bytes: usize,
}

impl Default for ServiceStats {
    fn default() -> Self {
        Self {
            service_name: String::new(),
            is_running: false,
            start_time: None,
            requests_processed: 0,
            error_count: 0,
            avg_response_time_ms: 0.0,
            memory_usage_bytes: 0,
        }
    }
}

/// 服务依赖关系
#[derive(Debug, Clone)]
pub struct ServiceDependency {
    /// 依赖的服务名称
    pub service_name: String,
    /// 是否为必需依赖
    pub required: bool,
    /// 依赖描述
    pub description: String,
}

/// 可配置的服务trait
pub trait ConfigurableService: Service {
    type Config;
    
    /// 更新服务配置
    fn update_config(&mut self, config: Self::Config) -> Result<(), ServiceError>;
    
    /// 获取当前配置
    fn get_config(&self) -> &Self::Config;
}

/// 可监控的服务trait
pub trait MonitorableService: Service {
    /// 获取详细的监控指标
    fn get_metrics(&self) -> Vec<ServiceMetric>;
    
    /// 设置监控回调
    fn set_monitor_callback(&mut self, callback: Box<dyn Fn(&ServiceMetric) + Send + Sync>);
}

/// 服务监控指标
#[derive(Debug, Clone)]
pub struct ServiceMetric {
    /// 指标名称
    pub name: String,
    /// 指标值
    pub value: MetricValue,
    /// 时间戳
    pub timestamp: std::time::Instant,
    /// 标签
    pub labels: std::collections::HashMap<String, String>,
}

/// 指标值类型
#[derive(Debug, Clone)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
    Boolean(bool),
    String(String),
}

/// 异步服务trait
#[async_trait::async_trait]
pub trait AsyncService: Send + Sync {
    /// 异步启动服务
    async fn start_async(&mut self) -> Result<(), ServiceError>;
    
    /// 异步停止服务
    async fn stop_async(&mut self) -> Result<(), ServiceError>;
    
    /// 异步健康检查
    async fn health_check_async(&self) -> ServiceHealth;
}

/// 服务工厂trait
pub trait ServiceFactory<T: Service> {
    /// 创建服务实例
    fn create_service(&self, config: &dyn std::any::Any) -> Result<Box<T>, ServiceError>;
    
    /// 获取服务类型名称
    fn service_type(&self) -> &'static str;
}

/// 服务注册表
pub struct ServiceRegistry {
    factories: std::collections::HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            factories: std::collections::HashMap::new(),
        }
    }
    
    /// 注册服务工厂
    pub fn register_factory<T: Service + 'static>(
        &mut self,
        factory: Box<dyn ServiceFactory<T> + Send + Sync>,
    ) {
        let service_type = factory.service_type().to_string();
        self.factories.insert(service_type, Box::new(factory));
    }
    
    /// 创建服务
    pub fn create_service<T: Service + 'static>(
        &self,
        service_type: &str,
        config: &dyn std::any::Any,
    ) -> Result<Box<T>, ServiceError> {
        let factory = self.factories.get(service_type)
            .ok_or_else(|| ServiceError::ConfigurationError(
                format!("未找到服务类型: {}", service_type)
            ))?;
            
        // 这里需要类型转换，实际实现中可能需要更复杂的处理
        Err(ServiceError::InternalError("服务创建未实现".to_string()))
    }
}