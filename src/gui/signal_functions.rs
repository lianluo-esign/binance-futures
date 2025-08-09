use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::app::ReactiveApp;
use crate::gui::SignalRenderer;

/// 渲染信号面板的主函数 - 从main.rs移动过来
pub fn render_signals(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    // 使用新的SignalRenderer
    static mut SIGNAL_RENDERER: Option<SignalRenderer> = None;
    
    // 初始化渲染器（只在第一次调用时）
    let renderer = unsafe {
        if SIGNAL_RENDERER.is_none() {
            SIGNAL_RENDERER = Some(SignalRenderer::new());
        }
        SIGNAL_RENDERER.as_mut().unwrap()
    };
    
    // 使用新的渲染器进行渲染
    renderer.render(f, app, area);
}

// 渲染订单簿失衡信号 - 与备份文件保持一致
pub fn render_orderbook_imbalance(f: &mut Frame, app: &ReactiveApp, area: Rect) {
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
pub fn render_order_momentum(f: &mut Frame, _app: &ReactiveApp, area: Rect) {
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
pub fn render_price_speed(f: &mut Frame, _app: &ReactiveApp, area: Rect) {
    let block = Block::default()
        .title("Price Speed")
        .borders(Borders::ALL);

    let mut lines = Vec::new();

    // 模拟价格速度数据
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // 显示当前价格速度
    let current_speed: f64 = 0.15; // 模拟当前速度
    let speed_direction = if current_speed > 0.0 { "↗" } else { "↘" };
    let speed_color = if current_speed > 0.0 { Color::Green } else { Color::Red };

    let speed_line = format!("当前速度: {} {:.3} USD/s", speed_direction, current_speed.abs());
    lines.push(Line::from(Span::styled(speed_line, Style::default().fg(speed_color))));

    // 显示速度历史
    lines.push(Line::from(Span::raw(""))); // 空行
    lines.push(Line::from(Span::styled("最近速度变化:", Style::default().fg(Color::White))));

    for i in 0..6 {
        let time_offset = i * 10000; // 每10秒一个数据点
        let time = SystemTime::UNIX_EPOCH + Duration::from_millis(current_time - time_offset);
        let seconds = time.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let minutes = (seconds / 60) % 60;
        let secs = seconds % 60;
        let formatted_time = format!("{:02}:{:02}", minutes, secs);

        // 模拟速度数据
        let speed = 0.1 + (i as f64 * 0.02) * if i % 2 == 0 { 1.0 } else { -1.0 };
        let direction = if speed > 0.0 { "↗" } else { "↘" };
        let color = if speed > 0.0 { Color::Green } else { Color::Red };

        let speed_entry = format!("[{}] {} {:.3}", formatted_time, direction, speed.abs());
        lines.push(Line::from(Span::styled(speed_entry, Style::default().fg(color))));
    }

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

// 渲染波动率函数 - 与备份文件保持一致
pub fn render_volatility(f: &mut Frame, _app: &ReactiveApp, area: Rect) {
    let block = Block::default()
        .title("Price Volatility")
        .borders(Borders::ALL);

    let mut lines = Vec::new();

    // 模拟波动率数据
    let volatility_1m = 0.0023; // 1分钟波动率
    let volatility_5m = 0.0087; // 5分钟波动率
    let volatility_15m = 0.0156; // 15分钟波动率

    // 显示不同时间窗口的波动率
    lines.push(Line::from(Span::styled("时间窗口波动率:", Style::default().fg(Color::White))));
    lines.push(Line::from(Span::raw(""))); // 空行

    let vol_1m_line = format!("1分钟:  {:.4} ({:.2}%)", volatility_1m, volatility_1m * 100.0);
    let vol_5m_line = format!("5分钟:  {:.4} ({:.2}%)", volatility_5m, volatility_5m * 100.0);
    let vol_15m_line = format!("15分钟: {:.4} ({:.2}%)", volatility_15m, volatility_15m * 100.0);

    // 根据波动率大小设置颜色
    let vol_1m_color = if volatility_1m > 0.002 { Color::Red } else { Color::Green };
    let vol_5m_color = if volatility_5m > 0.008 { Color::Red } else { Color::Green };
    let vol_15m_color = if volatility_15m > 0.015 { Color::Red } else { Color::Green };

    lines.push(Line::from(Span::styled(vol_1m_line, Style::default().fg(vol_1m_color))));
    lines.push(Line::from(Span::styled(vol_5m_line, Style::default().fg(vol_5m_color))));
    lines.push(Line::from(Span::styled(vol_15m_line, Style::default().fg(vol_15m_color))));

    lines.push(Line::from(Span::raw(""))); // 空行

    // 显示波动率状态
    let volatility_status = if volatility_15m > 0.015 {
        ("高波动", Color::Red)
    } else if volatility_15m > 0.010 {
        ("中等波动", Color::Yellow)
    } else {
        ("低波动", Color::Green)
    };

    let status_line = format!("市场状态: {}", volatility_status.0);
    lines.push(Line::from(Span::styled(status_line, Style::default().fg(volatility_status.1))));

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// 渲染监控面板 - 显示内部健康状态和性能监控
pub fn render_monitoring_panel(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    // 将监控区域分为多个垂直部分
    let monitoring_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25), // 健康状态摘要
            Constraint::Percentage(25), // 缓冲区监控
            Constraint::Percentage(25), // 事件处理监控
            Constraint::Percentage(25), // WebSocket健康监控
        ])
        .split(area);

    render_health_summary(f, app, monitoring_chunks[0]);
    render_buffer_monitoring(f, app, monitoring_chunks[1]);
    render_event_processing_monitoring(f, app, monitoring_chunks[2]);
    render_websocket_health_monitoring(f, app, monitoring_chunks[3]);
}

/// 渲染健康状态摘要
pub fn render_health_summary(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let monitor = app.get_internal_monitor();
    let health_summary = monitor.get_health_summary();

    let block = Block::default()
        .title("Health Summary")
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
pub fn render_buffer_monitoring(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let monitor = app.get_internal_monitor();
    let buffer = &monitor.buffer_monitor;

    let block = Block::default()
        .title("Buffer Monitoring")
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
pub fn render_event_processing_monitoring(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let monitor = app.get_internal_monitor();
    let event_processing = &monitor.event_processing;

    let block = Block::default()
        .title("Event Processing")
        .borders(Borders::ALL);

    let mut lines = Vec::new();

    // 事件处理速率
    lines.push(Line::from(Span::styled(
        format!("处理速率: {:.1} 事件/秒", event_processing.events_per_second),
        Style::default().fg(Color::Cyan)
    )));

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
pub fn render_websocket_health_monitoring(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let monitor = app.get_internal_monitor();
    let websocket = &monitor.websocket_health;

    let block = Block::default()
        .title("WebSocket Health")
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