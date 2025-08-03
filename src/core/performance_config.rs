use std::time::Duration;
use serde::{Serialize, Deserialize};

/// 性能优化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// GUI 渲染配置
    pub gui: GUIPerformanceConfig,
    /// 缓冲区配置
    pub buffer: BufferConfig,
    /// 数据处理配置
    pub processing: DataProcessingConfig,
    /// 监控配置
    pub monitoring: MonitoringConfig,
}

/// GUI 性能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GUIPerformanceConfig {
    /// 目标帧率 (FPS)
    pub target_fps: u32,
    /// 最小帧率 (自适应)
    pub min_fps: u32,
    /// 最大帧率 (自适应)
    pub max_fps: u32,
    /// 自适应帧率开关
    pub adaptive_fps: bool,
    /// 渲染批处理大小
    pub render_batch_size: usize,
    /// V-Sync 开关
    pub vsync_enabled: bool,
}

/// 缓冲区配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferConfig {
    /// 事件缓冲区大小
    pub event_buffer_size: usize,
    /// 自动扩容开关
    pub auto_resize: bool,
    /// 最大缓冲区大小
    pub max_buffer_size: usize,
    /// 缓冲区使用率阈值 (触发扩容)
    pub resize_threshold: f64,
    /// 背压控制开关
    pub backpressure_enabled: bool,
    /// 背压阈值
    pub backpressure_threshold: f64,
}

/// 数据处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataProcessingConfig {
    /// 每次事件循环最大处理事件数
    pub max_events_per_cycle: usize,
    /// 数据聚合批处理大小
    pub aggregation_batch_size: usize,
    /// 价格精度
    pub price_precision: f64,
    /// 数据缓存开关
    pub caching_enabled: bool,
    /// 缓存过期时间 (毫秒)
    pub cache_expiry_ms: u64,
}

/// 监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// 性能指标收集间隔
    pub metrics_interval_ms: u64,
    /// 日志级别过滤
    pub log_level_filter: String,
    /// 性能报告间隔
    pub report_interval_ms: u64,
    /// 内存使用监控开关
    pub memory_monitoring: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            gui: GUIPerformanceConfig::default(),
            buffer: BufferConfig::default(),
            processing: DataProcessingConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

impl Default for GUIPerformanceConfig {
    fn default() -> Self {
        Self {
            target_fps: 30,      // 降低到30FPS以减少CPU负载
            min_fps: 15,         // 最低15FPS保证基本响应
            max_fps: 60,         // 最高60FPS
            adaptive_fps: true,  // 启用自适应帧率
            render_batch_size: 100,
            vsync_enabled: false, // 禁用V-Sync以减少延迟
        }
    }
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            event_buffer_size: 16384,        // 增加到16K
            auto_resize: true,               // 启用自动扩容
            max_buffer_size: 65536,          // 最大64K
            resize_threshold: 0.8,           // 80%使用率触发扩容
            backpressure_enabled: true,      // 启用背压控制
            backpressure_threshold: 0.9,     // 90%使用率触发背压
        }
    }
}

impl Default for DataProcessingConfig {
    fn default() -> Self {
        Self {
            max_events_per_cycle: 500,       // 每次处理500个事件
            aggregation_batch_size: 1000,    // 批处理1000条数据
            price_precision: 0.1,            // 0.1美元精度
            caching_enabled: true,           // 启用缓存
            cache_expiry_ms: 1000,           // 1秒缓存过期
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            metrics_interval_ms: 1000,       // 1秒收集一次指标
            log_level_filter: "info".to_string(),
            report_interval_ms: 5000,        // 5秒报告一次
            memory_monitoring: true,
        }
    }
}

impl PerformanceConfig {
    /// 从文件加载配置
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: PerformanceConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 保存配置到文件
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 获取GUI更新间隔
    pub fn gui_update_interval(&self) -> Duration {
        Duration::from_millis(1000 / self.gui.target_fps as u64)
    }

    /// 获取自适应FPS范围
    pub fn fps_range(&self) -> (u32, u32) {
        (self.gui.min_fps, self.gui.max_fps)
    }

    /// 检查是否需要扩容缓冲区
    pub fn should_resize_buffer(&self, usage_ratio: f64) -> bool {
        self.buffer.auto_resize && usage_ratio >= self.buffer.resize_threshold
    }

    /// 检查是否触发背压控制
    pub fn should_apply_backpressure(&self, usage_ratio: f64) -> bool {
        self.buffer.backpressure_enabled && usage_ratio >= self.buffer.backpressure_threshold
    }

    /// 计算新的缓冲区大小
    pub fn calculate_new_buffer_size(&self, current_size: usize) -> usize {
        let new_size = current_size * 2;
        new_size.min(self.buffer.max_buffer_size)
    }
}

/// 性能指标收集器
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// 当前FPS
    pub current_fps: f64,
    /// 缓冲区使用率
    pub buffer_usage_ratio: f64,
    /// 事件处理延迟 (毫秒)
    pub event_processing_latency_ms: f64,
    /// 内存使用量 (MB)
    pub memory_usage_mb: f64,
    /// CPU使用率
    pub cpu_usage_percent: f64,
    /// 丢弃的事件数量
    pub dropped_events: u64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            current_fps: 0.0,
            buffer_usage_ratio: 0.0,
            event_processing_latency_ms: 0.0,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            dropped_events: 0,
        }
    }
}

