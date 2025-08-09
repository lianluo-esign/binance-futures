// Provider Launcher - Provider动态启动管理器
//
// 本模块实现了Provider的动态启动和管理功能，包括：
// - 异步Provider启动流程
// - 启动进度跟踪和回调
// - 错误处理和恢复机制
// - Provider生命周期管理
//
// 设计原则：
// 1. 异步友好：支持非阻塞启动流程
// 2. 进度透明：提供详细的启动进度反馈
// 3. 错误恢复：完善的错误处理和重试机制
// 4. 资源管理：正确的资源分配和清理

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;

use super::{AnyProvider, DataProvider, ProviderError, ProviderResult};
use super::provider_selector::{ProviderSelector, ProviderOption};
use crate::events::LockFreeEventDispatcher;

/// Provider启动器
/// 
/// 负责协调Provider的异步启动流程，包括初始化、配置验证、连接建立等步骤
pub struct ProviderLauncher {
    /// 事件分发器
    event_dispatcher: Arc<Mutex<LockFreeEventDispatcher>>,
    /// 启动进度发送器
    progress_sender: Option<mpsc::UnboundedSender<LaunchProgress>>,
    /// 当前启动的Provider
    current_provider: Option<Arc<Mutex<AnyProvider>>>,
    /// 启动配置
    launch_config: LaunchConfig,
}

/// 启动配置
#[derive(Debug, Clone)]
pub struct LaunchConfig {
    /// 连接超时时间（秒）
    pub connection_timeout: u64,
    /// 最大重试次数
    pub max_retry_attempts: usize,
    /// 重试间隔（毫秒）
    pub retry_interval_ms: u64,
    /// 启动后等待时间（毫秒）
    pub post_launch_wait_ms: u64,
    /// 是否启用详细日志
    pub verbose_logging: bool,
}

/// 启动进度信息
#[derive(Debug, Clone)]
pub struct LaunchProgress {
    /// 当前步骤描述
    pub step_description: String,
    /// 进度百分比 (0.0 - 1.0)
    pub progress_percent: f64,
    /// 当前步骤索引
    pub current_step: usize,
    /// 总步骤数
    pub total_steps: usize,
    /// 步骤开始时间
    pub step_start_time: Instant,
    /// 预估剩余时间（毫秒）
    pub estimated_remaining_ms: Option<u64>,
}

/// 启动结果
#[derive(Debug)]
pub struct LaunchResult {
    /// 启动的Provider
    pub provider: Arc<Mutex<AnyProvider>>,
    /// 启动耗时
    pub launch_duration: Duration,
    /// 启动步骤日志
    pub launch_log: Vec<LaunchLogEntry>,
}

/// 启动日志条目
#[derive(Debug, Clone)]
pub struct LaunchLogEntry {
    /// 时间戳
    pub timestamp: Instant,
    /// 日志级别
    pub level: LogLevel,
    /// 消息内容
    pub message: String,
    /// 关联的启动步骤
    pub step: String,
}

/// 日志级别
#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// 启动步骤枚举
#[derive(Debug, Clone, PartialEq)]
enum LaunchStep {
    PreparingLaunch,
    ValidatingConfig,
    CreatingProvider,
    InitializingProvider,
    StartingProvider,
    VerifyingConnection,
    PostLaunchSetup,
    Completed,
}

impl LaunchStep {
    fn description(&self) -> &'static str {
        match self {
            LaunchStep::PreparingLaunch => "准备启动环境...",
            LaunchStep::ValidatingConfig => "验证配置文件...",
            LaunchStep::CreatingProvider => "创建Provider实例...",
            LaunchStep::InitializingProvider => "初始化Provider...",
            LaunchStep::StartingProvider => "启动Provider...",
            LaunchStep::VerifyingConnection => "验证连接状态...",
            LaunchStep::PostLaunchSetup => "完成启动后设置...",
            LaunchStep::Completed => "启动完成",
        }
    }
    
    fn progress_weight(&self) -> f64 {
        match self {
            LaunchStep::PreparingLaunch => 0.05,
            LaunchStep::ValidatingConfig => 0.10,
            LaunchStep::CreatingProvider => 0.15,
            LaunchStep::InitializingProvider => 0.25,
            LaunchStep::StartingProvider => 0.30,
            LaunchStep::VerifyingConnection => 0.10,
            LaunchStep::PostLaunchSetup => 0.05,
            LaunchStep::Completed => 0.0,
        }
    }
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            connection_timeout: 30,
            max_retry_attempts: 3,
            retry_interval_ms: 2000,
            post_launch_wait_ms: 1000,
            verbose_logging: true,
        }
    }
}

