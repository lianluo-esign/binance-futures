// Provider系统演示示例（更新为重构后架构）
//
// 本示例演示如何使用重构后的Provider系统：
// 1. 创建使用新Provider系统的ReactiveApp
// 2. 演示按交易所分类的Provider类型
// 3. 查看Provider状态信息
// 4. 演示向后兼容性
// 5. 展示Provider系统的基本功能

use binance_futures::{Config, ReactiveApp};
use binance_futures::app::reactive_app::ProviderInfo;
use binance_futures::core::provider::{
    ProviderManagerConfig, BinanceProviderConfig, BinanceConnectionMode, 
    HistoricalDataConfig, HistoricalDataFormat,
    ProviderMetadata, ProviderType, ProviderCreator,
    BinanceWebSocketConfig, StreamType, UpdateSpeed, // 向后兼容
};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Provider系统演示（重构后架构）");
    println!("==============================");

    // 创建基础配置
    let config = Config::default();
    println!("✓ 创建应用配置");

    // 1. 演示新架构Provider类型
    println!("\n1. 新架构Provider类型演示:");
    demo_new_provider_types();
    
    // 2. 演示传统模式兼容性
    println!("\n2. 传统WebSocket模式兼容性演示:");
    let traditional_app = ReactiveApp::new(config.clone());
    
    println!("  - 使用Provider系统: {}", traditional_app.is_using_provider_system());
    println!("  - Provider管理器: {:?}", traditional_app.provider_manager().is_some());
    println!("  - 活跃Provider状态: {:?}", traditional_app.get_active_provider_status().is_some());

    // 2. 演示Provider系统模式
    println!("\n2. Provider系统模式演示:");
    
    // 创建Provider管理器配置
    let provider_config = ProviderManagerConfig {
        default_provider_id: "demo_binance_websocket".to_string(),
        event_buffer_size: 5000,
        failover_enabled: true,
        health_check_interval_ms: 10000, // 10秒
        switch_timeout_ms: 15000, // 15秒
        ..Default::default()
    };

    // 创建使用Provider系统的应用
    let provider_app = ReactiveApp::new_with_provider_system(config, Some(provider_config))?;
    
    println!("  - 使用Provider系统: {}", provider_app.is_using_provider_system());
    println!("  - Provider管理器: {:?}", provider_app.provider_manager().is_some());
    
    // 3. 查看Provider状态信息
    println!("\n3. Provider状态信息:");
    
    if let Some(manager_status) = provider_app.get_provider_manager_status() {
        println!("  管理器状态:");
        println!("    - 注册的Provider数: {}", manager_status.registered_providers);
        println!("    - 健康的Provider数: {}", manager_status.healthy_providers);
        println!("    - 活跃Provider ID: {:?}", manager_status.active_provider_id);
        println!("    - 是否运行: {}", manager_status.is_running);
        println!("    - 切换次数: {}", manager_status.switch_count);
        println!("    - 处理事件总数: {}", manager_status.total_events_processed);
    }

    if let Some(active_status) = provider_app.get_active_provider_status() {
        println!("\n  活跃Provider状态:");
        println!("    - 是否连接: {}", active_status.is_connected);
        println!("    - 是否运行: {}", active_status.is_running);
        println!("    - 是否健康: {}", active_status.is_healthy);
        println!("    - 接收事件数: {}", active_status.events_received);
        println!("    - 错误计数: {}", active_status.error_count);
        println!("    - 连续错误: {}", active_status.consecutive_errors);
        println!("    - Provider指标: {}", active_status.provider_metrics.summary());
    }

    // 4. 获取所有Provider信息
    println!("\n4. 所有Provider信息:");
    let providers_info = provider_app.get_all_providers_info();
    for (index, provider) in providers_info.iter().enumerate() {
        println!("  Provider #{}: {}", index + 1, provider.id);
        println!("    - 类型: {}", provider.provider_type);
        println!("    - 连接状态: {}", provider.is_connected);
        println!("    - 健康状态: {}", provider.is_healthy);
        println!("    - 接收事件: {}", provider.events_received);
        println!("    - 错误计数: {}", provider.error_count);
        if let Some(last_event_time) = provider.last_event_time {
            println!("    - 最后事件时间: {}", last_event_time);
        }
    }

    // 5. 演示配置信息
    println!("\n5. 配置验证演示:");
    
    // 演示BinanceWebSocketConfig
    let ws_config = BinanceWebSocketConfig {
        symbol: "BTCUSDT".to_string(),
        streams: vec![
            StreamType::BookTicker,
            StreamType::Depth { 
                levels: 20, 
                update_speed: UpdateSpeed::Ms100 
            },
            StreamType::Trade,
            StreamType::Kline { 
                interval: "1m".to_string() 
            },
        ],
        heartbeat_interval_secs: 30,
        max_buffer_size: 1000,
        compression_enabled: false,
        ..Default::default()
    };

    println!("  WebSocket配置:");
    println!("    - 交易对: {}", ws_config.symbol);
    println!("    - 数据流数量: {}", ws_config.streams.len());
    println!("    - 心跳间隔: {}秒", ws_config.heartbeat_interval_secs);
    println!("    - 缓冲区大小: {}", ws_config.max_buffer_size);
    
    // 演示流名称生成
    println!("\n  生成的Binance流名称:");
    for (index, stream) in ws_config.streams.iter().enumerate() {
        let stream_name = stream.to_binance_stream(&ws_config.symbol);
        println!("    {}. {} -> {}", index + 1, format!("{:?}", stream), stream_name);
    }

    // 6. 演示Provider类型和事件类型
    println!("\n6. 类型系统演示:");
    
    let provider_types = vec![
        ProviderType::RealTime,
        ProviderType::Historical,
        ProviderType::Hybrid,
    ];

    for provider_type in provider_types {
        println!("  ProviderType::{} -> \"{}\" (支持播放控制: {})", 
            provider_type.as_str(), 
            provider_type.as_str(),
            provider_type.supports_playback_control()
        );
    }

    // 7. 演示Provider切换（仅API调用，不实际切换）
    println!("\n7. Provider切换演示:");
    
    // 这会失败，因为我们只有一个Provider
    match provider_app.switch_provider("non_existent_provider") {
        Ok(()) => println!("  Provider切换成功"),
        Err(e) => println!("  Provider切换失败: {}", e),
    }

    // 尝试切换到存在的Provider
    if let Some(first_provider) = providers_info.first() {
        match provider_app.switch_provider(&first_provider.id) {
            Ok(()) => println!("  Provider切换到 '{}' 成功", first_provider.id),
            Err(e) => println!("  Provider切换到 '{}' 失败: {}", first_provider.id, e),
        }
    }

    println!("\n✓ Provider系统演示完成!");
    println!("注意: 实际的网络连接可能无法在演示环境中建立，这是正常的。");
    println!("Provider系统的抽象层和管理功能已经完全实现并可用。");

    Ok(())
}

