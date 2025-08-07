// Provider管理器 - 统一数据源管理
//
// 本文件实现了Provider的统一管理系统，负责：
// - Provider注册和生命周期管理
// - 运行时Provider切换
// - 事件路由和分发
// - 状态监控和健康检查
//
// 设计原则：
// 1. 单一责任：专注于Provider管理，不处理具体业务逻辑
// 2. 线程安全：支持多线程环境下的安全操作
// 3. 错误隔离：单个Provider错误不影响整个系统
// 4. 性能优化：最小化运行时开销

use super::{
    DataProvider, ProviderType, ProviderStatus, ProviderFactory,
    error::{ProviderError, ProviderResult},
};
use crate::events::{EventType, Event, LockFreeEventDispatcher};

use std::collections::HashMap;
use std::sync::{Arc, RwLock, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use std::thread;
use serde::{Serialize, Deserialize};

/// Provider管理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderManagerConfig {
    /// 默认Provider ID
    pub default_provider_id: String,
    
    /// 自动切换配置
    pub auto_switch_config: AutoSwitchConfig,
    
    /// 健康检查间隔（毫秒）
    pub health_check_interval_ms: u64,
    
    /// 事件缓冲区大小
    pub event_buffer_size: usize,
    
    /// 是否启用故障转移
    pub failover_enabled: bool,
    
    /// Provider切换超时（毫秒）
    pub switch_timeout_ms: u64,
}

impl Default for ProviderManagerConfig {
    fn default() -> Self {
        Self {
            default_provider_id: "default".to_string(),
            auto_switch_config: AutoSwitchConfig::default(),
            health_check_interval_ms: 5000, // 5秒
            event_buffer_size: 10000,
            failover_enabled: true,
            switch_timeout_ms: 30000, // 30秒
        }
    }
}

/// 自动切换配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSwitchConfig {
    /// 是否启用自动切换
    pub enabled: bool,
    
    /// 切换策略
    pub strategy: SwitchStrategy,
    
    /// 健康检查失败阈值
    pub failure_threshold: u32,
    
    /// 恢复检查间隔（毫秒）
    pub recovery_check_interval_ms: u64,
}

impl Default for AutoSwitchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: SwitchStrategy::FailoverOnly,
            failure_threshold: 3,
            recovery_check_interval_ms: 30000, // 30秒
        }
    }
}

/// Provider切换策略
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SwitchStrategy {
    /// 仅故障转移：只在当前Provider失败时切换
    FailoverOnly,
    
    /// 负载均衡：根据负载情况自动切换
    LoadBalance,
    
    /// 质量优先：优先选择质量最好的Provider
    QualityFirst,
    
    /// 手动控制：仅响应手动切换请求
    Manual,
}

/// Provider注册信息
#[derive(Clone)]
struct ProviderRegistration {
    /// Provider实例
    provider: Arc<RwLock<dyn DataProvider<Error = ProviderError> + Send + Sync>>,
    
    /// Provider工厂（用于重建实例）
    factory: Option<Arc<dyn ProviderFactory<Provider = Box<dyn DataProvider<Error = ProviderError> + Send + Sync>, Config = serde_json::Value> + Send + Sync>>,
    
    /// Provider元数据
    metadata: ProviderMetadata,
    
    /// 注册时间
    registered_at: Instant,
    
    /// 是否启用
    enabled: bool,
}

impl std::fmt::Debug for ProviderRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderRegistration")
            .field("metadata", &self.metadata)
            .field("registered_at", &self.registered_at)
            .field("enabled", &self.enabled)
            .field("has_factory", &self.factory.is_some())
            .finish_non_exhaustive()
    }
}

/// Provider元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetadata {
    /// Provider ID
    pub id: String,
    
    /// Provider名称
    pub name: String,
    
    /// Provider描述
    pub description: String,
    
    /// Provider类型
    pub provider_type: ProviderType,
    
    /// 优先级（数值越大优先级越高）
    pub priority: i32,
    
    /// 是否为备用Provider
    pub is_fallback: bool,
    
    /// 自定义标签
    pub tags: HashMap<String, String>,
}

