/// 服务健康监控相关类型

use serde::{Deserialize, Serialize};

/// 服务健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// 服务是否健康
    pub is_healthy: bool,
    /// 健康状态描述
    pub status: String,
    /// 最后检查时间
    pub last_check: std::time::SystemTime,
    /// 响应时间 (毫秒)
    pub response_time_ms: u64,
    /// 内存使用 (字节)
    pub memory_usage_bytes: u64,
    /// CPU使用率 (百分比)
    pub cpu_usage_percent: f64,
    /// 错误计数
    pub error_count: u64,
    /// 警告计数
    pub warning_count: u64,
}

impl Default for ServiceHealth {
    fn default() -> Self {
        Self {
            is_healthy: true,
            status: "健康".to_string(),
            last_check: std::time::SystemTime::now(),
            response_time_ms: 0,
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
            error_count: 0,
            warning_count: 0,
        }
    }
}

/// 服务统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStats {
    /// 运行时间
    pub uptime: std::time::Duration,
    /// 处理的消息总数
    pub messages_processed: u64,
    /// 发送的消息总数
    pub messages_sent: u64,
    /// 错误次数
    pub error_count: u64,
    /// 平均响应时间 (毫秒)
    pub avg_response_time_ms: f64,
    /// 峰值内存使用 (字节)
    pub peak_memory_usage_bytes: u64,
    /// 最后活动时间
    pub last_activity: std::time::SystemTime,
}

impl Default for ServiceStats {
    fn default() -> Self {
        Self {
            uptime: std::time::Duration::ZERO,
            messages_processed: 0,
            messages_sent: 0,
            error_count: 0,
            avg_response_time_ms: 0.0,
            peak_memory_usage_bytes: 0,
            last_activity: std::time::SystemTime::now(),
        }
    }
}

/// 健康检查器trait
#[async_trait::async_trait]
pub trait HealthChecker: Send + Sync {
    /// 执行健康检查
    async fn check_health(&self) -> ServiceHealth;
    
    /// 获取健康检查间隔
    fn check_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(60)
    }
}

/// 基础健康检查器
pub struct BasicHealthChecker {
    /// 服务启动时间
    start_time: std::time::SystemTime,
    /// 最后活动时间
    last_activity: std::time::SystemTime,
    /// 错误计数
    error_count: std::sync::atomic::AtomicU64,
    /// 警告计数
    warning_count: std::sync::atomic::AtomicU64,
}

impl BasicHealthChecker {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now();
        Self {
            start_time: now,
            last_activity: now,
            error_count: std::sync::atomic::AtomicU64::new(0),
            warning_count: std::sync::atomic::AtomicU64::new(0),
        }
    }
    
    /// 记录活动
    pub fn record_activity(&mut self) {
        self.last_activity = std::time::SystemTime::now();
    }
    
    /// 增加错误计数
    pub fn increment_error(&self) {
        self.error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    
    /// 增加警告计数
    pub fn increment_warning(&self) {
        self.warning_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[async_trait::async_trait]
impl HealthChecker for BasicHealthChecker {
    async fn check_health(&self) -> ServiceHealth {
        let now = std::time::SystemTime::now();
        let error_count = self.error_count.load(std::sync::atomic::Ordering::Relaxed);
        let warning_count = self.warning_count.load(std::sync::atomic::Ordering::Relaxed);
        
        // 简单的健康判断逻辑
        let is_healthy = error_count < 10 && warning_count < 50;
        let status = if is_healthy {
            "健康".to_string()
        } else if error_count > 0 {
            format!("错误: {}", error_count)
        } else {
            format!("警告: {}", warning_count)
        };
        
        ServiceHealth {
            is_healthy,
            status,
            last_check: now,
            response_time_ms: 1, // 模拟响应时间
            memory_usage_bytes: 0, // 实际应该获取真实内存使用
            cpu_usage_percent: 0.0, // 实际应该获取真实CPU使用率
            error_count,
            warning_count,
        }
    }
}