use crate::websocket::exchange_trait::{ExchangeWebSocketManager, ExchangeConnectionState, ExchangeStats};
use crate::events::event_types::{Event, EventType};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::error::Error;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

pub struct BitgetWebSocketManager {
    connection_state: ExchangeConnectionState,
    stats: ExchangeStats,
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    event_sender: Option<mpsc::UnboundedSender<Event>>,
}

impl BitgetWebSocketManager {
    pub fn new() -> Self {
        Self {
            connection_state: ExchangeConnectionState::Disconnected,
            stats: ExchangeStats::default(),
            ws_stream: None,
            event_sender: None,
        }
    }

    fn convert_symbol(&self, symbol: &str) -> String {
        // Bitget期货使用 BTCUSDT_UMCBL 格式
        if symbol == "BTCUSDT" {
            "BTCUSDT_UMCBL".to_string()
        } else {
            format!("{}_UMCBL", symbol)
        }
    }

    async fn subscribe_to_channels(
        ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        symbol: &str,
    ) -> Result<(), Box<dyn Error>> {
        let bitget_symbol = if symbol == "BTCUSDT" {
            "BTCUSDT_UMCBL".to_string()
        } else {
            format!("{}_UMCBL", symbol)
        };

        // 订阅深度数据
        let depth_sub = json!({
            "op": "subscribe",
            "args": [{
                "instType": "UMCBL",
                "channel": "books",
                "instId": bitget_symbol
            }]
        });

        // 订阅成交数据
        let trade_sub = json!({
            "op": "subscribe",
            "args": [{
                "instType": "UMCBL",
                "channel": "trade",
                "instId": bitget_symbol
            }]
        });

        // 订阅最优买卖价
        let ticker_sub = json!({
            "op": "subscribe",
            "args": [{
                "instType": "UMCBL",
                "channel": "ticker",
                "instId": bitget_symbol
            }]
        });

        // 发送订阅消息
        ws_stream.send(Message::Text(depth_sub.to_string())).await?;
        ws_stream.send(Message::Text(trade_sub.to_string())).await?;
        ws_stream.send(Message::Text(ticker_sub.to_string())).await?;

        println!("Bitget WebSocket subscriptions sent for symbol: {}", bitget_symbol);
        Ok(())
    }

    async fn handle_message(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
        match message {
            Message::Text(text) => {
                self.stats.total_messages_received += 1;
                self.stats.total_bytes_received += text.len() as u64;

                if let Ok(data) = serde_json::from_str::<Value>(&text) {
                    if let Some(event) = self.parse_message(&data) {
                        if let Some(sender) = &self.event_sender {
                            if let Err(e) = sender.send(event) {
                                println!("Failed to send event: {}", e);
                            }
                        }
                    }
                } else {
                    self.stats.parse_errors += 1;
                    println!("Failed to parse Bitget message: {}", text);
                }
            }
            Message::Ping(payload) => {
                if let Some(ws_stream) = &mut self.ws_stream {
                    ws_stream.send(Message::Pong(payload)).await?;
                }
            }
            Message::Pong(_) => {
                // 处理pong消息
            }
            Message::Close(_) => {
                self.connection_state = ExchangeConnectionState::Disconnected;
                println!("Bitget WebSocket connection closed");
            }
            _ => {}
        }
        Ok(())
    }