/// Provider管理器状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderManagerStatus {
    /// 当前活跃Provider ID
    pub active_provider_id: Option<String>,
    
    /// 已注册的Provider数量
    pub registered_providers: usize,
    
    /// 健康的Provider数量
    pub healthy_providers: usize,
    
    /// 总处理事件数
    pub total_events_processed: u64,
    
    /// 最后事件时间
    pub last_event_time: Option<u64>,
    
    /// 切换次数
    pub switch_count: u32,
    
    /// 管理器启动时间
    pub start_time: u64,
    
    /// 是否正在运行
    pub is_running: bool,
}

/// Provider管理器实现
pub struct ProviderManager {
    /// 管理器配置
    config: ProviderManagerConfig,
    
    /// 注册的Provider
    providers: Arc<RwLock<HashMap<String, ProviderRegistration>>>,
    
    /// 当前活跃的Provider ID
    active_provider_id: Arc<RwLock<Option<String>>>,
    
    /// 事件分发器
    event_dispatcher: Arc<LockFreeEventDispatcher>,
    
    /// 管理器状态
    status: Arc<RwLock<ProviderManagerStatus>>,
    
    /// 运行标志
    is_running: Arc<AtomicBool>,
    
    /// 健康检查线程句柄
    health_check_handle: Option<thread::JoinHandle<()>>,
    
    /// 统计信息
    total_events_processed: Arc<std::sync::atomic::AtomicU64>,
}

