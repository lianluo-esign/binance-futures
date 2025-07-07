use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use log::{info, error};
use env_logger;

use flow_sight::websocket::lock_free_threaded_multi_exchange_manager::{
    LockFreeThreadedMultiExchangeManager, LockFreeThreadedMultiExchangeManagerBuilder,
    LockFreeThreadedMultiExchangeConfig, ExchangeType, create_lock_free_threaded_manager
};
use flow_sight::websocket::ExchangeConfig;
use flow_sight::events::{LockFreeEventBus, Event, EventType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    info!("启动无锁线程化多交易所管理器测试...");

    // 方式1：使用便捷创建函数
    let (mut manager, event_bus) = create_lock_free_threaded_manager(100000);
    
    // 或者方式2：手动创建
    // let event_bus = Arc::new(LockFreeEventBus::new(100000));
    // let mut manager = LockFreeThreadedMultiExchangeManagerBuilder::new()
    //     .with_exchanges(vec![
    //         ExchangeType::Okx,
    //         ExchangeType::Bybit,
    //         ExchangeType::Bitget,
    //     ])
    //     .with_event_buffer_size(50000)
    //     .with_batch_size(200)
    //     .with_processing_interval_ms(1)
    //     .build(event_bus.clone());

    // 设置事件处理器
    setup_event_handlers(&event_bus).await;

    // 启动管理器
    info!("启动无锁多交易所管理器...");
    if let Err(e) = manager.start().await {
        error!("启动管理器失败: {}", e);
        return Err(e);
    }

    info!("管理器启动成功，活跃交易所数量: {}", manager.active_exchanges_count());

    // 运行一段时间来收集数据
    let mut stats_interval = tokio::time::interval(Duration::from_secs(10));
    let mut event_process_interval = tokio::time::interval(Duration::from_millis(100));
    
    for i in 0..30 {  // 运行5分钟
        tokio::select! {
            _ = stats_interval.tick() => {
                print_statistics(&manager, &event_bus).await;
            }
            _ = event_process_interval.tick() => {
                // 处理事件总线中的事件
                let processed = manager.process_pending_events(1000);
                if processed > 0 {
                    info!("处理了 {} 个事件", processed);
                }
            }
        }
    }

    // 停止管理器
    info!("停止管理器...");
    manager.stop().await?;

    info!("测试完成");
    Ok(())
}

/// 设置事件处理器
async fn setup_event_handlers(event_bus: &Arc<LockFreeEventBus>) {
    // 注意：LockFreeEventBus的订阅方法需要可变引用
    // 在实际应用中，你需要在创建事件总线后立即设置处理器
    // 这里我们使用Arc::get_mut来获取可变引用（仅在单一引用时有效）
    
    info!("设置事件处理器...");
    
    // 由于Arc<LockFreeEventBus>不能直接获取可变引用，
    // 我们需要在创建时就设置好处理器，或者使用其他方式
    // 这里我们展示如何处理这个问题
    
    // 在实际应用中，你应该在创建事件总线时就设置好所有处理器
    info!("事件处理器设置完成（注意：实际应用中需要在创建时设置）");
}

/// 打印统计信息
async fn print_statistics(
    manager: &LockFreeThreadedMultiExchangeManager,
    event_bus: &Arc<LockFreeEventBus>,
) {
    info!("========== 无锁多交易所管理器统计信息 ==========");
    
    // 管理器统计
    let all_stats = manager.get_all_stats().await;
    info!("活跃交易所数量: {}", manager.active_exchanges_count());
    info!("运行状态: {}", manager.is_running().await);
    
    // 各交易所统计
    for (exchange_type, stats) in all_stats {
        info!("[{}] 消息数: {}, 错误数: {}, 最后消息时间: {:?}", 
              exchange_type.name(), 
              stats.total_messages_received, 
              stats.connection_errors,
              stats.last_message_time);
    }
    
    // 事件总线统计
    let event_stats = manager.get_event_bus_stats();
    let (pending, capacity) = manager.get_event_bus_usage();
    
    info!("事件总线统计:");
    info!("  已发布事件: {}", event_stats.total_events_published);
    info!("  已处理事件: {}", event_stats.total_events_processed);
    info!("  处理器错误: {}", event_stats.handler_errors);
    info!("  待处理事件: {} / {}", pending, capacity);
    info!("  缓冲区使用率: {:.2}%", (pending as f64 / capacity as f64) * 100.0);
    
    info!("================================================");
}

/// 创建带有预配置事件处理器的无锁事件总线
fn create_configured_event_bus(capacity: usize) -> Arc<LockFreeEventBus> {
    let mut event_bus = LockFreeEventBus::new(capacity);
    
    // 设置深度更新处理器
    event_bus.subscribe("DepthUpdate", |event| {
        if let EventType::DepthUpdate(data) = &event.event_type {
            // 处理深度更新数据
            // println!("处理深度更新: {:?}", data);
        }
    });
    
    // 设置交易数据处理器
    event_bus.subscribe("Trade", |event| {
        if let EventType::Trade(data) = &event.event_type {
            // 处理交易数据
            // println!("处理交易数据: {:?}", data);
        }
    });
    
    // 设置全局事件处理器
    event_bus.subscribe_global(|event| {
        // 全局事件统计
        // println!("收到事件: {}", event.event_type.type_name());
    });
    
    Arc::new(event_bus)
}

/// 高级示例：自定义配置的无锁管理器
#[allow(dead_code)]
async fn advanced_example() -> Result<(), Box<dyn std::error::Error>> {
    // 创建配置好的事件总线
    let event_bus = create_configured_event_bus(200000);
    
    // 创建自定义配置
    let config = LockFreeThreadedMultiExchangeConfig {
        enabled_exchanges: vec![
            ExchangeType::Okx,
            ExchangeType::Bybit,
            ExchangeType::Bitget,
            ExchangeType::Bitfinex,
        ],
        exchange_configs: std::collections::HashMap::new(),
        event_buffer_size: 100000,
        batch_size: 500,
        processing_interval_ms: 1,
    };
    
    // 创建管理器
    let mut manager = LockFreeThreadedMultiExchangeManager::new(config, event_bus.clone());
    
    // 添加特定交易所配置
    manager.add_exchange_config(ExchangeType::Okx, ExchangeConfig {
        reconnect_interval: Duration::from_secs(5),
        max_reconnect_attempts: 10,
        ping_interval: Duration::from_secs(30),
        connection_timeout: Duration::from_secs(10),
    });
    
    // 启动管理器
    manager.start().await?;
    
    // 运行一段时间
    sleep(Duration::from_secs(60)).await;
    
    // 停止管理器
    manager.stop().await?;
    
    Ok(())
}