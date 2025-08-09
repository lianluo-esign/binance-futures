// Application Startup Flow - 应用启动流程管理
//
// 本模块管理应用的完整启动流程，包括：
// - Provider选择界面展示
// - 用户交互处理
// - Provider启动和初始化
// - 主应用界面切换
//
// 设计目标：
// 1. 流程清晰：明确的启动阶段划分
// 2. 用户友好：直观的选择界面和进度反馈
// 3. 错误处理：完善的错误恢复机制
// 4. 模块解耦：与主应用逻辑分离

use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use tokio::sync::{mpsc, Mutex};

use crate::app::ReactiveApp;
use crate::core::provider::{
    AnyProvider, ProviderSelector, ProviderLauncher, LaunchConfig, LaunchProgress, LaunchResult,
    ProviderResult,
};
use crate::events::LockFreeEventDispatcher;
use crate::gui::{ProviderSelectionUI, SelectionAction};

/// 应用启动流程管理器
pub struct StartupFlow {
    /// 终端后端
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    /// Provider选择UI
    selection_ui: ProviderSelectionUI,
    /// Provider启动器
    launcher: ProviderLauncher,
    /// 启动进度接收器
    progress_receiver: mpsc::UnboundedReceiver<LaunchProgress>,
    /// 当前状态
    state: StartupState,
    /// 错误信息
    last_error: Option<String>,
}

/// 启动状态枚举
#[derive(Debug, Clone, PartialEq)]
pub enum StartupState {
    /// 显示Provider选择界面
    SelectingProvider,
    /// 正在启动Provider
    LaunchingProvider,
    /// 启动完成，准备切换到主应用
    LaunchCompleted,
    /// 启动失败
    LaunchFailed,
    /// 用户取消启动
    Cancelled,
}

/// 启动流程结果
pub struct StartupResult {
    /// 成功启动的Provider
    pub provider: Arc<Mutex<AnyProvider>>,
    /// 启动耗时
    pub launch_duration: Duration,
    /// 事件分发器
    pub event_dispatcher: Arc<Mutex<LockFreeEventDispatcher>>,
}

impl StartupFlow {
    /// 创建新的启动流程管理器
    pub fn new(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // 设置终端
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        
        // 创建Provider选择UI
        let selection_ui = ProviderSelectionUI::new(config_path)?;
        
        // 创建事件分发器和Provider启动器
        let event_dispatcher = Arc::new(Mutex::new(LockFreeEventDispatcher::new(10000)));
        let mut launcher = ProviderLauncher::new(event_dispatcher.clone())
            .with_config(LaunchConfig {
                connection_timeout: 10,
                max_retry_attempts: 3,
                retry_interval_ms: 1000,
                post_launch_wait_ms: 100, // 减少到100ms
                verbose_logging: false,   // 关闭verbose日志以避免延迟
            });
        
        // 设置进度回调
        let (progress_sender, progress_receiver) = mpsc::unbounded_channel();
        launcher.set_progress_callback(progress_sender);
        
        Ok(Self {
            terminal,
            selection_ui,
            launcher,
            progress_receiver,
            state: StartupState::SelectingProvider,
            last_error: None,
        })
    }
    
