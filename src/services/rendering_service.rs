use super::{Service, ServiceError, ServiceHealth, ServiceStats, ConfigurableService};
use crate::core::PerformanceMetrics;
use std::sync::{Arc, RwLock, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// 渲染服务 - 负责GUI渲染和显示逻辑
pub struct RenderingService {
    /// 服务配置
    config: Arc<RwLock<RenderingConfig>>,
    /// 运行状态
    is_running: AtomicBool,
    /// 启动时间
    start_time: Option<Instant>,
    /// 统计信息
    stats: RenderingStats,
    /// 渲染队列
    render_queue: Arc<RwLock<RenderingQueue>>,
    /// 帧率控制器
    frame_controller: Arc<RwLock<FrameRateController>>,
    /// 异步任务句柄
    task_handles: Vec<tokio::task::JoinHandle<()>>,
    /// 性能指标
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
}

/// 渲染配置
#[derive(Debug, Clone)]
pub struct RenderingConfig {
    /// 目标帧率
    pub target_fps: u32,
    /// 最小帧率
    pub min_fps: u32,
    /// 最大帧率
    pub max_fps: u32,
    /// 自适应帧率
    pub adaptive_fps: bool,
    /// 垂直同步
    pub vsync: bool,
    /// 批渲染大小
    pub batch_size: usize,
    /// GPU加速
    pub gpu_acceleration: bool,
    /// 渲染线程数
    pub render_threads: usize,
    /// 最大队列大小
    pub max_queue_size: usize,
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            target_fps: 30,      // 默认30FPS，平衡性能和流畅度
            min_fps: 15,         // 最低15FPS
            max_fps: 60,         // 最高60FPS
            adaptive_fps: true,  // 启用自适应帧率
            vsync: false,        // 禁用垂直同步以减少延迟
            batch_size: 100,     // 批渲染100个命令
            gpu_acceleration: true,
            render_threads: 2,   // 2个渲染线程
            max_queue_size: 1000,
        }
    }
}

/// 渲染统计信息
#[derive(Debug)]
struct RenderingStats {
    /// 渲染帧数
    frames_rendered: AtomicU64,
    /// 跳过的帧数
    frames_dropped: AtomicU64,
    /// 渲染命令数
    commands_processed: AtomicU64,
    /// 错误计数
    error_count: AtomicU64,
    /// 平均帧时间
    avg_frame_time: Arc<RwLock<f64>>,
    /// 当前FPS
    current_fps: Arc<RwLock<f64>>,
    /// GPU内存使用
    gpu_memory_usage: AtomicU64,
}

impl Default for RenderingStats {
    fn default() -> Self {
        Self {
            frames_rendered: AtomicU64::new(0),
            frames_dropped: AtomicU64::new(0),
            commands_processed: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            avg_frame_time: Arc::new(RwLock::new(0.0)),
            current_fps: Arc::new(RwLock::new(0.0)),
            gpu_memory_usage: AtomicU64::new(0),
        }
    }
}

/// 渲染队列
pub struct RenderingQueue {
    /// 渲染命令队列
    commands: Vec<RenderCommand>,
    /// 队列大小限制
    max_size: usize,
    /// 优先级队列
    priority_commands: Vec<RenderCommand>,
}

/// 渲染命令
#[derive(Debug, Clone)]
pub enum RenderCommand {
    /// 清空屏幕
    Clear { color: [f32; 4] },
    /// 绘制表格
    DrawTable { 
        data: TableData,
        position: Position,
        size: Size,
    },
    /// 绘制图表
    DrawChart {
        data: ChartData,
        position: Position,
        size: Size,
    },
    /// 绘制文本
    DrawText {
        text: String,
        position: Position,
        font_size: f32,
        color: [f32; 4],
    },
    /// 绘制形状
    DrawShape {
        shape: Shape,
        position: Position,
        color: [f32; 4],
    },
    /// 批量命令
    Batch(Vec<RenderCommand>),
}

/// 表格数据
#[derive(Debug, Clone)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub cell_colors: Option<Vec<Vec<[f32; 4]>>>,
}

/// 图表数据
#[derive(Debug, Clone)]
pub struct ChartData {
    pub points: Vec<(f64, f64)>,
    pub chart_type: ChartType,
    pub color: [f32; 4],
    pub line_width: f32,
}

/// 图表类型
#[derive(Debug, Clone)]
pub enum ChartType {
    Line,
    Bar,
    Candlestick,
}

/// 形状类型
#[derive(Debug, Clone)]
pub enum Shape {
    Rectangle { width: f32, height: f32 },
    Circle { radius: f32 },
    Line { start: Position, end: Position },
}

