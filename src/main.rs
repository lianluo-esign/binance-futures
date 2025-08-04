use binance_futures::{init_logging, Config, ReactiveApp};
use binance_futures::gui::{render_signals, VolumeProfileWidget};
use binance_futures::orderbook::render_orderbook;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Frame, Terminal,
};
use std::{
    env,
    io,
    time::Duration,
};



fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    init_logging();

    // 获取交易对参数
    let symbol = env::args().nth(1).unwrap_or_else(|| "BTCUSDT".to_string());

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

    if let Err(err) = result {
        // 应用程序错误写入日志文件，不输出到控制台以避免干扰UI
        log::error!("应用程序错误: {:?}", err);
    }

    Ok(())
}

/// 运行应用程序主循环 - 基于稳定的备份版本架构
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut ReactiveApp,
) -> io::Result<()> {
    // 创建Volume Profile widget
    let mut volume_profile_widget = VolumeProfileWidget::new();
    
    // 主事件循环 - 集成WebSocket处理和UI刷新，与备份版本保持一致
    loop {
        // 处理事件循环
        app.event_loop();

        // 更新Volume Profile数据
        update_volume_profile(&mut volume_profile_widget, app);

        // 刷新UI
        terminal.draw(|f| ui(f, app, &volume_profile_widget))?;

        // 处理UI事件（非阻塞）- 与备份版本完全一致
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

/// UI渲染函数 - 三列布局版本
fn ui(f: &mut Frame, app: &ReactiveApp, volume_profile_widget: &VolumeProfileWidget) {
    let size = f.area();

    // 创建三列布局：订单薄(40%)、Volume Profile(40%)、信号(20%)
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // 订单薄占40%
            Constraint::Percentage(40), // Volume Profile占40%
            Constraint::Percentage(20), // 市场信号占20%
        ])
        .split(size);

    let orderbook_area = horizontal_chunks[0];
    let volume_profile_area = horizontal_chunks[1];
    let signal_area = horizontal_chunks[2];

    // 渲染各个组件
    render_orderbook(f, app, orderbook_area);
    
    // 渲染Volume Profile widget
    render_volume_profile(f, app, volume_profile_widget, volume_profile_area);
    
    render_signals(f, app, signal_area);
}

/// 更新Volume Profile数据
fn update_volume_profile(volume_profile_widget: &mut VolumeProfileWidget, app: &ReactiveApp) {
    // 直接从应用的Volume Profile管理器获取数据
    // 这个管理器只在实际交易事件发生时才会更新
    let app_volume_manager = app.get_volume_profile_manager();
    let app_data = app_volume_manager.get_data();
    
    // 获取widget的管理器
    let widget_manager = volume_profile_widget.get_manager_mut();
    
    // 清空widget管理器的旧数据
    widget_manager.clear_data();
    
    // 直接复制应用管理器中的数据，避免重复累加
    for (price_key, volume_level) in &app_data.price_volumes {
        let price = price_key.0;
        
        // 直接设置成交量数据，而不是累加
        widget_manager.set_volume_data(price, volume_level.buy_volume, volume_level.sell_volume);
    }
}

/// 渲染Volume Profile widget
fn render_volume_profile(
    f: &mut Frame, 
    app: &ReactiveApp, 
    volume_profile_widget: &VolumeProfileWidget, 
    area: ratatui::layout::Rect
) {
    // 获取当前可见的价格范围（与orderbook同步）
    let visible_price_range = get_visible_price_range(app);
    
    // 渲染Volume Profile widget
    volume_profile_widget.render(f, area, &visible_price_range);
}

/// 获取当前可见的价格范围（与orderbook完全同步）
fn get_visible_price_range(app: &ReactiveApp) -> Vec<f64> {
    // 直接复制orderbook_renderer.rs中prepare_render_data的逻辑
    let snapshot = app.get_market_snapshot();
    let order_flows = app.get_orderbook_manager().get_order_flows();
    let price_precision = app.get_price_precision();

    // 应用价格精度聚合，处理bid/ask冲突 - 与orderbook完全相同
    let aggregated_order_flows = binance_futures::orderbook::display_formatter::aggregate_price_levels_with_conflict_resolution(
        &order_flows, 
        snapshot.best_bid_price,
        snapshot.best_ask_price,
        price_precision
    );

    // 构建价格层级列表 - 与orderbook完全相同
    let mut price_levels: Vec<_> = aggregated_order_flows.keys().collect();
    price_levels.sort_by(|a, b| b.cmp(a)); // 从高价到低价排序

    let max_levels = app.get_max_visible_rows().min(price_levels.len());
    let all_price_levels: Vec<f64> = price_levels.iter()
        .take(max_levels)
        .map(|k| k.0)
        .collect();

    // 计算可见范围 - 复制orderbook_renderer.rs中calculate_visible_range的逻辑
    let visible_rows = get_actual_visible_rows();
    
    // 优先使用当前交易价格，如果没有则使用best_bid，最后使用best_ask
    let reference_price = snapshot.current_price
        .or(snapshot.best_bid_price)
        .or(snapshot.best_ask_price);
        
    if let Some(price) = reference_price {
        let center_offset = calculate_center_offset(price, &all_price_levels, visible_rows);
        let end_offset = (center_offset + visible_rows).min(all_price_levels.len());
        all_price_levels[center_offset..end_offset].to_vec()
    } else {
        all_price_levels.into_iter().take(visible_rows).collect()
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











