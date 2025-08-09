use std::time::{Duration, Instant};
use std::collections::VecDeque;

/// 内部监控系统 - 用于诊断应用程序健康状态和性能问题
#[derive(Debug, Clone)]
pub struct InternalMonitor {
    // 事件处理监控
    pub event_processing: EventProcessingMonitor,
    // WebSocket连接监控
    pub websocket_health: WebSocketHealthMonitor,
    // 缓冲区监控
    pub buffer_monitor: BufferMonitor,
    // 性能监控
    pub performance_monitor: PerformanceMonitor,
    // 阻塞检测
    pub blocking_detector: BlockingDetector,

    // 内部状态跟踪
    last_event_count: usize,
    last_message_count: u64,
    last_rate_update: Option<Instant>,
}

/// 事件处理监控
#[derive(Debug, Clone)]
pub struct EventProcessingMonitor {
    pub events_per_second: f64,
    pub last_event_processed: Option<Instant>,
    pub processing_latency_ms: f64,
    pub failed_events: u64,
    pub event_queue_size: usize,
    pub max_queue_size_reached: usize,
    pub event_types_processed: std::collections::HashMap<String, u64>,
}

/// WebSocket健康监控
#[derive(Debug, Clone)]
pub struct WebSocketHealthMonitor {
    pub connection_status: String,
    pub last_message_received: Option<Instant>,
    pub messages_per_second: f64,
    pub connection_uptime: Duration,
    pub reconnection_count: u32,
    pub last_ping_latency_ms: f64,
    pub consecutive_errors: u32,
    pub total_bytes_received: u64,
}

/// 缓冲区监控
#[derive(Debug, Clone)]
pub struct BufferMonitor {
    pub current_usage: usize,
    pub max_capacity: usize,
    pub usage_percentage: f64,
    pub peak_usage: usize,
    pub buffer_overflows: u64,
    pub average_usage: f64,
    pub usage_history: VecDeque<(Instant, usize)>, // 最近的使用历史
}

/// 性能监控
#[derive(Debug, Clone)]
pub struct PerformanceMonitor {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub event_loop_frequency_hz: f64,
    pub ui_render_frequency_hz: f64,
    pub last_successful_operation: Option<Instant>,
    pub operation_timeouts: u64,
}

/// 阻塞检测器
#[derive(Debug, Clone)]
pub struct BlockingDetector {
    pub is_blocked: bool,
    pub blocked_since: Option<Instant>,
    pub blocking_component: Option<String>,
    pub mutex_wait_times: std::collections::HashMap<String, Duration>,
    pub long_operations: Vec<LongOperation>,
    pub deadlock_detected: bool,
}

/// 长时间运行的操作记录
#[derive(Debug, Clone)]
pub struct LongOperation {
    pub operation_name: String,
    pub start_time: Instant,
    pub duration: Duration,
    pub is_still_running: bool,
}

impl InternalMonitor {
    pub fn new() -> Self {
        Self {
            event_processing: EventProcessingMonitor::new(),
            websocket_health: WebSocketHealthMonitor::new(),
            buffer_monitor: BufferMonitor::new(),
            performance_monitor: PerformanceMonitor::new(),
            blocking_detector: BlockingDetector::new(),
            last_event_count: 0,
            last_message_count: 0,
            last_rate_update: None,
        }
    }

    /// 更新事件处理统计
    pub fn update_event_processing(&mut self, events_processed: usize, queue_size: usize, event_types: &std::collections::HashMap<String, u64>) {
        let now = Instant::now();

        if events_processed > 0 {
            self.event_processing.last_event_processed = Some(now);
        }

        self.event_processing.event_queue_size = queue_size;
        self.event_processing.max_queue_size_reached = self.event_processing.max_queue_size_reached.max(queue_size);
        self.event_processing.event_types_processed = event_types.clone();

        // 计算事件处理速率 - 使用实例变量
        self.last_event_count += events_processed;

        if let Some(last_time) = self.last_rate_update {
            let time_diff = now.duration_since(last_time).as_secs_f64();
            if time_diff >= 1.0 { // 每秒更新一次速率
                self.event_processing.events_per_second = self.last_event_count as f64 / time_diff;
                self.last_event_count = 0;
                self.last_rate_update = Some(now);
            }
        } else {
            self.last_rate_update = Some(now);
        }
    }