/// 位置
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

/// 尺寸
#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

/// 帧率控制器
pub struct FrameRateController {
    /// 目标帧时间
    target_frame_time: Duration,
    /// 上一帧时间
    last_frame_time: Instant,
    /// 帧时间历史
    frame_times: std::collections::VecDeque<Duration>,
    /// 自适应调整
    adaptive_enabled: bool,
    /// 当前FPS
    current_fps: f64,
}

impl RenderingService {
    /// 创建新的渲染服务
    pub fn new(config: RenderingConfig) -> Self {
        let frame_controller = FrameRateController::new(config.target_fps, config.adaptive_fps);
        
        Self {
            config: Arc::new(RwLock::new(config)),
            is_running: AtomicBool::new(false),
            start_time: None,
            stats: RenderingStats::default(),
            render_queue: Arc::new(RwLock::new(RenderingQueue::new(1000))),
            frame_controller: Arc::new(RwLock::new(frame_controller)),
            task_handles: Vec::new(),
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
        }
    }

    /// 提交渲染命令
    pub async fn submit_command(&self, command: RenderCommand) -> Result<(), ServiceError> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        let mut queue = self.render_queue.write().unwrap();
        if queue.commands.len() >= queue.max_size {
            // 队列已满，丢弃最旧的命令
            queue.commands.remove(0);
            self.stats.frames_dropped.fetch_add(1, Ordering::Relaxed);
        }

        queue.commands.push(command);
        Ok(())
    }

    /// 提交优先级渲染命令
    pub async fn submit_priority_command(&self, command: RenderCommand) -> Result<(), ServiceError> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        let mut queue = self.render_queue.write().unwrap();
        queue.priority_commands.push(command);
        Ok(())
    }

    /// 批量提交渲染命令
    pub async fn submit_commands_batch(&self, commands: Vec<RenderCommand>) -> Result<(), ServiceError> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        let batch_command = RenderCommand::Batch(commands);
        self.submit_command(batch_command).await
    }

    /// 获取当前FPS
    pub fn get_current_fps(&self) -> f64 {
        *self.stats.current_fps.read().unwrap()
    }

    /// 设置目标FPS
    pub fn set_target_fps(&self, fps: u32) {
        let mut config = self.config.write().unwrap();
        config.target_fps = fps;
        
        let mut controller = self.frame_controller.write().unwrap();
        controller.set_target_fps(fps);
    }

    /// 渲染下一帧
    pub async fn render_frame(&self) -> Result<RenderFrameResult, ServiceError> {
        let start_time = Instant::now();
        
        // 检查帧率控制
        let should_render = {
            let mut controller = self.frame_controller.write().unwrap();
            controller.should_render_frame()
        };
        
        if !should_render {
            return Ok(RenderFrameResult {
                rendered: false,
                frame_time: start_time.elapsed(),
                commands_processed: 0,
            });
        }

        // 获取渲染命令
        let commands = {
            let mut queue = self.render_queue.write().unwrap();
            self.collect_render_commands(&mut queue)
        };

        if commands.is_empty() {
            return Ok(RenderFrameResult {
                rendered: false,
                frame_time: start_time.elapsed(),
                commands_processed: 0,
            });
        }

        // 执行渲染命令
        let commands_count = commands.len();
        let render_result = self.execute_render_commands(commands).await?;

        // 更新统计信息
        let frame_time = start_time.elapsed();
        {
            let mut controller = self.frame_controller.write().unwrap();
            controller.record_frame_time(frame_time);
        }
        self.update_frame_stats(frame_time);
        self.stats.frames_rendered.fetch_add(1, Ordering::Relaxed);
        self.stats.commands_processed.fetch_add(commands_count as u64, Ordering::Relaxed);

        Ok(RenderFrameResult {
            rendered: true,
            frame_time,
            commands_processed: commands_count,
        })
    }

    /// 收集渲染命令
    fn collect_render_commands(&self, queue: &mut RenderingQueue) -> Vec<RenderCommand> {
        let mut commands = Vec::new();
        
        // 优先处理优先级命令
        commands.extend(queue.priority_commands.drain(..));
        
        // 处理普通命令
        let batch_size = self.config.read().unwrap().batch_size;
        let take_count = queue.commands.len().min(batch_size);
        commands.extend(queue.commands.drain(..take_count));
        
        commands
    }

    /// 执行渲染命令
    async fn execute_render_commands(&self, commands: Vec<RenderCommand>) -> Result<(), ServiceError> {
        // 在后台线程执行渲染
        let config = self.config.clone();
        
        tokio::task::spawn_blocking(move || {
            let gpu_acceleration = config.read().unwrap().gpu_acceleration;
            
            for command in commands {
                if let Err(e) = Self::execute_single_command(command, gpu_acceleration) {
                    log::warn!("渲染命令执行失败: {}", e);
                }
            }
        }).await.map_err(|e| ServiceError::InternalError(format!("渲染任务失败: {}", e)))?;
        
        Ok(())
    }

    /// 执行单个渲染命令
    fn execute_single_command(command: RenderCommand, gpu_acceleration: bool) -> Result<(), String> {
        match command {
            RenderCommand::Clear { color } => {
                // 清空屏幕实现
                log::debug!("清空屏幕，颜色: {:?}", color);
                Ok(())
            }
            RenderCommand::DrawTable { data, position, size } => {
                // 绘制表格实现
                log::debug!("绘制表格 at {:?}, size: {:?}", position, size);
                Ok(())
            }
            RenderCommand::DrawChart { data, position, size } => {
                // 绘制图表实现
                log::debug!("绘制图表 at {:?}, size: {:?}", position, size);
                Ok(())
            }
            RenderCommand::DrawText { text, position, font_size, color } => {
                // 绘制文本实现
                log::debug!("绘制文本: '{}' at {:?}", text, position);
                Ok(())
            }
            RenderCommand::DrawShape { shape, position, color } => {
                // 绘制形状实现
                log::debug!("绘制形状 {:?} at {:?}", shape, position);
                Ok(())
            }
            RenderCommand::Batch(commands) => {
                // 批量执行命令
                for cmd in commands {
                    Self::execute_single_command(cmd, gpu_acceleration)?;
                }
                Ok(())
            }
        }
    }

    /// 更新帧统计信息
    fn update_frame_stats(&self, frame_time: Duration) {
        let frame_time_ms = frame_time.as_secs_f64() * 1000.0;
        
        // 更新平均帧时间
        let mut avg_frame_time = self.stats.avg_frame_time.write().unwrap();
        let frames_rendered = self.stats.frames_rendered.load(Ordering::Relaxed);
        
        if frames_rendered == 0 {
            *avg_frame_time = frame_time_ms;
        } else {
            *avg_frame_time = (*avg_frame_time * frames_rendered as f64 + frame_time_ms) / (frames_rendered + 1) as f64;
        }
        
        // 更新当前FPS
        if frame_time_ms > 0.0 {
            let current_fps = 1000.0 / frame_time_ms;
            *self.stats.current_fps.write().unwrap() = current_fps;
        }
    }
}

