use super::{Service, ServiceError, ServiceHealth, ServiceStats, ConfigurableService, MonitorableService, ServiceMetric, MetricValue};
use crate::core::{PerformanceConfig, PerformanceMetrics, AdaptivePerformanceTuner};
use std::sync::{Arc, RwLock, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::time::{Duration, Instant};
use std::collections::HashMap;

/// 性能监控服务 - 负责系统性能监控和自动调优
pub struct PerformanceService {
    /// 服务配置
    config: Arc<RwLock<PerformanceServiceConfig>>,
    /// 运行状态
    is_running: AtomicBool,
    /// 启动时间
    start_time: Option<Instant>,
    /// 统计信息
    stats: PerformanceServiceStats,
    /// 性能监控器
    monitor: Arc<RwLock<PerformanceMonitor>>,
    /// 自适应调优器
    tuner: Arc<RwLock<AdaptivePerformanceTuner>>,
    /// 异步任务句柄
    task_handles: Vec<tokio::task::JoinHandle<()>>,
    /// 监控回调函数
    monitor_callbacks: Arc<RwLock<Vec<Box<dyn Fn(&ServiceMetric) + Send + Sync>>>>,
}

/// 性能服务配置
#[derive(Debug, Clone)]
pub struct PerformanceServiceConfig {
    /// 监控间隔
    pub monitoring_interval: Duration,
    /// 报告间隔
    pub reporting_interval: Duration,
    /// 启用自动调优
    pub auto_tuning_enabled: bool,
    /// 内存监控
    pub memory_monitoring_enabled: bool,
    /// CPU监控
    pub cpu_monitoring_enabled: bool,
    /// 网络监控
    pub network_monitoring_enabled: bool,
    /// 指标历史保留时间
    pub metrics_retention: Duration,
    /// 性能阈值
    pub performance_thresholds: PerformanceThresholds,
}

/// 性能阈值配置
#[derive(Debug, Clone)]
pub struct PerformanceThresholds {
    /// 最低FPS阈值
    pub min_fps: f64,
    /// 最大缓冲区使用率
    pub max_buffer_usage: f64,
    /// 最大事件处理延迟 (毫秒)
    pub max_event_latency_ms: f64,
    /// 最大内存使用量 (MB)
    pub max_memory_usage_mb: f64,
    /// 最大CPU使用率
    pub max_cpu_usage_percent: f64,
}

impl Default for PerformanceServiceConfig {
    fn default() -> Self {
        Self {
            monitoring_interval: Duration::from_secs(1),
            reporting_interval: Duration::from_secs(10),
            auto_tuning_enabled: true,
            memory_monitoring_enabled: true,
            cpu_monitoring_enabled: true,
            network_monitoring_enabled: false,
            metrics_retention: Duration::from_secs(3600), // 1 hour
            performance_thresholds: PerformanceThresholds::default(),
        }
    }
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            min_fps: 15.0,
            max_buffer_usage: 0.9,
            max_event_latency_ms: 100.0,
            max_memory_usage_mb: 512.0,
            max_cpu_usage_percent: 80.0,
        }
    }
}

/// 性能服务统计
#[derive(Debug)]
struct PerformanceServiceStats {
    /// 监控周期数
    monitoring_cycles: AtomicU64,
    /// 自动调优次数
    tuning_adjustments: AtomicU64,
    /// 性能警告数
    performance_warnings: AtomicU64,
    /// 性能错误数
    performance_errors: AtomicU64,
    /// 监控错误数
    monitoring_errors: AtomicU64,
}

impl Default for PerformanceServiceStats {
    fn default() -> Self {
        Self {
            monitoring_cycles: AtomicU64::new(0),
            tuning_adjustments: AtomicU64::new(0),
            performance_warnings: AtomicU64::new(0),
            performance_errors: AtomicU64::new(0),
            monitoring_errors: AtomicU64::new(0),
        }
    }
}

/// 性能监控器
pub struct PerformanceMonitor {
    /// 性能指标历史
    metrics_history: std::collections::VecDeque<TimedMetrics>,
    /// 系统信息收集器
    system_collector: SystemMetricsCollector,
    /// 上次收集时间
    last_collection_time: Instant,
    /// 监控配置
    config: PerformanceServiceConfig,
    /// 最后一次指标
    pub last_metrics: Option<PerformanceMetrics>,
}

/// 带时间戳的指标
#[derive(Debug, Clone)]
struct TimedMetrics {
    timestamp: Instant,
    metrics: PerformanceMetrics,
    system_info: SystemInfo,
}

