use binance_futures::gui::VolumeBarChartRenderer;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let mut volume_chart = VolumeBarChartRenderer::new();
    
    let base_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // 添加一些简单的测试数据
    println!("Adding test data...");
    
    // 第1分钟：0.5 BTC
    volume_chart.add_trade_data(base_timestamp, 0.5, false);
    
    // 第2分钟：1.0 BTC  
    volume_chart.add_trade_data(base_timestamp + 60000, 1.0, true);
    
    // 第3分钟：1.5 BTC
    volume_chart.add_trade_data(base_timestamp + 120000, 1.5, false);

    let stats = volume_chart.get_stats();
    println!("Stats: {:?}", stats);
    
    // 手动测试高度计算
    println!("\nTesting height calculation:");
    println!("Volume 0.5 BTC should give height: {}", calculate_test_height(0.5));
    println!("Volume 1.0 BTC should give height: {}", calculate_test_height(1.0));
    println!("Volume 1.5 BTC should give height: {}", calculate_test_height(1.5));
}

fn calculate_test_height(volume: f64) -> usize {
    if volume <= 0.0 {
        return 0;
    }

    // 基于实际BTC成交量计算高度，每个字符行代表固定的BTC量
    // 每个字符行代表的BTC量 = 1.0 / 8 = 0.125 BTC
    let btc_per_char_line = 1.0 / 8.0;
    let total_char_lines_needed = (volume / btc_per_char_line).ceil() as usize;
    
    total_char_lines_needed.max(1)
}