impl ProviderLauncher {
    /// 创建新的Provider启动器
    pub fn new(event_dispatcher: Arc<Mutex<LockFreeEventDispatcher>>) -> Self {
        Self {
            event_dispatcher,
            progress_sender: None,
            current_provider: None,
            launch_config: LaunchConfig::default(),
        }
    }
    
    /// 设置启动配置
    pub fn with_config(mut self, config: LaunchConfig) -> Self {
        self.launch_config = config;
        self
    }
    
    /// 设置进度回调
    pub fn set_progress_callback(&mut self, sender: mpsc::UnboundedSender<LaunchProgress>) {
        self.progress_sender = Some(sender);
    }
    
    /// 启动Provider（异步）
    pub async fn launch_provider_async(
        &mut self,
        selector: &ProviderSelector,
    ) -> ProviderResult<LaunchResult> {
        let start_time = Instant::now();
        let mut launch_log = Vec::new();
        
        // 获取选中的Provider选项
        let selected_option = selector.get_selected_option()
            .ok_or_else(|| ProviderError::state(
                "没有选中的Provider",
                "no_selection",
                "provider_selected",
                "launch_provider"
            ))?;
        
        self.log_step(&mut launch_log, LogLevel::Info, 
            &format!("开始启动Provider: {}", selected_option.name),
            "launch_start");
        
        // 执行启动步骤
        let steps = vec![
            LaunchStep::PreparingLaunch,
            LaunchStep::ValidatingConfig,
            LaunchStep::CreatingProvider,
            LaunchStep::InitializingProvider,
            LaunchStep::StartingProvider,
            LaunchStep::VerifyingConnection,
            LaunchStep::PostLaunchSetup,
        ];
        
        let mut cumulative_progress = 0.0;
        
        for (index, step) in steps.iter().enumerate() {
            let step_start = Instant::now();
            let step_progress = LaunchProgress {
                step_description: step.description().to_string(),
                progress_percent: cumulative_progress,
                current_step: index + 1,
                total_steps: steps.len(),
                step_start_time: step_start,
                estimated_remaining_ms: None,
            };
            
            self.send_progress(step_progress);
            
            self.log_step(&mut launch_log, LogLevel::Info, 
                step.description(), &format!("step_{:?}", step));
            
            // 执行具体步骤
            match step {
                LaunchStep::PreparingLaunch => {
                    self.execute_prepare_launch(selected_option, &mut launch_log).await?;
                }
                LaunchStep::ValidatingConfig => {
                    self.execute_validate_config(selected_option, &mut launch_log).await?;
                }
                LaunchStep::CreatingProvider => {
                    let provider = self.execute_create_provider(selector, &mut launch_log).await?;
                    self.current_provider = Some(Arc::new(Mutex::new(provider)));
                }
                LaunchStep::InitializingProvider => {
                    self.execute_initialize_provider(&mut launch_log).await?;
                }
                LaunchStep::StartingProvider => {
                    self.execute_start_provider(&mut launch_log).await?;
                }
                LaunchStep::VerifyingConnection => {
                    self.execute_verify_connection(&mut launch_log).await?;
                }
                LaunchStep::PostLaunchSetup => {
                    self.execute_post_launch_setup(&mut launch_log).await?;
                }
                LaunchStep::Completed => {}
            }
            
            cumulative_progress += step.progress_weight();
            
            // 短暂延迟以提供视觉反馈
            if self.launch_config.verbose_logging {
                sleep(Duration::from_millis(200)).await;
            }
        }
        
        // 最终进度更新
        let final_progress = LaunchProgress {
            step_description: "启动完成！".to_string(),
            progress_percent: 1.0,
            current_step: steps.len() + 1,
            total_steps: steps.len(),
            step_start_time: Instant::now(),
            estimated_remaining_ms: Some(0),
        };
        self.send_progress(final_progress);
        
        let launch_duration = start_time.elapsed();
        self.log_step(&mut launch_log, LogLevel::Info, 
            &format!("Provider启动完成，耗时: {:?}", launch_duration),
            "launch_complete");
        
        // 等待一段时间确保Provider稳定
        sleep(Duration::from_millis(self.launch_config.post_launch_wait_ms)).await;
        
        Ok(LaunchResult {
            provider: self.current_provider.as_ref().unwrap().clone(),
            launch_duration,
            launch_log,
        })
    }
    
