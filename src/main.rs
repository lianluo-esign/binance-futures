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
    layout::{Constraint, Direction, Layout, Rect},
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

    // 创建三列布局：订单薄(30%)、Volume Profile(50%)、信号(20%)
    // 直接使用整个屏幕区域，不保留边距
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // 订单薄占30%
            Constraint::Percentage(50), // Volume Profile占50%
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
    // 根据实际widget区域高度计算可见行数
    let actual_visible_rows = calculate_visible_rows_from_area(area);
    
    // 获取当前可见的价格范围（基于实际widget高度）
    let visible_price_range = get_visible_price_range_for_area(app, actual_visible_rows);
    
    // 获取最新交易价格用于高亮显示
    let latest_trade_price = app.get_market_snapshot().current_price;
    
    // 渲染Volume Profile widget
    volume_profile_widget.render(f, area, &visible_price_range, latest_trade_price);
}

/// 获取当前可见的价格范围（为Volume Profile动态生成价格层级）
/// 修复：动态扩展100个层级上下，跟随当前价格变化
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
    
    // 优先使用当前交易价格，如果没有则使用best_bid，最后使用best_ask
    let reference_price = snapshot.current_price
        .or(snapshot.best_bid_price)
        .or(snapshot.best_ask_price);
        
    if let Some(center_price) = reference_price {
        // 动态生成价格层级：以当前价格为中心，上下各扩展足够的层级
        // 使用1美元精度（与VolumeProfileManager的price_precision保持一致）
        let price_precision = 1.0;
        
        // 计算中心价格的聚合值（向下取整到最近的美元）
        let center_aggregated = (center_price / price_precision).floor() * price_precision;
        
        // 根据可见行数动态计算需要的层级数
        // 确保有足够的层级来填满整个widget
        let half_visible = visible_rows / 2;
        let levels_above = half_visible + 50; // 额外50层级作为缓冲
        let levels_below = half_visible + 50;
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