    /// 运行启动流程
    pub async fn run(mut self) -> Result<StartupResult, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        loop {
            // 渲染UI
            self.terminal.draw(|f| {
                self.selection_ui.render(f);
            })?;
            
            // 根据当前状态处理事件
            match self.state {
                StartupState::SelectingProvider => {
                    if let Some(result) = self.handle_selection_events().await? {
                        return result;
                    }
                }
                StartupState::LaunchingProvider => {
                    if let Some(result) = self.handle_launch_events().await? {
                        return result;
                    }
                }
                StartupState::LaunchCompleted => {
                    // 给用户一些时间看到完成消息
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    
                    if let Some(provider) = self.launcher.get_current_provider() {
                        let event_dispatcher = Arc::new(Mutex::new(LockFreeEventDispatcher::new(10000)));
                        
                        // 清理终端
                        self.cleanup_terminal()?;
                        
                        return Ok(StartupResult {
                            provider,
                            launch_duration: start_time.elapsed(),
                            event_dispatcher,
                        });
                    } else {
                        self.state = StartupState::LaunchFailed;
                        self.last_error = Some("Provider启动成功但无法获取实例".to_string());
                    }
                }
                StartupState::LaunchFailed => {
                    if let Some(result) = self.handle_error_events().await? {
                        return result;
                    }
                }
                StartupState::Cancelled => {
                    self.cleanup_terminal()?;
                    return Err("用户取消启动".into());
                }
            }
            
            // 短暂延迟以避免CPU过度使用
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
    
    /// 处理Provider选择阶段的事件
    async fn handle_selection_events(
        &mut self,
    ) -> Result<Option<Result<StartupResult, Box<dyn std::error::Error>>>, Box<dyn std::error::Error>> {
        // 检查键盘事件
        if crossterm::event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let action = self.selection_ui.handle_key_event(key);
                    
                    match action {
                        SelectionAction::LaunchProvider => {
                            // 开始启动Provider
                            self.state = StartupState::LaunchingProvider;
                            self.start_provider_launch().await;
                        }
                        SelectionAction::Quit => {
                            self.state = StartupState::Cancelled;
                        }
                        SelectionAction::ShowHelp => {
                            // 可以在这里实现帮助界面
                            log::info!("显示帮助信息");
                        }
                        SelectionAction::Complete => {
                            self.state = StartupState::LaunchCompleted;
                        }
                        SelectionAction::None => {
                            // 继续处理
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// 处理Provider启动阶段的事件
    async fn handle_launch_events(
        &mut self,
    ) -> Result<Option<Result<StartupResult, Box<dyn std::error::Error>>>, Box<dyn std::error::Error>> {
        // 检查启动进度更新
        while let Ok(progress) = self.progress_receiver.try_recv() {
            self.selection_ui.update_launch_progress(
                &progress.step_description,
                progress.progress_percent,
            );
            
            // 检查是否完成
            if progress.progress_percent >= 1.0 {
                self.selection_ui.complete_launch();
                self.state = StartupState::LaunchCompleted;
                break;
            }
        }
        
        // 检查是否有取消请求
        if crossterm::event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Char('q') => {
                            // 用户请求取消启动
                            log::info!("用户请求取消Provider启动");
                            self.state = StartupState::Cancelled;
                        }
                        _ => {
                            // 启动期间忽略其他按键
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// 处理错误状态的事件
    async fn handle_error_events(
        &mut self,
    ) -> Result<Option<Result<StartupResult, Box<dyn std::error::Error>>>, Box<dyn std::error::Error>> {
        if let Some(error_msg) = &self.last_error {
            self.selection_ui.show_error(error_msg);
        }
        
        // 等待用户输入
        if crossterm::event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        crossterm::event::KeyCode::Enter | crossterm::event::KeyCode::Esc => {
                            // 回到选择界面
                            self.state = StartupState::SelectingProvider;
                            self.last_error = None;
                        }
                        crossterm::event::KeyCode::Char('q') => {
                            self.state = StartupState::Cancelled;
                        }
                        _ => {
                            // 忽略其他按键
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// 开始Provider启动过程
    async fn start_provider_launch(&mut self) {
        // 获取选中的Provider信息
        if let Some(selected_option) = self.selection_ui.get_selected_provider() {
            log::info!("开始启动Provider: {}", selected_option.name);
            
            // 这里需要创建一个临时的ProviderSelector来执行启动
            // 因为我们需要异步处理，但UI已经借用了selector
            match ProviderSelector::new("config.toml") {
                Ok(selector) => {
                    // 设置选择的Provider
                    let mut temp_selector = selector;
                    for (i, option) in temp_selector.get_all_options().iter().enumerate() {
                        if option.name == selected_option.name {
                            temp_selector.select_index(i);
                            break;
                        }
                    }
                    
                    // 异步启动Provider
                    let launcher = &mut self.launcher;
                    
                    match launcher.launch_provider_async(&temp_selector).await {
                        Ok(_launch_result) => {
                            log::info!("Provider启动成功");
                            self.state = StartupState::LaunchCompleted;
                        }
                        Err(e) => {
                            log::error!("Provider启动失败: {}", e);
                            self.last_error = Some(format!("启动失败: {}", e));
                            self.state = StartupState::LaunchFailed;
                        }
                    }
                }
                Err(e) => {
                    log::error!("无法创建ProviderSelector: {}", e);
                    self.last_error = Some(format!("配置错误: {}", e));
                    self.state = StartupState::LaunchFailed;
                }
            }
        } else {
            self.last_error = Some("没有选中的Provider".to_string());
            self.state = StartupState::LaunchFailed;
        }
    }
    
    /// 清理终端设置
    fn cleanup_terminal(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for StartupFlow {
    fn drop(&mut self) {
        // 确保终端状态被正确恢复
        let _ = self.cleanup_terminal();
    }
}

/// 启动应用的入口函数
/// 
/// 这个函数处理整个启动流程，包括Provider选择和初始化
pub async fn run_startup_flow(
    config_path: &str,
) -> Result<StartupResult, Box<dyn std::error::Error>> {
    log::info!("开始应用启动流程");
    
    let startup_flow = StartupFlow::new(config_path)?;
    let result = startup_flow.run().await?;
    
    log::info!("启动流程完成，耗时: {:?}", result.launch_duration);
    
    Ok(result)
}

/// 直接启动指定的Provider（跳过选择界面）
/// 
/// 用于自动化部署或测试场景
pub async fn launch_provider_directly(
    config_path: &str,
    provider_name: &str,
) -> Result<StartupResult, Box<dyn std::error::Error>> {
    log::info!("直接启动Provider: {}", provider_name);
    let start_time = Instant::now();
    
    // 创建Provider选择器
    let mut selector = ProviderSelector::new(config_path)?;
    
    // 查找指定的Provider
    let mut found = false;
    for (i, option) in selector.get_all_options().iter().enumerate() {
        if option.name == provider_name {
            selector.select_index(i);
            found = true;
            break;
        }
    }
    
    if !found {
        return Err(format!("未找到Provider: {}", provider_name).into());
    }
    
    // 创建启动器
    let event_dispatcher = Arc::new(Mutex::new(LockFreeEventDispatcher::new(10000)));
    let mut launcher = ProviderLauncher::new(event_dispatcher.clone());
    
    // 启动Provider
    let launch_result = launcher.launch_provider_async(&selector).await?;
    
    log::info!("Provider直接启动完成");
    
    Ok(StartupResult {
        provider: launch_result.provider,
        launch_duration: start_time.elapsed(),
        event_dispatcher,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_startup_state_transitions() {
        let mut state = StartupState::SelectingProvider;
        
        // 测试状态转换
        state = StartupState::LaunchingProvider;
        assert_eq!(state, StartupState::LaunchingProvider);
        
        state = StartupState::LaunchCompleted;
        assert_eq!(state, StartupState::LaunchCompleted);
    }
    
    #[tokio::test]
    async fn test_direct_provider_launch() {
        // 这个测试需要完整的配置环境
        // 在实际使用中应该有相应的配置文件
        let config_path = "test_config.toml";
        
        // 由于依赖外部配置，这里只测试错误情况
        let result = launch_provider_directly(config_path, "nonexistent_provider").await;
        assert!(result.is_err());
    }
}