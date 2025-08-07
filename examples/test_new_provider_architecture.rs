// 新Provider架构测试示例
//
// 此示例用于测试和验证重构后的Provider架构是否正常工作

use binance_futures::core::provider::{
    BinanceProvider, BinanceProviderConfig, BinanceConnectionMode,
    HistoricalDataProvider, HistoricalDataConfig, HistoricalDataFormat,
    ProviderType, ProviderCreator, AnyProvider,
    DataProvider, ControllableProvider,
    ProviderError,
};
use std::io::Write;
use std::fs::File;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("=== Provider架构测试 ===\n");
    
    // 测试1: Provider类型系统
    test_provider_types();
    
    // 测试2: Binance Provider创建和配置
    test_binance_provider()?;
    
    // 测试3: Historical Data Provider创建和配置
    test_historical_provider()?;
    
    // 测试4: Provider工厂方法
    test_provider_factory()?;
    
    // 测试5: AnyProvider枚举
    test_any_provider()?;
    
    // 测试6: 错误处理
    test_error_handling()?;
    
    println!("=== 所有测试通过! ===");
    Ok(())
}

/// 测试Provider类型系统
fn test_provider_types() {
    println!("测试1: Provider类型系统");
    
    let binance_ws = ProviderType::Binance { 
        mode: BinanceConnectionMode::WebSocket 
    };
    assert!(binance_ws.is_realtime());
    assert!(!binance_ws.supports_playback_control());
    assert!(binance_ws.is_exchange());
    println!("  ✓ Binance WebSocket类型测试通过");
    
    let binance_rest = ProviderType::Binance { 
        mode: BinanceConnectionMode::RestAPI 
    };
    assert!(!binance_rest.is_realtime());
    assert!(!binance_rest.supports_playback_control());
    assert!(binance_rest.is_exchange());
    println!("  ✓ Binance REST API类型测试通过");
    
    let historical = ProviderType::HistoricalData { 
        format: HistoricalDataFormat::JSON 
    };
    assert!(!historical.is_realtime());
    assert!(historical.supports_playback_control());
    assert!(!historical.is_exchange());
    println!("  ✓ Historical Data类型测试通过");
    
    let custom = ProviderType::Custom { 
        identifier: "TestExchange".to_string(),
        description: "测试交易所".to_string(),
    };
    assert!(!custom.is_realtime());
    assert!(!custom.supports_playback_control());
    assert!(!custom.is_exchange());
    println!("  ✓ Custom类型测试通过");
    
    // 测试字符串转换
    assert_eq!(binance_ws.as_str(), "Binance");
    assert_eq!(historical.as_str(), "HistoricalData");
    assert_eq!(custom.as_str(), "TestExchange");
    println!("  ✓ 字符串转换测试通过");
    
    // 测试from_str
    assert_eq!(
        ProviderType::from_str("binance").unwrap().as_str(),
        "Binance"
    );
    assert_eq!(
        ProviderType::from_str("historical").unwrap().as_str(),
        "HistoricalData"
    );
    println!("  ✓ from_str解析测试通过");
    
    println!("  Provider类型系统测试完成\n");
}

/// 测试Binance Provider
fn test_binance_provider() -> Result<(), Box<dyn std::error::Error>> {
    println!("测试2: Binance Provider");
    
    let config = BinanceProviderConfig {
        symbol: "BTCUSDT".to_string(),
        connection_mode: BinanceConnectionMode::WebSocket,
        websocket_config: Some(Default::default()),
        rest_api_config: None,
        failover_config: Default::default(),
    };
    
    let mut provider = BinanceProvider::new(config);
    println!("  ✓ Binance Provider创建成功");
    
    // 检查Provider类型
    assert_eq!(
        provider.provider_type(),
        ProviderType::Binance { mode: BinanceConnectionMode::WebSocket }
    );
    println!("  ✓ Provider类型验证通过");
    
    // 检查支持的事件类型
    assert!(!provider.supported_events().is_empty());
    println!("  ✓ 支持的事件类型非空");
    
    // 初始化测试
    provider.initialize()?;
    println!("  ✓ Provider初始化成功");
    
    // 检查状态
    let status = provider.get_status();
    assert!(!status.is_running); // 初始化后但未启动
    println!("  ✓ 状态检查通过");
    
    // 检查配置信息
    assert!(provider.get_config_info().is_some());
    println!("  ✓ 配置信息获取成功");
    
    // 健康检查
    let is_healthy = provider.health_check();
    println!("  ✓ 健康检查完成: {}", is_healthy);
    
    println!("  Binance Provider测试完成\n");
    Ok(())
}