    fn parse_message(&self, data: &Value) -> Option<Event> {
        if let Some(arg) = data.get("arg") {
            if let Some(channel) = arg.get("channel").and_then(|c| c.as_str()) {
                match channel {
                    "books" => {
                        if let Some(event_data) = data.get("data") {
                            let event = Event::new_with_exchange(
                                EventType::DepthUpdate(event_data.clone()),
                                "bitget_websocket".to_string(),
                                "bitget".to_string(),
                            );
                            return Some(event);
                        }
                    }
                    "trade" => {
                        if let Some(event_data) = data.get("data") {
                            let event = Event::new_with_exchange(
                                EventType::Trade(event_data.clone()),
                                "bitget_websocket".to_string(),
                                "bitget".to_string(),
                            );
                            return Some(event);
                        }
                    }
                    "ticker" => {
                        if let Some(event_data) = data.get("data") {
                            let event = Event::new_with_exchange(
                                EventType::BookTicker(event_data.clone()),
                                "bitget_websocket".to_string(),
                                "bitget".to_string(),
                            );
                            return Some(event);
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    pub async fn start_with_event_sender(
        &mut self,
        event_sender: mpsc::UnboundedSender<Event>,
        symbol: &str,
    ) -> Result<(), Box<dyn Error>> {
        self.event_sender = Some(event_sender);
        self.connect().await?;
        self.subscribe_btcusdt_perpetual().await?;
        
        // 启动消息处理循环
        while let Some(ws_stream) = &mut self.ws_stream {
            if let Some(message) = ws_stream.next().await {
                match message {
                    Ok(msg) => {
                        if let Err(e) = self.handle_message(msg).await {
                            println!("Error handling Bitget message: {}", e);
                            self.stats.parse_errors += 1;
                        }
                    }
                    Err(e) => {
                        println!("Bitget WebSocket error: {}", e);
                        self.stats.connection_errors += 1;
                        self.connection_state = ExchangeConnectionState::Failed(e.to_string());
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ExchangeWebSocketManager for BitgetWebSocketManager {
    fn exchange_name(&self) -> &str {
        "bitget"
    }

    async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        self.connection_state = ExchangeConnectionState::Connecting;
        
        let url = "wss://ws.bitget.com/v2/ws/public";
        let (ws_stream, _) = connect_async(url).await?;
        
        self.ws_stream = Some(ws_stream);
        self.connection_state = ExchangeConnectionState::Connected;
        self.stats.connection_start_time = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );
        
        println!("Bitget WebSocket connected successfully");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(mut ws_stream) = self.ws_stream.take() {
            ws_stream.close(None).await?;
        }
        self.connection_state = ExchangeConnectionState::Disconnected;
        println!("Bitget WebSocket disconnected");
        Ok(())
    }

    async fn subscribe_depth(&mut self, symbol: &str) -> Result<(), Box<dyn Error>> {
        let bitget_symbol = self.convert_symbol(symbol);
        if let Some(ws_stream) = &mut self.ws_stream {
            let sub_msg = json!({
                "op": "subscribe",
                "args": [{
                    "instType": "UMCBL",
                    "channel": "books",
                    "instId": bitget_symbol
                }]
            });
            
            ws_stream.send(Message::Text(sub_msg.to_string())).await?;
            println!("Subscribed to Bitget depth for symbol: {}", bitget_symbol);
        }
        Ok(())
    }

    async fn subscribe_trades(&mut self, symbol: &str) -> Result<(), Box<dyn Error>> {
        let bitget_symbol = self.convert_symbol(symbol);
        if let Some(ws_stream) = &mut self.ws_stream {
            let sub_msg = json!({
                "op": "subscribe",
                "args": [{
                    "instType": "UMCBL",
                    "channel": "trade",
                    "instId": bitget_symbol
                }]
            });
            
            ws_stream.send(Message::Text(sub_msg.to_string())).await?;
            println!("Subscribed to Bitget trades for symbol: {}", bitget_symbol);
        }
        Ok(())
    }

    async fn subscribe_book_ticker(&mut self, symbol: &str) -> Result<(), Box<dyn Error>> {
        let bitget_symbol = self.convert_symbol(symbol);
        if let Some(ws_stream) = &mut self.ws_stream {
            let sub_msg = json!({
                "op": "subscribe",
                "args": [{
                    "instType": "UMCBL",
                    "channel": "ticker",
                    "instId": bitget_symbol
                }]
            });
            
            ws_stream.send(Message::Text(sub_msg.to_string())).await?;
            println!("Subscribed to Bitget ticker for symbol: {}", bitget_symbol);
        }
        Ok(())
    }

    async fn read_messages(&mut self) -> Result<Vec<Value>, Box<dyn Error>> {
        let mut messages = Vec::new();
        
        if let Some(ws_stream) = &mut self.ws_stream {
            if let Some(message) = ws_stream.next().await {
                match message? {
                    Message::Text(text) => {
                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            messages.push(data);
                        }
                    }
                    _ => {}
                }
            }
        }
        
        Ok(messages)
    }

    async fn send_heartbeat(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(ws_stream) = &mut self.ws_stream {
            let ping_msg = json!({
                "op": "ping"
            });
            
            ws_stream.send(Message::Text(ping_msg.to_string())).await?;
        }
        Ok(())
    }

    fn get_connection_state(&self) -> ExchangeConnectionState {
        self.connection_state.clone()
    }

    fn should_reconnect(&self) -> bool {
        matches!(
            self.connection_state,
            ExchangeConnectionState::Disconnected | ExchangeConnectionState::Failed(_)
        )
    }

    async fn attempt_reconnect(&mut self) -> Result<(), Box<dyn Error>> {
        self.stats.reconnect_attempts += 1;
        self.connection_state = ExchangeConnectionState::Reconnecting;
        
        // 关闭现有连接
        if let Some(mut ws_stream) = self.ws_stream.take() {
            let _ = ws_stream.close(None).await;
        }
        
        // 重新连接
        self.connect().await?;
        
        // 重新订阅
        self.subscribe_btcusdt_perpetual().await?;
        
        println!("Bitget WebSocket reconnected successfully");
        Ok(())
    }

    fn get_stats(&self) -> ExchangeStats {
        let mut stats = self.stats.clone();
        stats.last_message_time = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );
        stats
    }
} 