impl Service for RenderingService {
    fn name(&self) -> &'static str {
        "RenderingService"
    }

    fn start(&mut self) -> Result<(), ServiceError> {
        if self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::AlreadyRunning);
        }

        // 启动渲染循环
        self.start_render_loop();
        
        self.is_running.store(true, Ordering::Relaxed);
        self.start_time = Some(Instant::now());
        
        log::info!("渲染服务已启动");
        Ok(())
    }

    fn stop(&mut self) -> Result<(), ServiceError> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(ServiceError::NotRunning);
        }

        // 停止渲染循环
        self.stop_render_loop();
        
        self.is_running.store(false, Ordering::Relaxed);
        
        log::info!("渲染服务已停止");
        Ok(())
    }

    fn health_check(&self) -> ServiceHealth {
        if !self.is_running.load(Ordering::Relaxed) {
            return ServiceHealth::Unhealthy("渲染服务未运行".to_string());
        }

        let current_fps = *self.stats.current_fps.read().unwrap();
        let target_fps = self.config.read().unwrap().target_fps as f64;
        let frames_dropped = self.stats.frames_dropped.load(Ordering::Relaxed);
        let total_frames = self.stats.frames_rendered.load(Ordering::Relaxed);

        let drop_rate = if total_frames > 0 {
            frames_dropped as f64 / total_frames as f64
        } else {
            0.0
        };

        if current_fps < target_fps * 0.5 {
            ServiceHealth::Unhealthy(format!("FPS过低: {:.1} (目标: {:.1})", current_fps, target_fps))
        } else if drop_rate > 0.1 {
            ServiceHealth::Warning(format!("丢帧率过高: {:.1}%", drop_rate * 100.0))
        } else if current_fps < target_fps * 0.8 {
            ServiceHealth::Warning(format!("FPS略低: {:.1} (目标: {:.1})", current_fps, target_fps))
        } else {
            ServiceHealth::Healthy
        }
    }

    fn stats(&self) -> ServiceStats {
        ServiceStats {
            service_name: self.name().to_string(),
            is_running: self.is_running.load(Ordering::Relaxed),
            start_time: self.start_time,
            requests_processed: self.stats.commands_processed.load(Ordering::Relaxed),
            error_count: self.stats.error_count.load(Ordering::Relaxed),
            avg_response_time_ms: *self.stats.avg_frame_time.read().unwrap(),
            memory_usage_bytes: self.stats.gpu_memory_usage.load(Ordering::Relaxed) as usize,
        }
    }
}

