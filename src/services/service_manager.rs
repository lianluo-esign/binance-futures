use super::{Service, ServiceError, ServiceHealth, ServiceStats};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

/// 服务管理器 - 负责管理所有服务的生命周期
pub struct ServiceManager {
    /// 服务容器
    services: Arc<RwLock<HashMap<String, Box<dyn Service>>>>,
    /// 服务依赖关系
    dependencies: HashMap<String, Vec<String>>,
    /// 服务启动顺序
    startup_order: Vec<String>,
    /// 运行状态
    is_running: Arc<std::sync::atomic::AtomicBool>,
    /// 监控任务句柄
    monitor_handle: Option<tokio::task::JoinHandle<()>>,
    /// 服务通信通道
    service_channels: HashMap<String, ServiceChannel>,
}

/// 服务容器 - 提供服务注册和查找功能
pub struct ServiceContainer {
    services: HashMap<String, Box<dyn Service>>,
    service_factories: HashMap<String, Box<dyn ServiceFactory>>,
}

/// 服务通道 - 用于服务间通信
#[derive(Clone)]
pub struct ServiceChannel {
    sender: mpsc::UnboundedSender<ServiceMessage>,
    receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<ServiceMessage>>>>,
}

/// 服务间消息
#[derive(Debug, Clone)]
pub enum ServiceMessage {
    /// 数据更新消息
    DataUpdate {
        source: String,
        data: serde_json::Value,
        timestamp: u64,
    },
    /// 渲染请求消息
    RenderRequest {
        commands: Vec<crate::services::rendering_service::RenderCommand>,
        priority: MessagePriority,
    },
    /// 配置更新消息
    ConfigUpdate {
        service_name: String,
        config: serde_json::Value,
    },
    /// 性能指标消息
    PerformanceMetrics {
        metrics: crate::core::PerformanceMetrics,
    },
    /// 控制消息
    Control(ControlMessage),
}

/// 消息优先级
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Critical,
}

/// 控制消息
#[derive(Debug, Clone)]
pub enum ControlMessage {
    Start,
    Stop,
    Restart,
    Pause,
    Resume,
    HealthCheck,
}

/// 服务工厂trait
pub trait ServiceFactory: Send + Sync {
    fn create(&self) -> Result<Box<dyn Service>, ServiceError>;
    fn service_name(&self) -> &str;
}

/// 服务启动结果
#[derive(Debug)]
pub struct ServiceStartResult {
    pub service_name: String,
    pub success: bool,
    pub error: Option<ServiceError>,
    pub start_time: Duration,
}

