/// GUI服务管理系统
/// 
/// 提供服务化的GUI架构，支持:
/// - 服务注册与发现
/// - 服务间消息通信  
/// - 服务生命周期管理
/// - 服务健康监控

pub mod types;
pub mod manager;
pub mod message;
pub mod health;

// 重新导出公共类型
pub use types::{
    ServiceId, ServiceState, ServiceConfig, ServiceMessageType, MessagePriority
};
pub use manager::{GUIServiceManager, GUIServiceManagerConfig};
pub use message::{ServiceMessage, MessageRouter};
pub use health::{ServiceHealth, ServiceStats};

use async_trait::async_trait;
use std::any::Any;
use super::component::{ComponentError, ComponentResult};

/// GUI服务trait
/// 
/// 所有GUI服务都必须实现此trait
#[async_trait]
pub trait GUIService: Send + Sync {
    /// 获取服务ID
    fn id(&self) -> &ServiceId;
    
    /// 获取服务配置
    fn config(&self) -> &ServiceConfig;
    
    /// 获取当前状态
    fn state(&self) -> ServiceState;
    
    /// 启动服务
    async fn start(&mut self) -> ComponentResult<()>;
    
    /// 停止服务
    async fn stop(&mut self) -> ComponentResult<()>;
    
    /// 暂停服务
    async fn pause(&mut self) -> ComponentResult<()>;
    
    /// 恢复服务
    async fn resume(&mut self) -> ComponentResult<()>;
    
    /// 处理消息
    async fn handle_message(&mut self, message: ServiceMessage) -> ComponentResult<Option<ServiceMessage>>;
    
    /// 健康检查
    async fn health_check(&self) -> ComponentResult<ServiceHealth>;
    
    /// 获取服务统计信息
    async fn get_stats(&self) -> ComponentResult<ServiceStats>;
    
    /// 重新加载配置
    async fn reload_config(&mut self, config: ServiceConfig) -> ComponentResult<()>;
    
    /// 转换为Any trait
    fn as_any(&self) -> &dyn Any;
    
    /// 转换为可变Any trait
    fn as_any_mut(&mut self) -> &mut dyn Any;
}