    /// 执行准备启动步骤
    async fn execute_prepare_launch(
        &self,
        _option: &ProviderOption,
        launch_log: &mut Vec<LaunchLogEntry>,
    ) -> ProviderResult<()> {
        self.log_step(launch_log, LogLevel::Debug, 
            "检查系统资源和依赖", "prepare");
        
        // 准备工作（移除模拟延迟）
        // sleep(Duration::from_millis(300)).await;
        
        self.log_step(launch_log, LogLevel::Info, 
            "系统准备完成", "prepare_done");
        
        Ok(())
    }
    
    /// 执行配置验证步骤
    async fn execute_validate_config(
        &self,
        option: &ProviderOption,
        launch_log: &mut Vec<LaunchLogEntry>,
    ) -> ProviderResult<()> {
        self.log_step(launch_log, LogLevel::Debug, 
            &format!("验证{}配置", option.name), "validate");
        
        if !option.enabled {
            let error_msg = format!("Provider配置无效: {}", option.status);
            self.log_step(launch_log, LogLevel::Error, &error_msg, "validate_error");
            return Err(ProviderError::configuration(error_msg));
        }
        
        // 配置验证（移除模拟延迟）
        // sleep(Duration::from_millis(500)).await;
        
        self.log_step(launch_log, LogLevel::Info, 
            "配置验证通过", "validate_ok");
        
        Ok(())
    }
    
    /// 执行创建Provider步骤
    async fn execute_create_provider(
        &self,
        selector: &ProviderSelector,
        launch_log: &mut Vec<LaunchLogEntry>,
    ) -> ProviderResult<AnyProvider> {
        self.log_step(launch_log, LogLevel::Debug, 
            "创建Provider实例", "create");
        
        let provider = selector.launch_selected_provider()?;
        
        self.log_step(launch_log, LogLevel::Info, 
            &format!("Provider实例创建成功: {:?}", provider.provider_type()), 
            "create_ok");
        
        Ok(provider)
    }
    
    /// 执行初始化Provider步骤
    async fn execute_initialize_provider(
        &self,
        launch_log: &mut Vec<LaunchLogEntry>,
    ) -> ProviderResult<()> {
        if let Some(provider_arc) = &self.current_provider {
            self.log_step(launch_log, LogLevel::Debug, 
                "初始化Provider", "initialize");
            
            let mut provider = provider_arc.lock().await;
            
            // 尝试初始化，带重试机制
            let mut attempts = 0;
            let max_attempts = self.launch_config.max_retry_attempts;
            
            while attempts < max_attempts {
                match provider.initialize() {
                    Ok(()) => {
                        self.log_step(launch_log, LogLevel::Info, 
                            "Provider初始化成功", "initialize_ok");
                        return Ok(());
                    }
                    Err(e) => {
                        attempts += 1;
                        let error_msg = format!("初始化失败 (尝试 {}/{}): {}", 
                                              attempts, max_attempts, e);
                        self.log_step(launch_log, LogLevel::Warn, &error_msg, "initialize_retry");
                        
                        if attempts < max_attempts {
                            sleep(Duration::from_millis(self.launch_config.retry_interval_ms)).await;
                        } else {
                            self.log_step(launch_log, LogLevel::Error, 
                                "Provider初始化最终失败", "initialize_failed");
                            return Err(ProviderError::internal(
                                format!("初始化失败，已重试{}次: {}", max_attempts, e),
                                "ProviderLauncher"
                            ));
                        }
                    }
                }
            }
        }
        
        Err(ProviderError::state(
            "没有可用的Provider实例",
            "no_provider",
            "provider_created",
            "initialize_provider"
        ))
    }
    
    /// 执行启动Provider步骤
    async fn execute_start_provider(
        &self,
        launch_log: &mut Vec<LaunchLogEntry>,
    ) -> ProviderResult<()> {
        if let Some(provider_arc) = &self.current_provider {
            self.log_step(launch_log, LogLevel::Debug, 
                "启动Provider服务", "start");
            
            let mut provider = provider_arc.lock().await;
            provider.start()?;
            
            self.log_step(launch_log, LogLevel::Info, 
                "Provider服务启动成功", "start_ok");
            
            return Ok(());
        }
        
        Err(ProviderError::state(
            "没有可用的Provider实例",
            "no_provider",
            "provider_initialized",
            "start_provider"
        ))
    }
    
