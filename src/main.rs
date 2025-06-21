use binance_futures::{init_logging, Config, ReactiveApp, OrderFlow};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame, Terminal,
};
use std::{
    collections::BTreeMap,
    env,
    io,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use ordered_float::OrderedFloat;

/// 根据价格精度聚合订单簿数据
/// precision: 价格精度（USD增量），例如1.0表示聚合到1美元增量
fn aggregate_price_levels(
    order_flows: &BTreeMap<OrderedFloat<f64>, OrderFlow>,
    precision: f64,
) -> BTreeMap<OrderedFloat<f64>, OrderFlow> {
    if precision <= 0.0 {
        return order_flows.clone(); // 如果精度无效，返回原始数据
    }

    let mut aggregated: BTreeMap<OrderedFloat<f64>, OrderFlow> = BTreeMap::new();

    for (price_key, order_flow) in order_flows {
        let original_price = price_key.0;

        // 使用floor函数进行价格聚合
        // 例如：10000.1 -> 10000.0, 10000.9 -> 10000.0
        let aggregated_price = (original_price / precision).floor() * precision;
        let aggregated_key = OrderedFloat(aggregated_price);

        // 获取或创建聚合价格级别
        let aggregated_flow = aggregated.entry(aggregated_key).or_insert_with(OrderFlow::new);

        // 聚合买卖价格和数量
        aggregated_flow.bid_ask.bid += order_flow.bid_ask.bid;
        aggregated_flow.bid_ask.ask += order_flow.bid_ask.ask;
        aggregated_flow.bid_ask.timestamp = aggregated_flow.bid_ask.timestamp.max(order_flow.bid_ask.timestamp);

        // 聚合交易记录
        aggregated_flow.history_trade_record.buy_volume += order_flow.history_trade_record.buy_volume;
        aggregated_flow.history_trade_record.sell_volume += order_flow.history_trade_record.sell_volume;
        aggregated_flow.history_trade_record.timestamp = aggregated_flow.history_trade_record.timestamp.max(order_flow.history_trade_record.timestamp);

        aggregated_flow.realtime_trade_record.buy_volume += order_flow.realtime_trade_record.buy_volume;
        aggregated_flow.realtime_trade_record.sell_volume += order_flow.realtime_trade_record.sell_volume;
        aggregated_flow.realtime_trade_record.timestamp = aggregated_flow.realtime_trade_record.timestamp.max(order_flow.realtime_trade_record.timestamp);

        // 聚合撤单记录
        aggregated_flow.realtime_cancel_records.bid_cancel += order_flow.realtime_cancel_records.bid_cancel;
        aggregated_flow.realtime_cancel_records.ask_cancel += order_flow.realtime_cancel_records.ask_cancel;
        aggregated_flow.realtime_cancel_records.timestamp = aggregated_flow.realtime_cancel_records.timestamp.max(order_flow.realtime_cancel_records.timestamp);

        // 聚合增加订单
        aggregated_flow.realtime_increase_order.bid += order_flow.realtime_increase_order.bid;
        aggregated_flow.realtime_increase_order.ask += order_flow.realtime_increase_order.ask;
        aggregated_flow.realtime_increase_order.timestamp = aggregated_flow.realtime_increase_order.timestamp.max(order_flow.realtime_increase_order.timestamp);
    }

    aggregated
}

/// 根据价格精度聚合交易价格
fn aggregate_trade_price(price: f64, precision: f64) -> f64 {
    if precision <= 0.0 {
        return price; // 如果精度无效，返回原始价格
    }
    (price / precision).floor() * precision
}

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
    // 主事件循环 - 集成WebSocket处理和UI刷新，与备份版本保持一致
    loop {
        // 处理事件循环
        app.event_loop();

        // 刷新UI
        terminal.draw(|f| ui(f, app))?;

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

/// UI渲染函数 - 双列布局版本
fn ui(f: &mut Frame, app: &ReactiveApp) {
    let size = f.area();

    // 创建双列布局：订单薄、信号
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70), // 订单薄占70%
            Constraint::Percentage(30), // 市场信号占30%
        ])
        .split(size);

    let orderbook_area = horizontal_chunks[0];
    let signal_area = horizontal_chunks[1];

    render_orderbook(f, app, orderbook_area);
    render_signals(f, app, signal_area);
}

