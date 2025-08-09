use binance_futures::{init_logging, init_cpu_affinity, check_affinity_status, Config, ReactiveApp};
use binance_futures::gui::{VolumeProfileWidget, PriceChartRenderer};
use binance_futures::orderbook::render_orderbook;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, Paragraph},
    text::{Line, Span},
    Frame, Terminal,
};
use std::{
    env,
    io,
    time::Duration,
};



fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // 创建配置
    let config = Config::new(symbol)
        .with_buffer_size(10000)
        .with_max_reconnects(5)
        .with_max_visible_rows(3000)    // 设置最大可见行数为3000
        .with_price_precision(0.01);    // 设置价格精度为0.01 USD (1分)

    // 创建应用程序
    let mut app = ReactiveApp::new(config);

    // 初始化应用程序
    app.initialize()?;

    // 设置终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 运行应用程序
    let result = run_app(&mut terminal, &mut app);

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

/// 运行应用程序主循环 - 基于稳定的备份版本架构
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut ReactiveApp,
) -> io::Result<()> {
    // 创建Volume Profile widget
    let mut volume_profile_widget = VolumeProfileWidget::new();
    
    // 创建价格图表渲染器，使用20000个数据点的滑动窗口
    let mut price_chart_renderer = PriceChartRenderer::new(20000);
    
    
    // 主事件循环 - 集成WebSocket处理和UI刷新，与备份版本保持一致
    loop {
        // 处理事件循环
        app.event_loop();

        // 更新Volume Profile数据
        update_volume_profile(&mut volume_profile_widget, app);
        
        // 更新价格图表数据
        update_price_chart(&mut price_chart_renderer, app);
        

        // 刷新UI
        terminal.draw(|f| ui(f, app, &volume_profile_widget, &price_chart_renderer))?;

        // 处理UI事件（非阻塞）
        if crossterm::event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Up => {
                            if app.get_scroll_offset() > 0 {
                                app.set_scroll_offset(app.get_scroll_offset() - 1);
                                app.set_auto_scroll(false);
                                app.set_auto_center_enabled(false); // 禁用自动居中
                            }
                        }
                        KeyCode::Down => {
                            app.set_scroll_offset(app.get_scroll_offset() + 1);
                            app.set_auto_scroll(false);
                            app.set_auto_center_enabled(false); // 禁用自动居中
                        }
                        KeyCode::Home => {
                            app.set_scroll_offset(0);
                            app.set_auto_scroll(false);
                            app.set_auto_center_enabled(false); // 禁用自动居中
                        }
                        KeyCode::End => {
                            app.set_auto_scroll(true);
                            app.set_auto_center_enabled(true); // 重新启用自动居中
                        }
                        KeyCode::Char(' ') => {
                            app.set_auto_scroll(!app.is_auto_scroll());
                            if app.is_auto_scroll() {
                                app.set_auto_center_enabled(true); // 启用自动滚动时重新启用自动居中
                            }
                        }
                        KeyCode::Char('c') => {
                            // 'c' 键切换自动居中功能
                            app.set_auto_center_enabled(!app.is_auto_center_enabled());
                        }
                        KeyCode::Char('t') => {
                            // 't' 键切换价格跟踪功能
                            app.set_price_tracking_enabled(!app.is_price_tracking_enabled());
                        }
                        KeyCode::Char('r') => {
                            // 'r' 键手动重新居中到当前交易价格
                            let current_price = app.get_market_snapshot().current_price;
                            if let Some(price) = current_price {
                                // 临时启用价格跟踪来触发居中
                                let was_tracking = app.is_price_tracking_enabled();
                                app.set_price_tracking_enabled(true);
                                // 通过设置阈值为0来强制触发重新居中
                                app.force_recenter_on_current_price();
                                app.set_price_tracking_enabled(was_tracking);
                            }
                        }
                        _ => {}
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

/// UI渲染函数 - 带状态栏的三列布局版本
fn ui(
    f: &mut Frame, 
    app: &ReactiveApp, 
    volume_profile_widget: &VolumeProfileWidget, 
    price_chart_renderer: &PriceChartRenderer
) {
    let size = f.area();

    // 垂直布局：顶部状态栏，下方主要内容区域
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // 顶部状态栏高度改为2
            Constraint::Min(0),         // 主要内容区域
        ])
        .split(size);

    let status_bar_area = main_chunks[0];
    let content_area = main_chunks[1];

    // 渲染顶部状态栏
    render_status_bar(f, app, status_bar_area);

    // 创建三列布局：订单薄(20%)、Volume Profile(30%)、图表区域(50%)
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // 订单薄占20%
            Constraint::Percentage(40), // Volume Profile占30%
            Constraint::Percentage(40), // 图表区域占50%
        ])
        .split(content_area);

    let orderbook_area = horizontal_chunks[0];
    let volume_profile_area = horizontal_chunks[1];
    let chart_area = horizontal_chunks[2];

    let price_chart_area = chart_area;

    // 渲染各个组件
    render_orderbook(f, app, orderbook_area);
    
    // 渲染Volume Profile widget
    render_volume_profile(f, app, volume_profile_widget, volume_profile_area);
    
    // 渲染价格图表
    render_price_chart(f, price_chart_renderer, price_chart_area);
}

