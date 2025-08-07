// 重构后的Provider架构演示
//
// 本示例演示了重构后的Provider系统的使用方法：
// 1. 按交易所分类的Provider（Binance、Historical等）
// 2. 统一的事件接口和状态管理
// 3. 向后兼容性验证
// 4. Provider工厂和动态创建

use binance_futures::core::provider::{
    // 新架构导入
    BinanceProvider, HistoricalDataProvider,
    ProviderType, ProviderCreator, AnyProvider,
    DataProvider, ControllableProvider,
    BinanceConnectionMode, HistoricalDataFormat,
    binance_provider::{BinanceProviderConfig},
    historical_provider::{HistoricalDataConfig},
    
    // 向后兼容导入
    BinanceWebSocketProvider,
    
    // 通用类型
    ProviderError,
};
use binance_futures::core::BinanceWebSocketConfig;
use binance_futures::events::EventType;
use serde_json;
use std::time::Duration;
use std::thread::sleep;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("=== 重构后Provider架构演示 ===\n");
    
    // 1. 演示新架构 - Binance Provider
    println!("1. 演示Binance Provider（新架构）");
    demo_binance_provider()?;
    
    // 2. 演示Historical Data Provider
    println!("\n2. 演示Historical Data Provider");
    demo_historical_provider()?;
    
    // 3. 演示Provider工厂模式
    println!("\n3. 演示Provider工厂模式");
    demo_provider_factory()?;
    
    // 4. 演示动态Provider创建
    println!("\n4. 演示动态Provider创建");
    demo_dynamic_provider_creation()?;
    
    // 5. 演示向后兼容性
    println!("\n5. 演示向后兼容性");
    demo_backward_compatibility()?;
    
    println!("\n=== 演示完成 ===");
    Ok(())
}

/// 演示Binance Provider的新架构
fn demo_binance_provider() -> Result<(), Box<dyn std::error::Error>> {
    let config = BinanceProviderConfig {
        symbol: "BTCUSDT".to_string(),
        connection_mode: BinanceConnectionMode::WebSocket,
        websocket_config: Some(Default::default()),
        rest_api_config: None,
        failover_config: Default::default(),
    };
    
    let mut provider = BinanceProvider::new(config);
    
    println!("  Provider类型: {:?}", provider.provider_type());
    println!("  支持的事件类型: {:?}", provider.supported_events());
    println!("  配置信息: {:?}", provider.get_config_info());
    
    // 初始化Provider
    println!("  正在初始化Provider...");
    provider.initialize()?;
    
    let status = provider.get_status();
    println!("  初始化状态: 运行={}, 连接={}", status.is_running, status.is_connected);
    
    // 注意：这里不实际启动WebSocket连接，因为演示环境
    println!("  Binance Provider演示完成");
    
    Ok(())
}

/// 演示Historical Data Provider
fn demo_historical_provider() -> Result<(), Box<dyn std::error::Error>> {
    // 创建一个临时的历史数据文件
    let temp_file = create_temp_historical_file()?;
    
    let config = HistoricalDataConfig {
        file_path: temp_file.clone(),
        format: HistoricalDataFormat::JSON,
        playback_config: Default::default(),
        buffer_config: Default::default(),
        time_config: Default::default(),
    };
    
    let mut provider = HistoricalDataProvider::new(config);
    
    println!("  Provider类型: {:?}", provider.provider_type());
    println!("  支持的事件类型: {:?}", provider.supported_events());
    println!("  配置信息: {:?}", provider.get_config_info());
    
    // 初始化Provider
    println!("  正在初始化Historical Provider...");
    provider.initialize()?;
    
    let status = provider.get_status();
    println!("  初始化状态: 运行={}, 连接={}", status.is_running, status.is_connected);
    
    // 演示播放控制功能
    println!("  演示播放控制功能...");
    provider.pause()?;
    println!("    已暂停播放");
    
    provider.set_playback_speed(2.0)?;
    println!("    播放速度设置为2.0x");
    
    provider.resume()?;
    println!("    恢复播放");
    
    if let Some(playback_info) = provider.get_playback_info() {
        println!("    播放信息: 速度={:.1}x, 进度={:.1}%", 
                playback_info.playback_speed, 
                playback_info.progress * 100.0);
    }
    
    println!("  Historical Provider演示完成");
    
    // 清理临时文件
    std::fs::remove_file(&temp_file).ok();
    
    Ok(())
}

/// 演示Provider工厂模式
fn demo_provider_factory() -> Result<(), Box<dyn std::error::Error>> {
    println!("  使用工厂方法创建Binance Provider...");
    
    let binance_config = BinanceProviderConfig {
        symbol: "ETHUSDT".to_string(),
        connection_mode: BinanceConnectionMode::WebSocket,
        websocket_config: Some(Default::default()),
        rest_api_config: None,
        failover_config: Default::default(),
    };
    
    let binance_provider = ProviderCreator::create_binance(binance_config)?;
    println!("    已创建Binance Provider: {:?}", binance_provider.provider_type());
    
    println!("  使用工厂方法创建Historical Provider...");
    
    let temp_file = create_temp_historical_file()?;
    let historical_config = HistoricalDataConfig {
        file_path: temp_file.clone(),
        format: HistoricalDataFormat::JSON,
        playback_config: Default::default(),
        buffer_config: Default::default(),
        time_config: Default::default(),
    };
    
    let historical_provider = ProviderCreator::create_historical(historical_config)?;
    println!("    已创建Historical Provider: {:?}", historical_provider.provider_type());
    
    println!("  工厂模式演示完成");
    
    // 清理
    std::fs::remove_file(&temp_file).ok();
    
    Ok(())
}