impl ServiceManager {
    /// 创建新的服务管理器
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            dependencies: HashMap::new(),
            startup_order: Vec::new(),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            monitor_handle: None,
            service_channels: HashMap::new(),
        }
    }

    /// 注册服务
    pub fn register_service(
        &mut self,
        name: String,
        service: Box<dyn Service>,
        dependencies: Vec<String>,
    ) -> Result<(), ServiceError> {
        // 检查循环依赖
        if self.has_circular_dependency(&name, &dependencies) {
            return Err(ServiceError::DependencyError(
                format!("检测到循环依赖: {}", name)
            ));
        }

        // 注册服务
        self.services.write().unwrap().insert(name.clone(), service);
        self.dependencies.insert(name.clone(), dependencies);

        // 创建服务通信通道
        let (sender, receiver) = mpsc::unbounded_channel();
        self.service_channels.insert(name.clone(), ServiceChannel {
            sender,
            receiver: Arc::new(RwLock::new(Some(receiver))),
        });

        // 重新计算启动顺序
        self.calculate_startup_order()?;

        log::info!("服务已注册: {}", name);
        Ok(())
    }

    /// 启动所有服务
    pub async fn start_all(&mut self) -> Result<Vec<ServiceStartResult>, ServiceError> {
        if self.is_running.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(ServiceError::AlreadyRunning);
        }

        let mut results = Vec::new();

        // 按依赖顺序启动服务
        for service_name in &self.startup_order.clone() {
            let result = self.start_service(service_name).await;
            results.push(result);
        }

        // 启动监控任务
        self.start_monitoring().await;

        self.is_running.store(true, std::sync::atomic::Ordering::Relaxed);
        log::info!("所有服务已启动");

        Ok(results)
    }

    /// 停止所有服务
    pub async fn stop_all(&mut self) -> Result<(), ServiceError> {
        if !self.is_running.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        // 停止监控任务
        if let Some(handle) = self.monitor_handle.take() {
            handle.abort();
        }

        // 按相反顺序停止服务
        for service_name in self.startup_order.iter().rev() {
            if let Err(e) = self.stop_service(service_name).await {
                log::warn!("停止服务 {} 失败: {}", service_name, e);
            }
        }

        self.is_running.store(false, std::sync::atomic::Ordering::Relaxed);
        log::info!("所有服务已停止");

        Ok(())
    }

    /// 启动单个服务
    pub async fn start_service(&mut self, service_name: &str) -> ServiceStartResult {
        let start_time = Instant::now();

        // 检查依赖是否已启动
        if let Some(deps) = self.dependencies.get(service_name) {
            for dep in deps {
                let service_health = self.get_service_health(dep).await;
                if service_health != ServiceHealth::Healthy {
                    return ServiceStartResult {
                        service_name: service_name.to_string(),
                        success: false,
                        error: Some(ServiceError::DependencyError(
                            format!("依赖服务 {} 未就绪", dep)
                        )),
                        start_time: start_time.elapsed(),
                    };
                }
            }
        }

        // 启动服务
        let mut services = self.services.write().unwrap();
        if let Some(service) = services.get_mut(service_name) {
            match service.start() {
                Ok(()) => {
                    log::info!("服务 {} 启动成功", service_name);
                    ServiceStartResult {
                        service_name: service_name.to_string(),
                        success: true,
                        error: None,
                        start_time: start_time.elapsed(),
                    }
                }
                Err(e) => {
                    log::error!("服务 {} 启动失败: {}", service_name, e);
                    ServiceStartResult {
                        service_name: service_name.to_string(),
                        success: false,
                        error: Some(e),
                        start_time: start_time.elapsed(),
                    }
                }
            }
        } else {
            ServiceStartResult {
                service_name: service_name.to_string(),
                success: false,
                error: Some(ServiceError::ConfigurationError(
                    format!("服务 {} 未找到", service_name)
                )),
                start_time: start_time.elapsed(),
            }
        }
    }

    /// 停止单个服务
    pub async fn stop_service(&mut self, service_name: &str) -> Result<(), ServiceError> {
        let mut services = self.services.write().unwrap();
        if let Some(service) = services.get_mut(service_name) {
            service.stop()?;
            log::info!("服务 {} 已停止", service_name);
            Ok(())
        } else {
            Err(ServiceError::ConfigurationError(
                format!("服务 {} 未找到", service_name)
            ))
        }
    }

    /// 重启服务
    pub async fn restart_service(&mut self, service_name: &str) -> Result<(), ServiceError> {
        self.stop_service(service_name).await?;
        tokio::time::sleep(Duration::from_millis(100)).await; // 短暂等待
        let result = self.start_service(service_name).await;
        if result.success {
            Ok(())
        } else {
            result.error.ok_or_else(|| ServiceError::InternalError("重启失败".to_string()))
        }
    }

    /// 获取服务健康状态
    pub async fn get_service_health(&self, service_name: &str) -> ServiceHealth {
        let services = self.services.read().unwrap();
        if let Some(service) = services.get(service_name) {
            service.health_check()
        } else {
            ServiceHealth::Unknown
        }
    }

    /// 获取所有服务状态
    pub async fn get_all_services_health(&self) -> HashMap<String, ServiceHealth> {
        let mut health_map = HashMap::new();
        let services = self.services.read().unwrap();
        
        for (name, service) in services.iter() {
            health_map.insert(name.clone(), service.health_check());
        }
        
        health_map
    }

    /// 获取服务统计信息
    pub async fn get_service_stats(&self, service_name: &str) -> Option<ServiceStats> {
        let services = self.services.read().unwrap();
        services.get(service_name).map(|service| service.stats())
    }

    /// 获取所有服务统计信息
    pub async fn get_all_services_stats(&self) -> HashMap<String, ServiceStats> {
        let mut stats_map = HashMap::new();
        let services = self.services.read().unwrap();
        
        for (name, service) in services.iter() {
            stats_map.insert(name.clone(), service.stats());
        }
        
        stats_map
    }

    /// 发送消息到服务
    pub async fn send_message(&self, target_service: &str, message: ServiceMessage) -> Result<(), ServiceError> {
        if let Some(channel) = self.service_channels.get(target_service) {
            channel.sender.send(message)
                .map_err(|e| ServiceError::InternalError(format!("发送消息失败: {}", e)))?;
            Ok(())
        } else {
            Err(ServiceError::ConfigurationError(
                format!("服务 {} 的通信通道未找到", target_service)
            ))
        }
    }

    /// 广播消息到所有服务
    pub async fn broadcast_message(&self, message: ServiceMessage) -> Result<(), ServiceError> {
        for (service_name, channel) in &self.service_channels {
            if let Err(e) = channel.sender.send(message.clone()) {
                log::warn!("向服务 {} 发送广播消息失败: {}", service_name, e);
            }
        }
        Ok(())
    }

    /// 获取服务通信通道
    pub fn get_service_channel(&self, service_name: &str) -> Option<ServiceChannel> {
        self.service_channels.get(service_name).cloned()
    }

    /// 检查循环依赖
    fn has_circular_dependency(&self, service_name: &str, dependencies: &[String]) -> bool {
        for dep in dependencies {
            if self.check_dependency_chain(dep, service_name, &mut Vec::new()) {
                return true;
            }
        }
        false
    }

    /// 检查依赖链
    fn check_dependency_chain(&self, current: &str, target: &str, visited: &mut Vec<String>) -> bool {
        if current == target {
            return true;
        }

        if visited.contains(&current.to_string()) {
            return false;
        }

        visited.push(current.to_string());

        if let Some(deps) = self.dependencies.get(current) {
            for dep in deps {
                if self.check_dependency_chain(dep, target, visited) {
                    return true;
                }
            }
        }

        visited.pop();
        false
    }

    /// 计算启动顺序
    fn calculate_startup_order(&mut self) -> Result<(), ServiceError> {
        let mut order = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();

        for service_name in self.services.read().unwrap().keys() {
            if !visited.contains(service_name) {
                self.visit_service(service_name, &mut order, &mut visited, &mut visiting)?;
            }
        }

        self.startup_order = order;
        Ok(())
    }

    /// 访问服务 (拓扑排序)
    fn visit_service(
        &self,
        service_name: &str,
        order: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
    ) -> Result<(), ServiceError> {
        if visiting.contains(service_name) {
            return Err(ServiceError::DependencyError(
                format!("检测到循环依赖: {}", service_name)
            ));
        }

        if visited.contains(service_name) {
            return Ok(());
        }

        visiting.insert(service_name.to_string());

        if let Some(dependencies) = self.dependencies.get(service_name) {
            for dep in dependencies {
                self.visit_service(dep, order, visited, visiting)?;
            }
        }

        visiting.remove(service_name);
        visited.insert(service_name.to_string());
        order.push(service_name.to_string());

        Ok(())
    }

    /// 启动监控任务
    async fn start_monitoring(&mut self) {
        let services = self.services.clone();
        let is_running = self.is_running.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                if !is_running.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }

                // 监控所有服务健康状态
                let service_guard = services.read().unwrap();
                for (name, service) in service_guard.iter() {
                    let health = service.health_check();
                    match health {
                        ServiceHealth::Unhealthy(reason) => {
                            log::error!("服务 {} 不健康: {}", name, reason);
                        }
                        ServiceHealth::Warning(reason) => {
                            log::warn!("服务 {} 警告: {}", name, reason);
                        }
                        ServiceHealth::Healthy => {
                            log::debug!("服务 {} 健康", name);
                        }
                        ServiceHealth::Unknown => {
                            log::debug!("服务 {} 状态未知", name);
                        }
                    }
                }
            }
        });

        self.monitor_handle = Some(handle);
    }
}