/// 更新Volume Profile数据
fn update_volume_profile(volume_profile_widget: &mut VolumeProfileWidget, app: &ReactiveApp) {
    // 直接从应用的Volume Profile管理器获取数据
    // 这个管理器只在实际交易事件发生时才会更新
    let app_volume_manager = app.get_volume_profile_manager();
    let app_data = app_volume_manager.get_data();
    
    
    
    // 获取 orderbook 的 order flow 数据
    let order_flows = app.get_orderbook_manager().get_order_flows();
    
    // 获取widget的管理器
    let widget_manager = volume_profile_widget.get_manager_mut();
    
    // 直接同步应用层的volume profile累积数据到widget
    // 保持原有的累积数据和最新更新信息
    for (price_key, app_volume_level) in &app_data.price_volumes {
        let price = price_key.0;
        
        // 检查widget中是否已有这个价格层级的数据
        let widget_data = widget_manager.get_data();
        let existing_level = widget_data.price_volumes.get(price_key);
        
        // 如果是新数据或者数据有更新，则同步并保持更新状态
        match existing_level {
            Some(existing) => {
                // 检查是否有新的交易数据
                if existing.total_volume != app_volume_level.total_volume {
                    // 数据有更新，直接设置完整的level数据，包括last_update_side
                    widget_manager.sync_volume_level_with_update_info(
                        price, 
                        app_volume_level.buy_volume, 
                        app_volume_level.sell_volume,
                        app_volume_level.timestamp,
                        app_volume_level.last_update_side.clone()
                    );
                }
            },
            None => {
                // 新的价格层级，同步数据
                widget_manager.sync_volume_level_with_update_info(
                    price, 
                    app_volume_level.buy_volume, 
                    app_volume_level.sell_volume,
                    app_volume_level.timestamp,
                    app_volume_level.last_update_side.clone()
                );
            }
        }
    }
}

/// 渲染Volume Profile widget
fn render_volume_profile(
    f: &mut Frame, 
    app: &ReactiveApp, 
    volume_profile_widget: &VolumeProfileWidget, 
    area: ratatui::layout::Rect
) {
    // 根据实际widget区域高度计算可见行数
    let actual_visible_rows = calculate_visible_rows_from_area(area);
    
    // 获取当前可见的价格范围（基于实际widget高度）
    let visible_price_range = get_visible_price_range_for_area(app, actual_visible_rows);
    
    // 获取最新交易价格用于高亮显示
    let latest_trade_price = app.get_market_snapshot().current_price;
    
    // 获取orderbook数据用于显示buy/sell列
    let orderbook_data = app.get_orderbook_manager().get_order_flows();
    
    // 渲染Volume Profile widget
    volume_profile_widget.render(f, area, &visible_price_range, latest_trade_price, Some(orderbook_data));
}

