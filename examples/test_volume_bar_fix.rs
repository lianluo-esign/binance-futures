use binance_futures::gui::RatatuiVolumeBarChartRenderer;
use ratatui::{
    backend::TestBackend,
    Terminal,
    layout::Rect,
};
use std::time::{SystemTime, UNIX_EPOCH};

/// 测试volume bar chart修复是否有效
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing volume bar chart fix...");
    
    // 创建renderer
    let mut renderer = RatatuiVolumeBarChartRenderer::new();
    
    // 添加一些测试数据（最近3分钟有数据）
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    // 添加3个分钟的测试数据
    for i in 0..3 {
        let timestamp = current_time - ((2 - i) * 60 * 1000); // 最近3分钟
        let volume = (i + 1) as f64 * 0.5; // 0.5, 1.0, 1.5 BTC
        renderer.add_trade_data(timestamp, volume, false);
        println!("Added data for minute -{}: {} BTC", 2 - i, volume);
    }
    
    // 测试prepare_bar_data
    let bar_data = renderer.prepare_bar_data();
    println!("\nBar data length: {} (should be 20)", bar_data.len());
    
    // 显示bar数据
    println!("\nBar data contents:");
    for (i, (label, value)) in bar_data.iter().enumerate() {
        if *value > 0 {
            println!("  {}: {} (volume: {:.3} BTC)", label, value, *value as f64 / 1000.0);
        } else if i >= 17 { // 只显示最后几个空bar作为示例
            println!("  {}: {} (no data)", label, value);
        }
    }
    
    // 统计信息
    let stats = renderer.get_stats();
    println!("\nStatistics:");
    println!("  Total minutes with data: {}", stats.total_minutes);
    println!("  Total volume: {:.3} BTC", stats.total_volume);
    println!("  Average volume: {:.3} BTC", stats.avg_volume);
    
    // 测试渲染到buffer（验证不会崩溃）
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend)?;
    
    terminal.draw(|f| {
        let area = Rect::new(0, 0, 80, 20);
        renderer.render(f, area);
    })?;
    
    println!("\nRendering test completed successfully!");
    println!("The volume bar chart should now display 20 bars filling the entire window width.");
    
    Ok(())
}