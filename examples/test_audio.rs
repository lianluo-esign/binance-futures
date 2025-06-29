use binance_futures::audio::{play_buy_sound, play_sell_sound, play_tick_pressure_sound};
use std::thread;
use std::time::Duration;

fn main() {
    println!("测试音频播放功能...");
    
    // 测试买单音效
    println!("播放买单音效...");
    play_buy_sound();
    thread::sleep(Duration::from_secs(2));
    
    // 测试卖单音效
    println!("播放卖单音效...");
    play_sell_sound();
    thread::sleep(Duration::from_secs(2));
    
    // 测试ΔTick Pressure音效
    println!("播放ΔTick Pressure买单信号音效...");
    play_tick_pressure_sound(true);
    thread::sleep(Duration::from_secs(2));
    
    println!("播放ΔTick Pressure卖单信号音效...");
    play_tick_pressure_sound(false);
    thread::sleep(Duration::from_secs(2));
    
    println!("音频测试完成！");
    
    // 如果没有听到声音，请检查：
    println!("\n如果没有听到声音，请检查：");
    println!("1. 音频文件是否存在：src/audio/buy.mp3 和 src/audio/sell.mp3");
    println!("2. 音频文件格式是否正确（支持MP3, WAV, FLAC, OGG）");
    println!("3. 系统音量是否开启");
    println!("4. 音频设备是否正常工作");
}
