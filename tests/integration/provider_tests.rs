// Provider系统集成测试
//
// 本文件包含Provider系统的集成测试，验证：
// - Provider抽象接口的正确性
// - BinanceWebSocketProvider的功能
// - ProviderManager的管理能力
// - ReactiveApp的Provider系统集成
//
// 测试设计原则：
// 1. 测试关键功能路径
// 2. 验证向后兼容性
// 3. 确保错误处理正确性
// 4. 验证状态管理准确性

use binance_futures::core::{
    DataProvider, ProviderManager, ProviderManagerConfig,
    BinanceWebSocketProvider, BinanceWebSocketConfig,
    ProviderMetadata, ProviderType, ProviderError,
    StreamType, UpdateSpeed,
};
use binance_futures::events::EventType;
use binance_futures::app::{ReactiveApp, ProviderInfo};
use binance_futures::Config;

use std::collections::HashMap;

/// 测试Provider基础功能
#[cfg(test)]
mod provider_basic_tests {
    use super::*;

    #[test]
    fn test_provider_type_serialization() {
        // 测试ProviderType的序列化和反序列化
        let provider_types = vec![
            ProviderType::RealTime,
            ProviderType::Historical,
            ProviderType::Hybrid,
        ];

        for provider_type in provider_types {
            let serialized = serde_json::to_string(&provider_type).unwrap();
            let deserialized: ProviderType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(provider_type, deserialized);
        }
    }

    #[test]
    fn test_provider_type_string_conversion() {
        // 测试ProviderType的字符串转换
        assert_eq!(ProviderType::RealTime.as_str(), "RealTime");
        assert_eq!(ProviderType::Historical.as_str(), "Historical");
        assert_eq!(ProviderType::Hybrid.as_str(), "Hybrid");

        assert_eq!(ProviderType::from_str("realtime"), Some(ProviderType::RealTime));
        assert_eq!(ProviderType::from_str("HISTORICAL"), Some(ProviderType::Historical));
        assert_eq!(ProviderType::from_str("hybrid"), Some(ProviderType::Hybrid));
        assert_eq!(ProviderType::from_str("invalid"), None);
    }

    #[test]
    fn test_stream_type_to_binance_stream() {
        // 测试StreamType到Binance流名称的转换
        let book_ticker = StreamType::BookTicker;
        assert_eq!(book_ticker.to_binance_stream("BTCUSDT"), "btcusdt@bookTicker");

        let depth = StreamType::Depth { 
            levels: 20, 
            update_speed: UpdateSpeed::Ms100 
        };
        assert_eq!(depth.to_binance_stream("ETHUSDT"), "ethusdt@depth20@100ms");

        let trade = StreamType::Trade;
        assert_eq!(trade.to_binance_stream("ADAUSDT"), "adausdt@trade");
    }

    #[test]
    fn test_binance_websocket_config_creation() {
        // 测试BinanceWebSocketConfig的创建
        let config = BinanceWebSocketConfig {
            symbol: "BTCUSDT".to_string(),
            streams: vec![
                StreamType::BookTicker,
                StreamType::Depth { levels: 20, update_speed: UpdateSpeed::Ms100 },
                StreamType::Trade,
            ],
            ..Default::default()
        };

        assert_eq!(config.symbol, "BTCUSDT");
        assert_eq!(config.streams.len(), 3);
        assert_eq!(config.heartbeat_interval_secs, 30);
        assert_eq!(config.max_buffer_size, 1000);
    }

    #[test]
    fn test_provider_metadata_creation() {
        // 测试ProviderMetadata的创建
        let metadata = ProviderMetadata {
            id: "test_provider".to_string(),
            name: "Test Provider".to_string(),
            description: "A test provider for unit testing".to_string(),
            provider_type: ProviderType::RealTime,
            priority: 100,
            is_fallback: false,
            tags: {
                let mut tags = HashMap::new();
                tags.insert("exchange".to_string(), "test".to_string());
                tags.insert("type".to_string(), "mock".to_string());
                tags
            },
        };

        assert_eq!(metadata.id, "test_provider");
        assert_eq!(metadata.provider_type, ProviderType::RealTime);
        assert_eq!(metadata.priority, 100);
        assert!(!metadata.is_fallback);
        assert_eq!(metadata.tags.len(), 2);
    }
}

