/// GUI服务管理器实现

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::{ServiceId, ServiceState, ServiceConfig};
use super::message::{MessageRouter, ServiceMessage};
use super::health::{ServiceHealth, ServiceStats};
use super::GUIService;
use super::super::component::{ComponentError, ComponentResult};

/// GUI服务管理器配置
#[derive(Debug, Clone)]
pub struct GUIServiceManagerConfig {
    /// 最大并发启动数
    pub max_concurrent_starts: usize,
    /// 健康检查间隔
    pub health_check_interval: std::time::Duration,
    /// 消息超时时间
    pub message_timeout: std::time::Duration,
    /// 是否启用统计收集
    pub enable_stats: bool,
}

impl Default for GUIServiceManagerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_starts: 5,
            health_check_interval: std::time::Duration::from_secs(30),
            message_timeout: std::time::Duration::from_secs(10),
            enable_stats: true,
        }
    }
}

/// GUI服务管理器
/// 
/// 负责管理所有GUI服务的生命周期和消息路由
pub struct GUIServiceManager {
    /// 已注册的服务
    services: Arc<RwLock<HashMap<ServiceId, Box<dyn GUIService>>>>,
    /// 服务状态缓存
    service_states: Arc<RwLock<HashMap<ServiceId, ServiceState>>>,
    /// 消息路由器
    message_router: Arc<MessageRouter>,
    /// 服务依赖图
    dependency_graph: Arc<RwLock<HashMap<ServiceId, Vec<ServiceId>>>>,
    /// 运行状态
    is_running: Arc<tokio::sync::RwLock<bool>>,
    /// 管理器配置
    config: GUIServiceManagerConfig,
}

