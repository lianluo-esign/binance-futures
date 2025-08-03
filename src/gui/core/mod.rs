/// GUI核心模块 - 提供面向对象的GUI组件基础架构
/// 
/// 设计原则:
/// - 高内聚低耦合: 每个组件独立负责自己的功能
/// - 服务化: 通过服务接口进行组件间通信
/// - 可扩展: 支持动态添加新组件类型
/// - 生命周期管理: 统一的组件创建、更新、销毁流程

pub mod component;
pub mod service;
pub mod messaging;
pub mod lifecycle;
pub mod rendering;
pub mod state;

// 重新导出核心类型
pub use component::{
    GUIComponent, ComponentId, ComponentType, ComponentState, ComponentConfig,
    RenderContext, UpdateContext, ComponentError, ComponentResult
};
pub use service::{
    GUIService, GUIServiceManager, ServiceId, ServiceState, ServiceConfig, 
    ServiceMessage, ServiceMessageType, MessagePriority, ServiceHealth, ServiceStats
};
pub use messaging::{
    MessageBus, MessageHandler, EventChannel, EventFilter
};
pub use component::{ComponentEvent};
pub use lifecycle::{
    ComponentLifecycle, LifecycleManager, LifecyclePhase, ComponentRegistry
};
pub use rendering::{
    RenderManager, RenderCommand, RenderTarget, RenderPipeline, RenderQueue, RenderManagerConfig
};
pub use state::{
    StateManager, ComponentStateSnapshot, StateChangeEvent, StatePersistence, StateManagerConfig
};