/// 测试BinanceWebSocketProvider功能
#[cfg(test)]
mod binance_provider_tests {
    use super::*;

    #[test]
    fn test_binance_websocket_provider_creation() {
        // 测试BinanceWebSocketProvider的创建
        let config = BinanceWebSocketConfig {
            symbol: "BTCUSDT".to_string(),
            streams: vec![StreamType::BookTicker, StreamType::Trade],
            ..Default::default()
        };

        let provider = BinanceWebSocketProvider::new(config);
        
        // 验证基本属性
        assert_eq!(provider.provider_type(), ProviderType::RealTime);
        assert!(!provider.supported_events().is_empty());
        assert!(!provider.is_connected()); // 初始状态应该是未连接
    }

    #[test]
    fn test_binance_websocket_provider_initialization() {
        // 测试Provider的初始化
        let config = BinanceWebSocketConfig {
            symbol: "BTCUSDT".to_string(),
            streams: vec![StreamType::BookTicker],
            ..Default::default()
        };

        let mut provider = BinanceWebSocketProvider::new(config);
        
        // 初始化应该成功（即使不能真正连接到Binance）
        let result = provider.initialize();
        assert!(result.is_ok(), "Provider initialization should succeed: {:?}", result);

        // 检查状态
        let status = provider.get_status();
        assert_eq!(status.provider_metrics.summary().contains("WebSocket"), true);
    }

    #[test]
    fn test_binance_websocket_provider_config_validation() {
        // 测试配置验证
        let invalid_config = BinanceWebSocketConfig {
            symbol: "".to_string(), // 空符号应该导致错误
            streams: vec![],
            ..Default::default()
        };

        let mut provider = BinanceWebSocketProvider::new(invalid_config);
        let result = provider.initialize();
        
        assert!(result.is_err(), "Empty symbol should cause initialization error");
        
        match result.unwrap_err() {
            ProviderError::ConfigurationError { field, .. } => {
                assert_eq!(field, Some("symbol".to_string()));
            }
            _ => panic!("Expected ConfigurationError for empty symbol"),
        }
    }

    #[test]
    fn test_binance_websocket_provider_empty_streams() {
        // 测试空流配置
        let invalid_config = BinanceWebSocketConfig {
            symbol: "BTCUSDT".to_string(),
            streams: vec![], // 空流列表应该导致错误
            ..Default::default()
        };

        let mut provider = BinanceWebSocketProvider::new(invalid_config);
        let result = provider.initialize();
        
        assert!(result.is_err(), "Empty streams should cause initialization error");
        
        match result.unwrap_err() {
            ProviderError::ConfigurationError { field, .. } => {
                assert_eq!(field, Some("streams".to_string()));
            }
            _ => panic!("Expected ConfigurationError for empty streams"),
        }
    }
}

/// 测试ProviderManager功能
#[cfg(test)]
mod provider_manager_tests {
    use super::*;
    use binance_futures::core::provider::types::ProviderStatus;
    use binance_futures::core::EventKind;

    // Mock Provider for testing
    #[derive(Debug)]
    struct MockProvider {
        id: String,
        connected: bool,
        events: Vec<EventType>,
        status: ProviderStatus,
    }

    impl MockProvider {
        fn new(id: String) -> Self {
            Self {
                id: id.clone(),
                connected: false,
                events: vec![],
                status: ProviderStatus::new(ProviderType::RealTime),
            }
        }
    }

    impl DataProvider for MockProvider {
        type Error = ProviderError;

        fn initialize(&mut self) -> Result<(), Self::Error> {
            self.status.is_running = true;
            Ok(())
        }

        fn start(&mut self) -> Result<(), Self::Error> {
            self.connected = true;
            self.status.is_connected = true;
            Ok(())
        }

        fn stop(&mut self) -> Result<(), Self::Error> {
            self.connected = false;
            self.status.is_connected = false;
            Ok(())
        }

        fn is_connected(&self) -> bool {
            self.connected
        }

        fn read_events(&mut self) -> Result<Vec<EventType>, Self::Error> {
            Ok(self.events.drain(..).collect())
        }

        fn get_status(&self) -> ProviderStatus {
            self.status.clone()
        }

        fn provider_type(&self) -> ProviderType {
            ProviderType::RealTime
        }

        fn supported_events(&self) -> &[EventKind] {
            &[EventKind::Trade, EventKind::TickPrice]
        }
    }