/// 演示动态Provider创建
fn demo_dynamic_provider_creation() -> Result<(), Box<dyn std::error::Error>> {
    println!("  通过ProviderType和JSON配置创建Provider...");
    
    // 创建Binance Provider
    let binance_provider_type = ProviderType::Binance { 
        mode: BinanceConnectionMode::WebSocket 
    };
    
    let binance_config_json = serde_json::json!({
        "symbol": "ADAUSDT",
        "connection_mode": "WebSocket",
        "websocket_config": {
            "streams": ["BookTicker", {"Depth": {"levels": 20, "update_speed": "Ms100"}}],
            "reconnect_config": {
                "enabled": true,
                "max_attempts": 5
            }
        },
        "failover_config": {
            "enabled": true
        }
    });
    
    match ProviderCreator::create_any_provider(binance_provider_type, binance_config_json) {
        Ok(any_provider) => {
            println!("    成功创建动态Binance Provider: {:?}", any_provider.provider_type());
            println!("    连接状态: {}", any_provider.is_connected());
        }
        Err(e) => {
            println!("    创建失败（预期的，因为配置解析问题）: {}", e);
        }
    }
    
    // 创建Historical Provider
    let temp_file = create_temp_historical_file()?;
    let historical_provider_type = ProviderType::HistoricalData { 
        format: HistoricalDataFormat::JSON 
    };
    
    let historical_config_json = serde_json::json!({
        "file_path": temp_file,
        "format": "JSON",
        "playback_config": {
            "initial_speed": 1.0,
            "auto_start": true
        }
    });
    
    match ProviderCreator::create_any_provider(historical_provider_type, historical_config_json) {
        Ok(any_provider) => {
            println!("    成功创建动态Historical Provider: {:?}", any_provider.provider_type());
        }
        Err(e) => {
            println!("    创建失败: {}", e);
        }
    }
    
    println!("  动态创建演示完成");
    
    // 清理
    std::fs::remove_file(&temp_file).ok();
    
    Ok(())
}

/// 演示向后兼容性
fn demo_backward_compatibility() -> Result<(), Box<dyn std::error::Error>> {
    println!("  验证与旧版WebSocket Provider的兼容性...");
    
    // 使用旧版API创建Provider
    #[allow(deprecated)]
    let old_config = BinanceWebSocketConfig::default();
    
    #[allow(deprecated)]
    let old_provider = BinanceWebSocketProvider::new(old_config);
    
    println!("    旧版Provider类型: {:?}", old_provider.provider_type());
    println!("    旧版支持的事件: {:?}", old_provider.supported_events());
    
    // 转换为AnyProvider
    #[allow(deprecated)]
    let any_provider: AnyProvider = old_provider.into();
    println!("    成功转换为AnyProvider: {:?}", any_provider.provider_type());
    
    // 使用WebSocketManager创建新版Provider（兼容模式）
    println!("  使用WebSocketManager创建新版Provider...");
    
    let ws_manager = binance_futures::websocket::WebSocketManager::new(
        binance_futures::websocket::WebSocketConfig::new("BTCUSDT".to_string())
    );
    
    let new_provider_from_old = ProviderCreator::create_binance_from_websocket(
        ws_manager, 
        "BTCUSDT".to_string()
    )?;
    
    println!("    从WebSocketManager创建的Provider类型: {:?}", new_provider_from_old.provider_type());
    println!("    WebSocket管理器可用: {}", new_provider_from_old.websocket_manager().is_some());
    
    println!("  向后兼容性验证完成");
    
    Ok(())
}

/// 创建临时的历史数据文件用于演示
fn create_temp_historical_file() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    use std::io::Write;
    
    let temp_file = std::env::temp_dir().join("demo_historical_data.json");
    let mut file = std::fs::File::create(&temp_file)?;
    
    // 写入一些示例数据
    writeln!(file, r#"{{"timestamp": 1640995200000, "price": 47000.0, "volume": 1.5, "e": "trade"}}"#)?;
    writeln!(file, r#"{{"timestamp": 1640995201000, "price": 47001.0, "volume": 2.0, "e": "trade"}}"#)?;
    writeln!(file, r#"{{"timestamp": 1640995202000, "price": 47002.0, "volume": 1.0, "e": "trade"}}"#)?;
    writeln!(file, r#"{{"timestamp": 1640995203000, "bid": 46999.0, "ask": 47003.0, "e": "bookTicker"}}"#)?;
    writeln!(file, r#"{{"timestamp": 1640995204000, "price": 47005.0, "volume": 3.0, "e": "trade"}}"#)?;
    
    file.flush()?;
    
    Ok(temp_file)
}

/// 演示不同Provider类型的统一接口使用
fn demo_unified_interface(providers: Vec<AnyProvider>) -> Result<(), Box<dyn std::error::Error>> {
    println!("  演示统一接口使用...");
    
    for (index, provider) in providers.iter().enumerate() {
        println!("    Provider {}: {:?}", index + 1, provider.provider_type());
        println!("      连接状态: {}", provider.is_connected());
        println!("      健康状态: {}", provider.health_check());
        println!("      支持事件: {:?}", provider.supported_events());
        
        let status = provider.get_status();
        println!("      状态: 运行={}, 事件={}", status.is_running, status.events_received);
    }
    
    Ok(())
}