use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use crate::websocket::exchange_trait::{ExchangeWebSocketManager, ExchangeConnectionState, ExchangeStats};
use log::{info, warn, error, debug};
use async_trait::async_trait;

pub struct MexcWebSocketManager {
    connection_state: ExchangeConnectionState,
    stats: ExchangeStats,
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    event_sender: Option<mpsc::UnboundedSender<Value>>,
}

impl MexcWebSocketManager {
    pub fn new() -> Self {
        Self {
            connection_state: ExchangeConnectionState::Disconnected,
            stats: ExchangeStats::default(),
            ws_stream: None,
            event_sender: None,
        }
    }

    /// 转换符号格式：BTCUSDT -> BTC_USDT
    fn convert_symbol(&self, symbol: &str) -> String {
        if symbol == "BTCUSDT" {
            "BTC_USDT".to_string()
        } else {
            symbol.to_string()
        }
    }
}

#[async_trait]
impl ExchangeWebSocketManager for MexcWebSocketManager {
    fn exchange_name(&self) -> &str {
        "mexc"
    }

    async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Connecting to MEXC WebSocket...");
        self.connection_state = ExchangeConnectionState::Connecting;

        // 根据文档，WebSocket地址为 wss://contract.mexc.com/edge
        let url = "wss://contract.mexc.com/edge";
        let (ws_stream, _) = connect_async(url).await?;
        
        self.ws_stream = Some(ws_stream);
        self.connection_state = ExchangeConnectionState::Connected;
        self.stats.connection_start_time = Some(chrono::Utc::now().timestamp_millis() as u64);
        
        info!("Successfully connected to MEXC WebSocket");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Disconnecting from MEXC WebSocket...");
        
        if let Some(mut ws_stream) = self.ws_stream.take() {
            let _ = ws_stream.close(None).await;
        }
        
        self.connection_state = ExchangeConnectionState::Disconnected;
        info!("Disconnected from MEXC WebSocket");
        Ok(())
    }

    async fn subscribe_depth(&mut self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.ws_stream.is_none() {
            return Err("WebSocket not connected".into());
        }

        let mexc_symbol = self.convert_symbol(symbol);
        
        // MEXC使用sub.depth订阅深度数据（增量模式，启用压缩）
        let subscribe_msg = json!({
            "method": "sub.depth",
            "param": {
                "symbol": mexc_symbol,
                "compress": true
            }
        });

        if let Some(ws_stream) = &mut self.ws_stream {
            let msg = Message::Text(subscribe_msg.to_string());
            ws_stream.send(msg).await?;
            info!("Subscribed to MEXC depth data for {}", mexc_symbol);
        }

        Ok(())
    }

    async fn subscribe_trades(&mut self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.ws_stream.is_none() {
            return Err("WebSocket not connected".into());
        }

        let mexc_symbol = self.convert_symbol(symbol);
        
        // MEXC使用sub.deal订阅成交数据
        let subscribe_msg = json!({
            "method": "sub.deal",
            "param": {
                "symbol": mexc_symbol
            }
        });

        if let Some(ws_stream) = &mut self.ws_stream {
            let msg = Message::Text(subscribe_msg.to_string());
            ws_stream.send(msg).await?;
            info!("Subscribed to MEXC trades data for {}", mexc_symbol);
        }

        Ok(())
    }

    async fn subscribe_book_ticker(&mut self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        // MEXC合约API没有专门的book_ticker频道，我们可以通过深度数据获取最优买卖价
        // 这里简单返回成功，实际最优买卖价可以从深度数据中提取
        info!("MEXC does not have dedicated book_ticker channel, using depth data for best bid/ask");
        Ok(())
    }

    async fn read_messages(&mut self) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let mut messages = Vec::new();
        
        if let Some(ws_stream) = &mut self.ws_stream {
            if let Some(msg) = ws_stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        debug!("[MEXC] 收到消息: {}", text);
                        
                        if let Ok(value) = serde_json::from_str::<Value>(&text) {
                            // 处理订阅确认消息
                            if let Some(channel) = value.get("channel").and_then(|c| c.as_str()) {
                                if channel.starts_with("rs.") {
                                    debug!("[MEXC] 订阅确认: {}", text);
                                    return Ok(messages);
                                }
                            }
                            
                            messages.push(value);
                            self.stats.total_messages_received += 1;
                            self.stats.total_bytes_received += text.len() as u64;
                            self.stats.last_message_time = Some(chrono::Utc::now().timestamp_millis() as u64);
                        } else {
                            self.stats.parse_errors += 1;
                            warn!("[MEXC] 解析消息失败: {}", text);
                        }
                    }
                    Ok(Message::Close(_)) => {
                        warn!("MEXC WebSocket connection closed");
                        self.connection_state = ExchangeConnectionState::Disconnected;
                    }
                    Err(e) => {
                        error!("MEXC WebSocket error: {}", e);
                        self.stats.connection_errors += 1;
                        self.connection_state = ExchangeConnectionState::Failed(e.to_string());
                        return Err(e.into());
                    }
                    _ => {}
                }
            }
        }
        
        Ok(messages)
    }

    async fn send_heartbeat(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // MEXC WebSocket通常不需要主动发送心跳，服务器会自动维护连接
        // 如果需要，可以发送ping消息
        if let Some(ws_stream) = &mut self.ws_stream {
            let ping_msg = Message::Ping(vec![]);
            ws_stream.send(ping_msg).await?;
            debug!("Sent ping to MEXC");
        }
        Ok(())
    }

    fn get_connection_state(&self) -> ExchangeConnectionState {
        self.connection_state.clone()
    }

    fn get_stats(&self) -> ExchangeStats {
        self.stats.clone()
    }

    fn should_reconnect(&self) -> bool {
        matches!(self.connection_state, ExchangeConnectionState::Disconnected | ExchangeConnectionState::Failed(_))
    }

    async fn attempt_reconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Reconnecting to MEXC WebSocket...");
        self.connection_state = ExchangeConnectionState::Reconnecting;
        self.stats.reconnect_attempts += 1;
        
        // 先断开现有连接
        if let Some(mut ws_stream) = self.ws_stream.take() {
            let _ = ws_stream.close(None).await;
        }
        
        // 重新连接
        self.connect().await?;
        
        info!("Successfully reconnected to MEXC WebSocket");
        Ok(())
    }
} 