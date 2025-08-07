use binance_futures::gui::volume_profile::{VolumeProfileWidget, VolumeLevel, VolumeProfileRenderer};

fn main() {
    println!("测试 Volume Profile 修复后的功能...");

    // 测试 Unicode 字符块生成（使用静态方法）
    println!("\n=== 测试 Unicode 字符块生成 ===");
    
    let test_volumes = vec![0.001, 0.01, 0.1, 0.5, 1.0, 5.0, 10.0];
    
    for volume in test_volumes {
        let bar = VolumeProfileRenderer::create_unicode_bar_unlimited(volume);
        println!("音量 {} BTC -> Unicode 字符: '{}'", volume, bar);
    }

    // 测试 VolumeLevel 数据结构
    println!("\n=== 测试 VolumeLevel 渲染 ===");
    
    let mut volume_level = VolumeLevel::new();
    volume_level.buy_volume = 1.5;
    volume_level.sell_volume = 2.3;
    volume_level.total_volume = 3.8;
    volume_level.timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    println!("Volume Level: buy={}, sell={}, total={}", 
             volume_level.buy_volume, volume_level.sell_volume, volume_level.total_volume);
    
    // 测试超时功能
    println!("\n=== 测试超时清除功能 ===");
    
    // 创建一个过期的时间戳（3秒前）
    let old_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64 - 3000;

    let current_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    println!("当前时间戳: {}", current_timestamp);
    println!("旧时间戳: {} ({}ms前)", old_timestamp, current_timestamp - old_timestamp);
    
    // 模拟超时检查逻辑
    let is_expired = current_timestamp.saturating_sub(old_timestamp) > 2000;
    println!("数据是否过期 (>2秒): {}", is_expired);
    
    println!("\n✅ Volume Profile 修复测试完成！");
}