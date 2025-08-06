use binance_futures::gui::{VolumeBarChartRenderer, PriceChartRenderer};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
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

    // 创建渲染器
    let mut price_chart = PriceChartRenderer::new(1000);
    let mut volume_chart = VolumeBarChartRenderer::new();

    // 模拟实时数据流
    let mut counter = 0;
    let base_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    loop {
        // 模拟新的交易数据（每次循环添加一笔交易）
        let timestamp = base_timestamp + (counter * 5000); // 每5秒一笔交易
        let price = 50000.0 + (counter as f64 * 10.0); // 价格递增
        let volume = 0.001 + (counter as f64 * 0.0005); // 成交量递增
        let is_buyer_maker = counter % 3 == 0; // 每3笔交易中有1笔是卖单

        // 添加到价格图表
        price_chart.add_price_point(price, volume, is_buyer_maker);

        // 同步到成交量柱状图
        let trade_data: Vec<(u64, f64, bool)> = price_chart
            .get_data_points()
            .map(|point| (point.timestamp, point.volume, point.is_buyer_maker))
            .collect();
        volume_chart.sync_from_price_data(trade_data.into_iter());

        // 渲染界面
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(60), // 价格图表
                    Constraint::Percentage(40), // 成交量柱状图
                ])
                .split(f.area());

            // 渲染价格图表
            price_chart.render(f, chunks[0]);

            // 渲染成交量柱状图
            volume_chart.render(f, chunks[1]);
        })?;

        // 检查用户输入
        if event::poll(Duration::from_millis(500))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    _ => {}
                }
            }
        }

        counter += 1;
        
        // 限制最大交易数量，避免无限增长
        if counter > 200 {
            counter = 0;
            price_chart.clear_data();
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