impl ProviderManager {
    /// 创建新的Provider管理器
    pub fn new(config: ProviderManagerConfig) -> Self {
        let event_dispatcher = Arc::new(LockFreeEventDispatcher::new(config.event_buffer_size));
        
        let status = ProviderManagerStatus {
            active_provider_id: None,
            registered_providers: 0,
            healthy_providers: 0,
            total_events_processed: 0,
            last_event_time: None,
            switch_count: 0,
            start_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            is_running: false,
        };
        
        Self {
            config,
            providers: Arc::new(RwLock::new(HashMap::new())),
            active_provider_id: Arc::new(RwLock::new(None)),
            event_dispatcher,
            status: Arc::new(RwLock::new(status)),
            is_running: Arc::new(AtomicBool::new(false)),
            health_check_handle: None,
            total_events_processed: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// 注册Provider
    pub fn register_provider<P>(
        &mut self,
        provider: P,
        metadata: ProviderMetadata,
    ) -> ProviderResult<()>
    where
        P: DataProvider<Error = ProviderError> + Send + Sync + 'static,
    {
        let provider_id = metadata.id.clone();
        
        log::info!("注册Provider: {} ({})", metadata.name, provider_id);
        
        // 验证Provider ID唯一性
        {
            let providers_guard = self.providers.read()
                .map_err(|_| ProviderError::internal("获取Provider锁失败", "ProviderManager"))?;
            
            if providers_guard.contains_key(&provider_id) {
                return Err(ProviderError::configuration_field(
                    "Provider ID已存在",
                    "id",
                    Some("唯一标识符".to_string()),
                    Some(provider_id),
                ));
            }
        }
        
        // 创建注册信息
        let registration = ProviderRegistration {
            provider: Arc::new(RwLock::new(provider)),
            factory: None,
            metadata,
            registered_at: Instant::now(),
            enabled: true,
        };
        
        // 添加到Provider集合
        {
            let mut providers_guard = self.providers.write()
                .map_err(|_| ProviderError::internal("获取Provider写锁失败", "ProviderManager"))?;
            
            providers_guard.insert(provider_id.clone(), registration);
        }
        
        // 更新状态
        {
            let mut status_guard = self.status.write()
                .map_err(|_| ProviderError::internal("获取状态写锁失败", "ProviderManager"))?;
            
            status_guard.registered_providers += 1;
        }
        
        // 如果这是第一个Provider或默认Provider，设置为活跃
        if self.get_active_provider_id().is_none() || provider_id == self.config.default_provider_id {
            self.switch_to_provider(&provider_id)?;
        }
        
        log::info!("Provider注册成功: {}", provider_id);
        Ok(())
    }

    /// 注销Provider
    pub fn unregister_provider(&mut self, provider_id: &str) -> ProviderResult<()> {
        log::info!("注销Provider: {}", provider_id);
        
        // 检查是否为当前活跃Provider
        if let Some(active_id) = self.get_active_provider_id() {
            if active_id == provider_id {
                // 尝试切换到其他Provider
                if let Err(e) = self.switch_to_fallback_provider() {
                    log::warn!("切换到备用Provider失败: {}", e);
                    // 清空活跃Provider
                    *self.active_provider_id.write()
                        .map_err(|_| ProviderError::internal("获取活跃Provider写锁失败", "ProviderManager"))? = None;
                }
            }
        }
        
        // 从注册表中移除
        {
            let mut providers_guard = self.providers.write()
                .map_err(|_| ProviderError::internal("获取Provider写锁失败", "ProviderManager"))?;
            
            if let Some(registration) = providers_guard.remove(provider_id) {
                // 停止Provider
                if let Ok(mut provider_guard) = registration.provider.write() {
                    let _ = provider_guard.stop();
                }
            } else {
                return Err(ProviderError::configuration(
                    format!("Provider不存在: {}", provider_id)
                ));
            }
        }
        
        // 更新状态
        {
            let mut status_guard = self.status.write()
                .map_err(|_| ProviderError::internal("获取状态写锁失败", "ProviderManager"))?;
            
            status_guard.registered_providers = status_guard.registered_providers.saturating_sub(1);
        }
        
        log::info!("Provider注销成功: {}", provider_id);
        Ok(())
    }

    /// 切换到指定Provider
    pub fn switch_to_provider(&self, provider_id: &str) -> ProviderResult<()> {
        log::info!("切换到Provider: {}", provider_id);
        
        // 停止当前活跃Provider
        if let Some(current_id) = self.get_active_provider_id() {
            if current_id != provider_id {
                self.stop_provider(&current_id)?;
            }
        }
        
        // 启动新的Provider
        {
            let providers_guard = self.providers.read()
                .map_err(|_| ProviderError::internal("获取Provider锁失败", "ProviderManager"))?;
            
            let registration = providers_guard.get(provider_id)
                .ok_or_else(|| ProviderError::configuration(
                    format!("Provider不存在: {}", provider_id)
                ))?;
            
            if !registration.enabled {
                return Err(ProviderError::state(
                    format!("Provider已禁用: {}", provider_id),
                    "disabled",
                    "enabled",
                    "switch"
                ));
            }
            
            // 初始化并启动Provider
            {
                let mut provider_guard = registration.provider.write()
                    .map_err(|_| ProviderError::internal("获取Provider写锁失败", "ProviderManager"))?;
                
                provider_guard.initialize()
                    .map_err(|e| ProviderError::initialization_with_source(
                        format!("初始化Provider失败: {}", provider_id),
                        Box::new(e)
                    ))?;
                
                provider_guard.start()
                    .map_err(|e| ProviderError::connection(
                        format!("启动Provider失败: {}", provider_id),
                        None,
                        true
                    ))?;
            }
        }
        
        // 更新活跃Provider
        *self.active_provider_id.write()
            .map_err(|_| ProviderError::internal("获取活跃Provider写锁失败", "ProviderManager"))? = Some(provider_id.to_string());
        
        // 更新状态
        {
            let mut status_guard = self.status.write()
                .map_err(|_| ProviderError::internal("获取状态写锁失败", "ProviderManager"))?;
            
            status_guard.active_provider_id = Some(provider_id.to_string());
            status_guard.switch_count += 1;
        }
        
        log::info!("Provider切换成功: {}", provider_id);
        Ok(())
    }

    /// 启动管理器
    pub fn start(&mut self) -> ProviderResult<()> {
        if self.is_running.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        log::info!("启动Provider管理器");
        
        // 启动健康检查线程
        if self.config.auto_switch_config.enabled {
            self.start_health_monitor()?;
        }
        
        // 标记为运行状态
        self.is_running.store(true, Ordering::Relaxed);
        
        // 更新状态
        {
            let mut status_guard = self.status.write()
                .map_err(|_| ProviderError::internal("获取状态写锁失败", "ProviderManager"))?;
            status_guard.is_running = true;
        }
        
        log::info!("Provider管理器启动完成");
        Ok(())
    }

    /// 停止管理器
    pub fn stop(&mut self) -> ProviderResult<()> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        log::info!("停止Provider管理器");
        
        // 停止运行标志
        self.is_running.store(false, Ordering::Relaxed);
        
        // 停止健康检查线程
        if let Some(handle) = self.health_check_handle.take() {
            let _ = handle.join();
        }
        
        // 停止所有Provider
        {
            let providers_guard = self.providers.read()
                .map_err(|_| ProviderError::internal("获取Provider锁失败", "ProviderManager"))?;
            
            for (provider_id, registration) in providers_guard.iter() {
                if let Ok(mut provider_guard) = registration.provider.write() {
                    if let Err(e) = provider_guard.stop() {
                        log::warn!("停止Provider {}失败: {}", provider_id, e);
                    }
                }
            }
        }
        
        // 更新状态
        {
            let mut status_guard = self.status.write()
                .map_err(|_| ProviderError::internal("获取状态写锁失败", "ProviderManager"))?;
            status_guard.is_running = false;
        }
        
        log::info!("Provider管理器已停止");
        Ok(())
    }

    /// 处理事件（主事件循环调用）
    pub fn process_events(&self) -> ProviderResult<Vec<EventType>> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Ok(vec![]);
        }
        