impl ConfigurableService for RenderingService {
    type Config = RenderingConfig;

    fn update_config(&mut self, config: RenderingConfig) -> Result<(), ServiceError> {
        // 更新帧率控制器
        {
            let mut controller = self.frame_controller.write().unwrap();
            controller.set_target_fps(config.target_fps);
            controller.adaptive_enabled = config.adaptive_fps;
        }
        
        // 更新配置
        *self.config.write().unwrap() = config;
        
        Ok(())
    }

    fn get_config(&self) -> &Self::Config {
        unsafe { &*self.config.as_ptr() }
    }
}

impl RenderingService {
    /// 启动渲染循环
    fn start_render_loop(&mut self) {
        let render_service = Arc::new(std::sync::Mutex::new(self));
        
        // 主渲染循环
        let service_clone = render_service.clone();
        let render_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(16)); // 60 FPS基础间隔
            
            loop {
                interval.tick().await;
                
                let is_running = {
                    let service = service_clone.lock().unwrap();
                    service.is_running.load(Ordering::Relaxed)
                };
                
                if !is_running {
                    break;
                }
                
                // 获取service并调用render_frame，然后立即释放锁
                let render_result = {
                    let service = service_clone.lock().unwrap();
                    service.render_frame()
                }.await;
                
                if let Err(e) = render_result {
                    log::warn!("渲染帧失败: {}", e);
                }
            }
        });
        
        self.task_handles.push(render_task);
    }

    /// 停止渲染循环
    fn stop_render_loop(&mut self) {
        for handle in self.task_handles.drain(..) {
            handle.abort();
        }
    }
}

/// 渲染帧结果
#[derive(Debug)]
pub struct RenderFrameResult {
    /// 是否实际渲染了帧
    pub rendered: bool,
    /// 帧时间
    pub frame_time: Duration,
    /// 处理的命令数量
    pub commands_processed: usize,
}

impl RenderingQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            commands: Vec::with_capacity(max_size),
            max_size,
            priority_commands: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.commands.len() + self.priority_commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty() && self.priority_commands.is_empty()
    }
}

impl FrameRateController {
    pub fn new(target_fps: u32, adaptive: bool) -> Self {
        Self {
            target_frame_time: Duration::from_millis(1000 / target_fps as u64),
            last_frame_time: Instant::now(),
            frame_times: std::collections::VecDeque::with_capacity(60),
            adaptive_enabled: adaptive,
            current_fps: 0.0,
        }
    }

    pub fn should_render_frame(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time);
        
        if elapsed >= self.target_frame_time {
            self.last_frame_time = now;
            true
        } else {
            false
        }
    }

    pub fn record_frame_time(&mut self, frame_time: Duration) {
        self.frame_times.push_back(frame_time);
        if self.frame_times.len() > 60 {
            self.frame_times.pop_front();
        }

        // 计算平均FPS
        if !self.frame_times.is_empty() {
            let avg_frame_time: Duration = self.frame_times.iter().sum::<Duration>() / self.frame_times.len() as u32;
            self.current_fps = 1.0 / avg_frame_time.as_secs_f64();
        }

        // 自适应调整
        if self.adaptive_enabled {
            self.adjust_frame_rate();
        }
    }

    pub fn set_target_fps(&mut self, fps: u32) {
        self.target_frame_time = Duration::from_millis(1000 / fps as u64);
    }

    fn adjust_frame_rate(&mut self) {
        if self.frame_times.len() < 10 {
            return;
        }

        let recent_avg: Duration = self.frame_times.iter().rev().take(10).sum::<Duration>() / 10;
        
        // 如果渲染时间过长，降低帧率
        if recent_avg > self.target_frame_time * 2 {
            let new_target = self.target_frame_time + Duration::from_millis(5);
            self.target_frame_time = new_target.min(Duration::from_millis(66)); // 最低15 FPS
        }
        // 如果渲染时间很短，提高帧率
        else if recent_avg < self.target_frame_time / 2 {
            let new_target = self.target_frame_time.saturating_sub(Duration::from_millis(2));
            self.target_frame_time = new_target.max(Duration::from_millis(16)); // 最高60 FPS
        }
    }
}