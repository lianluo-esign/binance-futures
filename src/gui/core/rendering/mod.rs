/// GUI渲染管理系统
/// 
/// 提供高性能的渲染管道:
/// - 渲染命令队列
/// - 批量渲染优化
/// - GPU资源管理
/// - 渲染状态缓存

pub mod command;
pub mod queue;
pub mod pipeline;
pub mod manager;

// 重新导出公共类型
pub use command::{RenderCommand, RenderTarget};
pub use queue::RenderQueue;
pub use pipeline::{RenderPipeline, RenderState, RenderStats, RenderPipelineConfig};
pub use manager::{RenderManager, RenderManagerConfig};