impl ServiceContainer {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
            service_factories: HashMap::new(),
        }
    }

    /// 注册服务工厂
    pub fn register_factory(&mut self, factory: Box<dyn ServiceFactory>) {
        let name = factory.service_name().to_string();
        self.service_factories.insert(name, factory);
    }

    /// 创建服务
    pub fn create_service(&mut self, service_name: &str) -> Result<Box<dyn Service>, ServiceError> {
        if let Some(factory) = self.service_factories.get(service_name) {
            factory.create()
        } else {
            Err(ServiceError::ConfigurationError(
                format!("服务工厂 {} 未找到", service_name)
            ))
        }
    }

    /// 获取服务
    pub fn get_service(&self, service_name: &str) -> Option<&dyn Service> {
        self.services.get(service_name).map(|s| s.as_ref())
    }

    /// 获取可变服务引用
    pub fn get_service_mut(&mut self, service_name: &str) -> Option<&mut dyn Service> {
        self.services.get_mut(service_name).map(|s| s.as_mut())
    }
}

impl ServiceChannel {
    /// 发送消息
    pub async fn send(&self, message: ServiceMessage) -> Result<(), ServiceError> {
        self.sender.send(message)
            .map_err(|e| ServiceError::InternalError(format!("发送消息失败: {}", e)))
    }

