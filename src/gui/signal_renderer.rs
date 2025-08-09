use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::app::ReactiveApp;

/// 信号渲染器 - 负责渲染市场信号面板
pub struct SignalRenderer;

impl SignalRenderer {
    pub fn new() -> Self {
        Self
    }

    /// 渲染信号面板
    pub fn render(&self, f: &mut Frame, app: &ReactiveApp, area: Rect) {
        // 将右侧信号区域分为四个垂直部分
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
        self.render_orderbook_imbalance(f, app, imbalance_area);
        self.render_order_momentum(f, app, momentum_area);
        self.render_price_speed(f, app, price_speed_area);
        self.render_volatility(f, app, volatility_area);
    }

    /// 渲染订单簿失衡信号
    fn render_orderbook_imbalance(&self, f: &mut Frame, app: &ReactiveApp, area: Rect) {
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

    /// 渲染订单动量信号
    fn render_order_momentum(&self, f: &mut Frame, _app: &ReactiveApp, area: Rect) {
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

    /// 渲染价格速度信号
    fn render_price_speed(&self, f: &mut Frame, app: &ReactiveApp, area: Rect) {
        let block = Block::default()
            .title("Price Speed")
            .borders(Borders::ALL);

        let snapshot = app.get_market_snapshot();

        let mut lines = Vec::new();

        // 当前速度信息
        let speed_info = format!("当前速度: {:.2} ticks/100ms", snapshot.price_speed);
        lines.push(Line::from(Span::styled(speed_info, Style::default().fg(Color::White))));

        let avg_speed_info = format!("平均速度: {:.2} ticks/100ms", snapshot.avg_speed);
        lines.push(Line::from(Span::styled(avg_speed_info, Style::default().fg(Color::Cyan))));

        lines.push(Line::from(Span::raw(""))); // 空行

        // 速度等级指示器
        let speed_level = if snapshot.price_speed > 10.0 {
            ("极快", Color::Red)
        } else if snapshot.price_speed > 5.0 {
            ("快速", Color::Yellow)
        } else if snapshot.price_speed > 2.0 {
            ("正常", Color::Green)
        } else {
            ("缓慢", Color::Blue)
        };

        let level_line = format!("速度等级: {}", speed_level.0);
        lines.push(Line::from(Span::styled(level_line, Style::default().fg(speed_level.1))));

        // 创建速度条形图
        let bar_width = 25;
        let speed_ratio = (snapshot.price_speed / 15.0).min(1.0); // 最大15作为满格
        let filled_width = (speed_ratio * bar_width as f64) as usize;

        let mut speed_bar = String::new();
        for _ in 0..filled_width {
            speed_bar.push('█');
        }
        for _ in filled_width..bar_width {
            speed_bar.push('░');
        }

        lines.push(Line::from(Span::raw(""))); // 空行
        lines.push(Line::from(Span::styled(speed_bar, Style::default().fg(speed_level.1))));

        let text = Text::from(lines);
        let paragraph = Paragraph::new(text)
            .block(block)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    /// 渲染波动率信号
    fn render_volatility(&self, f: &mut Frame, app: &ReactiveApp, area: Rect) {
        let block = Block::default()
            .title("Price Volatility")
            .borders(Borders::ALL);

        let snapshot = app.get_market_snapshot();

        let mut lines = Vec::new();

        // 波动率信息
        let volatility_info = format!("当前波动率: {:.4}%", snapshot.volatility);
        lines.push(Line::from(Span::styled(volatility_info, Style::default().fg(Color::White))));

        // 波动率等级
        let volatility_level = if snapshot.volatility > 0.5 {
            ("极高", Color::Red)
        } else if snapshot.volatility > 0.2 {
            ("高", Color::Yellow)
        } else if snapshot.volatility > 0.1 {
            ("中等", Color::Green)
        } else {
            ("低", Color::Blue)
        };

        let level_line = format!("波动等级: {}", volatility_level.0);
        lines.push(Line::from(Span::styled(level_line, Style::default().fg(volatility_level.1))));

        lines.push(Line::from(Span::raw(""))); // 空行

        // 创建波动率条形图
        let bar_width = 25;
        let volatility_ratio = (snapshot.volatility / 1.0).min(1.0); // 最大1.0作为满格
        let filled_width = (volatility_ratio * bar_width as f64) as usize;

        let mut volatility_bar = String::new();
        for _ in 0..filled_width {
            volatility_bar.push('█');
        }
        for _ in filled_width..bar_width {
            volatility_bar.push('░');
        }

        lines.push(Line::from(Span::styled(volatility_bar, Style::default().fg(volatility_level.1))));

        // 添加历史波动率信息（模拟）
        lines.push(Line::from(Span::raw(""))); // 空行
        lines.push(Line::from(Span::styled("1小时平均: 0.15%", Style::default().fg(Color::Gray))));
        lines.push(Line::from(Span::styled("24小时平均: 0.25%", Style::default().fg(Color::Gray))));

        let text = Text::from(lines);
        let paragraph = Paragraph::new(text)
            .block(block)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }
}

impl Default for SignalRenderer {
    fn default() -> Self {
        Self::new()
    }
}