use binance_futures::gui::RatatuiVolumeBarChartRenderer;
use ratatui::{
    backend::TestBackend,
    layout::Rect,
    Terminal,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建测试后端
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    // 创建 ratatui volume bar chart 渲染器
    let mut volume_chart = RatatuiVolumeBarChartRenderer::new();

    // 添加测试数据
    let base_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    println!("Adding test data to Ratatui BarChart...");
    
    // 模拟5分钟的交易数据，每分钟递增的成交量
    for i in 0..5 {
        let timestamp = base_timestamp + (i * 60 * 1000); // 每分钟
        let volume = 0.5 + (i as f64 * 0.3); // 0.5, 0.8, 1.1, 1.4, 1.7 BTC
        let is_buyer_maker = i % 2 == 0; // 交替买卖
        
        volume_chart.add_trade_data(timestamp, volume, is_buyer_maker);
        println!("Added: {:.1} BTC at minute {}", volume, i + 1);
    }

    // 渲染图表
    terminal.draw(|f| {
        let area = Rect::new(0, 0, 80, 24);
        volume_chart.render(f, area);
    })?;

    // 打印统计信息
    let stats = volume_chart.get_stats();
    println!("\nRatatui BarChart Stats:");
    println!("  Total minutes: {}", stats.total_minutes);
    println!("  Total volume: {:.3} BTC", stats.total_volume);
    println!("  Total trades: {}", stats.total_trades);
    println!("  Average volume per minute: {:.3} BTC", stats.avg_volume);
    println!("  Max volume: {:.3} BTC", stats.max_volume);
    println!("  Buy volume: {:.3} BTC", stats.buy_volume);
    println!("  Sell volume: {:.3} BTC", stats.sell_volume);
    println!("  Buy/Sell ratio: {:.2}", stats.buy_sell_ratio);

    // 获取渲染后的内容
    let backend = terminal.backend();
    let buffer = backend.buffer();
    
    println!("\nRendered Ratatui BarChart:");
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            print!("{}", cell.symbol());
        }
        println!();
    }

    Ok(())
}