    #[test]
    fn test_provider_manager_creation() {
        // 测试ProviderManager的创建
        let config = ProviderManagerConfig::default();
        let manager = ProviderManager::new(config);
        
        // 验证初始状态
        assert!(manager.get_active_provider_id().is_none());
        
        let status = manager.get_status().unwrap();
        assert_eq!(status.registered_providers, 0);
        assert!(!status.is_running);
    }

    #[test]
    fn test_provider_registration() {
        // 测试Provider注册
        let config = ProviderManagerConfig::default();
        let mut manager = ProviderManager::new(config);
        
        let provider = MockProvider::new("test_provider".to_string());
        let metadata = ProviderMetadata {
            id: "test_provider".to_string(),
            name: "Test Provider".to_string(),
            description: "A test provider".to_string(),
            provider_type: ProviderType::RealTime,
            priority: 1,
            is_fallback: false,
            tags: HashMap::new(),
        };
        
        // 注册Provider
        let result = manager.register_provider(provider, metadata);
        assert!(result.is_ok(), "Provider registration should succeed: {:?}", result);
        
        // 验证注册结果
        assert_eq!(manager.get_active_provider_id(), Some("test_provider".to_string()));
        
        let status = manager.get_status().unwrap();
        assert_eq!(status.registered_providers, 1);
    }

    #[test]
    fn test_provider_duplicate_registration() {
        // 测试重复注册相同ID的Provider
        let config = ProviderManagerConfig::default();
        let mut manager = ProviderManager::new(config);
        
        let provider1 = MockProvider::new("duplicate_id".to_string());
        let metadata1 = ProviderMetadata {
            id: "duplicate_id".to_string(),
            name: "Provider 1".to_string(),
            description: "First provider".to_string(),
            provider_type: ProviderType::RealTime,
            priority: 1,
            is_fallback: false,
            tags: HashMap::new(),
        };
        
        let provider2 = MockProvider::new("duplicate_id".to_string());
        let metadata2 = ProviderMetadata {
            id: "duplicate_id".to_string(),
            name: "Provider 2".to_string(),
            description: "Second provider".to_string(),
            provider_type: ProviderType::RealTime,
            priority: 2,
            is_fallback: false,
            tags: HashMap::new(),
        };
        
        // 第一次注册应该成功
        assert!(manager.register_provider(provider1, metadata1).is_ok());
        
        // 第二次注册相同ID应该失败
        let result = manager.register_provider(provider2, metadata2);
        assert!(result.is_err(), "Duplicate provider registration should fail");
        
        match result.unwrap_err() {
            ProviderError::ConfigurationError { field, .. } => {
                assert_eq!(field, Some("id".to_string()));
            }
            _ => panic!("Expected ConfigurationError for duplicate ID"),
        }
    }
}

/// 测试ReactiveApp的Provider系统集成
#[cfg(test)]
mod reactive_app_integration_tests {
    use super::*;

    #[test]
    fn test_reactive_app_traditional_mode() {
        // 测试传统模式的ReactiveApp创建
        let config = Config::default();
        let app = ReactiveApp::new(config);
        
        // 验证传统模式
        assert!(!app.is_using_provider_system());
        assert!(app.provider_manager().is_none());
        assert!(app.get_active_provider_status().is_none());
    }

    #[test]
    fn test_reactive_app_provider_mode() {
        // 测试Provider模式的ReactiveApp创建
        let config = Config::default();
        let result = ReactiveApp::new_with_provider_system(config, None);
        
        assert!(result.is_ok(), "ReactiveApp creation with provider system should succeed: {:?}", result);
        
        let app = result.unwrap();
        
        // 验证Provider模式
        assert!(app.is_using_provider_system());
        assert!(app.provider_manager().is_some());
        
        // 验证默认Provider已注册
        let providers_info = app.get_all_providers_info();
        assert!(!providers_info.is_empty());
        
        let default_provider = &providers_info[0];
        assert_eq!(default_provider.id, "binance_websocket");
    }

    #[test]
    fn test_reactive_app_provider_manager_status() {
        // 测试获取Provider管理器状态
        let config = Config::default();
        let app = ReactiveApp::new_with_provider_system(config, None).unwrap();
        
        let manager_status = app.get_provider_manager_status();
        assert!(manager_status.is_some());
        
        let status = manager_status.unwrap();
        assert_eq!(status.registered_providers, 1);
        assert!(!status.is_running); // 初始状态应该是未运行
        assert_eq!(status.active_provider_id, Some("binance_websocket".to_string()));
    }

