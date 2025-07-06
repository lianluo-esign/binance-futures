use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use log::info;

use flow_sight::events::event_bus::EventBus;
use flow_sight::websocket::{
    MultiExchangeManagerBuilder, ExchangeConfig, ExchangeType
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 初始化日志
    env_logger::init();
    
    info!("启动多交易所WebSocket管理器示例");
    
    // 创建事件总线
    let event_bus = Arc::new(RwLock::new(EventBus::new(65536)));
    
    // 创建OKX配置
    let okx_config = ExchangeConfig {
        exchange_name: "OKX".to_string(),
        symbol: "BTCUSDT".to_string(),
        testnet: false,
        api_key: None,
        api_secret: None,
    };
    
    // 创建Bybit配置
    let bybit_config = ExchangeConfig {
        exchange_name: "Bybit".to_string(),
        symbol: "BTCUSDT".to_string(),
        testnet: false,
        api_key: None,
        api_secret: None,
    };
    
        // 创建Coinbase配置
    let coinbase_config = ExchangeConfig {
        exchange_name: "Coinbase".to_string(),
        symbol: "BTCUSDT".to_string(),
        testnet: false,
        api_key: None,
        api_secret: None,
    };

    // 创建Bitget配置
    let bitget_config = ExchangeConfig {
        exchange_name: "Bitget".to_string(),
        symbol: "BTCUSDT".to_string(),
        testnet: false,
        api_key: None,
        api_secret: None,
    };

    // 使用构建器创建多交易所管理器
    let mut manager = MultiExchangeManagerBuilder::new()
        .with_exchanges(vec![ExchangeType::Okx, ExchangeType::Bybit, ExchangeType::Coinbase, ExchangeType::Bitget])
        .with_exchange_config(ExchangeType::Okx, okx_config)
        .with_exchange_config(ExchangeType::Bybit, bybit_config)
        .with_exchange_config(ExchangeType::Coinbase, coinbase_config)
        .with_exchange_config(ExchangeType::Bitget, bitget_config)
        .with_auto_reconnect(true)
        .with_reconnect_interval(5)
        .with_max_reconnect_attempts(3)
        .build(event_bus.clone());
    
    // 初始化管理器
    info!("初始化多交易所管理器...");
    manager.initialize().await?;
    
    // 启动连接
    info!("启动WebSocket连接...");
    manager.start().await?;
    
    // 订阅BTCUSDT永续合约数据
    info!("订阅BTCUSDT永续合约数据...");
    manager.subscribe_btcusdt_perpetual().await?;
    
    // 添加交易所
    manager.add_exchange(ExchangeType::OKX).await?;
    manager.add_exchange(ExchangeType::Bybit).await?;
    manager.add_exchange(ExchangeType::Coinbase).await?;
    manager.add_exchange(ExchangeType::Bitget).await?;
    manager.add_exchange(ExchangeType::Bitfinex).await?;
    
    // 运行5分钟，处理消息
    info!("开始处理消息，将运行5分钟...");
    let end_time = std::time::Instant::now() + Duration::from_secs(300);
    
    while std::time::Instant::now() < end_time {
        // 处理消息
        if let Err(e) = manager.process_messages().await {
            eprintln!("处理消息时出错: {}", e);
        }
        
        // 检查连接健康状态
        let unhealthy = manager.check_health().await;
        if !unhealthy.is_empty() {
            info!("发现不健康的交易所连接: {:?}", unhealthy);
            info!("尝试重连...");
            if let Err(e) = manager.reconnect_all().await {
                eprintln!("重连失败: {}", e);
            }
        }
        
        // 每10秒打印一次统计信息
        if std::time::Instant::now().elapsed().as_secs() % 10 == 0 {
            let stats = manager.get_stats().await;
            info!("统计信息: 总连接数: {}, 活跃连接数: {}, 总消息数: {}, 总错误数: {}", 
                  stats.total_connections, stats.active_connections, 
                  stats.total_messages, stats.total_errors);
            
            // 打印各交易所连接状态
            let states = manager.get_connection_states().await;
            for (exchange, state) in states {
                info!("{} 连接状态: {:?}", exchange.name(), state);
            }
        }
        
        // 短暂休眠
        sleep(Duration::from_millis(100)).await;
    }
    
    // 停止管理器
    info!("停止多交易所管理器...");
    manager.stop().await?;
    
    info!("示例程序结束");
    Ok(())
} 