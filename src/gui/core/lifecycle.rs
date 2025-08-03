/// 组件生命周期管理系统
/// 
/// 提供统一的组件生命周期管理:
/// - 组件注册与发现
/// - 生命周期阶段管理
/// - 依赖注入和解析
/// - 资源管理和清理

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::component::{
    GUIComponent, ComponentId, ComponentType, ComponentState, ComponentConfig, 
    ComponentError, ComponentResult, UpdateContext
};

/// 生命周期阶段
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecyclePhase {
    /// 未创建
    NotCreated,
    /// 正在创建
    Creating,
    /// 已创建
    Created,
    /// 正在初始化
    Initializing,
    /// 已初始化
    Initialized,
    /// 正在启动
    Starting,
    /// 已启动 (活跃)
    Running,
    /// 正在暂停
    Pausing,
    /// 已暂停
    Paused,
    /// 正在恢复
    Resuming,
    /// 正在停止
    Stopping,
    /// 已停止
    Stopped,
    /// 正在销毁
    Destroying,
    /// 已销毁
    Destroyed,
    /// 错误状态
    Error(String),
}

/// 组件依赖信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDependency {
    /// 依赖的组件ID
    pub component_id: ComponentId,
    /// 依赖类型
    pub dependency_type: DependencyType,
    /// 是否为可选依赖
    pub optional: bool,
    /// 最小版本要求
    pub min_version: Option<String>,
}

/// 依赖类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyType {
    /// 强依赖 (必须在此组件之前启动)
    Required,
    /// 弱依赖 (推荐在此组件之前启动)
    Preferred,
    /// 互斥依赖 (不能同时运行)
    Conflicts,
    /// 数据依赖 (需要数据交换)
    Data,
    /// 服务依赖 (需要服务接口)
    Service,
}

/// 组件注册信息
#[derive(Debug, Clone)]
pub struct ComponentRegistration {
    /// 组件配置
    pub config: ComponentConfig,
    /// 组件依赖
    pub dependencies: Vec<ComponentDependency>,
    /// 组件工厂函数
    pub factory: Arc<dyn ComponentFactory + Send + Sync>,
    /// 注册时间
    pub registered_at: std::time::SystemTime,
    /// 版本信息
    pub version: String,
    /// 描述信息
    pub description: String,
}

/// 组件工厂trait
#[async_trait]
pub trait ComponentFactory {
    /// 创建组件实例
    async fn create_component(&self, config: ComponentConfig) -> ComponentResult<Box<dyn GUIComponent>>;
    
    /// 获取组件类型
    fn component_type(&self) -> ComponentType;
    
    /// 获取默认配置
    fn default_config(&self) -> ComponentConfig;
    
    /// 验证配置
    fn validate_config(&self, config: &ComponentConfig) -> ComponentResult<()>;
}

/// 组件注册表
/// 
/// 管理所有可用的组件类型和实例
pub struct ComponentRegistry {
    /// 已注册的组件类型
    registrations: Arc<RwLock<HashMap<ComponentType, ComponentRegistration>>>,
    /// 活跃的组件实例
    instances: Arc<RwLock<HashMap<ComponentId, Arc<RwLock<Box<dyn GUIComponent>>>>>>,
    /// 组件状态缓存
    states: Arc<RwLock<HashMap<ComponentId, LifecyclePhase>>>,
    /// 依赖关系图
    dependency_graph: Arc<RwLock<HashMap<ComponentId, Vec<ComponentDependency>>>>,
}