    /// 接收消息
    pub async fn receive(&self) -> Option<ServiceMessage> {
        let mut receiver_guard = self.receiver.write().unwrap();
        if let Some(receiver) = receiver_guard.as_mut() {
            receiver.recv().await
        } else {
            None
        }
    }

    /// 尝试接收消息 (非阻塞)
    pub fn try_receive(&self) -> Result<ServiceMessage, ServiceError> {
        let mut receiver_guard = self.receiver.write().unwrap();
        if let Some(receiver) = receiver_guard.as_mut() {
            receiver.try_recv()
                .map_err(|e| ServiceError::InternalError(format!("接收消息失败: {}", e)))
        } else {
            Err(ServiceError::InternalError("接收器不可用".to_string()))
        }
    }
}

/// 服务运行时统计
#[derive(Debug, Clone)]
pub struct ServiceRuntimeStats {
    pub total_services: usize,
    pub running_services: usize,
    pub healthy_services: usize,
    pub warning_services: usize,
    pub unhealthy_services: usize,
    pub total_uptime: Duration,
    pub total_requests: u64,
    pub total_errors: u64,
}

impl ServiceManager {
    /// 获取运行时统计
    pub async fn get_runtime_stats(&self) -> ServiceRuntimeStats {
        let services = self.services.read().unwrap();
        let mut stats = ServiceRuntimeStats {
            total_services: services.len(),
            running_services: 0,
            healthy_services: 0,
            warning_services: 0,
            unhealthy_services: 0,
            total_uptime: Duration::from_secs(0),
            total_requests: 0,
            total_errors: 0,
        };

        for (_name, service) in services.iter() {
            let service_stats = service.stats();
            let health = service.health_check();

            if service_stats.is_running {
                stats.running_services += 1;
            }

            match health {
                ServiceHealth::Healthy => stats.healthy_services += 1,
                ServiceHealth::Warning(_) => stats.warning_services += 1,
                ServiceHealth::Unhealthy(_) => stats.unhealthy_services += 1,
                ServiceHealth::Unknown => {}
            }

            if let Some(start_time) = service_stats.start_time {
                stats.total_uptime += start_time.elapsed();
            }

            stats.total_requests += service_stats.requests_processed;
            stats.total_errors += service_stats.error_count;
        }

        stats
    }
}