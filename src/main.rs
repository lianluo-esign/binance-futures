use binance_futures::{init_logging, init_cpu_affinity, check_affinity_status, Config, ReactiveApp};
use binance_futures::gui::UIManager;
use binance_futures::startup_flow::run_startup_flow;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{
    env,
    io,
    time::Duration,
};
use tokio;



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 首先初始化日志系统
    init_logging();

    // 2. 立即设置CPU亲和性 - 在任何其他操作之前进行
    // 检查命令行参数中是否指定了CPU核心
    let cpu_core = env::args()
        .position(|arg| arg == "--cpu-core")
        .and_then(|pos| env::args().nth(pos + 1))
        .and_then(|core_str| core_str.parse::<usize>().ok());
    
    // 设置CPU亲和性（默认绑定到核心1）
    match init_cpu_affinity(cpu_core) {
        Ok(()) => {
            log::info!("🚀 CPU亲和性设置成功，程序现在运行在专用CPU核心上");
            // 输出到终端让用户知道绑定成功（在UI启动前）
            println!("🚀 CPU亲和性设置成功! 程序已绑定到CPU核心 {} 运行", cpu_core.unwrap_or(1));
            println!("📈 性能优化已启用: L1/L2缓存优化, 减少延迟");
        }
        Err(e) => {
            log::warn!("⚠️ CPU亲和性设置失败: {}, 程序将继续运行", e);
            println!("⚠️ 警告: CPU亲和性设置失败: {}", e);
            println!("程序将继续运行，但可能无法获得最佳性能");
        }
    }

    // 3. 获取交易对参数
    let symbol = env::args()
        .find(|arg| !arg.starts_with("--") && arg != &env::args().next().unwrap())
        .unwrap_or_else(|| "BTCFDUSD".to_string());

    // 4. 运行Provider选择流程
    println!("🔧 启动Provider选择界面...");
    let startup_result = run_startup_flow("config.toml").await?;
    
    println!("✅ Provider启动完成，启动时间: {:?}", startup_result.launch_duration);
    println!("🎯 正在启动主应用界面...");
    
    // 创建配置
    let config = Config::new(symbol)
        .with_buffer_size(10000)
        .with_max_reconnects(5)
        .with_max_visible_rows(3000)    // 设置最大可见行数为3000
        .with_price_precision(0.01);    // 设置价格精度为0.01 USD (1分)

    // 创建应用程序并传入已启动的Provider
    let mut app = ReactiveApp::with_provider(config, startup_result.provider, startup_result.event_dispatcher)?;

    // 初始化应用程序
    app.initialize()?;

    // 设置终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 运行应用程序
    let result = run_app(&mut terminal, &mut app).await;

    // 清理终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // 停止应用程序
    app.stop();

    // 在程序退出前检查CPU亲和性状态
    check_affinity_status();

    if let Err(err) = result {
        // 应用程序错误写入日志文件，不输出到控制台以避免干扰UI
        log::error!("应用程序错误: {:?}", err);
    }

    // 程序退出消息
    println!("👋 程序已退出，CPU绑定已释放");
    
    Ok(())
}

/// 运行应用程序主循环 - 使用UIManager管理所有GUI功能
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut ReactiveApp,
) -> io::Result<()> {
    // 创建UI管理器
    let mut ui_manager = UIManager::new();
    
    // 主事件循环 - 集成WebSocket处理和UI刷新
    loop {
        // 处理事件循环
        app.event_loop();

        // 更新所有UI组件数据
        ui_manager.update_data(app);

        // 刷新UI
        terminal.draw(|f| ui_manager.render_ui(f, app))?;

        // 处理UI事件（非阻塞）
        if crossterm::event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // 使用UI管理器处理键盘事件
                    if ui_manager.handle_key_event(app, key.code) {
                        return Ok(()); // 退出应用
                    }
                }
            }
        }

        if !app.is_running() {
            break;
        }
    }

    Ok(())
}