/// 系统信息
#[derive(Debug, Clone)]
pub struct SystemInfo {
    /// CPU使用率
    pub cpu_usage_percent: f64,
    /// 内存使用量 (字节)
    pub memory_usage_bytes: u64,
    /// 可用内存 (字节)
    pub available_memory_bytes: u64,
    /// 网络接收字节数
    pub network_rx_bytes: u64,
    /// 网络发送字节数
    pub network_tx_bytes: u64,
    /// 磁盘读取字节数
    pub disk_read_bytes: u64,
    /// 磁盘写入字节数
    pub disk_write_bytes: u64,
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_usage_bytes: 0,
            available_memory_bytes: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
        }
    }
}

/// 系统指标收集器
pub struct SystemMetricsCollector {
    /// 进程ID
    pid: u32,
    /// 上次CPU时间
    last_cpu_time: Option<std::time::Duration>,
    /// 上次采样时间
    last_sample_time: Option<Instant>,
}

impl PerformanceService {
    /// 创建新的性能监控服务
    pub fn new(performance_config: PerformanceConfig) -> Self {
        let service_config = PerformanceServiceConfig::default();
        let tuner = AdaptivePerformanceTuner::new(performance_config);
        let monitor = PerformanceMonitor::new(service_config.clone());

        Self {
            config: Arc::new(RwLock::new(service_config)),
            is_running: AtomicBool::new(false),
            start_time: None,
            stats: PerformanceServiceStats::default(),
            monitor: Arc::new(RwLock::new(monitor)),
            tuner: Arc::new(RwLock::new(tuner)),
            task_handles: Vec::new(),
            monitor_callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 记录性能指标
    pub async fn record_metrics(&self, metrics: PerformanceMetrics) -> Result<(), ServiceError> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        // 记录到监控器
        {
            let mut monitor = self.monitor.write().unwrap();
            monitor.record_metrics(metrics.clone()).await?;
        }

        // 记录到自适应调优器
        {
            let mut tuner = self.tuner.write().unwrap();
            tuner.record_metrics(metrics);
        }

        Ok(())
    }

    /// 获取当前性能指标
    pub async fn get_current_metrics(&self) -> Result<PerformanceMetrics, ServiceError> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        let monitor = self.monitor.read().unwrap();
        monitor.get_current_metrics()
    }

    /// 获取性能历史
    pub async fn get_metrics_history(&self, duration: Duration) -> Result<Vec<PerformanceMetrics>, ServiceError> {
        let monitor = self.monitor.read().unwrap();
        Ok(monitor.get_metrics_history(duration))
    }

    /// 获取系统信息
    pub async fn get_system_info(&self) -> Result<SystemInfo, ServiceError> {
        let monitor = self.monitor.read().unwrap();
        monitor.collect_system_metrics()
    }

    /// 触发手动调优
    pub async fn trigger_tuning(&self) -> Result<bool, ServiceError> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        let mut tuner = self.tuner.write().unwrap();
        let adjusted = tuner.adapt_config();
        
        if adjusted {
            self.stats.tuning_adjustments.fetch_add(1, Ordering::Relaxed);
            log::info!("手动性能调优已执行");
        }

