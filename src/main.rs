use binance_futures::{init_logging, Config, ReactiveApp};
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
    env,
    io,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    init_logging();

    // 获取交易对参数
    let symbol = env::args().nth(1).unwrap_or_else(|| "BTCUSDT".to_string());

    // 创建配置
    let config = Config::new(symbol)
        .with_buffer_size(10000)
        .with_max_reconnects(5);

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

/// 运行应用程序主循环
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut ReactiveApp,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(100); // 10 FPS - 更合理的刷新率

    loop {
        // 处理事件循环
        app.event_loop();

        // 绘制UI
        terminal.draw(|f| ui(f, app))?;

        // 处理用户输入
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Up => {
                            if app.get_scroll_offset() > 0 {
                                app.set_scroll_offset(app.get_scroll_offset() - 1);
                                app.set_auto_scroll(false);
                            }
                        }
                        KeyCode::Down => {
                            app.set_scroll_offset(app.get_scroll_offset() + 1);
                            app.set_auto_scroll(false);
                        }
                        KeyCode::Home => {
                            app.set_scroll_offset(0);
                            app.set_auto_scroll(false);
                        }
                        KeyCode::End => {
                            app.set_auto_scroll(true);
                        }
                        KeyCode::Char(' ') => {
                            app.set_auto_scroll(!app.is_auto_scroll());
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        if !app.is_running() {
            break;
        }
    }

    Ok(())
}

/// UI渲染函数 - 与备份文件保持一致
fn ui(f: &mut Frame, app: &ReactiveApp) {
    let size = f.area();

    // 创建左右布局 - 与备份文件完全一致
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

    let title = format!("Binance Futures Order Book - {} | 事件/秒: {:.1} | 状态: {}",
        app.get_symbol(), stats.events_processed_per_second, connection_status);

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

    // 如果有真实数据，使用真实数据；否则生成模拟数据用于演示
    if !order_flows.is_empty() {
        // 使用真实订单簿数据
        let mut price_levels: Vec<_> = order_flows.keys().collect();
        price_levels.sort_by(|a, b| b.cmp(a)); // 从高价到低价排序

        // 限制显示的价格层级数量
        let max_levels = 50.min(price_levels.len());

        for &price_key in price_levels.iter().take(max_levels) {
            let price = price_key.0;
            let order_flow = &order_flows[price_key];

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
                    if is_current_price {
                        // 根据价格变化设置高亮颜色
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
                    if is_current_price {
                        // 根据价格变化设置高亮颜色
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

    // 创建并渲染表格 - 与备份文件完全一致的表头和列宽
    let table = Table::new(
        rows,
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