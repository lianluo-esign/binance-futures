use binance_futures::gui::VolumeBarChartRenderer;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{BarChart, Block, Borders},
    Terminal,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 设置终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 创建数据收集器
    let mut volume_chart = VolumeBarChartRenderer::new();
    let mut counter = 0;
    let base_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    loop {
        // 模拟新的交易数据
        let timestamp = base_timestamp + (counter * 60000); // 每分钟
        let volume = 0.5 + (counter as f64 * 0.3); // 递增的成交量
        let is_buyer_maker = counter % 2 == 0;

        volume_chart.add_trade_data(timestamp, volume, is_buyer_maker);

        // 渲染界面
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(50), // 自定义 volume bar chart
                    Constraint::Percentage(50), // ratatui 内置 BarChart
                ])
                .split(f.area());

            // 渲染自定义的 volume bar chart
            volume_chart.render(f, chunks[0]);

            // 渲染 ratatui 内置的 BarChart
            render_ratatui_bar_chart(f, &volume_chart, chunks[1]);
        })?;

        // 检查用户输入
        if event::poll(Duration::from_millis(1000))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    _ => {}
                }
            }
        }

        counter += 1;
        
        if counter > 10 {
            counter = 0;
            volume_chart.clear_data();
        }
    }

    // 恢复终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

/// 使用 ratatui 内置的 BarChart 组件渲染成交量数据
fn render_ratatui_bar_chart(f: &mut ratatui::Frame, volume_chart: &VolumeBarChartRenderer, area: Rect) {
    let stats = volume_chart.get_stats();
    
    // 准备 BarChart 数据
    let mut bar_data = Vec::new();
    
    // 模拟从 volume_chart 中提取数据（这里简化处理）
    for i in 0..stats.total_minutes.min(10) {
        let volume = (i + 1) as f64 * 0.5; // 模拟递增的成交量
        let label = format!("M{}", i + 1); // 分钟标签
        bar_data.push((label.as_str(), volume as u64));
    }

    // 创建 ratatui 内置的 BarChart
    let barchart = BarChart::default()
        .block(
            Block::default()
                .title(format!(
                    "Ratatui BarChart | {} mins | Total: {:.3} BTC", 
                    stats.total_minutes, 
                    stats.total_volume
                ))
                .borders(Borders::ALL)
        )
        .data(&bar_data)
        .bar_width(3)
        .bar_style(Style::default().fg(Color::Yellow))
        .value_style(Style::default().fg(Color::White))
        .label_style(Style::default().fg(Color::Gray))
        .bar_gap(1);

    f.render_widget(barchart, area);
}