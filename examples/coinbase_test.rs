use flow_sight::websocket::exchanges::coinbase::CoinbaseWebSocketManager;
use flow_sight::websocket::exchange_trait::{ExchangeWebSocketManager, ExchangeConfig};
use log::{info, error};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    info!("开始测试Coinbase WebSocket连接...");
    
    // 创建Coinbase WebSocket管理器配置
    let config = ExchangeConfig {
        symbol: "BTC-USD".to_string(),  // Coinbase使用BTC-USD格式
        testnet: false,
        reconnect_interval: Duration::from_secs(5),
        heartbeat_interval: Duration::from_secs(30),
        max_reconnect_attempts: 5,
    };
    
    let mut manager = CoinbaseWebSocketManager::new(config);
    
    // 连接到Coinbase WebSocket
    match manager.connect().await {
        Ok(_) => {
            info!("成功连接到Coinbase WebSocket");
            
            // 等待连接稳定
            sleep(Duration::from_secs(2)).await;
            
            // 订阅BTC-USD数据
            info!("开始订阅BTC-USD现货数据...");
            
            if let Err(e) = manager.subscribe_depth("BTC-USD").await {
                error!("订阅深度数据失败: {}", e);
            }
            
            if let Err(e) = manager.subscribe_trades("BTC-USD").await {
                error!("订阅成交数据失败: {}", e);
            }
            
            if let Err(e) = manager.subscribe_book_ticker("BTC-USD").await {
                error!("订阅ticker数据失败: {}", e);
            }
            
            info!("所有订阅完成，开始接收消息...");
            
            // 接收消息
            let mut message_count = 0;
            let max_messages = 25;
            
            while message_count < max_messages {
                match manager.read_messages().await {
                    Ok(messages) => {
                        for message in messages {
                            message_count += 1;
                            info!("收到消息 #{}: {}", message_count, serde_json::to_string_pretty(&message)?);
                            
                            // 分析消息类型
                            if let Some(msg_type) = message.get("type").and_then(|t| t.as_str()) {
                                match msg_type {
                                    "snapshot" => info!("  -> 深度快照"),
                                    "l2update" => info!("  -> 深度数据更新"),
                                    "match" => info!("  -> 成交数据"),
                                    "ticker" => info!("  -> 最优买卖价"),
                                    "subscriptions" => info!("  -> 订阅确认"),
                                    "error" => info!("  -> 错误消息"),
                                    _ => info!("  -> 其他消息类型: {}", msg_type),
                                }
                            }
                            
                            if message_count >= max_messages {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("读取消息失败: {}", e);
                        break;
                    }
                }
                
                // 定期发送心跳
                if message_count % 8 == 0 {
                    if let Err(e) = manager.send_heartbeat().await {
                        error!("发送心跳失败: {}", e);
                    }
                }
                
                sleep(Duration::from_millis(100)).await;
            }
            
            // 显示统计信息
            let stats = manager.get_stats();
            info!("=== Coinbase WebSocket统计信息 ===");
            info!("总消息数: {}", stats.total_messages_received);
            info!("总字节数: {}", stats.total_bytes_received);
            info!("解析错误: {}", stats.parse_errors);
            info!("连接错误: {}", stats.connection_errors);
            info!("订阅错误: {}", stats.subscription_errors);
            info!("重连次数: {}", stats.reconnect_attempts);
            info!("连接状态: {:?}", manager.get_connection_state());
            
            // 断开连接
            if let Err(e) = manager.disconnect().await {
                error!("断开连接失败: {}", e);
            } else {
                info!("已断开Coinbase WebSocket连接");
            }
        }
        Err(e) => {
            error!("连接Coinbase WebSocket失败: {}", e);
        }
    }
    
    info!("Coinbase WebSocket测试完成");
    Ok(())
} 