/// 获取当前可见的价格范围（为Volume Profile动态生成价格层级）
/// 动态扩展上下各100个层级，跟随当前价格变化
fn get_visible_price_range(app: &ReactiveApp) -> Vec<f64> {
    let snapshot = app.get_market_snapshot();
    let visible_rows = get_actual_visible_rows();
    
    // 优先使用当前交易价格，如果没有则使用best_bid，最后使用best_ask
    let reference_price = snapshot.current_price
        .or(snapshot.best_bid_price)
        .or(snapshot.best_ask_price);
        
    if let Some(center_price) = reference_price {
        // 动态生成价格层级：以当前价格为中心，上下各扩展100个层级
        // 使用1美元精度（与VolumeProfileManager的price_precision保持一致）
        let price_precision = 1.0;
        
        // 计算中心价格的聚合值（向下取整到最近的美元）
        let center_aggregated = (center_price / price_precision).floor() * price_precision;
        
        // 动态扩展：上下各100个层级，总共201个层级（包含中心价格）
        let levels_above = 100;
        let levels_below = 100;
        let total_levels = levels_above + levels_below + 1;
        
        let mut price_levels = Vec::with_capacity(total_levels);
        
        // 从高价到低价生成价格层级
        // 上方100个层级
        for i in (1..=levels_above).rev() {
            let price = center_aggregated + (i as f64) * price_precision;
            price_levels.push(price);
        }
        
        // 中心价格
        price_levels.push(center_aggregated);
        
        // 下方100个层级
        for i in 1..=levels_below {
            let price = center_aggregated - (i as f64) * price_precision;
            price_levels.push(price);
        }
        
        // 计算可见范围：显示所有生成的价格层级，或者根据可见行数截取
        if visible_rows >= total_levels {
            // 如果可见行数足够，显示所有层级
            price_levels
        } else {
            // 如果可见行数不够，以中心价格为基准截取可见范围
            let center_index = levels_above; // 中心价格在数组中的索引
            let half_visible = visible_rows / 2;
            
            let start_index = center_index.saturating_sub(half_visible);
            let end_index = (start_index + visible_rows).min(price_levels.len());
            
            price_levels[start_index..end_index].to_vec()
        }
    } else {
        // 如果没有参考价格，生成一个默认的价格范围（以110000为中心）
        let default_center = 110000.0;
        let price_precision = 1.0;
        let levels_above = 100;
        let levels_below = 100;
        let total_levels = levels_above + levels_below + 1;
        
        let mut price_levels = Vec::with_capacity(total_levels);
        
        // 从高价到低价生成价格层级
        for i in (1..=levels_above).rev() {
            let price = default_center + (i as f64) * price_precision;
            price_levels.push(price);
        }
        
        price_levels.push(default_center);
        
        for i in 1..=levels_below {
            let price = default_center - (i as f64) * price_precision;
            price_levels.push(price);
        }
        
        if visible_rows >= total_levels {
            price_levels
        } else {
            let center_index = levels_above;
            let half_visible = visible_rows / 2;
            let start_index = center_index.saturating_sub(half_visible);
            let end_index = (start_index + visible_rows).min(price_levels.len());
            
            price_levels[start_index..end_index].to_vec()
        }
    }
}

/// 计算居中偏移 - 复制price_tracker的逻辑
fn calculate_center_offset(price: f64, price_levels: &[f64], visible_rows: usize) -> usize {
    // 查找最接近目标价格的索引
    let mut closest_index = 0;
    let mut closest_distance = f64::MAX;
    
    for (i, &level_price) in price_levels.iter().enumerate() {
        let distance = (level_price - price).abs();
        if distance < closest_distance {
            closest_distance = distance;
            closest_index = i;
        }
    }
    
    // 计算居中偏移，使目标价格尽可能居中
    let half_visible = visible_rows / 2;
    if closest_index >= half_visible {
        let center_offset = closest_index - half_visible;
        let max_offset = price_levels.len().saturating_sub(visible_rows);
        center_offset.min(max_offset)
    } else {
        0
    }
}

