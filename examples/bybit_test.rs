use flow_sight::websocket::exchanges::bybit::BybitWebSocketManager;
use flow_sight::websocket::exchange_trait::{ExchangeWebSocketManager, ExchangeConfig};
use log::{info, error};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    info!("开始测试Bybit WebSocket连接...");
    
    // 创建Bybit WebSocket管理器配置
    let config = ExchangeConfig {
        symbol: "BTCUSDT".to_string(),
        testnet: false,
        reconnect_interval: Duration::from_secs(5),
        heartbeat_interval: Duration::from_secs(20),
        max_reconnect_attempts: 5,
    };
    
    let mut manager = BybitWebSocketManager::new(config);
    
    // 连接到Bybit WebSocket
    match manager.connect().await {
        Ok(_) => {
            info!("成功连接到Bybit WebSocket");
            
            // 等待连接稳定
            sleep(Duration::from_secs(2)).await;
            
            // 订阅BTCUSDT数据
            info!("开始订阅BTCUSDT永续合约数据...");
            
            if let Err(e) = manager.subscribe_depth("BTCUSDT").await {
                error!("订阅深度数据失败: {}", e);
            }
            
            if let Err(e) = manager.subscribe_trades("BTCUSDT").await {
                error!("订阅成交数据失败: {}", e);
            }
            
            if let Err(e) = manager.subscribe_book_ticker("BTCUSDT").await {
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
                            if let Some(topic) = message.get("topic").and_then(|t| t.as_str()) {
                                if topic.starts_with("orderbook") {
                                    info!("  -> 深度数据更新");
                                } else if topic.starts_with("publicTrade") {
                                    info!("  -> 成交数据");
                                } else if topic.starts_with("tickers") {
                                    info!("  -> 最优买卖价");
                                } else {
                                    info!("  -> 其他消息类型: {}", topic);
                                }
                            } else if let Some(op) = message.get("op").and_then(|o| o.as_str()) {
                                match op {
                                    "subscribe" => info!("  -> 订阅确认"),
                                    "pong" => info!("  -> 心跳响应"),
                                    _ => info!("  -> 操作类型: {}", op),
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
            info!("=== Bybit WebSocket统计信息 ===");
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
                info!("已断开Bybit WebSocket连接");
            }
        }
        Err(e) => {
            error!("连接Bybit WebSocket失败: {}", e);
        }
    }
    
    info!("Bybit WebSocket测试完成");
    Ok(())
} 