impl PerformanceMetrics {
    /// 评估系统性能等级
    pub fn performance_grade(&self) -> PerformanceGrade {
        if self.current_fps >= 25.0 && self.buffer_usage_ratio < 0.7 && self.event_processing_latency_ms < 50.0 {
            PerformanceGrade::Excellent
        } else if self.current_fps >= 15.0 && self.buffer_usage_ratio < 0.85 && self.event_processing_latency_ms < 100.0 {
            PerformanceGrade::Good
        } else if self.current_fps >= 10.0 && self.buffer_usage_ratio < 0.95 {
            PerformanceGrade::Fair
        } else {
            PerformanceGrade::Poor
        }
    }

    /// 是否需要性能调优
    pub fn needs_optimization(&self) -> bool {
        matches!(self.performance_grade(), PerformanceGrade::Fair | PerformanceGrade::Poor)
    }
}

/// 性能等级枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PerformanceGrade {
    Excellent,
    Good,
    Fair,
    Poor,
}

/// 自适应性能调优器
pub struct AdaptivePerformanceTuner {
    config: PerformanceConfig,
    metrics_history: std::collections::VecDeque<PerformanceMetrics>,
    max_history_size: usize,
}

impl AdaptivePerformanceTuner {
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            config,
            metrics_history: std::collections::VecDeque::with_capacity(60), // 保存60秒历史
            max_history_size: 60,
        }
    }

    /// 记录性能指标
    pub fn record_metrics(&mut self, metrics: PerformanceMetrics) {
        self.metrics_history.push_back(metrics);
        if self.metrics_history.len() > self.max_history_size {
            self.metrics_history.pop_front();
        }
    }

    /// 自适应调整配置
    pub fn adapt_config(&mut self) -> bool {
        if self.metrics_history.len() < 5 {
            return false; // 需要足够的历史数据
        }

        let recent_metrics: Vec<_> = self.metrics_history.iter().rev().take(5).collect();
        let avg_fps = recent_metrics.iter().map(|m| m.current_fps).sum::<f64>() / recent_metrics.len() as f64;
        let avg_buffer_usage = recent_metrics.iter().map(|m| m.buffer_usage_ratio).sum::<f64>() / recent_metrics.len() as f64;

        let mut config_changed = false;

        // 自适应调整FPS
        if avg_fps < self.config.gui.min_fps as f64 * 0.9 {
            // 性能不足，降低目标FPS
            if self.config.gui.target_fps > self.config.gui.min_fps {
                self.config.gui.target_fps = (self.config.gui.target_fps - 5).max(self.config.gui.min_fps);
                config_changed = true;
                log::info!("自适应调整: 降低目标FPS到 {}", self.config.gui.target_fps);
            }
        } else if avg_fps > self.config.gui.target_fps as f64 * 1.1 {
            // 性能充足，提高目标FPS
            if self.config.gui.target_fps < self.config.gui.max_fps {
                self.config.gui.target_fps = (self.config.gui.target_fps + 5).min(self.config.gui.max_fps);
                config_changed = true;
                log::info!("自适应调整: 提高目标FPS到 {}", self.config.gui.target_fps);
            }
        }

        // 自适应调整缓冲区
        if avg_buffer_usage > 0.9 {
            // 缓冲区使用率过高，减少每次处理的事件数
            if self.config.processing.max_events_per_cycle > 100 {
                self.config.processing.max_events_per_cycle = 
                    (self.config.processing.max_events_per_cycle * 8 / 10).max(100);
                config_changed = true;
                log::info!("自适应调整: 减少每次处理事件数到 {}", self.config.processing.max_events_per_cycle);
            }
        } else if avg_buffer_usage < 0.5 {
            // 缓冲区使用率较低，增加每次处理的事件数
            if self.config.processing.max_events_per_cycle < 1000 {
                self.config.processing.max_events_per_cycle = 
                    (self.config.processing.max_events_per_cycle * 12 / 10).min(1000);
                config_changed = true;
                log::info!("自适应调整: 增加每次处理事件数到 {}", self.config.processing.max_events_per_cycle);
            }
        }

        config_changed
    }

    /// 获取当前配置
    pub fn config(&self) -> &PerformanceConfig {
        &self.config
    }

    /// 获取性能报告
    pub fn generate_report(&self) -> String {
        if self.metrics_history.is_empty() {
            return "没有性能数据".to_string();
        }

        let latest = self.metrics_history.back().unwrap();
        let grade = latest.performance_grade();

        format!(
            "性能报告:\n等级: {:?}\nFPS: {:.1}\n缓冲区使用率: {:.1}%\n事件处理延迟: {:.1}ms\n内存使用: {:.1}MB\n丢弃事件: {}",
            grade,
            latest.current_fps,
            latest.buffer_usage_ratio * 100.0,
            latest.event_processing_latency_ms,
            latest.memory_usage_mb,
            latest.dropped_events
        )
    }
}