fn render_orderbook(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let stats = app.get_stats();
    let connection_status = if stats.websocket_connected {
        "已连接"
    } else {
        "断开连接"
    };

    // 获取缓冲区使用情况
    let (current_buffer_size, max_buffer_capacity) = app.get_buffer_usage();

    // 获取价格跟踪状态
    let price_tracking_status = if app.is_price_tracking_enabled() { "跟踪" } else { "关闭" };
    let auto_center_status = if app.is_auto_center_enabled() { "居中" } else { "关闭" };

    let title = format!("Binance Futures Order Book - {} | 缓冲区: {}/{} | 事件/秒: {:.1} | 状态: {} | 价格跟踪: {} | 自动居中: {}",
        app.get_symbol(), current_buffer_size, max_buffer_capacity, stats.events_processed_per_second, connection_status, price_tracking_status, auto_center_status);

    // 计算订单薄表格区域
    let table_width = area.width.saturating_sub(2);
    let table_height = area.height.saturating_sub(2);

    let centered_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: table_width,
        height: table_height,
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL);

    // 创建表格数据
    let mut rows = Vec::new();
    let snapshot = app.get_market_snapshot();

    // 获取真实的订单簿数据
    let order_flows = app.get_orderbook_manager().get_order_flows();
    let current_price = snapshot.current_price.unwrap_or(50000.0);

    // 获取配置参数
    let max_visible_rows = app.get_max_visible_rows();
    let price_precision = app.get_price_precision();

    // 应用价格精度聚合
    let aggregated_order_flows = aggregate_price_levels(&order_flows, price_precision);

    // 获取最近交易高亮信息并应用价格聚合
    let (last_trade_price, last_trade_side, _) = app.get_orderbook_manager().get_last_trade_highlight();
    let aggregated_last_trade_price = last_trade_price.map(|price| aggregate_trade_price(price, price_precision));
    let should_highlight_trade = app.get_orderbook_manager().should_show_trade_highlight(3000); // 3秒高亮

    // 准备价格值列表用于自动居中计算
    let price_values: Vec<f64>;

    // 如果有真实数据，使用聚合后的数据；否则生成模拟数据用于演示
    if !aggregated_order_flows.is_empty() {
        // 使用聚合后的订单簿数据
        let mut price_levels: Vec<_> = aggregated_order_flows.keys().collect();
        price_levels.sort_by(|a, b| b.cmp(a)); // 从高价到低价排序

        // 使用配置的最大可见行数限制显示的价格层级数量
        let max_levels = max_visible_rows.min(price_levels.len());

        // 提取价格值用于自动居中计算
        price_values = price_levels.iter().take(max_levels).map(|k| k.0).collect();

        for &price_key in price_levels.iter().take(max_levels) {
            let price = price_key.0;
            let order_flow = &aggregated_order_flows[price_key];

            // 获取真实的订单流数据
            let bid_vol = order_flow.bid_ask.bid;
            let ask_vol = order_flow.bid_ask.ask;
            let buy_trade_vol = order_flow.realtime_trade_record.buy_volume;
            let sell_trade_vol = order_flow.realtime_trade_record.sell_volume;
            let bid_cancel_vol = order_flow.realtime_cancel_records.bid_cancel;
            let ask_cancel_vol = order_flow.realtime_cancel_records.ask_cancel;
            let bid_increase_vol = order_flow.realtime_increase_order.bid;
            let ask_increase_vol = order_flow.realtime_increase_order.ask;
            let history_buy_vol = order_flow.history_trade_record.buy_volume;
            let history_sell_vol = order_flow.history_trade_record.sell_volume;

            // 判断是否为当前价格层级
            let is_current_price = (price - current_price).abs() < 0.1;

            // 构建显示字符串（只有数值大于0才显示，否则显示空字符串）
            let bid_str = if bid_vol > 0.0 { format!("{:.3}", bid_vol) } else { String::new() };
            let ask_str = if ask_vol > 0.0 { format!("{:.3}", ask_vol) } else { String::new() };
            let sell_trade_str = if sell_trade_vol > 0.0 { format!("@{:.3}", sell_trade_vol) } else { String::new() };
            let buy_trade_str = if buy_trade_vol > 0.0 { format!("@{:.3}", buy_trade_vol) } else { String::new() };
            let bid_cancel_str = if bid_cancel_vol > 0.0 { format!("-{:.3}", bid_cancel_vol) } else { String::new() };
            let ask_cancel_str = if ask_cancel_vol > 0.0 { format!("-{:.3}", ask_cancel_vol) } else { String::new() };
            let bid_increase_str = if bid_increase_vol > 0.0 { format!("+{:.3}", bid_increase_vol) } else { String::new() };
            let ask_increase_str = if ask_increase_vol > 0.0 { format!("+{:.3}", ask_increase_vol) } else { String::new() };

            // 创建行 - 与备份文件完全一致的列结构
            let row = Row::new(vec![
                Cell::from(bid_cancel_str).style(Style::default().fg(Color::Gray)),
                Cell::from(sell_trade_str).style(Style::default().fg(Color::Red)),
                Cell::from(bid_str).style(Style::default().fg(Color::Green)),
                {
                    let price_str = format!("{:.2}", price);
                    let mut price_cell = Cell::from(price_str).style(Style::default().fg(Color::White));

                    // 检查是否为最近交易价格，优先显示交易高亮
                    if should_highlight_trade &&
                       aggregated_last_trade_price.map_or(false, |trade_price| (price - trade_price).abs() < 0.001) {
                        // 根据交易方向设置背景颜色
                        let trade_bg_color = match last_trade_side.as_deref() {
                            Some("buy") => Color::Green,   // 买单用绿色背景
                            Some("sell") => Color::Red,    // 卖单用红色背景
                            _ => Color::Yellow,            // 未知方向用黄色背景
                        };
                        price_cell = price_cell.style(Style::default()
                            .fg(Color::Black)
                            .bg(trade_bg_color)
                            .add_modifier(Modifier::BOLD));
                    } else if is_current_price {
                        // 如果不是交易高亮，但是当前价格，使用原有的高亮逻辑
                        let highlight_color = if price >= current_price {
                            Color::Green
                        } else {
                            Color::Red
                        };
                        price_cell = price_cell.style(Style::default().fg(Color::Black).bg(highlight_color).add_modifier(Modifier::BOLD));
                    }
                    price_cell
                },
                Cell::from(ask_str).style(Style::default().fg(Color::Red)),
                Cell::from(buy_trade_str).style(Style::default().fg(Color::Green)),
                Cell::from(ask_cancel_str).style(Style::default().fg(Color::Gray)),
                Cell::from(bid_increase_str).style(Style::default().fg(Color::Blue)),
                Cell::from(ask_increase_str).style(Style::default().fg(Color::Blue)),
                {
                    let total_vol = history_buy_vol + history_sell_vol;
                    let active_trade_str = if total_vol > 0.0 {
                        format!("B:{:.3} S:{:.3} T:{:.3}",
                            history_buy_vol,
                            history_sell_vol,
                            total_vol)
                    } else {
                        String::new()
                    };
                    Cell::from(active_trade_str).style(Style::default().fg(Color::White))
                },
            ]);

            rows.push(row);
        }
    } else {
        // 如果没有真实数据，使用模拟数据进行演示
        let price_range = 50; // 显示50个价格层级
        let price_step = 0.1; // 价格步长

        // 为模拟数据创建价格值列表
        price_values = (0..price_range)
            .map(|i| current_price + (price_range as f64 / 2.0 - i as f64) * price_step)
            .collect();

        // 从高价到低价排列
        for i in 0..price_range {
            let price = current_price + (price_range as f64 / 2.0 - i as f64) * price_step;

            // 模拟订单数据
            let (bid_vol, ask_vol, buy_trade_vol, sell_trade_vol, bid_cancel_vol, ask_cancel_vol, bid_increase_vol, ask_increase_vol, history_buy_vol, history_sell_vol) =
                simulate_order_data_detailed(price, current_price);

            // 判断是否为当前价格层级
            let is_current_price = (price - current_price).abs() < price_step / 2.0;

            // 构建显示字符串（只有数值大于0才显示，否则显示空字符串）
            let bid_str = if bid_vol > 0.0 { format!("{:.3}", bid_vol) } else { String::new() };
            let ask_str = if ask_vol > 0.0 { format!("{:.3}", ask_vol) } else { String::new() };
            let sell_trade_str = if sell_trade_vol > 0.0 { format!("@{:.3}", sell_trade_vol) } else { String::new() };
            let buy_trade_str = if buy_trade_vol > 0.0 { format!("@{:.3}", buy_trade_vol) } else { String::new() };
            let bid_cancel_str = if bid_cancel_vol > 0.0 { format!("-{:.3}", bid_cancel_vol) } else { String::new() };
            let ask_cancel_str = if ask_cancel_vol > 0.0 { format!("-{:.3}", ask_cancel_vol) } else { String::new() };
            let bid_increase_str = if bid_increase_vol > 0.0 { format!("+{:.3}", bid_increase_vol) } else { String::new() };
            let ask_increase_str = if ask_increase_vol > 0.0 { format!("+{:.3}", ask_increase_vol) } else { String::new() };

            // 创建行 - 与备份文件完全一致的列结构
            let row = Row::new(vec![
                Cell::from(bid_cancel_str).style(Style::default().fg(Color::Gray)),
                Cell::from(sell_trade_str).style(Style::default().fg(Color::Red)),
                Cell::from(bid_str).style(Style::default().fg(Color::Green)),
                {
                    let price_str = format!("{:.2}", price);
                    let mut price_cell = Cell::from(price_str).style(Style::default().fg(Color::White));

                    // 检查是否为最近交易价格，优先显示交易高亮
                    if should_highlight_trade &&
                       aggregated_last_trade_price.map_or(false, |trade_price| (price - trade_price).abs() < 0.001) {
                        // 根据交易方向设置背景颜色
                        let trade_bg_color = match last_trade_side.as_deref() {
                            Some("buy") => Color::Green,   // 买单用绿色背景
                            Some("sell") => Color::Red,    // 卖单用红色背景
                            _ => Color::Yellow,            // 未知方向用黄色背景
                        };
                        price_cell = price_cell.style(Style::default()
                            .fg(Color::Black)
                            .bg(trade_bg_color)
                            .add_modifier(Modifier::BOLD));
                    } else if is_current_price {
                        // 如果不是交易高亮，但是当前价格，使用原有的高亮逻辑
                        let highlight_color = if price >= current_price {
                            Color::Green
                        } else {
                            Color::Red
                        };
                        price_cell = price_cell.style(Style::default().fg(Color::Black).bg(highlight_color).add_modifier(Modifier::BOLD));
                    }
                    price_cell
                },
                Cell::from(ask_str).style(Style::default().fg(Color::Red)),
                Cell::from(buy_trade_str).style(Style::default().fg(Color::Green)),
                Cell::from(ask_cancel_str).style(Style::default().fg(Color::Gray)),
                Cell::from(bid_increase_str).style(Style::default().fg(Color::Blue)),
                Cell::from(ask_increase_str).style(Style::default().fg(Color::Blue)),
                {
                    let total_vol = history_buy_vol + history_sell_vol;
                    let active_trade_str = if total_vol > 0.0 {
                        format!("B:{:.3} S:{:.3} T:{:.3}",
                            history_buy_vol,
                            history_sell_vol,
                            total_vol)
                    } else {
                        String::new()
                    };
                    Cell::from(active_trade_str).style(Style::default().fg(Color::White))
                },
            ]);

            rows.push(row);
        }
    }

    // 如果没有数据，显示等待状态
    if rows.is_empty() {
        let status_message = if stats.websocket_connected {
            "连接正常，等待订单薄数据..."
        } else {
            "WebSocket连接断开，尝试重连中..."
        };

        let empty_row = Row::new(vec![
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(status_message).style(Style::default().fg(Color::Yellow)),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ]);
        rows.push(empty_row);
    }

    // 应用自动居中逻辑
    let visible_rows_count = centered_area.height.saturating_sub(3) as usize; // 减去边框和表头
    let auto_center_scroll = app.calculate_auto_center_scroll(&price_values, visible_rows_count);

    // 应用滚动偏移
    let effective_scroll = if app.is_auto_scroll() {
        // 自动滚动模式：使用当前滚动偏移（已在事件循环中更新）
        app.get_scroll_offset()
    } else {
        auto_center_scroll
    };

    // 应用滚动偏移到行数据
    let visible_rows: Vec<_> = rows.into_iter().skip(effective_scroll).collect();

    // 创建并渲染表格 - 与备份文件完全一致的表头和列宽
    let table = Table::new(
        visible_rows,
        [
            Constraint::Length(10), // Bid Cancel
            Constraint::Length(10), // Sell Trade
            Constraint::Length(10), // Bid Vol
            Constraint::Length(12), // Price
            Constraint::Length(10), // Ask Vol
            Constraint::Length(10), // Buy Trade
            Constraint::Length(10), // Ask Cancel
            Constraint::Length(10), // Bid Increase
            Constraint::Length(10), // Ask Increase
            Constraint::Length(25), // History Trades
        ]
    )
    .header(
        Row::new(vec![
            Cell::from("Bid Cancel").style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Cell::from("Sell Trade").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Cell::from("Bid Vol").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Cell::from("Price").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Cell::from("Ask Vol").style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Cell::from("Buy Trade").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Ask Cancel").style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Cell::from("Bid Increase").style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
            Cell::from("Ask Increase").style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
            Cell::from("History Trades").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ])
    )
    .block(block);

    f.render_widget(table, centered_area);
}





