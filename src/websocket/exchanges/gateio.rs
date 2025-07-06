use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use crate::websocket::exchange_trait::{ExchangeWebSocketManager, ExchangeConnectionState, ExchangeStats};
use log::{info, warn, error, debug};
use async_trait::async_trait;

/// Gate.io WebSocket管理器
/// 使用BTC结算的永续合约WebSocket服务
pub struct GateioWebSocketManager {
    connection_state: ExchangeConnectionState,
    stats: ExchangeStats,
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    event_sender: Option<mpsc::UnboundedSender<Value>>,
}

impl GateioWebSocketManager {
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

    /// 发送请求到Gate.io WebSocket
    async fn send_request(&mut self, channel: &str, event: &str, payload: Option<Value>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ws_stream) = &mut self.ws_stream {
            let mut request = json!({
                "time": chrono::Utc::now().timestamp(),
                "channel": channel,
                "event": event
            });

            if let Some(payload) = payload {
                request["payload"] = payload;
            }

            let msg = Message::Text(request.to_string());
            ws_stream.send(msg).await?;
            debug!("[Gate.io] 发送请求: {}", request);
        }
        Ok(())
    }
}

#[async_trait]
impl ExchangeWebSocketManager for GateioWebSocketManager {
    fn exchange_name(&self) -> &str {
        "gateio"
    }

    async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Connecting to Gate.io WebSocket...");
        self.connection_state = ExchangeConnectionState::Connecting;

        // 使用BTC结算的WebSocket服务地址
        let url = "wss://fx-ws.gateio.ws/v4/ws/usdt";
        let (ws_stream, _) = connect_async(url).await?;
        
        self.ws_stream = Some(ws_stream);
        self.connection_state = ExchangeConnectionState::Connected;
        self.stats.connection_start_time = Some(chrono::Utc::now().timestamp_millis() as u64);
        
        info!("Successfully connected to Gate.io WebSocket (BTC settlement)");
        
        // 发送ping消息
        self.send_heartbeat().await?;
        
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Disconnecting from Gate.io WebSocket...");
        
        if let Some(mut ws_stream) = self.ws_stream.take() {
            let _ = ws_stream.close(None).await;
        }
        
        self.connection_state = ExchangeConnectionState::Disconnected;
        info!("Disconnected from Gate.io WebSocket");
        Ok(())
    }

    async fn subscribe_depth(&mut self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.ws_stream.is_none() {
            return Err("WebSocket not connected".into());
        }

        let gate_symbol = self.convert_symbol(symbol);
        
        // 订阅合约深度更新推送
        // 使用futures.order_book频道
        let payload = json!([gate_symbol, "20", "0"]);
        self.send_request("futures.order_book", "subscribe", Some(payload)).await?;
        
        info!("Subscribed to Gate.io futures depth data for {}", gate_symbol);
        Ok(())
    }

    async fn subscribe_trades(&mut self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.ws_stream.is_none() {
            return Err("WebSocket not connected".into());
        }

        let gate_symbol = self.convert_symbol(symbol);
        
        // 订阅公有成交数据
        // 使用futures.trades频道
        let payload = json!([gate_symbol]);
        self.send_request("futures.trades", "subscribe", Some(payload)).await?;
        
        info!("Subscribed to Gate.io futures trades data for {}", gate_symbol);
        Ok(())
    }

    async fn subscribe_book_ticker(&mut self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.ws_stream.is_none() {
            return Err("WebSocket not connected".into());
        }

        let gate_symbol = self.convert_symbol(symbol);
        
        // 订阅最佳买卖价
        // 使用futures.book_ticker频道
        let payload = json!([gate_symbol]);
        self.send_request("futures.book_ticker", "subscribe", Some(payload)).await?;
        
        info!("Subscribed to Gate.io futures book ticker data for {}", gate_symbol);
        Ok(())
    }

    async fn read_messages(&mut self) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let mut messages = Vec::new();
        
        if let Some(ws_stream) = &mut self.ws_stream {
            if let Some(msg) = ws_stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        debug!("[Gate.io] 收到消息: {}", text);
                        
                        if let Ok(value) = serde_json::from_str::<Value>(&text) {
                            // 处理心跳响应
                            if let Some(channel) = value.get("channel").and_then(|c| c.as_str()) {
                                if channel == "futures.pong" {
                                    debug!("[Gate.io] 收到心跳响应");
                                    return Ok(messages);
                                }
                            }
                            
                            // 处理订阅确认消息
                            if let Some(event) = value.get("event").and_then(|e| e.as_str()) {
                                if event == "subscribe" {
                                    if let Some(error) = value.get("error") {
                                        if !error.is_null() {
                                            warn!("[Gate.io] 订阅失败: {}", error);
                                            self.stats.subscription_errors += 1;
                                        } else {
                                            debug!("[Gate.io] 订阅成功: {}", text);
                                        }
                                    }
                                    return Ok(messages);
                                }
                            }
                            
                            messages.push(value);
                            self.stats.total_messages_received += 1;
                            self.stats.total_bytes_received += text.len() as u64;
                            self.stats.last_message_time = Some(chrono::Utc::now().timestamp_millis() as u64);
                        } else {
                            self.stats.parse_errors += 1;
                            warn!("[Gate.io] 解析消息失败: {}", text);
                        }
                    }
                    Ok(Message::Close(_)) => {
                        warn!("Gate.io WebSocket connection closed");
                        self.connection_state = ExchangeConnectionState::Disconnected;
                    }
                    Err(e) => {
                        error!("Gate.io WebSocket error: {}", e);
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
        // 发送futures.ping心跳消息
        self.send_request("futures.ping", "", None).await?;
        debug!("Sent ping to Gate.io");
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
        info!("Reconnecting to Gate.io WebSocket...");
        self.connection_state = ExchangeConnectionState::Reconnecting;
        self.stats.reconnect_attempts += 1;
        
        // 先断开现有连接
        if let Some(mut ws_stream) = self.ws_stream.take() {
            let _ = ws_stream.close(None).await;
        }
        
        // 重新连接
        self.connect().await?;
        
        info!("Successfully reconnected to Gate.io WebSocket");
        Ok(())
    }
} 