/// 演示新架构的Provider类型
fn demo_new_provider_types() {
    println!("  新架构Provider类型:");
    
    // Binance Provider类型
    let binance_ws_type = ProviderType::Binance { 
        mode: BinanceConnectionMode::WebSocket 
    };
    println!("    - {}: {}", binance_ws_type.as_str(), binance_ws_type.detailed_description());
    println!("      实时数据: {}, 播放控制: {}", 
             binance_ws_type.is_realtime(), 
             binance_ws_type.supports_playback_control());
    
    let binance_rest_type = ProviderType::Binance { 
        mode: BinanceConnectionMode::RestAPI 
    };
    println!("    - {}: {}", binance_rest_type.as_str(), binance_rest_type.detailed_description());
    
    // Historical Data Provider类型
    let historical_json_type = ProviderType::HistoricalData { 
        format: HistoricalDataFormat::JSON 
    };
    println!("    - {}: {}", historical_json_type.as_str(), historical_json_type.detailed_description());
    println!("      实时数据: {}, 播放控制: {}", 
             historical_json_type.is_realtime(), 
             historical_json_type.supports_playback_control());
    
    let historical_csv_type = ProviderType::HistoricalData { 
        format: HistoricalDataFormat::CSV 
    };
    println!("    - {}: {}", historical_csv_type.as_str(), historical_csv_type.detailed_description());
    
    // Custom Provider类型
    let custom_type = ProviderType::Custom { 
        identifier: "MyExchange".to_string(),
        description: "自定义交易所接口".to_string(),
    };
    println!("    - {}: {}", custom_type.as_str(), custom_type.detailed_description());
}

/// 演示Provider创建和配置
fn demo_provider_creation() -> Result<(), Box<dyn std::error::Error>> {
    println!("  Provider创建演示:");
    
    // 创建Binance Provider配置
    let binance_config = BinanceProviderConfig {
        symbol: "BTCUSDT".to_string(),
        connection_mode: BinanceConnectionMode::WebSocket,
        websocket_config: Some(Default::default()),
        rest_api_config: None,
        failover_config: Default::default(),
    };
    
    // 使用工厂创建Provider
    let binance_provider = ProviderCreator::create_binance(binance_config)?;
    println!("    ✓ 创建Binance Provider: {:?}", binance_provider.provider_type());
    
    // 创建Historical Data Provider配置
    let historical_config = HistoricalDataConfig {
        file_path: "data/demo.json".into(),
        format: HistoricalDataFormat::JSON,
        playback_config: Default::default(),
        buffer_config: Default::default(),
        time_config: Default::default(),
    };
    
    let historical_provider = ProviderCreator::create_historical(historical_config)?;
    println!("    ✓ 创建Historical Provider: {:?}", historical_provider.provider_type());
    
    Ok(())
}