    /// 执行验证连接步骤
    async fn execute_verify_connection(
        &self,
        launch_log: &mut Vec<LaunchLogEntry>,
    ) -> ProviderResult<()> {
        if let Some(provider_arc) = &self.current_provider {
            self.log_step(launch_log, LogLevel::Debug, 
                "验证Provider连接状态", "verify");
            
            // 等待连接建立
            let timeout_duration = Duration::from_secs(self.launch_config.connection_timeout);
            let start_time = Instant::now();
            
            while start_time.elapsed() < timeout_duration {
                let provider = provider_arc.lock().await;
                if provider.is_connected() {
                    self.log_step(launch_log, LogLevel::Info, 
                        "Provider连接验证成功", "verify_ok");
                    return Ok(());
                }
                drop(provider);
                
                sleep(Duration::from_millis(100)).await; // 减少验证间隔
            }
            
            let error_msg = format!("连接验证超时 ({}秒)", self.launch_config.connection_timeout);
            self.log_step(launch_log, LogLevel::Error, &error_msg, "verify_timeout");
            return Err(ProviderError::connection(
                format!("连接验证失败: {}", error_msg),
                Some(error_msg),
                false
            ));
        }
        
        Err(ProviderError::state(
            "没有可用的Provider实例",
            "no_provider",
            "provider_started",
            "verify_connection"
        ))
    }
    
    /// 执行启动后设置步骤
    async fn execute_post_launch_setup(
        &self,
        launch_log: &mut Vec<LaunchLogEntry>,
    ) -> ProviderResult<()> {
        self.log_step(launch_log, LogLevel::Debug, 
            "执行启动后设置", "post_setup");
        
        // 可以在这里添加其他初始化逻辑
        // 例如：注册事件监听器、设置定时任务等
        
        self.log_step(launch_log, LogLevel::Info, 
            "启动后设置完成", "post_setup_ok");
        
        Ok(())
    }
    
    /// 发送进度更新
    fn send_progress(&self, progress: LaunchProgress) {
        if let Some(sender) = &self.progress_sender {
            if let Err(_) = sender.send(progress) {
                log::warn!("无法发送进度更新，接收器可能已关闭");
            }
        }
    }
    
    /// 记录启动步骤日志
    fn log_step(
        &self,
        launch_log: &mut Vec<LaunchLogEntry>,
        level: LogLevel,
        message: &str,
        step: &str,
    ) {
        let entry = LaunchLogEntry {
            timestamp: Instant::now(),
            level: level.clone(),
            message: message.to_string(),
            step: step.to_string(),
        };
        
        launch_log.push(entry);
        
        // 同时输出到标准日志
        match level {
            LogLevel::Debug => log::debug!("[{}] {}", step, message),
            LogLevel::Info => log::info!("[{}] {}", step, message),
            LogLevel::Warn => log::warn!("[{}] {}", step, message),
            LogLevel::Error => log::error!("[{}] {}", step, message),
        }
    }
    
    /// 获取当前Provider实例
    pub fn get_current_provider(&self) -> Option<Arc<Mutex<AnyProvider>>> {
        self.current_provider.clone()
    }
    
    /// 停止当前Provider
    pub async fn stop_current_provider(&mut self) -> ProviderResult<()> {
        if let Some(provider_arc) = &self.current_provider {
            let mut provider = provider_arc.lock().await;
            provider.stop()?;
            log::info!("Provider已停止");
        }
        self.current_provider = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    #[tokio::test]
    async fn test_launch_config_default() {
        let config = LaunchConfig::default();
        assert_eq!(config.connection_timeout, 30);
        assert_eq!(config.max_retry_attempts, 3);
        assert!(config.verbose_logging);
    }
    
    #[tokio::test]
    async fn test_launcher_creation() {
        let event_dispatcher = Arc::new(Mutex::new(
            EventDispatcher::new(1000)
        ));
        let launcher = ProviderLauncher::new(event_dispatcher);
        assert!(launcher.current_provider.is_none());
    }
    
    #[test]
    fn test_launch_step_descriptions() {
        assert_eq!(LaunchStep::PreparingLaunch.description(), "准备启动环境...");
        assert_eq!(LaunchStep::ValidatingConfig.description(), "验证配置文件...");
        assert_eq!(LaunchStep::Completed.description(), "启动完成");
    }
    
    #[test]
    fn test_log_levels() {
        assert_eq!(LogLevel::Info, LogLevel::Info);
        assert_ne!(LogLevel::Info, LogLevel::Error);
    }
}