        Ok(adjusted)
    }

    /// 获取调优建议
    pub async fn get_tuning_recommendations(&self) -> Result<Vec<String>, ServiceError> {
        let monitor = self.monitor.read().unwrap();
        let current_metrics = monitor.get_current_metrics()?;
        let thresholds = &self.config.read().unwrap().performance_thresholds;

        let mut recommendations = Vec::new();

        if current_metrics.current_fps < thresholds.min_fps {
            recommendations.push(format!(
                "FPS过低 ({:.1})，建议降低渲染质量或减少更新频率",
                current_metrics.current_fps
            ));
        }

        if current_metrics.buffer_usage_ratio > thresholds.max_buffer_usage {
            recommendations.push(format!(
                "缓冲区使用率过高 ({:.1}%)，建议增加缓冲区大小或提高处理速度",
                current_metrics.buffer_usage_ratio * 100.0
            ));
        }

        if current_metrics.event_processing_latency_ms > thresholds.max_event_latency_ms {
            recommendations.push(format!(
                "事件处理延迟过高 ({:.1}ms)，建议优化处理逻辑或增加处理线程",
                current_metrics.event_processing_latency_ms
            ));
        }

        if current_metrics.memory_usage_mb > thresholds.max_memory_usage_mb {
            recommendations.push(format!(
                "内存使用过高 ({:.1}MB)，建议检查内存泄漏或增加内存限制",
                current_metrics.memory_usage_mb
            ));
        }

        if recommendations.is_empty() {
            recommendations.push("系统性能良好，无需调优".to_string());
        }

        Ok(recommendations)
    }

    /// 生成性能报告
    pub async fn generate_performance_report(&self) -> Result<String, ServiceError> {
        let monitor = self.monitor.read().unwrap();
        let tuner = self.tuner.read().unwrap();
        let current_metrics = monitor.get_current_metrics()?;
        let system_info = monitor.collect_system_metrics()?;

        let report = format!(
            "=== 性能监控报告 ===\n\
            时间: {}\n\
            \n\
            === 应用性能 ===\n\
            FPS: {:.1}\n\
            缓冲区使用率: {:.1}%\n\
            事件处理延迟: {:.1}ms\n\
            内存使用: {:.1}MB\n\
            CPU使用率: {:.1}%\n\
            丢弃事件: {}\n\
            \n\
            === 系统信息 ===\n\
            系统内存使用: {:.1}MB\n\
            可用内存: {:.1}MB\n\
            CPU使用率: {:.1}%\n\
            \n\
            === 服务统计 ===\n\
            监控周期: {}\n\
            自动调优次数: {}\n\
            性能警告: {}\n\
            性能错误: {}\n\
            \n\
            === 自适应调优 ===\n\
            {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            current_metrics.current_fps,
            current_metrics.buffer_usage_ratio * 100.0,
            current_metrics.event_processing_latency_ms,
            current_metrics.memory_usage_mb,
            current_metrics.cpu_usage_percent,
            current_metrics.dropped_events,
            system_info.memory_usage_bytes as f64 / 1024.0 / 1024.0,
            system_info.available_memory_bytes as f64 / 1024.0 / 1024.0,
            system_info.cpu_usage_percent,
            self.stats.monitoring_cycles.load(Ordering::Relaxed),
            self.stats.tuning_adjustments.load(Ordering::Relaxed),
            self.stats.performance_warnings.load(Ordering::Relaxed),
            self.stats.performance_errors.load(Ordering::Relaxed),
            tuner.generate_report()
        );

        Ok(report)
    }
}

impl Service for PerformanceService {
    fn name(&self) -> &'static str {
        "PerformanceService"
    }

    fn start(&mut self) -> Result<(), ServiceError> {
        if self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::AlreadyRunning);
        }

        // 启动监控任务
        self.start_monitoring_tasks();

        self.is_running.store(true, Ordering::Relaxed);
        self.start_time = Some(Instant::now());

        log::info!("性能监控服务已启动");
        Ok(())
    }

    fn stop(&mut self) -> Result<(), ServiceError> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        // 停止监控任务
        self.stop_monitoring_tasks();

        self.is_running.store(false, Ordering::Relaxed);

        log::info!("性能监控服务已停止");
        Ok(())
    }

    fn health_check(&self) -> ServiceHealth {
        if !self.is_running.load(Ordering::Relaxed) {
            return ServiceHealth::Unhealthy("性能监控服务未运行".to_string());
        }

        let monitoring_errors = self.stats.monitoring_errors.load(Ordering::Relaxed);
        let monitoring_cycles = self.stats.monitoring_cycles.load(Ordering::Relaxed);

        if monitoring_cycles == 0 {
            return ServiceHealth::Warning("尚未完成监控周期".to_string());
        }

        let error_rate = monitoring_errors as f64 / monitoring_cycles as f64;
        if error_rate > 0.1 {
            ServiceHealth::Unhealthy(format!("监控错误率过高: {:.2}%", error_rate * 100.0))
        } else if error_rate > 0.05 {
            ServiceHealth::Warning(format!("监控错误率偏高: {:.2}%", error_rate * 100.0))
        } else {
            ServiceHealth::Healthy
        }
    }

    fn stats(&self) -> ServiceStats {
        ServiceStats {
            service_name: self.name().to_string(),
            is_running: self.is_running.load(Ordering::Relaxed),
            start_time: self.start_time,
            requests_processed: self.stats.monitoring_cycles.load(Ordering::Relaxed),
            error_count: self.stats.monitoring_errors.load(Ordering::Relaxed),
            avg_response_time_ms: 0.0, // 监控服务不适用响应时间
            memory_usage_bytes: 0, // TODO: 实现内存使用统计
        }
    }
}

impl ConfigurableService for PerformanceService {
    type Config = PerformanceServiceConfig;

    fn update_config(&mut self, config: PerformanceServiceConfig) -> Result<(), ServiceError> {
        *self.config.write().unwrap() = config;
        Ok(())
    }

    fn get_config(&self) -> &Self::Config {
        // SAFETY: 这个实现是临时的，在生产代码中应该重新设计
        Box::leak(Box::new(self.config.read().unwrap().clone()))
    }
}

