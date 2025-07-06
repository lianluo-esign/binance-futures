use flow_sight::{Config, ReactiveApp, Event, EventType};
use serde_json::json;
use std::time::Duration;

#[test]
fn test_event_bus_basic_functionality() {
    // 创建配置
    let config = Config::new("BTCUSDT".to_string())
        .with_buffer_size(65536);
    
    // 创建应用程序
    let mut app = ReactiveApp::new(config);
    
    // 验证初始状态
    assert_eq!(app.get_symbol(), "BTCUSDT");
    assert!(!app.is_running());
    
    // 获取初始统计信息
    let stats = app.get_stats();
    assert_eq!(stats.total_events_processed, 0);
    assert_eq!(stats.pending_events, 0);
}

#[test]
fn test_market_snapshot() {
    let config = Config::new("ETHUSDT".to_string());
    let app = ReactiveApp::new(config);
    
    // 获取市场快照
    let snapshot = app.get_market_snapshot();
    
    // 验证初始状态
    assert!(snapshot.best_bid_price.is_none());
    assert!(snapshot.best_ask_price.is_none());
    assert!(snapshot.current_price.is_none());
    assert_eq!(snapshot.bid_volume_ratio, 0.5);
    assert_eq!(snapshot.ask_volume_ratio, 0.5);
}

#[test]
fn test_event_creation() {
    // 测试事件创建
    let trade_data = json!({
        "p": "50000.00",
        "q": "0.1",
        "m": false
    });
    
    let event = Event::new(
        EventType::Trade(trade_data),
        "test".to_string()
    );
    
    // 验证事件属性
    assert_eq!(event.source, "test");
    assert!(event.timestamp > 0);
    
    match event.event_type {
        EventType::Trade(_) => {
            // 正确的事件类型
        }
        _ => panic!("错误的事件类型"),
    }
}

#[test]
fn test_config_builder() {
    let config = Config::new("ADAUSDT".to_string())
        .with_buffer_size(65536)
        .with_max_reconnects(10)
        .with_log_level("debug".to_string());
    
    assert_eq!(config.symbol, "ADAUSDT");
    assert_eq!(config.event_buffer_size, 65536);
    assert_eq!(config.max_reconnect_attempts, 10);
    assert_eq!(config.log_level, "debug");
}

#[test]
fn test_event_type_classification() {
    let trade_data = json!({"p": "100", "q": "1"});
    let depth_data = json!({"b": [], "a": []});
    let signal_data = json!({"signal_type": "test"});
    
    let trade_event = EventType::Trade(trade_data);
    let depth_event = EventType::DepthUpdate(depth_data);
    let signal_event = EventType::Signal(signal_data);
    
    assert_eq!(trade_event.type_name(), "Trade");
    assert_eq!(depth_event.type_name(), "DepthUpdate");
    assert_eq!(signal_event.type_name(), "Signal");
    
    assert!(trade_event.is_market_data());
    assert!(depth_event.is_market_data());
    assert!(!signal_event.is_market_data());
    
    assert!(signal_event.is_signal());
    assert!(!trade_event.is_signal());
}