/// 获取实际可见行数（基于终端高度）
fn get_actual_visible_rows() -> usize {
    // 这里使用一个合理的默认值，实际应该基于终端高度计算
    // 减去边框和表头的高度
    45 // 假设终端高度约50行，减去边框和表头
}

/// 查找价格在价格列表中的居中偏移（模拟price_tracker的逻辑）
fn find_price_center_offset(target_price: f64, price_levels: &[f64], visible_rows: usize) -> usize {
    // 查找最接近目标价格的索引
    let mut closest_index = 0;
    let mut closest_distance = f64::MAX;
    
    for (i, &price) in price_levels.iter().enumerate() {
        let distance = (price - target_price).abs();
        if distance < closest_distance {
            closest_distance = distance;
            closest_index = i;
        }
    }
    
    // 计算居中偏移
    let center_offset = closest_index.saturating_sub(visible_rows / 2);
    let max_offset = price_levels.len().saturating_sub(visible_rows);
    center_offset.min(max_offset)
}

/// 根据widget区域计算实际可见行数
fn calculate_visible_rows_from_area(area: ratatui::layout::Rect) -> usize {
    // 减去边框（上下各1行）和表头（1行）
    let available_height = area.height.saturating_sub(3); // 边框2行 + 表头1行
    available_height as usize
}

/// 获取基于实际widget区域的价格范围
fn get_visible_price_range_for_area(app: &ReactiveApp, visible_rows: usize) -> Vec<f64> {
    let snapshot = app.get_market_snapshot();
    
    // 获取volume profile数据范围
    let volume_manager = app.get_volume_profile_manager();
    let volume_data = volume_manager.get_data();
    
    // 优先使用当前交易价格，如果没有则使用best_bid，最后使用best_ask
    let reference_price = snapshot.current_price
        .or(snapshot.best_bid_price)
        .or(snapshot.best_ask_price);
    
    // 如果有volume profile数据，扩展价格范围以包含这些数据
    let (min_volume_price, max_volume_price) = if !volume_data.price_volumes.is_empty() {
        let min_price = volume_data.price_volumes.keys().next().unwrap().0;
        let max_price = volume_data.price_volumes.keys().next_back().unwrap().0;
        (Some(min_price), Some(max_price))
    } else {
        (None, None)
    };
        
    if let Some(center_price) = reference_price {
        // 动态生成价格层级：以当前价格为中心，上下各扩展足够的层级
        // 使用1美元精度（与VolumeProfileManager的price_precision保持一致）
        let price_precision = 1.0;
        
        // 计算中心价格的聚合值（向下取整到最近的美元）
        let center_aggregated = (center_price / price_precision).floor() * price_precision;
        
        // 根据可见行数和volume数据范围动态计算需要的层级数
        let half_visible = visible_rows / 2;
        let mut levels_above = half_visible + 20; // 减少缓冲，因为我们要包含volume数据
        let mut levels_below = half_visible + 20;
        
        // 如果有volume数据，扩展范围以包含这些价格
        if let Some(max_vol_price) = max_volume_price {
            let levels_needed_above = ((max_vol_price - center_aggregated) / price_precision).ceil() as usize;
            levels_above = levels_above.max(levels_needed_above + 10);
        }
        if let Some(min_vol_price) = min_volume_price {
            let levels_needed_below = ((center_aggregated - min_vol_price) / price_precision).ceil() as usize;
            levels_below = levels_below.max(levels_needed_below + 10);
        }
        
        let total_levels = levels_above + levels_below + 1;
        
        let mut price_levels = Vec::with_capacity(total_levels);
        
        // 从高价到低价生成价格层级
        // 上方层级
        for i in (1..=levels_above).rev() {
            let price = center_aggregated + (i as f64) * price_precision;
            price_levels.push(price);
        }
        
        // 中心价格
        price_levels.push(center_aggregated);
        
        // 下方层级
        for i in 1..=levels_below {
            let price = center_aggregated - (i as f64) * price_precision;
            price_levels.push(price);
        }
        
        // 截取可见范围：以中心价格为基准
        let center_index = levels_above; // 中心价格在数组中的索引
        let start_index = center_index.saturating_sub(half_visible);
        let end_index = (start_index + visible_rows).min(price_levels.len());
        
        price_levels[start_index..end_index].to_vec()
    } else {
        // 如果没有参考价格，生成一个默认的价格范围（以110000为中心）
        let default_center = 110000.0;
        let price_precision = 1.0;
        let half_visible = visible_rows / 2;
        let levels_above = half_visible + 50;
        let levels_below = half_visible + 50;
        
        let mut price_levels = Vec::with_capacity(levels_above + levels_below + 1);
        
        // 从高价到低价生成价格层级
        for i in (1..=levels_above).rev() {
            let price = default_center + (i as f64) * price_precision;
            price_levels.push(price);
        }
        
        price_levels.push(default_center);
        
        for i in 1..=levels_below {
            let price = default_center - (i as f64) * price_precision;
            price_levels.push(price);
        }
        
        let center_index = levels_above;
        let start_index = center_index.saturating_sub(half_visible);
        let end_index = (start_index + visible_rows).min(price_levels.len());
        
        price_levels[start_index..end_index].to_vec()
    }
}