/// 测试Historical Data Provider
fn test_historical_provider() -> Result<(), Box<dyn std::error::Error>> {
    println!("测试3: Historical Data Provider");
    
    // 创建临时测试文件
    let temp_path = std::env::temp_dir().join("test_historical_data.json");
    let mut temp_file = File::create(&temp_path)?;
    writeln!(temp_file, r#"{{"timestamp": 1640995200000, "price": 47000.0, "volume": 1.5, "e": "trade"}}"#)?;
    writeln!(temp_file, r#"{{"timestamp": 1640995201000, "price": 47001.0, "volume": 2.0, "e": "trade"}}"#)?;
    writeln!(temp_file, r#"{{"timestamp": 1640995202000, "price": 47002.0, "volume": 1.0, "e": "trade"}}"#)?;
    temp_file.flush()?;
    drop(temp_file); // Close the file
    
    let config = HistoricalDataConfig {
        file_path: temp_path,
        format: HistoricalDataFormat::JSON,
        playback_config: Default::default(),
        buffer_config: Default::default(),
        time_config: Default::default(),
    };
    
    let mut provider = HistoricalDataProvider::new(config);
    println!("  ✓ Historical Data Provider创建成功");
    
    // 检查Provider类型
    assert_eq!(
        provider.provider_type(),
        ProviderType::HistoricalData { format: HistoricalDataFormat::JSON }
    );
    println!("  ✓ Provider类型验证通过");
    
    // 初始化测试
    provider.initialize()?;
    println!("  ✓ Provider初始化成功");
    
    // 测试播放控制功能
    provider.pause()?;
    println!("  ✓ 暂停功能正常");
    
    provider.set_playback_speed(2.0)?;
    println!("  ✓ 播放速度设置成功");
    
    provider.resume()?;
    println!("  ✓ 恢复播放成功");
    
    // 检查播放信息
    if let Some(playback_info) = provider.get_playback_info() {
        assert_eq!(playback_info.playback_speed, 2.0);
        println!("  ✓ 播放信息获取成功");
    }
    
    // 测试无效播放速度
    let result = provider.set_playback_speed(1000.0);
    assert!(result.is_err());
    println!("  ✓ 无效播放速度正确拒绝");
    
    println!("  Historical Data Provider测试完成\n");
    Ok(())
}

/// 测试Provider工厂方法
fn test_provider_factory() -> Result<(), Box<dyn std::error::Error>> {
    println!("测试4: Provider工厂方法");
    
    // 测试Binance Provider创建
    let binance_config = BinanceProviderConfig {
        symbol: "ETHUSDT".to_string(),
        connection_mode: BinanceConnectionMode::WebSocket,
        websocket_config: Some(Default::default()),
        rest_api_config: None,
        failover_config: Default::default(),
    };
    
    let binance_provider = ProviderCreator::create_binance(binance_config)?;
    assert_eq!(binance_provider.provider_type().as_str(), "Binance");
    println!("  ✓ Binance Provider工厂创建成功");
    
    // 测试Historical Provider创建
    let temp_path2 = std::env::temp_dir().join("test_historical_data2.json");
    let _ = File::create(&temp_path2)?; // Create the file
    let historical_config = HistoricalDataConfig {
        file_path: temp_path2,
        format: HistoricalDataFormat::JSON,
        playback_config: Default::default(),
        buffer_config: Default::default(),
        time_config: Default::default(),
    };
    
    let historical_provider = ProviderCreator::create_historical(historical_config)?;
    assert_eq!(historical_provider.provider_type().as_str(), "HistoricalData");
    println!("  ✓ Historical Data Provider工厂创建成功");
    
    println!("  Provider工厂方法测试完成\n");
    Ok(())
}

/// 测试AnyProvider枚举
fn test_any_provider() -> Result<(), Box<dyn std::error::Error>> {
    println!("测试5: AnyProvider枚举");
    
    // 创建不同类型的Provider
    let binance_config = BinanceProviderConfig {
        symbol: "ADAUSDT".to_string(),
        connection_mode: BinanceConnectionMode::WebSocket,
        websocket_config: Some(Default::default()),
        rest_api_config: None,
        failover_config: Default::default(),
    };
    
    let binance_provider = ProviderCreator::create_binance(binance_config)?;
    let any_binance: AnyProvider = binance_provider.into();
    
    let temp_path3 = std::env::temp_dir().join("test_historical_data3.json");
    let _ = File::create(&temp_path3)?; // Create the file
    let historical_config = HistoricalDataConfig {
        file_path: temp_path3,
        format: HistoricalDataFormat::JSON,
        playback_config: Default::default(),
        buffer_config: Default::default(),
        time_config: Default::default(),
    };
    
    let historical_provider = ProviderCreator::create_historical(historical_config)?;
    let any_historical: AnyProvider = historical_provider.into();
    
    // 测试统一接口
    let providers = vec![any_binance, any_historical];
    
    for provider in &providers {
        let provider_type = provider.provider_type();
        let is_connected = provider.is_connected();
        let health_status = provider.health_check();
        let supported_events = provider.supported_events();
        
        println!("  ✓ Provider类型: {}", provider_type.as_str());
        println!("    连接状态: {}", is_connected);
        println!("    健康状态: {}", health_status);
        println!("    支持事件数量: {}", supported_events.len());
    }
    
    println!("  AnyProvider枚举测试完成\n");
    Ok(())
}

/// 测试错误处理
fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    println!("测试6: 错误处理");
    
    // 测试配置错误
    let invalid_config = BinanceProviderConfig {
        symbol: String::new(), // 无效的空符号
        connection_mode: BinanceConnectionMode::WebSocket,
        websocket_config: Some(Default::default()),
        rest_api_config: None,
        failover_config: Default::default(),
    };
    
    let mut invalid_provider = BinanceProvider::new(invalid_config);
    let result = invalid_provider.initialize();
    assert!(result.is_err());
    
    if let Err(ProviderError::ConfigurationError { field, .. }) = result {
        assert_eq!(field, Some("symbol".to_string()));
        println!("  ✓ 配置错误正确检测和报告");
    } else {
        panic!("期望配置错误");
    }
    
    // 测试文件不存在错误
    let nonexistent_config = HistoricalDataConfig {
        file_path: "/nonexistent/file.json".into(),
        format: HistoricalDataFormat::JSON,
        playback_config: Default::default(),
        buffer_config: Default::default(),
        time_config: Default::default(),
    };
    
    let mut nonexistent_provider = HistoricalDataProvider::new(nonexistent_config);
    let result = nonexistent_provider.initialize();
    assert!(result.is_err());
    println!("  ✓ 文件不存在错误正确处理");
    
    println!("  错误处理测试完成\n");
    Ok(())
}