impl ComponentRegistry {
    /// 创建新的组件注册表
    pub fn new() -> Self {
        Self {
            registrations: Arc::new(RwLock::new(HashMap::new())),
            instances: Arc::new(RwLock::new(HashMap::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
            dependency_graph: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 注册组件类型
    pub async fn register_component_type(
        &self,
        registration: ComponentRegistration,
    ) -> ComponentResult<()> {
        let component_type = registration.config.component_type.clone();
        
        // 验证配置
        registration.factory.validate_config(&registration.config)?;
        
        let mut registrations = self.registrations.write().await;
        
        if registrations.contains_key(&component_type) {
            return Err(ComponentError::ConfigError(
                format!("组件类型 {:?} 已注册", component_type)
            ));
        }
        
        registrations.insert(component_type.clone(), registration);
        log::info!("组件类型 {:?} 注册成功", component_type);
        
        Ok(())
    }
    
    /// 创建组件实例
    pub async fn create_component(
        &self,
        component_type: ComponentType,
        config: ComponentConfig,
    ) -> ComponentResult<ComponentId> {
        let component_id = config.id.clone();
        
        // 检查组件是否已存在
        {
            let instances = self.instances.read().await;
            if instances.contains_key(&component_id) {
                return Err(ComponentError::ConfigError(
                    format!("组件 {} 已存在", component_id)
                ));
            }
        }
        
        // 获取注册信息
        let registration = {
            let registrations = self.registrations.read().await;
            registrations.get(&component_type).cloned().ok_or_else(|| {
                ComponentError::ConfigError(
                    format!("组件类型 {:?} 未注册", component_type)
                )
            })?
        };
        
        // 创建组件实例
        let component = registration.factory.create_component(config).await?;
        
        // 存储组件实例和状态
        {
            let mut instances = self.instances.write().await;
            let mut states = self.states.write().await;
            let mut deps = self.dependency_graph.write().await;
            
            instances.insert(component_id.clone(), Arc::new(RwLock::new(component)));
            states.insert(component_id.clone(), LifecyclePhase::Created);
            deps.insert(component_id.clone(), registration.dependencies.clone());
        }
        
        log::info!("组件 {} ({:?}) 创建成功", component_id, component_type);
        Ok(component_id)
    }
    
    /// 获取组件实例
    pub async fn get_component(
        &self,
        component_id: &ComponentId,
    ) -> Option<Arc<RwLock<Box<dyn GUIComponent>>>> {
        let instances = self.instances.read().await;
        instances.get(component_id).cloned()
    }
    
    /// 获取组件状态
    pub async fn get_component_state(&self, component_id: &ComponentId) -> Option<LifecyclePhase> {
        let states = self.states.read().await;
        states.get(component_id).cloned()
    }
    
    /// 设置组件状态
    pub async fn set_component_state(
        &self,
        component_id: &ComponentId,
        state: LifecyclePhase,
    ) -> ComponentResult<()> {
        let mut states = self.states.write().await;
        
        if let Some(old_state) = states.get(component_id) {
            if old_state != &state {
                log::debug!("组件 {} 状态变更: {:?} -> {:?}", component_id, old_state, state);
            }
        }
        
        states.insert(component_id.clone(), state);
        Ok(())
    }
    
    /// 销毁组件
    pub async fn destroy_component(&self, component_id: &ComponentId) -> ComponentResult<()> {
        // 设置销毁状态
        self.set_component_state(component_id, LifecyclePhase::Destroying).await?;
        
        // 获取组件实例并清理
        if let Some(component_arc) = self.get_component(component_id).await {
            let mut component = component_arc.write().await;
            component.cleanup().await?;
        }
        
        // 从注册表中移除
        {
            let mut instances = self.instances.write().await;
            let mut states = self.states.write().await;
            let mut deps = self.dependency_graph.write().await;
            
            instances.remove(component_id);
            states.insert(component_id.clone(), LifecyclePhase::Destroyed);
            deps.remove(component_id);
        }
        
        log::info!("组件 {} 销毁成功", component_id);
        Ok(())
    }
    
    /// 获取所有组件状态
    pub async fn get_all_component_states(&self) -> HashMap<ComponentId, LifecyclePhase> {
        let states = self.states.read().await;
        states.clone()
    }
    
    /// 检查依赖关系
    pub async fn check_dependencies(&self, component_id: &ComponentId) -> ComponentResult<()> {
        let deps = self.dependency_graph.read().await;
        let states = self.states.read().await;
        
        if let Some(dependencies) = deps.get(component_id) {
            for dep in dependencies {
                if dep.optional {
                    continue; // 跳过可选依赖
                }
                
                match dep.dependency_type {
                    DependencyType::Required => {
                        if let Some(dep_state) = states.get(&dep.component_id) {
                            if !matches!(dep_state, LifecyclePhase::Running) {
                                return Err(ComponentError::ConfigError(
                                    format!("依赖组件 {} 未运行", dep.component_id)
                                ));
                            }
                        } else {
                            return Err(ComponentError::ConfigError(
                                format!("依赖组件 {} 不存在", dep.component_id)
                            ));
                        }
                    }
                    DependencyType::Conflicts => {
                        if let Some(dep_state) = states.get(&dep.component_id) {
                            if matches!(dep_state, LifecyclePhase::Running) {
                                return Err(ComponentError::ConfigError(
                                    format!("冲突组件 {} 正在运行", dep.component_id)
                                ));
                            }
                        }
                    }
                    _ => {} // 其他依赖类型的检查逻辑
                }
            }
        }
        
        Ok(())
    }
}

/// 生命周期管理器
/// 
/// 负责管理组件的生命周期状态转换
pub struct LifecycleManager {
    /// 组件注册表
    registry: Arc<ComponentRegistry>,
    /// 生命周期监听器
    listeners: Arc<RwLock<Vec<Arc<dyn LifecycleListener + Send + Sync>>>>,
    /// 管理器配置
    config: LifecycleManagerConfig,
}

/// 生命周期管理器配置
#[derive(Debug, Clone)]
pub struct LifecycleManagerConfig {
    /// 组件启动超时时间
    pub startup_timeout: std::time::Duration,
    /// 组件停止超时时间
    pub shutdown_timeout: std::time::Duration,
    /// 最大并发操作数
    pub max_concurrent_operations: usize,
    /// 是否启用依赖检查
    pub enable_dependency_check: bool,
}

impl Default for LifecycleManagerConfig {
    fn default() -> Self {
        Self {
            startup_timeout: std::time::Duration::from_secs(30),
            shutdown_timeout: std::time::Duration::from_secs(10),
            max_concurrent_operations: 10,
            enable_dependency_check: true,
        }
    }
}

impl LifecycleManager {
    /// 创建新的生命周期管理器
    pub fn new(registry: Arc<ComponentRegistry>, config: LifecycleManagerConfig) -> Self {
        Self {
            registry,
            listeners: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }
    
    /// 初始化组件
    pub async fn initialize_component(&self, component_id: &ComponentId) -> ComponentResult<()> {
        // 检查依赖
        if self.config.enable_dependency_check {
            self.registry.check_dependencies(component_id).await?;
        }
        
        // 设置初始化状态
        self.registry.set_component_state(component_id, LifecyclePhase::Initializing).await?;
        
        // 通知监听器
        self.notify_listeners(component_id, LifecyclePhase::Initializing).await;
        
        // 获取组件并初始化
        if let Some(component_arc) = self.registry.get_component(component_id).await {
            let mut component = component_arc.write().await;
            let config = component.config().clone();
            
            // 执行初始化
            let result = tokio::time::timeout(
                self.config.startup_timeout,
                component.initialize(config)
            ).await;
            
            match result {
                Ok(Ok(())) => {
                    self.registry.set_component_state(component_id, LifecyclePhase::Initialized).await?;
                    self.notify_listeners(component_id, LifecyclePhase::Initialized).await;
                    log::info!("组件 {} 初始化成功", component_id);
                }
                Ok(Err(e)) => {
                    let error_msg = format!("初始化失败: {}", e);
                    self.registry.set_component_state(component_id, LifecyclePhase::Error(error_msg.clone())).await?;
                    return Err(ComponentError::InitializationFailed(error_msg));
                }
                Err(_) => {
                    let error_msg = "初始化超时".to_string();
                    self.registry.set_component_state(component_id, LifecyclePhase::Error(error_msg.clone())).await?;
                    return Err(ComponentError::InitializationFailed(error_msg));
                }
            }
        } else {
            return Err(ComponentError::ConfigError(
                format!("组件 {} 不存在", component_id)
            ));
        }
        
        Ok(())
    }
    
    /// 启动组件
    pub async fn start_component(&self, component_id: &ComponentId) -> ComponentResult<()> {
        // 检查当前状态
        let current_state = self.registry.get_component_state(component_id).await
            .ok_or_else(|| ComponentError::ConfigError(format!("组件 {} 不存在", component_id)))?;
        
        if !matches!(current_state, LifecyclePhase::Initialized | LifecyclePhase::Stopped) {
            return Err(ComponentError::StateError(
                format!("组件 {} 当前状态 {:?} 不能启动", component_id, current_state)
            ));
        }
        
        // 设置启动状态
        self.registry.set_component_state(component_id, LifecyclePhase::Starting).await?;
        self.notify_listeners(component_id, LifecyclePhase::Starting).await;
        
        // 启动成功
        self.registry.set_component_state(component_id, LifecyclePhase::Running).await?;
        self.notify_listeners(component_id, LifecyclePhase::Running).await;
        
        log::info!("组件 {} 启动成功", component_id);
        Ok(())
    }
    
    /// 停止组件
    pub async fn stop_component(&self, component_id: &ComponentId) -> ComponentResult<()> {
        // 检查当前状态
        let current_state = self.registry.get_component_state(component_id).await
            .ok_or_else(|| ComponentError::ConfigError(format!("组件 {} 不存在", component_id)))?;
        
        if !matches!(current_state, LifecyclePhase::Running | LifecyclePhase::Paused) {
            return Err(ComponentError::StateError(
                format!("组件 {} 当前状态 {:?} 不能停止", component_id, current_state)
            ));
        }
        
        // 设置停止状态
        self.registry.set_component_state(component_id, LifecyclePhase::Stopping).await?;
        self.notify_listeners(component_id, LifecyclePhase::Stopping).await;
        
        // 停止完成
        self.registry.set_component_state(component_id, LifecyclePhase::Stopped).await?;
        self.notify_listeners(component_id, LifecyclePhase::Stopped).await;
        
        log::info!("组件 {} 停止成功", component_id);
        Ok(())
    }
    
    /// 暂停组件
    pub async fn pause_component(&self, component_id: &ComponentId) -> ComponentResult<()> {
        let current_state = self.registry.get_component_state(component_id).await
            .ok_or_else(|| ComponentError::ConfigError(format!("组件 {} 不存在", component_id)))?;
        
        if !matches!(current_state, LifecyclePhase::Running) {
            return Err(ComponentError::StateError(
                format!("组件 {} 当前状态 {:?} 不能暂停", component_id, current_state)
            ));
        }
        
        self.registry.set_component_state(component_id, LifecyclePhase::Pausing).await?;
        self.notify_listeners(component_id, LifecyclePhase::Pausing).await;
        
        self.registry.set_component_state(component_id, LifecyclePhase::Paused).await?;
        self.notify_listeners(component_id, LifecyclePhase::Paused).await;
        
        log::info!("组件 {} 暂停成功", component_id);
        Ok(())
    }
    
    /// 恢复组件
    pub async fn resume_component(&self, component_id: &ComponentId) -> ComponentResult<()> {
        let current_state = self.registry.get_component_state(component_id).await
            .ok_or_else(|| ComponentError::ConfigError(format!("组件 {} 不存在", component_id)))?;
        
        if !matches!(current_state, LifecyclePhase::Paused) {
            return Err(ComponentError::StateError(
                format!("组件 {} 当前状态 {:?} 不能恢复", component_id, current_state)
            ));
        }
        
        self.registry.set_component_state(component_id, LifecyclePhase::Resuming).await?;
        self.notify_listeners(component_id, LifecyclePhase::Resuming).await;
        
        self.registry.set_component_state(component_id, LifecyclePhase::Running).await?;
        self.notify_listeners(component_id, LifecyclePhase::Running).await;
        
        log::info!("组件 {} 恢复成功", component_id);
        Ok(())
    }
    
    /// 添加生命周期监听器
    pub async fn add_listener(&self, listener: Arc<dyn LifecycleListener + Send + Sync>) {
        let mut listeners = self.listeners.write().await;
        listeners.push(listener);
    }
    
    /// 移除生命周期监听器
    pub async fn remove_listener(&self, listener_name: &str) {
        let mut listeners = self.listeners.write().await;
        listeners.retain(|l| l.name() != listener_name);
    }
    
    /// 通知所有监听器
    async fn notify_listeners(&self, component_id: &ComponentId, phase: LifecyclePhase) {
        let listeners = self.listeners.read().await;
        
        for listener in listeners.iter() {
            if let Err(e) = listener.on_lifecycle_change(component_id, &phase).await {
                log::warn!("生命周期监听器 {} 处理失败: {}", listener.name(), e);
            }
        }
    }
}

/// 生命周期监听器trait
#[async_trait]
pub trait LifecycleListener {
    /// 生命周期变更回调
    async fn on_lifecycle_change(
        &self,
        component_id: &ComponentId,
        phase: &LifecyclePhase,
    ) -> ComponentResult<()>;
    
    /// 获取监听器名称
    fn name(&self) -> &str;
}

/// 简单生命周期监听器
pub struct SimpleLifecycleListener<F>
where
    F: Fn(&ComponentId, &LifecyclePhase) -> ComponentResult<()> + Send + Sync,
{
    name: String,
    callback: F,
}

impl<F> SimpleLifecycleListener<F>
where
    F: Fn(&ComponentId, &LifecyclePhase) -> ComponentResult<()> + Send + Sync,
{
    pub fn new(name: String, callback: F) -> Self {
        Self { name, callback }
    }
}

#[async_trait]
impl<F> LifecycleListener for SimpleLifecycleListener<F>
where
    F: Fn(&ComponentId, &LifecyclePhase) -> ComponentResult<()> + Send + Sync,
{
    async fn on_lifecycle_change(
        &self,
        component_id: &ComponentId,
        phase: &LifecyclePhase,
    ) -> ComponentResult<()> {
        (self.callback)(component_id, phase)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

/// 组件生命周期trait - 定义组件生命周期行为
#[async_trait]
pub trait ComponentLifecycle {
    /// 初始化组件
    async fn initialize(&mut self) -> ComponentResult<()>;
    
    /// 启动组件
    async fn start(&mut self) -> ComponentResult<()>;
    
    /// 暂停组件
    async fn pause(&mut self) -> ComponentResult<()>;
    
    /// 恢复组件
    async fn resume(&mut self) -> ComponentResult<()>;
    
    /// 停止组件
    async fn stop(&mut self) -> ComponentResult<()>;
    
    /// 销毁组件
    async fn destroy(&mut self) -> ComponentResult<()>;
    
    /// 获取当前生命周期阶段
    fn current_phase(&self) -> LifecyclePhase;
    
    /// 检查是否可以转换到指定阶段
    fn can_transition_to(&self, phase: &LifecyclePhase) -> bool;
}