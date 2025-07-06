use flow_sight::websocket::exchanges::gateio::GateioWebSocketManager;
use flow_sight::websocket::exchange_trait::ExchangeWebSocketManager;
use log::{info, error};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    info!("开始测试Gate.io WebSocket连接（改进版）...");
    
    // 创建Gate.io WebSocket管理器
    let mut manager = GateioWebSocketManager::new();
    
    // 连接到Gate.io WebSocket
    match manager.connect().await {
        Ok(_) => {
            info!("成功连接到Gate.io WebSocket（BTC结算）");
            
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
                            if let Some(channel) = message.get("channel").and_then(|c| c.as_str()) {
                                match channel {
                                    "futures.order_book" => info!("  -> 深度数据更新"),
                                    "futures.trades" => info!("  -> 成交数据"),
                                    "futures.book_ticker" => info!("  -> 最优买卖价"),
                                    _ => info!("  -> 其他消息类型: {}", channel),
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
            info!("=== Gate.io WebSocket统计信息 ===");
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
                info!("已断开Gate.io WebSocket连接");
            }
        }
        Err(e) => {
            error!("连接Gate.io WebSocket失败: {}", e);
        }
    }
    
    info!("Gate.io WebSocket测试完成");
    Ok(())
} 