/// 详细的订单数据模拟函数 - 与备份文件保持一致
fn simulate_order_data_detailed(price: f64, current_price: f64) -> (f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // 使用价格作为种子生成伪随机数据
    let mut hasher = DefaultHasher::new();
    ((price * 1000.0) as u64).hash(&mut hasher);
    let seed = hasher.finish();

    let distance = (price - current_price).abs();
    let base_volume = if distance < 1.0 { 10.0 } else { 5.0 / (distance + 1.0) };

    // 根据距离当前价格的远近调整成交量
    let bid_vol = if price < current_price {
        base_volume * (1.0 + (seed % 100) as f64 / 100.0)
    } else {
        base_volume * 0.3 * (1.0 + (seed % 50) as f64 / 100.0)
    };

    let ask_vol = if price > current_price {
        base_volume * (1.0 + ((seed >> 8) % 100) as f64 / 100.0)
    } else {
        base_volume * 0.3 * (1.0 + ((seed >> 8) % 50) as f64 / 100.0)
    };

    // 交易量
    let buy_trade_vol = if (seed >> 16) % 10 < 3 { bid_vol * 0.1 } else { 0.0 };
    let sell_trade_vol = if (seed >> 20) % 10 < 3 { ask_vol * 0.1 } else { 0.0 };

    // 撤单量
    let bid_cancel_vol = if (seed >> 24) % 20 < 2 { bid_vol * 0.2 } else { 0.0 };
    let ask_cancel_vol = if (seed >> 28) % 20 < 2 { ask_vol * 0.2 } else { 0.0 };

    // 增单量
    let bid_increase_vol = if (seed >> 32) % 15 < 2 { base_volume * 0.3 } else { 0.0 };
    let ask_increase_vol = if (seed >> 36) % 15 < 2 { base_volume * 0.3 } else { 0.0 };

    // 历史交易量
    let history_buy_vol = buy_trade_vol * 5.0;
    let history_sell_vol = sell_trade_vol * 5.0;

    (bid_vol, ask_vol, buy_trade_vol, sell_trade_vol, bid_cancel_vol, ask_cancel_vol,
     bid_increase_vol, ask_increase_vol, history_buy_vol, history_sell_vol)
}

