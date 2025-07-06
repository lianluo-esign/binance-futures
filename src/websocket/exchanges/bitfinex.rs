use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use crate::websocket::exchange_trait::ExchangeWebSocketManager;
use crate::events::event_types::Event;
use crate::websocket::{ExchangeConnectionState, ExchangeStats};
use log::{info, warn, error, debug};

pub struct BitfinexWebSocketManager {
    connection_state: ExchangeConnectionState,
    stats: ExchangeStats,
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    event_sender: Option<mpsc::UnboundedSender<Event>>,
}

impl BitfinexWebSocketManager {
    pub fn new() -> Self {
        Self {
            connection_state: ExchangeConnectionState::Disconnected,
            stats: ExchangeStats::default(),
            ws_stream: None,
            event_sender: None,
        }
    }

    /// 转换BTCUSDT到Bitfinex永续合约格式
    fn convert_symbol_to_bitfinex_format(&self, symbol: &str) -> String {
        // 对于永续合约，Bitfinex使用tBTCF0:USTF0格式
        // 对于现货交易，使用tBTCUSD格式
        // 这里我们使用永续合约格式
        match symbol {
            "BTCUSDT" => "tBTCF0:USTF0".to_string(),
            _ => format!("t{}", symbol.replace("USDT", "USD")),
        }
    }
}

#[async_trait::async_trait]
impl ExchangeWebSocketManager for BitfinexWebSocketManager {
    fn exchange_name(&self) -> &str {
        "bitfinex"
    }

    async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Connecting to Bitfinex WebSocket...");
        self.connection_state = ExchangeConnectionState::Connecting;

        let url = "wss://api-pub.bitfinex.com/ws/2";
        let (ws_stream, _) = connect_async(url).await?;
        
        self.ws_stream = Some(ws_stream);
        self.connection_state = ExchangeConnectionState::Connected;
        self.stats.connection_start_time = Some(chrono::Utc::now().timestamp_millis() as u64);
        
        info!("Successfully connected to Bitfinex WebSocket");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Disconnecting from Bitfinex WebSocket...");
        
        if let Some(mut ws_stream) = self.ws_stream.take() {
            let _ = ws_stream.close(None).await;
        }
        
        self.connection_state = ExchangeConnectionState::Disconnected;
        info!("Disconnected from Bitfinex WebSocket");
        Ok(())
    }

    async fn subscribe_depth(&mut self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.ws_stream.is_none() {
            return Err("WebSocket not connected".into());
        }

        let bitfinex_symbol = self.convert_symbol_to_bitfinex_format(symbol);
        let subscribe_msg = json!({
            "event": "subscribe",
            "channel": "book",
            "symbol": bitfinex_symbol,
            "prec": "P0",
            "freq": "F0",
            "len": "25"
        });

        if let Some(ws_stream) = &mut self.ws_stream {
            let msg = Message::Text(subscribe_msg.to_string());
            ws_stream.send(msg).await?;
            info!("Subscribed to Bitfinex depth data for {}", bitfinex_symbol);
        }

        Ok(())
    }

    async fn subscribe_trades(&mut self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.ws_stream.is_none() {
            return Err("WebSocket not connected".into());
        }

        let bitfinex_symbol = self.convert_symbol_to_bitfinex_format(symbol);
        let subscribe_msg = json!({
            "event": "subscribe",
            "channel": "trades",
            "symbol": bitfinex_symbol
        });

        if let Some(ws_stream) = &mut self.ws_stream {
            let msg = Message::Text(subscribe_msg.to_string());
            ws_stream.send(msg).await?;
            info!("Subscribed to Bitfinex trades data for {}", bitfinex_symbol);
        }

        Ok(())
    }

    async fn subscribe_book_ticker(&mut self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.ws_stream.is_none() {
            return Err("WebSocket not connected".into());
        }

        let bitfinex_symbol = self.convert_symbol_to_bitfinex_format(symbol);
        let subscribe_msg = json!({
            "event": "subscribe",
            "channel": "ticker",
            "symbol": bitfinex_symbol
        });

        if let Some(ws_stream) = &mut self.ws_stream {
            let msg = Message::Text(subscribe_msg.to_string());
            ws_stream.send(msg).await?;
            info!("Subscribed to Bitfinex ticker data for {}", bitfinex_symbol);
        }

        Ok(())
    }

    async fn read_messages(&mut self) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let mut messages = Vec::new();
        
        if let Some(ws_stream) = &mut self.ws_stream {
            if let Some(msg) = ws_stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(value) = serde_json::from_str::<Value>(&text) {
                            messages.push(value);
                            self.stats.total_messages_received += 1;
                            self.stats.total_bytes_received += text.len() as u64;
                            self.stats.last_message_time = Some(chrono::Utc::now().timestamp_millis() as u64);
                        } else {
                            self.stats.parse_errors += 1;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        warn!("Bitfinex WebSocket connection closed");
                        self.connection_state = ExchangeConnectionState::Disconnected;
                    }
                    Err(e) => {
                        error!("Bitfinex WebSocket error: {}", e);
                        self.stats.connection_errors += 1;
                        return Err(e.into());
                    }
                    _ => {}
                }
            }
        }
        
        Ok(messages)
    }

    async fn send_heartbeat(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ws_stream) = &mut self.ws_stream {
            let ping_msg = json!({
                "event": "ping"
            });
            let msg = Message::Text(ping_msg.to_string());
            ws_stream.send(msg).await?;
            debug!("Sent ping to Bitfinex");
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
        matches!(self.connection_state, ExchangeConnectionState::Disconnected)
    }

    async fn attempt_reconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Reconnecting to Bitfinex WebSocket...");
        self.connection_state = ExchangeConnectionState::Reconnecting;
        self.stats.reconnect_attempts += 1;
        
        // 断开现有连接
        let _ = self.disconnect().await;
        
        // 等待一段时间后重连
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        // 重新连接
        self.connect().await?;
        
        info!("Successfully reconnected to Bitfinex WebSocket");
        Ok(())
    }

    async fn reconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.attempt_reconnect().await
    }
} 