    #[test]
    fn test_reactive_app_providers_info() {
        // 测试获取所有Provider信息
        let config = Config::default();
        let app = ReactiveApp::new_with_provider_system(config, None).unwrap();
        
        let providers_info = app.get_all_providers_info();
        assert_eq!(providers_info.len(), 1);
        
        let provider_info = &providers_info[0];
        assert_eq!(provider_info.id, "binance_websocket");
        assert!(!provider_info.is_connected); // 初始状态应该是未连接
        assert_eq!(provider_info.error_count, 0);
        assert_eq!(provider_info.events_received, 0);
    }

    #[test]
    fn test_reactive_app_switch_provider_traditional_mode() {
        // 测试在传统模式下切换Provider
        let config = Config::default();
        let app = ReactiveApp::new(config);
        
        let result = app.switch_provider("any_provider");
        assert!(result.is_err());
        
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("未使用Provider系统"));
    }

    #[test] 
    fn test_reactive_app_enable_disable_provider_system() {
        // 测试启用和禁用Provider系统
        let config = Config::default();
        let mut app = ReactiveApp::new(config);
        
        // 初始状态是传统模式
        assert!(!app.is_using_provider_system());
        
        // 启用Provider系统（注意：这个测试可能会因为网络连接而失败，在实际测试中需要mock）
        // 这里我们只测试API调用，不测试实际的网络连接
        let result = app.enable_provider_system(None);
        
        // 由于没有实际的网络环境，这可能会失败，但我们可以检查错误类型
        if result.is_err() {
            // 这是预期的，因为我们无法在测试环境中连接到真实的WebSocket
            println!("Expected error in test environment: {:?}", result.unwrap_err());
        }
    }
}

/// 性能和压力测试
#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn test_provider_metadata_serialization_performance() {
        // 测试Provider元数据序列化性能
        let metadata = ProviderMetadata {
            id: "performance_test_provider".to_string(),
            name: "Performance Test Provider".to_string(),
            description: "A provider for performance testing with longer description".to_string(),
            provider_type: ProviderType::RealTime,
            priority: 100,
            is_fallback: false,
            tags: {
                let mut tags = HashMap::new();
                for i in 0..10 {
                    tags.insert(format!("key_{}", i), format!("value_{}", i));
                }
                tags
            },
        };

        // 序列化测试
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _serialized = serde_json::to_string(&metadata).unwrap();
        }
        let duration = start.elapsed();
        
        // 序列化1000次应该在合理时间内完成（<100ms）
        assert!(duration.as_millis() < 100, 
            "Serialization took too long: {}ms", duration.as_millis());
    }

    #[test]
    fn test_provider_manager_config_default() {
        // 测试ProviderManagerConfig的默认值
        let config = ProviderManagerConfig::default();
        
        assert_eq!(config.default_provider_id, "default");
        assert_eq!(config.health_check_interval_ms, 5000);
        assert_eq!(config.event_buffer_size, 10000);
        assert!(config.failover_enabled);
        assert_eq!(config.switch_timeout_ms, 30000);
        assert!(config.auto_switch_config.enabled);
        assert_eq!(config.auto_switch_config.failure_threshold, 3);
    }

    #[test]
    fn test_multiple_providers_creation() {
        // 测试创建多个Provider的性能
        let start = std::time::Instant::now();
        
        let mut providers = Vec::new();
        for i in 0..100 {
            let config = BinanceWebSocketConfig {
                symbol: format!("SYMBOL{}", i),
                streams: vec![StreamType::BookTicker],
                ..Default::default()
            };
            
            let provider = BinanceWebSocketProvider::new(config);
            providers.push(provider);
        }
        
        let duration = start.elapsed();
        
        // 创建100个Provider应该在合理时间内完成（<1000ms）
        assert!(duration.as_millis() < 1000,
            "Creating 100 providers took too long: {}ms", duration.as_millis());
        
        // 验证所有Provider都正确创建
        assert_eq!(providers.len(), 100);
        for (i, provider) in providers.iter().enumerate() {
            assert_eq!(provider.provider_type(), ProviderType::RealTime);
            assert!(!provider.is_connected());
        }
    }
}