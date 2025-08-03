use super::{Service, ServiceError, ServiceHealth, ServiceStats};
use crate::core::PerformanceConfig;
use std::sync::{Arc, RwLock, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::time::Instant;
use std::collections::HashMap;

/// 配置管理服务
pub struct ConfigurationService {
    /// 运行状态
    is_running: AtomicBool,
    /// 启动时间
    start_time: Option<Instant>,
    /// 配置存储
    configs: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    /// 统计信息
    stats: ConfigurationStats,
}

#[derive(Debug)]
struct ConfigurationStats {
    config_updates: AtomicU64,
    config_reads: AtomicU64,
    error_count: AtomicU64,
}

impl Default for ConfigurationStats {
    fn default() -> Self {
        Self {
            config_updates: AtomicU64::new(0),
            config_reads: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
        }
    }
}

impl ConfigurationService {
    pub fn new() -> Self {
        Self {
            is_running: AtomicBool::new(false),
            start_time: None,
            configs: Arc::new(RwLock::new(HashMap::new())),
            stats: ConfigurationStats::default(),
        }
    }

    pub async fn set_config<T>(&self, key: &str, config: &T) -> Result<(), ServiceError> 
    where
        T: serde::Serialize,
    {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        let value = serde_json::to_value(config)
            .map_err(|e| ServiceError::ConfigurationError(format!("序列化失败: {}", e)))?;

        self.configs.write().unwrap().insert(key.to_string(), value);
        self.stats.config_updates.fetch_add(1, Ordering::Relaxed);
        
        log::info!("配置已更新: {}", key);
        Ok(())
    }

    pub async fn get_config<T>(&self, key: &str) -> Result<T, ServiceError>
    where 
        T: serde::de::DeserializeOwned,
    {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        let configs = self.configs.read().unwrap();
        let value = configs.get(key)
            .ok_or_else(|| ServiceError::ConfigurationError(format!("配置不存在: {}", key)))?;

        let config = serde_json::from_value(value.clone())
            .map_err(|e| ServiceError::ConfigurationError(format!("反序列化失败: {}", e)))?;

        self.stats.config_reads.fetch_add(1, Ordering::Relaxed);
        Ok(config)
    }
}

impl Service for ConfigurationService {
    fn name(&self) -> &'static str {
        "ConfigurationService"
    }

    fn start(&mut self) -> Result<(), ServiceError> {
        if self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::AlreadyRunning);
        }

        self.is_running.store(true, Ordering::Relaxed);
        self.start_time = Some(Instant::now());
        
        log::info!("配置管理服务已启动");
        Ok(())
    }

    fn stop(&mut self) -> Result<(), ServiceError> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        self.is_running.store(false, Ordering::Relaxed);
        
        log::info!("配置管理服务已停止");
        Ok(())
    }

    fn health_check(&self) -> ServiceHealth {
        if !self.is_running.load(Ordering::Relaxed) {
            return ServiceHealth::Unhealthy("配置管理服务未运行".to_string());
        }

        ServiceHealth::Healthy
    }

    fn stats(&self) -> ServiceStats {
        ServiceStats {
            service_name: self.name().to_string(),
            is_running: self.is_running.load(Ordering::Relaxed),
            start_time: self.start_time,
            requests_processed: self.stats.config_reads.load(Ordering::Relaxed) + 
                               self.stats.config_updates.load(Ordering::Relaxed),
            error_count: self.stats.error_count.load(Ordering::Relaxed),
            avg_response_time_ms: 0.0,
            memory_usage_bytes: 0,
        }
    }
}

/// 配置管理器trait
pub trait ConfigManager {
    fn load_config(&self, path: &str) -> Result<serde_json::Value, ServiceError>;
    fn save_config(&self, path: &str, config: &serde_json::Value) -> Result<(), ServiceError>;
    fn validate_config(&self, config: &serde_json::Value) -> Result<(), ServiceError>;
}