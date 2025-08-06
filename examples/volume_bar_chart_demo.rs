use binance_futures::gui::volume_bar_chart::VolumeBarChartRenderer;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal, Frame,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Stdout};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 设置终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 创建Volume Bar Chart渲染器
    let mut volume_chart = VolumeBarChartRenderer::new();
    
    // 添加一些测试数据
    add_demo_data(&mut volume_chart);

    // 运行应用
    let result = run_app(&mut terminal, &mut volume_chart);

    // 恢复终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        println!("{:?}", err)
    }

    Ok(())
}

fn add_demo_data(chart: &mut VolumeBarChartRenderer) {
    let base_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    // 添加10分钟的测试数据，模拟不同的成交量模式
    let volumes = [
        0.05, 0.15, 0.25, 0.35, 0.45,  // 递增趋势
        0.40, 0.30, 0.20, 0.10, 0.05,  // 递减趋势
        0.30, 0.45, 0.20, 0.55, 0.25,  // 波动趋势
    ];
    
    for (i, volume) in volumes.iter().enumerate() {
        let timestamp = base_timestamp - ((volumes.len() - i) as u64 * 60 * 1000);
        let is_buyer_maker = i % 2 == 0; // 交替买卖
        
        // 每分钟添加多笔交易
        for j in 0..3 {
            let trade_timestamp = timestamp + (j * 20 * 1000); // 每20秒一笔
            let trade_volume = volume / 3.0; // 分成3笔交易
            chart.add_trade_data(trade_timestamp, trade_volume, is_buyer_maker);
        }
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    chart: &mut VolumeBarChartRenderer,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, chart))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('r') => {
                    // 重新加载数据
                    chart.clear_data();
                    add_demo_data(chart);
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, chart: &VolumeBarChartRenderer) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // 标题
            Constraint::Min(10),    // Volume Chart
            Constraint::Length(5),  // 统计信息
        ])
        .split(f.size());

    // 标题
    let title = Paragraph::new("Volume Bar Chart Demo")
        .block(Block::default().borders(Borders::ALL).title("Demo"))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(title, chunks[0]);

    // 渲染Volume Bar Chart
    chart.render(f, chunks[1]);

    // 显示统计信息
    let stats = chart.get_stats();
    let stats_text = vec![
        Line::from(vec![
            Span::styled("统计信息 | ", Style::default().fg(Color::Blue)),
            Span::styled(format!("分钟数: {} | ", stats.total_minutes), Style::default().fg(Color::White)),
            Span::styled(format!("总成交量: {:.3} BTC", stats.total_volume), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled(format!("平均成交量: {:.3} BTC | ", stats.avg_volume), Style::default().fg(Color::Cyan)),
            Span::styled(format!("最大成交量: {:.3} BTC", stats.max_volume), Style::default().fg(Color::Red)),
        ]),
        Line::from(vec![
            Span::styled(format!("买入量: {:.3} BTC | ", stats.buy_volume), Style::default().fg(Color::Green)),
            Span::styled(format!("卖出量: {:.3} BTC | ", stats.sell_volume), Style::default().fg(Color::Red)),
            Span::styled(format!("买卖比: {:.2}", stats.buy_sell_ratio), Style::default().fg(Color::Yellow)),
        ]),
    ];

    let stats_paragraph = Paragraph::new(stats_text)
        .block(Block::default().borders(Borders::ALL).title("统计信息"))
        .style(Style::default());
    
    f.render_widget(stats_paragraph, chunks[2]);
    
    // 在底部显示帮助信息
    let help_text = "按 'q' 或 ESC 退出，按 'r' 重新加载数据";
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray));
    
    // 在屏幕底部显示帮助
    let help_area = Rect::new(
        1,
        f.size().height.saturating_sub(1),
        f.size().width.saturating_sub(2),
        1
    );
    f.render_widget(help, help_area);
}