impl MonitorableService for PerformanceService {
    fn get_metrics(&self) -> Vec<ServiceMetric> {
        let mut metrics = Vec::new();

        // 添加服务级别指标
        metrics.push(ServiceMetric {
            name: "monitoring_cycles".to_string(),
            value: MetricValue::Counter(self.stats.monitoring_cycles.load(Ordering::Relaxed)),
            timestamp: Instant::now(),
            labels: HashMap::from([("service".to_string(), "performance".to_string())]),
        });

        metrics.push(ServiceMetric {
            name: "tuning_adjustments".to_string(),
            value: MetricValue::Counter(self.stats.tuning_adjustments.load(Ordering::Relaxed)),
            timestamp: Instant::now(),
            labels: HashMap::from([("service".to_string(), "performance".to_string())]),
        });

        // 添加性能指标
        if let Ok(current_metrics) = self.monitor.read().unwrap().get_current_metrics() {
            metrics.push(ServiceMetric {
                name: "current_fps".to_string(),
                value: MetricValue::Gauge(current_metrics.current_fps),
                timestamp: Instant::now(),
                labels: HashMap::from([("type".to_string(), "performance".to_string())]),
            });

            metrics.push(ServiceMetric {
                name: "buffer_usage_ratio".to_string(),
                value: MetricValue::Gauge(current_metrics.buffer_usage_ratio),
                timestamp: Instant::now(),
                labels: HashMap::from([("type".to_string(), "performance".to_string())]),
            });
        }

        metrics
    }

    fn set_monitor_callback(&mut self, callback: Box<dyn Fn(&ServiceMetric) + Send + Sync>) {
        self.monitor_callbacks.write().unwrap().push(callback);
    }
}

impl PerformanceService {
    /// 启动监控任务
    fn start_monitoring_tasks(&mut self) {
        let monitor = self.monitor.clone();
        let tuner = self.tuner.clone();
        let config = self.config.clone();
        let callbacks = self.monitor_callbacks.clone();

        // 主监控任务
        let monitoring_task = tokio::spawn(async move {
            loop {
                let monitoring_interval = config.read().unwrap().monitoring_interval;
                let mut interval = tokio::time::interval(monitoring_interval);

                interval.tick().await;

                // 收集性能指标
                if let Err(e) = Self::collect_and_process_metrics(&monitor, &tuner, &config, &callbacks).await {
                    log::error!("性能监控失败: {}", e);
                } else {
                    log::debug!("性能监控周期完成");
                }
            }
        });

        self.task_handles.push(monitoring_task);

        // 报告任务
        let reporting_task = self.create_reporting_task();
        self.task_handles.push(reporting_task);
    }

    /// 停止监控任务
    fn stop_monitoring_tasks(&mut self) {
        for handle in self.task_handles.drain(..) {
            handle.abort();
        }
    }

    /// 收集和处理指标
    async fn collect_and_process_metrics(
        monitor: &Arc<RwLock<PerformanceMonitor>>,
        tuner: &Arc<RwLock<AdaptivePerformanceTuner>>,
        config: &Arc<RwLock<PerformanceServiceConfig>>,
        callbacks: &Arc<RwLock<Vec<Box<dyn Fn(&ServiceMetric) + Send + Sync>>>>,
    ) -> Result<(), ServiceError> {
        // 收集系统指标
        let system_info = {
            let monitor = monitor.write().unwrap();
            monitor.collect_system_metrics()?
        };

        // 创建性能指标
        let metrics = PerformanceMetrics {
            current_fps: 0.0, // 需要从其他服务获取
            buffer_usage_ratio: 0.0, // 需要从其他服务获取
            event_processing_latency_ms: 0.0, // 需要从其他服务获取
            memory_usage_mb: system_info.memory_usage_bytes as f64 / 1024.0 / 1024.0,
            cpu_usage_percent: system_info.cpu_usage_percent,
            dropped_events: 0, // 需要从其他服务获取
        };

        // 记录指标 (简化为同步操作)
        {
            let mut monitor = monitor.write().unwrap();
            // 将指标存储到监控器中
            monitor.last_metrics = Some(metrics.clone());
            let timed_metrics = TimedMetrics {
                timestamp: Instant::now(),
                metrics: metrics.clone(),
                system_info: system_info,
            };
            monitor.metrics_history.push_back(timed_metrics);
            // 限制历史记录大小
            if monitor.metrics_history.len() > 1000 {
                monitor.metrics_history.pop_front();
            }
        }

        // 自动调优
        let auto_tuning_enabled = config.read().unwrap().auto_tuning_enabled;
        if auto_tuning_enabled {
            let mut tuner = tuner.write().unwrap();
            tuner.record_metrics(metrics.clone());
            if tuner.adapt_config() {
                log::debug!("性能调优已执行");
            }
        }

        // 调用监控回调
        let callbacks_guard = callbacks.read().unwrap();
        for callback in callbacks_guard.iter() {
            let metric = ServiceMetric {
                name: "performance_update".to_string(),
                value: MetricValue::Gauge(metrics.current_fps),
                timestamp: Instant::now(),
                labels: HashMap::new(),
            };
            callback(&metric);
        }

        Ok(())
    }

