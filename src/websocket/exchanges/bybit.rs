use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use log::{info, warn, error, debug};

use crate::events::event_types::{Event, EventType, Exchange};
use crate::websocket::exchange_trait::{
    ExchangeWebSocketManager, ExchangeConfig, ExchangeConnectionState, ExchangeStats
};

/// Bybit WebSocket管理器
pub struct BybitWebSocketManager {
    config: ExchangeConfig,
    connection_state: ExchangeConnectionState,
    stats: ExchangeStats,
    last_ping_time: Instant,
    last_pong_time: Instant,
    message_buffer: Vec<Value>,
    subscriptions: Vec<String>,
    ws_sender: Option<futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>>,
}

/// Bybit WebSocket消息格式
#[derive(Debug, Deserialize)]
struct BybitWebSocketMessage {
    topic: Option<String>,
    #[serde(rename = "type")]
    msg_type: Option<String>,
    data: Option<Value>,
    ts: Option<u64>,
    success: Option<bool>,
    ret_msg: Option<String>,
    op: Option<String>,
    conn_id: Option<String>,
}

/// Bybit订阅响应
#[derive(Debug, Deserialize)]
struct BybitSubscriptionResponse {
    success: bool,
    ret_msg: String,
    op: String,
    conn_id: String,
}

/// Bybit Ping响应
#[derive(Debug, Deserialize)]
struct BybitPongResponse {
    success: bool,
    ret_msg: String,
    op: String,
    conn_id: String,
}

/// Bybit OrderBook数据
#[derive(Debug, Deserialize)]
struct BybitOrderBookData {
    s: String,  // symbol
    b: Vec<[String; 2]>,  // bids [[price, size]]
    a: Vec<[String; 2]>,  // asks [[price, size]]
    u: u64,     // update_id
    seq: u64,   // sequence
}

/// Bybit Trade数据
#[derive(Debug, Deserialize)]
struct BybitTradeData {
    #[serde(rename = "T")]
    timestamp: u64,
    s: String,  // symbol
    #[serde(rename = "S")]
    side: String,  // Buy/Sell
    v: String,  // volume
    p: String,  // price
    #[serde(rename = "L")]
    trade_time_ms: u64,
    i: String,  // trade_id
    #[serde(rename = "BT")]
    block_trade: bool,
}