        // 获取当前活跃Provider
        let provider_id = match self.get_active_provider_id() {
            Some(id) => id,
            None => return Ok(vec![]), // 没有活跃Provider
        };
        
        // 读取事件
        let events = {
            let providers_guard = self.providers.read()
                .map_err(|_| ProviderError::internal("获取Provider锁失败", "ProviderManager"))?;
            
            let registration = providers_guard.get(&provider_id)
                .ok_or_else(|| ProviderError::internal(
                    format!("活跃Provider不存在: {}", provider_id),
                    "ProviderManager"
                ))?;
            
            let mut provider_guard = registration.provider.write()
                .map_err(|_| ProviderError::internal("获取Provider写锁失败", "ProviderManager"))?;
            
            provider_guard.read_events()
                .map_err(|e| {
                    log::warn!("Provider {}读取事件失败: {}", provider_id, e);
                    e
                })
        };
        
        match events {
            Ok(event_list) => {
                // 更新统计信息
                let event_count = event_list.len() as u64;
                self.total_events_processed.fetch_add(event_count, Ordering::Relaxed);
                
                if !event_list.is_empty() {
                    let mut status_guard = self.status.write()
                        .map_err(|_| ProviderError::internal("获取状态写锁失败", "ProviderManager"))?;
                    
                    status_guard.total_events_processed += event_count;
                    status_guard.last_event_time = Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64
                    );
                }
                
                Ok(event_list)
            }
            Err(e) => {
                // 如果启用了自动切换，尝试故障转移
                if self.config.failover_enabled && e.is_recoverable() {
                    log::warn!("Provider {}出现可恢复错误，尝试故障转移: {}", provider_id, e);
                    
                    if let Err(switch_err) = self.switch_to_fallback_provider() {
                        log::error!("故障转移失败: {}", switch_err);
                    }
                }
                
                Err(e)
            }
        }
    }

    /// 获取管理器状态
    pub fn get_status(&self) -> ProviderResult<ProviderManagerStatus> {
        let status_guard = self.status.read()
            .map_err(|_| ProviderError::internal("获取状态锁失败", "ProviderManager"))?;
        
        Ok(status_guard.clone())
    }

    /// 获取活跃Provider ID
    pub fn get_active_provider_id(&self) -> Option<String> {
        self.active_provider_id.read().ok()?.as_ref().cloned()
    }

    /// 获取所有Provider的状态
    pub fn get_all_provider_status(&self) -> ProviderResult<HashMap<String, ProviderStatus>> {
        let providers_guard = self.providers.read()
            .map_err(|_| ProviderError::internal("获取Provider锁失败", "ProviderManager"))?;
        
        let mut status_map = HashMap::new();
        
        for (provider_id, registration) in providers_guard.iter() {
            if let Ok(provider_guard) = registration.provider.read() {
                status_map.insert(provider_id.clone(), provider_guard.get_status());
            }
        }
        
        Ok(status_map)
    }

    /// 获取事件分发器的引用
    pub fn event_dispatcher(&self) -> &Arc<LockFreeEventDispatcher> {
        &self.event_dispatcher
    }

    // 私有辅助方法
    
    /// 停止指定Provider
    fn stop_provider(&self, provider_id: &str) -> ProviderResult<()> {
        let providers_guard = self.providers.read()
            .map_err(|_| ProviderError::internal("获取Provider锁失败", "ProviderManager"))?;
        
        if let Some(registration) = providers_guard.get(provider_id) {
            let mut provider_guard = registration.provider.write()
                .map_err(|_| ProviderError::internal("获取Provider写锁失败", "ProviderManager"))?;
            
            provider_guard.stop()
        } else {
            Err(ProviderError::configuration(
                format!("Provider不存在: {}", provider_id)
            ))
        }
    }

    /// 切换到备用Provider
    fn switch_to_fallback_provider(&self) -> ProviderResult<()> {
        let providers_guard = self.providers.read()
            .map_err(|_| ProviderError::internal("获取Provider锁失败", "ProviderManager"))?;
        
        // 寻找最合适的备用Provider
        let mut candidates: Vec<_> = providers_guard
            .iter()
            .filter(|(_, registration)| {
                registration.enabled && registration.metadata.is_fallback
            })
            .collect();
        
        // 如果没有专门的备用Provider，选择任何可用的Provider
        if candidates.is_empty() {
            candidates = providers_guard
                .iter()
                .filter(|(id, registration)| {
                    registration.enabled && 
                    Some((*id).clone()) != self.get_active_provider_id()
                })
                .collect();
        }
        
        if candidates.is_empty() {
            return Err(ProviderError::state(
                "没有可用的备用Provider",
                "no_fallback",
                "fallback_available",
                "switch"
            ));
        }
        
        // 按优先级排序
        candidates.sort_by(|a, b| b.1.metadata.priority.cmp(&a.1.metadata.priority));
        
        // 尝试切换到优先级最高的Provider
        let fallback_id = candidates[0].0.clone();
        drop(providers_guard); // 释放读锁
        
        self.switch_to_provider(&fallback_id)
    }

    /// 启动健康监控线程
    fn start_health_monitor(&mut self) -> ProviderResult<()> {
        let providers = Arc::clone(&self.providers);
        let active_provider_id = Arc::clone(&self.active_provider_id);
        let is_running = Arc::clone(&self.is_running);
        let config = self.config.clone();
        
        let handle = thread::spawn(move || {
            let check_interval = Duration::from_millis(config.health_check_interval_ms);
            
            while is_running.load(Ordering::Relaxed) {
                thread::sleep(check_interval);
                
                // 执行健康检查
                if let Err(e) = Self::perform_health_check(
                    &providers,
                    &active_provider_id,
                    &config.auto_switch_config
                ) {
                    log::warn!("健康检查失败: {}", e);
                }
            }
        });
        
        self.health_check_handle = Some(handle);
        Ok(())
    }

    /// 执行健康检查
    fn perform_health_check(
        providers: &Arc<RwLock<HashMap<String, ProviderRegistration>>>,
        active_provider_id: &Arc<RwLock<Option<String>>>,
        auto_switch_config: &AutoSwitchConfig,
    ) -> ProviderResult<()> {
        let providers_guard = providers.read()
            .map_err(|_| ProviderError::internal("获取Provider锁失败", "HealthCheck"))?;
        
        let active_id = active_provider_id.read()
            .map_err(|_| ProviderError::internal("获取活跃Provider锁失败", "HealthCheck"))?
            .clone();
        
        if let Some(active_id) = active_id {
            if let Some(registration) = providers_guard.get(&active_id) {
                let provider_guard = registration.provider.read()
                    .map_err(|_| ProviderError::internal("获取Provider读锁失败", "HealthCheck"))?;
                
                // 执行健康检查
                if !provider_guard.health_check() {
                    log::warn!("Provider {}健康检查失败", active_id);
                    
                    // TODO: 实现更复杂的健康检查和自动切换逻辑
                    // 这里可以根据auto_switch_config的配置决定是否自动切换
                }
            }
        }
        
        Ok(())
    }
}