    /// 创建报告任务
    fn create_reporting_task(&self) -> tokio::task::JoinHandle<()> {
        let monitor = self.monitor.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            loop {
                let reporting_interval = config.read().unwrap().reporting_interval;
                let mut interval = tokio::time::interval(reporting_interval);

                interval.tick().await;

                // 生成并输出性能报告
                if let Ok(current_metrics) = monitor.read().unwrap().get_current_metrics() {
                    if current_metrics.needs_optimization() {
                        log::warn!("系统性能需要优化: FPS={:.1}, 缓冲区使用率={:.1}%, 延迟={:.1}ms",
                                 current_metrics.current_fps,
                                 current_metrics.buffer_usage_ratio * 100.0,
                                 current_metrics.event_processing_latency_ms);
                    } else {
                        log::info!("系统性能良好: FPS={:.1}, 缓冲区使用率={:.1}%",
                                 current_metrics.current_fps,
                                 current_metrics.buffer_usage_ratio * 100.0);
                    }
                }
            }
        })
    }
}

impl PerformanceMonitor {
    pub fn new(config: PerformanceServiceConfig) -> Self {
        Self {
            metrics_history: std::collections::VecDeque::with_capacity(3600), // 1小时历史
            system_collector: SystemMetricsCollector::new(),
            last_collection_time: Instant::now(),
            last_metrics: None,
            config,
        }
    }

    pub async fn record_metrics(&mut self, metrics: PerformanceMetrics) -> Result<(), ServiceError> {
        let system_info = self.collect_system_metrics()?;
        
        let timed_metrics = TimedMetrics {
            timestamp: Instant::now(),
            metrics,
            system_info,
        };

        self.metrics_history.push_back(timed_metrics);

        // 清理过期数据
        let retention = self.config.metrics_retention;
        let cutoff_time = Instant::now() - retention;
        
        while let Some(front) = self.metrics_history.front() {
            if front.timestamp < cutoff_time {
                self.metrics_history.pop_front();
            } else {
                break;
            }
        }

        Ok(())
    }

    pub fn get_current_metrics(&self) -> Result<PerformanceMetrics, ServiceError> {
        self.metrics_history.back()
            .map(|timed| timed.metrics.clone())
            .ok_or_else(|| ServiceError::InternalError("没有可用的性能指标".to_string()))
    }

    pub fn get_metrics_history(&self, duration: Duration) -> Vec<PerformanceMetrics> {
        let cutoff_time = Instant::now() - duration;
        
        self.metrics_history.iter()
            .filter(|timed| timed.timestamp >= cutoff_time)
            .map(|timed| timed.metrics.clone())
            .collect()
    }

    pub fn collect_system_metrics(&self) -> Result<SystemInfo, ServiceError> {
        // 简化的系统指标收集实现
        // 实际实现中应该使用系统API获取真实数据
        Ok(SystemInfo {
            cpu_usage_percent: self.estimate_cpu_usage(),
            memory_usage_bytes: self.estimate_memory_usage(),
            available_memory_bytes: self.estimate_available_memory(),
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
        })
    }

    fn estimate_cpu_usage(&self) -> f64 {
        // 简化的CPU使用率估算
        // 实际实现应该读取 /proc/stat 或使用系统API
        rand::random::<f64>() * 50.0 // 模拟0-50%的CPU使用率
    }

    fn estimate_memory_usage(&self) -> u64 {
        // 简化的内存使用量估算
        // 实际实现应该读取 /proc/meminfo 或使用系统API
        1024 * 1024 * 256 // 模拟256MB内存使用
    }

    fn estimate_available_memory(&self) -> u64 {
        // 简化的可用内存估算
        1024 * 1024 * 1024 * 2 // 模拟2GB可用内存
    }
}

impl SystemMetricsCollector {
    pub fn new() -> Self {
        Self {
            pid: std::process::id(),
            last_cpu_time: None,
            last_sample_time: None,
        }
    }
}