impl BybitWebSocketManager {
    /// 创建新的Bybit WebSocket管理器
    pub fn new(config: ExchangeConfig) -> Self {
        let now = Instant::now();
        
        Self {
            config,
            connection_state: ExchangeConnectionState::Disconnected,
            stats: ExchangeStats {
                total_messages_received: 0,
                total_bytes_received: 0,
                parse_errors: 0,
                connection_errors: 0,
                subscription_errors: 0,
                reconnect_attempts: 0,
                last_message_time: None,
                connection_start_time: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64),
            },
            last_ping_time: now,
            last_pong_time: now,
            message_buffer: Vec::new(),
            subscriptions: Vec::new(),
            ws_sender: None,
        }
    }

    /// 获取WebSocket URL
    fn get_websocket_url(&self) -> String {
        if self.config.testnet {
            "wss://stream-testnet.bybit.com/v5/public/linear".to_string()
        } else {
            "wss://stream.bybit.com/v5/public/linear".to_string()
        }
    }

    /// 处理WebSocket消息
    async fn handle_message(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
        match message {
            Message::Text(text) => {
                self.stats.total_messages_received += 1;
                self.stats.total_bytes_received += text.len() as u64;
                self.stats.last_message_time = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64);
                
                debug!("收到Bybit WebSocket消息: {}", text);
                
                match serde_json::from_str::<BybitWebSocketMessage>(&text) {
                    Ok(msg) => {
                        self.process_websocket_message(msg).await?;
                    }
                    Err(e) => {
                        warn!("解析Bybit WebSocket消息失败: {} - 原始消息: {}", e, text);
                        self.stats.parse_errors += 1;
                    }
                }
            }
            Message::Ping(data) => {
                debug!("收到Bybit WebSocket Ping");
                if let Some(sender) = &mut self.ws_sender {
                    sender.send(Message::Pong(data)).await?;
                }
            }
            Message::Pong(_) => {
                debug!("收到Bybit WebSocket Pong");
                self.last_pong_time = Instant::now();
            }
            Message::Close(_) => {
                warn!("Bybit WebSocket连接关闭");
                self.connection_state = ExchangeConnectionState::Disconnected;
            }
            _ => {}
        }
        
        Ok(())
    }

    /// 处理WebSocket业务消息
    async fn process_websocket_message(&mut self, msg: BybitWebSocketMessage) -> Result<(), Box<dyn Error>> {
        // 处理订阅确认
        if let Some(op) = &msg.op {
            match op.as_str() {
                "subscribe" => {
                    if msg.success.unwrap_or(false) {
                        info!("Bybit订阅成功: {}", msg.ret_msg.unwrap_or_default());
                    } else {
                        warn!("Bybit订阅失败: {}", msg.ret_msg.unwrap_or_default());
                    }
                    return Ok(());
                }
                "pong" => {
                    debug!("收到Bybit pong响应");
                    self.last_pong_time = Instant::now();
                    return Ok(());
                }
                _ => {}
            }
        }

        // 处理数据消息
        if let Some(topic) = &msg.topic {
            if let Some(data) = &msg.data {
                if topic.starts_with("orderbook") {
                    self.process_orderbook_message(data).await?;
                } else if topic.starts_with("publicTrade") {
                    self.process_trade_message(data).await?;
                }
            }
        }

        Ok(())
    }

    /// 处理订单簿消息
    async fn process_orderbook_message(&mut self, data: &Value) -> Result<(), Box<dyn Error>> {
        if let Ok(orderbook_data) = serde_json::from_value::<BybitOrderBookData>(data.clone()) {
            let mut bids = HashMap::new();
            let mut asks = HashMap::new();

            // 处理买单 - 使用整数价格避免浮点数作为HashMap键
            for bid in orderbook_data.b {
                if let (Ok(price), Ok(size)) = (bid[0].parse::<f64>(), bid[1].parse::<f64>()) {
                    if size > 0.0 {
                        let price_key = (price * 100000.0) as i64; // 转换为整数键
                        bids.insert(price_key, size);
                    }
                }
            }

            // 处理卖单 - 使用整数价格避免浮点数作为HashMap键
            for ask in orderbook_data.a {
                if let (Ok(price), Ok(size)) = (ask[0].parse::<f64>(), ask[1].parse::<f64>()) {
                    if size > 0.0 {
                        let price_key = (price * 100000.0) as i64; // 转换为整数键
                        asks.insert(price_key, size);
                    }
                }
            }

            let event_data = json!({
                "exchange": "bybit",
                "symbol": orderbook_data.s,
                "bids": bids,
                "asks": asks,
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            });

            // 将事件添加到消息缓冲区
            self.message_buffer.push(event_data.clone());

            debug!("处理Bybit订单簿事件: {} bids, {} asks", bids.len(), asks.len());
            
            // 这里应该发送事件到事件总线，但现在先记录日志
            info!("Bybit OrderBook更新: {} - bids: {}, asks: {}", 
                  orderbook_data.s, bids.len(), asks.len());
        }

        Ok(())
    }

    /// 处理交易消息
    async fn process_trade_message(&mut self, data: &Value) -> Result<(), Box<dyn Error>> {
        if let Ok(trades) = serde_json::from_value::<Vec<BybitTradeData>>(data.clone()) {
            for trade_data in trades {
                if let (Ok(price), Ok(size)) = (trade_data.p.parse::<f64>(), trade_data.v.parse::<f64>()) {
                    let event_data = json!({
                        "exchange": "bybit",
                        "symbol": trade_data.s,
                        "price": price,
                        "size": size,
                        "side": if trade_data.side == "Buy" { "buy" } else { "sell" },
                        "timestamp": trade_data.trade_time_ms,
                        "trade_id": trade_data.i,
                    });

                    // 将事件添加到消息缓冲区
                    self.message_buffer.push(event_data.clone());

                    debug!("处理Bybit交易事件: {} {} @ {}", size, trade_data.s, price);
                    
                    // 这里应该发送事件到事件总线，但现在先记录日志
                    info!("Bybit Trade: {} {} {} @ {}", 
                          if trade_data.side == "Buy" { "buy" } else { "sell" }, 
                          size, trade_data.s, price);
                }
            }
        }

        Ok(())
    }

    /// 发送ping消息
    async fn send_ping(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(sender) = &mut self.ws_sender {
            let ping_msg = json!({
                "op": "ping"
            });
            
            sender.send(Message::Text(ping_msg.to_string())).await?;
            self.last_ping_time = Instant::now();
            
            debug!("发送Bybit ping消息");
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl ExchangeWebSocketManager for BybitWebSocketManager {
    fn exchange_name(&self) -> &str {
        "Bybit"
    }

    async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        let url = self.get_websocket_url();
        info!("连接到Bybit WebSocket: {}", url);
        
        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                let (sender, mut receiver) = ws_stream.split();
                self.ws_sender = Some(sender);
                self.connection_state = ExchangeConnectionState::Connected;
                
                info!("Bybit WebSocket连接成功");
                
                // 启动消息处理任务
                let mut manager = BybitWebSocketManager::new(self.config.clone());
                manager.connection_state = ExchangeConnectionState::Connected;
                
                tokio::spawn(async move {
                    while let Some(message) = receiver.next().await {
                        match message {
                            Ok(msg) => {
                                if let Err(e) = manager.handle_message(msg).await {
                                    error!("处理Bybit WebSocket消息失败: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("接收Bybit WebSocket消息失败: {}", e);
                                break;
                            }
                        }
                    }
                });
                
                Ok(())
            }
            Err(e) => {
                error!("连接Bybit WebSocket失败: {}", e);
                self.connection_state = ExchangeConnectionState::Disconnected;
                self.stats.connection_errors += 1;
                Err(e.into())
            }
        }
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(mut sender) = self.ws_sender.take() {
            sender.send(Message::Close(None)).await?;
            info!("Bybit WebSocket连接已断开");
        }
        
        self.connection_state = ExchangeConnectionState::Disconnected;
        Ok(())
    }

    async fn subscribe_depth(&mut self, symbol: &str) -> Result<(), Box<dyn Error>> {
        let topic = format!("orderbook.1.{}", symbol);
        
        if let Some(sender) = &mut self.ws_sender {
            let subscribe_msg = json!({
                "op": "subscribe",
                "args": [topic.clone()]
            });
            
            sender.send(Message::Text(subscribe_msg.to_string())).await?;
            self.subscriptions.push(topic.clone());
            
            info!("订阅Bybit深度数据: {}", topic);
        }
        
        Ok(())
    }

    async fn subscribe_trades(&mut self, symbol: &str) -> Result<(), Box<dyn Error>> {
        let topic = format!("publicTrade.{}", symbol);
        
        if let Some(sender) = &mut self.ws_sender {
            let subscribe_msg = json!({
                "op": "subscribe",
                "args": [topic.clone()]
            });
            
            sender.send(Message::Text(subscribe_msg.to_string())).await?;
            self.subscriptions.push(topic.clone());
            
            info!("订阅Bybit交易数据: {}", topic);
        }
        
        Ok(())
    }

    async fn subscribe_book_ticker(&mut self, symbol: &str) -> Result<(), Box<dyn Error>> {
        let topic = format!("bookTicker.{}", symbol);
        
        if let Some(sender) = &mut self.ws_sender {
            let subscribe_msg = json!({
                "op": "subscribe",
                "args": [topic.clone()]
            });
            
            sender.send(Message::Text(subscribe_msg.to_string())).await?;
            self.subscriptions.push(topic.clone());
            
            info!("订阅Bybit最优买卖价: {}", topic);
        }
        
        Ok(())
    }

    async fn read_messages(&mut self) -> Result<Vec<Value>, Box<dyn Error>> {
        let messages = self.message_buffer.clone();
        self.message_buffer.clear();
        Ok(messages)
    }

    async fn send_heartbeat(&mut self) -> Result<(), Box<dyn Error>> {
        self.send_ping().await
    }



    fn should_reconnect(&self) -> bool {
        self.connection_state == ExchangeConnectionState::Disconnected ||
        self.last_pong_time.elapsed() > Duration::from_secs(60)
    }

    async fn attempt_reconnect(&mut self) -> Result<(), Box<dyn Error>> {
        info!("尝试重连Bybit WebSocket");
        
        if let Err(e) = self.disconnect().await {
            warn!("断开连接时出错: {}", e);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        self.connect().await?;
        
        // 重新订阅之前的主题
        for topic in self.subscriptions.clone() {
            if topic.starts_with("orderbook") {
                let symbol = topic.split('.').nth(2).unwrap_or("BTCUSDT");
                if let Err(e) = self.subscribe_depth(symbol).await {
                    warn!("重新订阅深度数据失败: {}", e);
                }
            } else if topic.starts_with("publicTrade") {
                let symbol = topic.split('.').nth(1).unwrap_or("BTCUSDT");
                if let Err(e) = self.subscribe_trades(symbol).await {
                    warn!("重新订阅交易数据失败: {}", e);
                }
            } else if topic.starts_with("bookTicker") {
                let symbol = topic.split('.').nth(1).unwrap_or("BTCUSDT");
                if let Err(e) = self.subscribe_book_ticker(symbol).await {
                    warn!("重新订阅最优买卖价失败: {}", e);
                }
            }
        }
        
        self.stats.reconnect_attempts += 1;
        info!("Bybit WebSocket重连成功");
        
        Ok(())
    }

    fn get_connection_state(&self) -> ExchangeConnectionState {
        self.connection_state.clone()
    }

    fn get_stats(&self) -> ExchangeStats {
        self.stats.clone()
    }

    /// 判断是否为深度消息
    fn is_depth_message(&self, message: &Value) -> bool {
        if let Some(topic) = message.get("topic").and_then(|t| t.as_str()) {
            return topic.starts_with("orderbook.");
        }
        false
    }

    /// 判断是否为交易消息
    fn is_trade_message(&self, message: &Value) -> bool {
        if let Some(topic) = message.get("topic").and_then(|t| t.as_str()) {
            return topic.starts_with("publicTrade.");
        }
        false
    }
} 