impl Drop for ProviderManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::provider::{ProviderError, ProviderResult, ProviderStatus, EventKind};
    
    // Mock Provider for testing
    #[derive(Debug)]
    struct MockProvider {
        id: String,
        connected: bool,
        events: Vec<EventType>,
    }
    
    impl MockProvider {
        fn new(id: String) -> Self {
            Self {
                id,
                connected: false,
                events: vec![],
            }
        }
    }
    
    impl DataProvider for MockProvider {
        type Error = ProviderError;
        
        fn initialize(&mut self) -> ProviderResult<()> {
            Ok(())
        }
        
        fn start(&mut self) -> ProviderResult<()> {
            self.connected = true;
            Ok(())
        }
        
        fn stop(&mut self) -> ProviderResult<()> {
            self.connected = false;
            Ok(())
        }
        
        fn is_connected(&self) -> bool {
            self.connected
        }
        
        fn read_events(&mut self) -> ProviderResult<Vec<EventType>> {
            Ok(self.events.drain(..).collect())
        }
        
        fn get_status(&self) -> ProviderStatus {
            ProviderStatus::new(ProviderType::Binance { mode: super::BinanceConnectionMode::WebSocket })
        }
        
        fn provider_type(&self) -> ProviderType {
            ProviderType::Binance { mode: super::BinanceConnectionMode::WebSocket }
        }
        
        fn supported_events(&self) -> &[EventKind] {
            &[EventKind::Trade, EventKind::TickPrice]
        }
    }

    #[test]
    fn test_provider_manager_creation() {
        let config = ProviderManagerConfig::default();
        let manager = ProviderManager::new(config);
        
        assert!(!manager.is_running.load(Ordering::Relaxed));
        assert!(manager.get_active_provider_id().is_none());
    }

    #[test]
    fn test_provider_registration() {
        let config = ProviderManagerConfig::default();
        let mut manager = ProviderManager::new(config);
        
        let provider = MockProvider::new("test_provider".to_string());
        let metadata = ProviderMetadata {
            id: "test_provider".to_string(),
            name: "Test Provider".to_string(),
            description: "A test provider".to_string(),
            provider_type: ProviderType::Binance { mode: super::BinanceConnectionMode::WebSocket },
            priority: 1,
            is_fallback: false,
            tags: HashMap::new(),
        };
        
        let result = manager.register_provider(provider, metadata);
        assert!(result.is_ok());
        
        assert_eq!(manager.get_active_provider_id(), Some("test_provider".to_string()));
    }

    #[test]
    fn test_provider_switching() {
        let config = ProviderManagerConfig::default();
        let mut manager = ProviderManager::new(config);
        
        // Register two providers
        let provider1 = MockProvider::new("provider1".to_string());
        let metadata1 = ProviderMetadata {
            id: "provider1".to_string(),
            name: "Provider 1".to_string(),
            description: "First provider".to_string(),
            provider_type: ProviderType::Binance { mode: super::BinanceConnectionMode::WebSocket },
            priority: 1,
            is_fallback: false,
            tags: HashMap::new(),
        };
        
        let provider2 = MockProvider::new("provider2".to_string());
        let metadata2 = ProviderMetadata {
            id: "provider2".to_string(),
            name: "Provider 2".to_string(),
            description: "Second provider".to_string(),
            provider_type: ProviderType::Binance { mode: super::BinanceConnectionMode::WebSocket },
            priority: 2,
            is_fallback: true,
            tags: HashMap::new(),
        };
        
        assert!(manager.register_provider(provider1, metadata1).is_ok());
        assert!(manager.register_provider(provider2, metadata2).is_ok());
        
        // Switch to provider2
        assert!(manager.switch_to_provider("provider2").is_ok());
        assert_eq!(manager.get_active_provider_id(), Some("provider2".to_string()));
    }
}