/// 更新价格图表数据
fn update_price_chart(price_chart_renderer: &mut PriceChartRenderer, app: &ReactiveApp) {
    // 获取最新交易数据并添加为价格点（现在所有价格点都是交易数据点）
    let orderbook_manager = app.get_orderbook_manager();
    let (last_trade_price, last_trade_side, last_trade_timestamp) = orderbook_manager.get_last_trade_highlight();
    
    // 如果有最新交易数据，并且交易时间足够新（3秒内），添加价格点
    if let (Some(price), Some(side), Some(_timestamp)) = (last_trade_price, last_trade_side, last_trade_timestamp) {
        if orderbook_manager.should_show_trade_highlight(3000) { // 3秒内的交易
            // 确定交易方向：buy是买单（绿色），sell是卖单（红色）
            let is_buyer_maker = side == "sell";
            
            // 获取真实的成交量数据
            let volume = app.get_last_trade_volume().unwrap_or(0.001); // 使用真实成交量，默认0.001
            
            // 统一使用add_price_point，现在包含交易信息
            price_chart_renderer.add_price_point(price, volume, is_buyer_maker);
        }
    } else {
        // 如果没有最新交易数据，使用市场快照中的价格（作为默认的小量买单）
        let market_snapshot = app.get_market_snapshot();
        if let Some(current_price) = market_snapshot.current_price {
            price_chart_renderer.add_price_point(current_price, 0.001, false); // 默认小量买单
        }
    }
}

/// 渲染价格图表
fn render_price_chart(f: &mut Frame, price_chart_renderer: &PriceChartRenderer, area: Rect) {
    price_chart_renderer.render(f, area);
}

/// 渲染顶部状态栏
fn render_status_bar(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    // 获取应用统计信息
    let stats = app.get_stats();
    let connection_status = if stats.websocket_connected {
        "已连接"
    } else {
        "断开连接"
    };
    
    // 获取缓冲区使用情况
    let (current_buffer_size, max_buffer_capacity) = app.get_buffer_usage();
   
    // 创建状态信息文本（单行显示）
    let status_text = vec![
        Line::from(vec![
            Span::styled("Symbol: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(app.get_symbol(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(" | Buffer: ", Style::default().fg(Color::White)),
            Span::styled(format!("{}/{}", current_buffer_size, max_buffer_capacity), Style::default().fg(Color::Yellow)),
            Span::styled(" | Status: ", Style::default().fg(Color::White)),
            Span::styled(connection_status, if stats.websocket_connected { 
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) 
            } else { 
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD) 
            }),
            Span::styled(" | Events/s: ", Style::default().fg(Color::White)),
            Span::styled(format!("{:.1}", stats.events_processed_per_second), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),

        ]),
    ];
    
    // 创建状态栏段落
    let status_paragraph = Paragraph::new(status_text)
        .block(Block::default()
            .title("System Status")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
        );
    
    f.render_widget(status_paragraph, area);
}