    /// 更新WebSocket健康状态
    pub fn update_websocket_health(&mut self, connected: bool, messages_received: u64, reconnect_count: u32, ping_latency: f64) {
        self.websocket_health.connection_status = if connected { "已连接".to_string() } else { "断开连接".to_string() };

        // 更新消息接收时间和计算消息速率
        let now = Instant::now();
        if messages_received > 0 {
            self.websocket_health.last_message_received = Some(now);

            // 计算消息速率 - 使用实例变量
            if let Some(last_time) = self.last_rate_update {
                let time_diff = now.duration_since(last_time).as_secs_f64();
                if time_diff > 0.0 {
                    let message_diff = messages_received.saturating_sub(self.last_message_count);
                    self.websocket_health.messages_per_second = message_diff as f64 / time_diff;
                }
            }
            self.last_message_count = messages_received;
        }

        self.websocket_health.reconnection_count = reconnect_count;
        self.websocket_health.last_ping_latency_ms = ping_latency;
        self.websocket_health.total_bytes_received = messages_received; // 简化，使用消息数作为字节数的代理
    }

    /// 更新缓冲区使用情况
    pub fn update_buffer_usage(&mut self, current: usize, capacity: usize) {
        self.buffer_monitor.current_usage = current;
        self.buffer_monitor.max_capacity = capacity;
        self.buffer_monitor.usage_percentage = if capacity > 0 { (current as f64 / capacity as f64) * 100.0 } else { 0.0 };
        self.buffer_monitor.peak_usage = self.buffer_monitor.peak_usage.max(current);
        
        // 记录使用历史（保留最近100个记录）
        self.buffer_monitor.usage_history.push_back((Instant::now(), current));
        if self.buffer_monitor.usage_history.len() > 100 {
            self.buffer_monitor.usage_history.pop_front();
        }
        
        // 计算平均使用率
        if !self.buffer_monitor.usage_history.is_empty() {
            let sum: usize = self.buffer_monitor.usage_history.iter().map(|(_, usage)| *usage).sum();
            self.buffer_monitor.average_usage = sum as f64 / self.buffer_monitor.usage_history.len() as f64;
        }
    }

    /// 检测阻塞情况 - 增强版本，包含死锁检测
    pub fn detect_blocking(&mut self) {
        let now = Instant::now();

        // 检查是否有组件长时间没有活动
        let mut is_blocked = false;
        let mut blocking_component = None;

        // 检查事件处理是否停滞
        if let Some(last_event) = self.event_processing.last_event_processed {
            let stall_duration = now.duration_since(last_event);
            if stall_duration > Duration::from_secs(10) {
                is_blocked = true;
                blocking_component = Some("EventProcessing".to_string());

                // 检查是否可能是死锁
                if stall_duration > Duration::from_secs(60) {
                    self.blocking_detector.deadlock_detected = true;
                    log::error!("检测到可能的死锁：事件处理停滞超过60秒");
                }
            }
        }

        // 检查WebSocket是否停止接收消息
        if let Some(last_message) = self.websocket_health.last_message_received {
            let message_stall = now.duration_since(last_message);
            if message_stall > Duration::from_secs(30) {
                is_blocked = true;
                blocking_component = Some("WebSocket".to_string());

                // 检查是否可能是死锁
                if message_stall > Duration::from_secs(120) {
                    self.blocking_detector.deadlock_detected = true;
                    log::error!("检测到可能的死锁：WebSocket消息停滞超过120秒");
                }
            }
        }

        // 检查缓冲区是否满载且长时间未消费
        if self.buffer_monitor.usage_percentage > 95.0 {
            // 检查缓冲区使用历史，如果长时间保持高使用率可能是死锁
            let high_usage_duration = self.calculate_high_usage_duration();
            if high_usage_duration > Duration::from_secs(30) {
                is_blocked = true;
                blocking_component = Some("BufferOverflow".to_string());

                if high_usage_duration > Duration::from_secs(90) {
                    self.blocking_detector.deadlock_detected = true;
                    log::error!("检测到可能的死锁：缓冲区长时间满载");
                }
            }
        }

        // 更新阻塞状态
        if is_blocked && !self.blocking_detector.is_blocked {
            self.blocking_detector.is_blocked = true;
            self.blocking_detector.blocked_since = Some(now);
            self.blocking_detector.blocking_component = blocking_component.clone();
            log::warn!("检测到系统阻塞: {:?}", blocking_component);
        } else if !is_blocked {
            if self.blocking_detector.is_blocked {
                log::info!("系统阻塞已恢复");
            }
            self.blocking_detector.is_blocked = false;
            self.blocking_detector.blocked_since = None;
            self.blocking_detector.blocking_component = None;
            self.blocking_detector.deadlock_detected = false;
        }
    }