impl GUIServiceManager {
    /// 创建新的服务管理器
    pub fn new(config: GUIServiceManagerConfig) -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            service_states: Arc::new(RwLock::new(HashMap::new())),
            message_router: Arc::new(MessageRouter::new()),
            dependency_graph: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(tokio::sync::RwLock::new(false)),
            config,
        }
    }
    
    /// 注册服务
    pub async fn register_service(
        &self,
        service: Box<dyn GUIService>,
    ) -> ComponentResult<()> {
        let service_id = service.id().clone();
        let config = service.config().clone();
        
        // 检查服务是否已存在
        {
            let services = self.services.read().await;
            if services.contains_key(&service_id) {
                return Err(ComponentError::ConfigError(
                    format!("服务 {} 已存在", service_id)
                ));
            }
        }
        
        // 注册服务
        {
            let mut services = self.services.write().await;
            let mut states = self.service_states.write().await;
            let mut deps = self.dependency_graph.write().await;
            
            services.insert(service_id.clone(), service);
            states.insert(service_id.clone(), ServiceState::Stopped);
            deps.insert(service_id.clone(), config.dependencies.clone());
        }
        
        log::info!("服务 {} ({}) 注册成功", service_id, config.name);
        Ok(())
    }
    
    /// 启动服务
    pub async fn start_service(&self, service_id: &ServiceId) -> ComponentResult<()> {
        // 检查依赖
        self.check_dependencies(service_id).await?;
        
        // 启动服务
        {
            let mut services = self.services.write().await;
            let mut states = self.service_states.write().await;
            
            if let Some(service) = services.get_mut(service_id) {
                states.insert(service_id.clone(), ServiceState::Starting);
                
                match service.start().await {
                    Ok(()) => {
                        states.insert(service_id.clone(), ServiceState::Running);
                        log::info!("服务 {} 启动成功", service_id);
                    }
                    Err(e) => {
                        states.insert(service_id.clone(), ServiceState::Error(e.to_string()));
                        return Err(e);
                    }
                }
            } else {
                return Err(ComponentError::ConfigError(
                    format!("服务 {} 未找到", service_id)
                ));
            }
        }
        
        Ok(())
    }
    
    /// 停止服务
    pub async fn stop_service(&self, service_id: &ServiceId) -> ComponentResult<()> {
        let mut services = self.services.write().await;
        let mut states = self.service_states.write().await;
        
        if let Some(service) = services.get_mut(service_id) {
            states.insert(service_id.clone(), ServiceState::Stopping);
            
            match service.stop().await {
                Ok(()) => {
                    states.insert(service_id.clone(), ServiceState::Stopped);
                    log::info!("服务 {} 停止成功", service_id);
                }
                Err(e) => {
                    states.insert(service_id.clone(), ServiceState::Error(e.to_string()));
                    return Err(e);
                }
            }
        } else {
            return Err(ComponentError::ConfigError(
                format!("服务 {} 未找到", service_id)
            ));
        }
        
        Ok(())
    }
    
    /// 启动所有服务
    pub async fn start_all(&self) -> ComponentResult<()> {
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        
        // 根据依赖关系排序启动
        let startup_order = self.calculate_startup_order().await?;
        
        for service_id in startup_order {
            if let Err(e) = self.start_service(&service_id).await {
                log::error!("启动服务 {} 失败: {}", service_id, e);
                // 继续启动其他服务，不因单个服务失败而停止整个系统
            }
        }
        
        log::info!("所有服务启动完成");
        Ok(())
    }
    
    /// 停止所有服务
    pub async fn stop_all(&self) -> ComponentResult<()> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        let services_list: Vec<ServiceId> = {
            let services = self.services.read().await;
            services.keys().cloned().collect()
        };
        
        // 反向停止服务
        for service_id in services_list.into_iter().rev() {
            if let Err(e) = self.stop_service(&service_id).await {
                log::warn!("停止服务 {} 失败: {}", service_id, e);
            }
        }
        
        log::info!("所有服务停止完成");
        Ok(())
    }
    
    /// 发送消息到服务
    pub async fn send_message(&self, message: ServiceMessage) -> ComponentResult<()> {
        self.message_router.route_message(message).await
    }
    
    /// 获取服务状态
    pub async fn get_service_state(&self, service_id: &ServiceId) -> Option<ServiceState> {
        let states = self.service_states.read().await;
        states.get(service_id).cloned()
    }
    
    /// 获取所有服务状态
    pub async fn get_all_service_states(&self) -> HashMap<ServiceId, ServiceState> {
        let states = self.service_states.read().await;
        states.clone()
    }
    
    /// 健康检查
    pub async fn health_check(&self) -> ComponentResult<HashMap<ServiceId, ServiceHealth>> {
        let mut health_results = HashMap::new();
        let services = self.services.read().await;
        
        for (service_id, service) in services.iter() {
            match service.health_check().await {
                Ok(health) => {
                    health_results.insert(service_id.clone(), health);
                }
                Err(e) => {
                    log::warn!("服务 {} 健康检查失败: {}", service_id, e);
                }
            }
        }
        
        Ok(health_results)
    }
    
    /// 检查服务依赖
    async fn check_dependencies(&self, service_id: &ServiceId) -> ComponentResult<()> {
        let deps = self.dependency_graph.read().await;
        let states = self.service_states.read().await;
        
        if let Some(dependencies) = deps.get(service_id) {
            for dep_id in dependencies {
                if let Some(dep_state) = states.get(dep_id) {
                    if *dep_state != ServiceState::Running {
                        return Err(ComponentError::ConfigError(
                            format!("依赖服务 {} 未运行", dep_id)
                        ));
                    }
                } else {
                    return Err(ComponentError::ConfigError(
                        format!("依赖服务 {} 未找到", dep_id)
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    /// 计算服务启动顺序
    async fn calculate_startup_order(&self) -> ComponentResult<Vec<ServiceId>> {
        let deps = self.dependency_graph.read().await;
        let mut order = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();
        
        for service_id in deps.keys() {
            if !visited.contains(service_id) {
                self.topological_sort(
                    service_id,
                    &deps,
                    &mut visited,
                    &mut visiting,
                    &mut order,
                )?;
            }
        }
        
        Ok(order)
    }
    
    /// 拓扑排序 (深度优先)
    fn topological_sort(
        &self,
        service_id: &ServiceId,
        deps: &HashMap<ServiceId, Vec<ServiceId>>,
        visited: &mut std::collections::HashSet<ServiceId>,
        visiting: &mut std::collections::HashSet<ServiceId>,
        order: &mut Vec<ServiceId>,
    ) -> ComponentResult<()> {
        if visiting.contains(service_id) {
            return Err(ComponentError::ConfigError(
                format!("检测到服务依赖环: {}", service_id)
            ));
        }
        
        if visited.contains(service_id) {
            return Ok(());
        }
        
        visiting.insert(service_id.clone());
        
        if let Some(dependencies) = deps.get(service_id) {
            for dep_id in dependencies {
                self.topological_sort(dep_id, deps, visited, visiting, order)?;
            }
        }
        
        visiting.remove(service_id);
        visited.insert(service_id.clone());
        order.push(service_id.clone());
        
        Ok(())
    }
}