fn render_signals(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    // 将右侧信号区域分为四个垂直部分 - 与备份文件保持一致
    let signal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10), // Orderbook Imbalance 占10%
            Constraint::Percentage(60), // Order Momentum 占60%
            Constraint::Percentage(15), // Price Speed 占15%
            Constraint::Percentage(15), // Volatility 占15%
        ])
        .split(area);

    let imbalance_area = signal_chunks[0];
    let momentum_area = signal_chunks[1];
    let price_speed_area = signal_chunks[2];
    let volatility_area = signal_chunks[3];

    // 渲染各个信号区域
    render_orderbook_imbalance(f, app, imbalance_area);
    render_order_momentum(f, app, momentum_area);
    render_price_speed(f, app, price_speed_area);
    render_volatility(f, app, volatility_area);
}

// 渲染订单簿失衡信号 - 与备份文件保持一致
fn render_orderbook_imbalance(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let block = Block::default()
        .title("Orderbook Imbalance")
        .borders(Borders::ALL);

    let snapshot = app.get_market_snapshot();

    // 创建Text对象和Line列表
    let mut lines = Vec::new();

    // 添加基本信息
    let basic_info = format!("买单占比: {:.2}% | 卖单占比: {:.2}%",
        snapshot.bid_volume_ratio * 100.0,
        snapshot.ask_volume_ratio * 100.0);
    lines.push(Line::from(Span::raw(basic_info)));

    // 创建横向条
    let bar_width = 30; // 适应较小的区域
    let bid_bar_width = (snapshot.bid_volume_ratio * bar_width as f64) as usize;

    let mut bar = String::new();
    for _ in 0..bid_bar_width {
        bar.push('█');
    }
    for _ in bid_bar_width..bar_width {
        bar.push('░');
    }

    lines.push(Line::from(Span::raw(bar)));

    // 创建Text并渲染
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

// 渲染订单动量信号 - 与备份文件保持一致
fn render_order_momentum(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let block = Block::default()
        .title("Order Momentum")
        .borders(Borders::ALL);

    let mut lines = Vec::new();

    // 模拟订单冲击信号数据
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // 显示统计信息
    let stats_line = "5分钟内: 买入冲击3次 | 卖出冲击2次";
    lines.push(Line::from(Span::styled(stats_line, Style::default().fg(Color::Cyan))));
    lines.push(Line::from(Span::raw(""))); // 空行

    // 模拟最近的信号
    for i in 0..8 {
        let signal_time = current_time - (i * 30000); // 每30秒一个信号
        let time = SystemTime::UNIX_EPOCH + Duration::from_millis(signal_time);
        let seconds = time.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let hours = (seconds / 3600) % 24;
        let minutes = (seconds / 60) % 60;
        let secs = seconds % 60;
        let formatted_time = format!("{:02}:{:02}:{:02}", hours, minutes, secs);

        let is_buy = i % 3 != 0;
        let impact_ratio = 1.5 + (i as f64 * 0.3);
        let trade_price = 50000.0 + (i as f64 * 2.5);

        // 根据冲击强度设置颜色
        let intensity_color = if impact_ratio >= 3.0 {
            Color::Magenta // 强冲击
        } else if impact_ratio >= 2.0 {
            if is_buy { Color::Green } else { Color::Red }
        } else {
            Color::Yellow // 弱冲击
        };

        let (symbol, _direction_text) = if is_buy {
            ("↗", "买入冲击")
        } else {
            ("↘", "卖出冲击")
        };

        // 主信息行
        let main_line = format!(
            "[{}] {} {:.1} ({:.1}x)",
            formatted_time,
            symbol,
            trade_price,
            impact_ratio
        );

        lines.push(Line::from(Span::styled(main_line, Style::default().fg(intensity_color))));

        // 详细信息行
        let detail_line = format!(
            "  成交:{:.2} vs 挂单:{:.2}",
            impact_ratio * 0.5,
            1.0
        );

        lines.push(Line::from(Span::styled(detail_line, Style::default().fg(Color::Gray))));
    }

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

// 渲染Price Speed函数 - 与备份文件保持一致
fn render_price_speed(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let block = Block::default()
        .title("Price Speed")
        .borders(Borders::ALL);

    let mut lines = Vec::new();
    let snapshot = app.get_market_snapshot();

    // 获取当前的price_speed值和平均值
    let speed = snapshot.price_speed;
    let avg_speed = snapshot.price_speed * 0.8; // 模拟平均值

    // 添加基本信息
    let speed_info = format!("当前速度: {:.0} ticks/100ms", speed);
    lines.push(Line::from(Span::styled(speed_info, Style::default().fg(Color::Cyan))));

    // 创建色块来表示当前速度
    let max_blocks = 20; // 适应较小区域
    let blocks_to_show = speed.min(max_blocks as f64) as usize;

    // 根据速度值选择颜色
    let color = if speed >= 30.0 {
        Color::Red // 高速
    } else if speed >= 15.0 {
        Color::Yellow // 中速
    } else {
        Color::Green // 低速
    };

    // 创建色块字符串
    let mut blocks = String::new();
    for _ in 0..blocks_to_show {
        blocks.push('█');
    }

    lines.push(Line::from(Span::styled(blocks, Style::default().fg(color))));

    // 平均速度
    let avg_speed_info = format!("平均速度: {:.1} ticks", avg_speed);
    lines.push(Line::from(Span::styled(avg_speed_info, Style::default().fg(Color::Yellow))));

    // 添加速度级别说明
    let speed_level = if avg_speed >= 30.0 {
        "高速行情"
    } else if avg_speed >= 15.0 {
        "中速行情"
    } else if avg_speed >= 5.0 {
        "低速行情"
    } else {
        "平静行情"
    };

    lines.push(Line::from(Span::styled(
        format!("行情状态: {}", speed_level),
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    )));

    // 创建Text并渲染
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

// 渲染波动率函数 - 与备份文件保持一致
fn render_volatility(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let block = Block::default()
        .title("Price Volatility")
        .borders(Borders::ALL);

    let mut lines = Vec::new();
    let snapshot = app.get_market_snapshot();

    // 获取当前的波动率值
    let volatility = snapshot.volatility;

    // 添加基本信息
    let volatility_info = format!("5秒波动率: {:.2}", volatility);
    lines.push(Line::from(Span::styled(volatility_info, Style::default().fg(Color::Cyan))));

    lines.push(Line::from(Span::raw(""))); // 空行

    // 创建色块来表示波动率
    let blocks_to_show = ((volatility * 100.0) as usize).min(20); // 最多显示20个

    // 根据波动率值选择颜色
    let color = if volatility >= 0.5 {
        Color::Red // 高波动
    } else if volatility >= 0.2 {
        Color::Yellow // 中波动
    } else {
        Color::Green // 低波动
    };

    // 创建色块字符串
    let mut blocks = String::new();
    for _ in 0..blocks_to_show {
        blocks.push('#');
    }

    lines.push(Line::from(Span::styled(blocks, Style::default().fg(color))));

    // 添加波动率级别说明
    let volatility_level = if volatility >= 0.5 {
        "高波动市场"
    } else if volatility >= 0.2 {
        "中等波动市场"
    } else if volatility >= 0.1 {
        "低波动市场"
    } else {
        "平稳市场"
    };

    lines.push(Line::from(Span::styled(
        format!("市场状态: {}", volatility_level),
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    )));

    // 创建Text并渲染
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// 渲染监控面板 - 显示内部健康状态和性能监控
fn render_monitoring_panel(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    // 将监控区域分为多个垂直部分
    let monitoring_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20), // 健康状态摘要
            Constraint::Percentage(25), // 缓冲区监控
            Constraint::Percentage(25), // 事件处理监控
            Constraint::Percentage(30), // WebSocket健康监控
        ])
        .split(area);

    render_health_summary(f, app, monitoring_chunks[0]);
    render_buffer_monitoring(f, app, monitoring_chunks[1]);
    render_event_processing_monitoring(f, app, monitoring_chunks[2]);
    render_websocket_health_monitoring(f, app, monitoring_chunks[3]);
}

/// 渲染健康状态摘要
fn render_health_summary(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let monitor = app.get_internal_monitor();
    let health_summary = monitor.get_health_summary();

    let block = Block::default()
        .title("系统健康状态")
        .borders(Borders::ALL);

    let color = if monitor.blocking_detector.is_blocked {
        Color::Red
    } else if monitor.buffer_monitor.usage_percentage > 90.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let text = Text::from(Line::from(Span::styled(
        health_summary,
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    )));

    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// 渲染缓冲区监控
fn render_buffer_monitoring(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let monitor = app.get_internal_monitor();
    let buffer = &monitor.buffer_monitor;

    let block = Block::default()
        .title("缓冲区监控")
        .borders(Borders::ALL);

    let mut lines = Vec::new();

    // 当前使用情况
    lines.push(Line::from(Span::raw(format!(
        "使用: {}/{} ({:.1}%)",
        buffer.current_usage,
        buffer.max_capacity,
        buffer.usage_percentage
    ))));

    // 峰值使用
    lines.push(Line::from(Span::raw(format!(
        "峰值: {} ({:.1}%)",
        buffer.peak_usage,
        if buffer.max_capacity > 0 { (buffer.peak_usage as f64 / buffer.max_capacity as f64) * 100.0 } else { 0.0 }
    ))));

    // 平均使用
    lines.push(Line::from(Span::raw(format!(
        "平均: {:.1}",
        buffer.average_usage
    ))));

    // 使用率条形图
    let bar_width = 20;
    let filled_width = ((buffer.usage_percentage / 100.0) * bar_width as f64) as usize;
    let mut bar = String::new();
    for _ in 0..filled_width {
        bar.push('█');
    }
    for _ in filled_width..bar_width {
        bar.push('░');
    }

    let bar_color = if buffer.usage_percentage > 90.0 {
        Color::Red
    } else if buffer.usage_percentage > 70.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    lines.push(Line::from(Span::styled(bar, Style::default().fg(bar_color))));

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// 渲染事件处理监控
fn render_event_processing_monitoring(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let monitor = app.get_internal_monitor();
    let event_processing = &monitor.event_processing;

    let block = Block::default()
        .title("事件处理监控")
        .borders(Borders::ALL);

    let mut lines = Vec::new();

    // 事件处理速率
    lines.push(Line::from(Span::raw(format!(
        "速率: {:.1} 事件/秒",
        event_processing.events_per_second
    ))));

    // 队列大小
    lines.push(Line::from(Span::raw(format!(
        "队列: {}",
        event_processing.event_queue_size
    ))));

    // 最大队列大小
    lines.push(Line::from(Span::raw(format!(
        "峰值队列: {}",
        event_processing.max_queue_size_reached
    ))));

    // 失败事件数
    if event_processing.failed_events > 0 {
        lines.push(Line::from(Span::styled(
            format!("失败: {}", event_processing.failed_events),
            Style::default().fg(Color::Red)
        )));
    }

    // 最后处理时间
    if let Some(last_processed) = event_processing.last_event_processed {
        let elapsed = last_processed.elapsed().as_secs();
        let color = if elapsed > 10 { Color::Red } else if elapsed > 5 { Color::Yellow } else { Color::Green };
        lines.push(Line::from(Span::styled(
            format!("最后处理: {}秒前", elapsed),
            Style::default().fg(color)
        )));
    }

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// 渲染WebSocket健康监控
fn render_websocket_health_monitoring(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let monitor = app.get_internal_monitor();
    let websocket = &monitor.websocket_health;

    let block = Block::default()
        .title("WebSocket监控")
        .borders(Borders::ALL);

    let mut lines = Vec::new();

    // 连接状态
    let status_color = if websocket.connection_status == "已连接" {
        Color::Green
    } else {
        Color::Red
    };
    lines.push(Line::from(Span::styled(
        format!("状态: {}", websocket.connection_status),
        Style::default().fg(status_color)
    )));

    // 重连次数
    if websocket.reconnection_count > 0 {
        lines.push(Line::from(Span::styled(
            format!("重连: {}次", websocket.reconnection_count),
            Style::default().fg(Color::Yellow)
        )));
    }

    // 连续错误
    if websocket.consecutive_errors > 0 {
        let error_color = if websocket.consecutive_errors > 5 { Color::Red } else { Color::Yellow };
        lines.push(Line::from(Span::styled(
            format!("连续错误: {}", websocket.consecutive_errors),
            Style::default().fg(error_color)
        )));
    }

    // 最后消息时间
    if let Some(last_message) = websocket.last_message_received {
        let elapsed = last_message.elapsed().as_secs();
        let color = if elapsed > 30 { Color::Red } else if elapsed > 10 { Color::Yellow } else { Color::Green };
        lines.push(Line::from(Span::styled(
            format!("最后消息: {}秒前", elapsed),
            Style::default().fg(color)
        )));
    }

    // 消息速率
    lines.push(Line::from(Span::raw(format!(
        "消息/秒: {:.1}",
        websocket.messages_per_second
    ))));

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}