    /// 计算缓冲区高使用率持续时间
    fn calculate_high_usage_duration(&self) -> Duration {
        let now = Instant::now();
        let mut high_usage_start = None;

        // 从历史记录中找到最早的高使用率时间点
        for (timestamp, usage) in &self.buffer_monitor.usage_history {
            let usage_percentage = if self.buffer_monitor.max_capacity > 0 {
                (*usage as f64 / self.buffer_monitor.max_capacity as f64) * 100.0
            } else {
                0.0
            };

            if usage_percentage > 95.0 {
                if high_usage_start.is_none() {
                    high_usage_start = Some(*timestamp);
                }
            } else {
                high_usage_start = None; // 重置，因为使用率下降了
            }
        }

        if let Some(start_time) = high_usage_start {
            now.duration_since(start_time)
        } else {
            Duration::from_secs(0)
        }
    }

    /// 尝试自动恢复
    pub fn attempt_recovery(&mut self) -> bool {
        if !self.blocking_detector.is_blocked {
            return true; // 没有阻塞，不需要恢复
        }

        let recovery_action = match self.blocking_detector.blocking_component.as_deref() {
            Some("EventProcessing") => {
                log::info!("尝试恢复事件处理系统");
                // 这里可以实现事件处理系统的重启逻辑
                "重启事件处理器"
            }
            Some("WebSocket") => {
                log::info!("尝试恢复WebSocket连接");
                // 这里可以实现WebSocket重连逻辑
                "重连WebSocket"
            }
            Some("BufferOverflow") => {
                log::info!("尝试清理缓冲区");
                // 这里可以实现缓冲区清理逻辑
                "清理缓冲区"
            }
            _ => {
                log::warn!("未知的阻塞组件，无法自动恢复");
                return false;
            }
        };

        log::info!("执行恢复操作: {}", recovery_action);

        // 重置阻塞状态，给系统一个恢复的机会
        self.blocking_detector.is_blocked = false;
        self.blocking_detector.blocked_since = None;
        self.blocking_detector.blocking_component = None;

        true
    }

    /// 获取健康状态摘要
    pub fn get_health_summary(&self) -> String {
        if self.blocking_detector.is_blocked {
            format!("⚠️ 检测到阻塞: {}", 
                self.blocking_detector.blocking_component.as_ref().unwrap_or(&"未知".to_string()))
        } else if self.buffer_monitor.usage_percentage > 90.0 {
            "⚠️ 缓冲区使用率过高".to_string()
        } else if self.websocket_health.consecutive_errors > 5 {
            "⚠️ WebSocket连接不稳定".to_string()
        } else {
            "✅ 系统运行正常".to_string()
        }
    }
}

impl EventProcessingMonitor {
    pub fn new() -> Self {
        Self {
            events_per_second: 0.0,
            last_event_processed: None,
            processing_latency_ms: 0.0,
            failed_events: 0,
            event_queue_size: 0,
            max_queue_size_reached: 0,
            event_types_processed: std::collections::HashMap::new(),
        }
    }
}

impl WebSocketHealthMonitor {
    pub fn new() -> Self {
        Self {
            connection_status: "未连接".to_string(),
            last_message_received: None,
            messages_per_second: 0.0,
            connection_uptime: Duration::from_secs(0),
            reconnection_count: 0,
            last_ping_latency_ms: 0.0,
            consecutive_errors: 0,
            total_bytes_received: 0,
        }
    }
}

impl BufferMonitor {
    pub fn new() -> Self {
        Self {
            current_usage: 0,
            max_capacity: 0,
            usage_percentage: 0.0,
            peak_usage: 0,
            buffer_overflows: 0,
            average_usage: 0.0,
            usage_history: VecDeque::new(),
        }
    }
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0.0,
            event_loop_frequency_hz: 0.0,
            ui_render_frequency_hz: 0.0,
            last_successful_operation: Some(Instant::now()),
            operation_timeouts: 0,
        }
    }
}

impl BlockingDetector {
    pub fn new() -> Self {
        Self {
            is_blocked: false,
            blocked_since: None,
            blocking_component: None,
            mutex_wait_times: std::collections::HashMap::new(),
            long_operations: Vec::new(),
            deadlock_detected: false,
        }
    }
}
