use std::collections::HashMap;
use std::error::Error;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use log::{info, warn, error, debug};

use crate::websocket::exchange_trait::{
    ExchangeWebSocketManager, ExchangeConfig, ExchangeConnectionState, ExchangeStats
};

/// Coinbase WebSocket管理器
pub struct CoinbaseWebSocketManager {
    config: ExchangeConfig,
    connection_state: ExchangeConnectionState,
    stats: ExchangeStats,
    last_ping_time: Instant,
    last_pong_time: Instant,
    message_buffer: Vec<Value>,
    subscriptions: Vec<String>,
    ws_sender: Option<futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>>,
}

/// Coinbase WebSocket消息结构
#[derive(Debug, Deserialize)]
struct CoinbaseWebSocketMessage {
    #[serde(rename = "type")]
    msg_type: String,
    product_id: Option<String>,
    sequence: Option<u64>,
    time: Option<String>,
    changes: Option<Vec<Vec<String>>>,
    bids: Option<Vec<Vec<String>>>,
    asks: Option<Vec<Vec<String>>>,
    price: Option<String>,
    size: Option<String>,
    side: Option<String>,
    trade_id: Option<u64>,
    maker_order_id: Option<String>,
    taker_order_id: Option<String>,
    best_bid: Option<String>,
    best_ask: Option<String>,
    best_bid_size: Option<String>,
    best_ask_size: Option<String>,
}

/// Coinbase订阅请求结构
#[derive(Debug, Serialize)]
struct CoinbaseSubscriptionRequest {
    #[serde(rename = "type")]
    msg_type: String,
    channels: Vec<CoinbaseChannel>,
}

/// Coinbase频道结构
#[derive(Debug, Serialize)]
struct CoinbaseChannel {
    name: String,
    product_ids: Vec<String>,
}

impl CoinbaseWebSocketManager {
    /// 创建新的Coinbase WebSocket管理器
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
                connection_start_time: None,
            },
            last_ping_time: now,
            last_pong_time: now,
            subscriptions: Vec::new(),
            message_buffer: Vec::new(),
            ws_sender: None,
        }
    }

    /// 获取WebSocket连接URL
    fn get_websocket_url(&self) -> String {
        if self.config.testnet {
            "wss://ws-feed-public.sandbox.exchange.coinbase.com".to_string()
        } else {
            "wss://ws-feed.exchange.coinbase.com".to_string()
        }
    }

    /// 处理WebSocket消息
    async fn handle_message(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
        match message {
            Message::Text(text) => {
                self.stats.total_messages_received += 1;
                self.stats.total_bytes_received += text.len() as u64;
                self.stats.last_message_time = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64);
                
                match serde_json::from_str::<CoinbaseWebSocketMessage>(&text) {
                    Ok(msg) => {
                        self.process_websocket_message(msg).await?;
                    }
                    Err(e) => {
                        warn!("解析Coinbase WebSocket消息失败: {} - 原始消息: {}", e, text);
                        self.stats.parse_errors += 1;
                    }
                }
            }
            Message::Pong(_) => {
                self.last_pong_time = Instant::now();
                debug!("收到Coinbase pong响应");
            }
            Message::Close(_) => {
                warn!("Coinbase WebSocket连接被关闭");
                self.connection_state = ExchangeConnectionState::Disconnected;
            }
            _ => {
                debug!("收到其他类型的Coinbase WebSocket消息: {:?}", message);
            }
        }
        
        Ok(())
    }

    /// 处理WebSocket业务消息
    async fn process_websocket_message(&mut self, msg: CoinbaseWebSocketMessage) -> Result<(), Box<dyn Error>> {
        match msg.msg_type.as_str() {
            "subscriptions" => {
                info!("Coinbase订阅确认: {:?}", msg);
            }
            "snapshot" => {
                debug!("收到Coinbase快照消息");
                if let Some(product_id) = &msg.product_id {
                    self.process_snapshot_message(&msg, product_id).await?;
                }
            }
            "l2update" => {
                debug!("收到Coinbase L2更新消息");
                if let Some(product_id) = &msg.product_id {
                    self.process_l2update_message(&msg, product_id).await?;
                }
            }
            "ticker" => {
                debug!("收到Coinbase ticker消息");
                if let Some(product_id) = &msg.product_id {
                    self.process_ticker_message(&msg, product_id).await?;
                }
            }
            "match" => {
                debug!("收到Coinbase match消息");
                if let Some(product_id) = &msg.product_id {
                    self.process_match_message(&msg, product_id).await?;
                }
            }
            "error" => {
                error!("收到Coinbase错误消息: {:?}", msg);
            }
            _ => {
                debug!("收到未知类型的Coinbase消息: {}", msg.msg_type);
            }
        }
        
        Ok(())
    }

    /// 处理快照消息
    async fn process_snapshot_message(&mut self, msg: &CoinbaseWebSocketMessage, product_id: &str) -> Result<(), Box<dyn Error>> {
        if let (Some(bids_data), Some(asks_data)) = (&msg.bids, &msg.asks) {
            let mut bids = HashMap::new();
            let mut asks = HashMap::new();

            // 处理买单 - 使用整数价格避免浮点数作为HashMap键
            for bid in bids_data {
                if bid.len() >= 2 {
                    if let (Ok(price), Ok(size)) = (bid[0].parse::<f64>(), bid[1].parse::<f64>()) {
                        if size > 0.0 {
                            let price_key = (price * 100.0) as i64; // 转换为分
                            bids.insert(price_key, size);
                        }
                    }
                }
            }

            // 处理卖单
            for ask in asks_data {
                if ask.len() >= 2 {
                    if let (Ok(price), Ok(size)) = (ask[0].parse::<f64>(), ask[1].parse::<f64>()) {
                        if size > 0.0 {
                            let price_key = (price * 100.0) as i64; // 转换为分
                            asks.insert(price_key, size);
                        }
                    }
                }
            }

            let event_data = json!({
                "exchange": "coinbase",
                "symbol": product_id,
                "bids": bids,
                "asks": asks,
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            });

            // 将事件添加到消息缓冲区
            self.message_buffer.push(event_data.clone());

            debug!("处理Coinbase订单簿快照: {} bids, {} asks", bids.len(), asks.len());
            
            info!("Coinbase OrderBook快照: {} - bids: {}, asks: {}", 
                  product_id, bids.len(), asks.len());
        }

        Ok(())
    }

    /// 处理L2更新消息
    async fn process_l2update_message(&mut self, msg: &CoinbaseWebSocketMessage, product_id: &str) -> Result<(), Box<dyn Error>> {
        if let Some(changes) = &msg.changes {
            let mut bids = HashMap::new();
            let mut asks = HashMap::new();

            for change in changes {
                if change.len() >= 3 {
                    let side = &change[0];
                    if let (Ok(price), Ok(size)) = (change[1].parse::<f64>(), change[2].parse::<f64>()) {
                        let price_key = (price * 100.0) as i64; // 转换为分
                        
                        if side == "buy" {
                            if size > 0.0 {
                                bids.insert(price_key, size);
                            } else {
                                // size为0表示删除该价格层级
                                bids.insert(price_key, 0.0);
                            }
                        } else if side == "sell" {
                            if size > 0.0 {
                                asks.insert(price_key, size);
                            } else {
                                // size为0表示删除该价格层级
                                asks.insert(price_key, 0.0);
                            }
                        }
                    }
                }
            }

            let event_data = json!({
                "exchange": "coinbase",
                "symbol": product_id,
                "bids": bids,
                "asks": asks,
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            });

            // 将事件添加到消息缓冲区
            self.message_buffer.push(event_data.clone());

            debug!("处理Coinbase L2更新: {} changes", changes.len());
        }

        Ok(())
    }

    /// 处理ticker消息
    async fn process_ticker_message(&mut self, msg: &CoinbaseWebSocketMessage, product_id: &str) -> Result<(), Box<dyn Error>> {
        if let (Some(best_bid), Some(best_ask)) = (&msg.best_bid, &msg.best_ask) {
            if let (Ok(bid_price), Ok(ask_price)) = (best_bid.parse::<f64>(), best_ask.parse::<f64>()) {
                let event_data = json!({
                    "exchange": "coinbase",
                    "symbol": product_id,
                    "best_bid": bid_price,
                    "best_ask": ask_price,
                    "best_bid_size": msg.best_bid_size.as_ref().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
                    "best_ask_size": msg.best_ask_size.as_ref().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
                    "timestamp": SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                });

                // 将事件添加到消息缓冲区
                self.message_buffer.push(event_data.clone());

                debug!("处理Coinbase ticker: {} - bid: {}, ask: {}", product_id, bid_price, ask_price);
            }
        }

        Ok(())
    }

    /// 处理交易消息
    async fn process_match_message(&mut self, msg: &CoinbaseWebSocketMessage, product_id: &str) -> Result<(), Box<dyn Error>> {
        if let (Some(price_str), Some(size_str), Some(side)) = (&msg.price, &msg.size, &msg.side) {
            if let (Ok(price), Ok(size)) = (price_str.parse::<f64>(), size_str.parse::<f64>()) {
                let event_data = json!({
                    "exchange": "coinbase",
                    "symbol": product_id,
                    "price": price,
                    "size": size,
                    "side": side.to_lowercase(),
                    "timestamp": SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                    "trade_id": msg.trade_id.map(|id| id.to_string()).unwrap_or_default(),
                });

                // 将事件添加到消息缓冲区
                self.message_buffer.push(event_data.clone());

                debug!("处理Coinbase交易: {} {} @ {}", size, product_id, price);
                
                info!("Coinbase Trade: {} {} {} @ {}", 
                      side.to_lowercase(), size, product_id, price);
            }
        }

        Ok(())
    }

    /// 发送ping消息
    async fn send_ping(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(sender) = &mut self.ws_sender {
            let ping_msg = json!({
                "type": "ping"
            });
            
            sender.send(Message::Text(ping_msg.to_string())).await?;
            self.last_ping_time = Instant::now();
            
            debug!("发送Coinbase ping消息");
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl ExchangeWebSocketManager for CoinbaseWebSocketManager {
    fn exchange_name(&self) -> &str {
        "Coinbase"
    }

    async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        let url = self.get_websocket_url();
        info!("连接到Coinbase WebSocket: {}", url);
        
        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                let (sender, mut receiver) = ws_stream.split();
                self.ws_sender = Some(sender);
                self.connection_state = ExchangeConnectionState::Connected;
                
                info!("Coinbase WebSocket连接成功");
                
                // 启动消息处理任务
                let mut manager = CoinbaseWebSocketManager::new(self.config.clone());
                manager.connection_state = ExchangeConnectionState::Connected;
                
                tokio::spawn(async move {
                    while let Some(message) = receiver.next().await {
                        match message {
                            Ok(msg) => {
                                if let Err(e) = manager.handle_message(msg).await {
                                    error!("处理Coinbase WebSocket消息失败: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("接收Coinbase WebSocket消息失败: {}", e);
                                break;
                            }
                        }
                    }
                });
                
                Ok(())
            }
            Err(e) => {
                error!("连接Coinbase WebSocket失败: {}", e);
                self.connection_state = ExchangeConnectionState::Disconnected;
                self.stats.connection_errors += 1;
                Err(e.into())
            }
        }
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(mut sender) = self.ws_sender.take() {
            sender.send(Message::Close(None)).await?;
            info!("Coinbase WebSocket连接已断开");
        }
        
        self.connection_state = ExchangeConnectionState::Disconnected;
        Ok(())
    }

    async fn subscribe_depth(&mut self, symbol: &str) -> Result<(), Box<dyn Error>> {
        let product_id = if symbol.contains("-") {
            symbol.to_string() // 已经是Coinbase格式
        } else {
            symbol.replace("USDT", "-USD") // 转换为Coinbase格式 BTCUSDT -> BTC-USD
        };
        
        if let Some(sender) = &mut self.ws_sender {
            let subscribe_msg = CoinbaseSubscriptionRequest {
                msg_type: "subscribe".to_string(),
                channels: vec![
                    CoinbaseChannel {
                        name: "level2".to_string(),
                        product_ids: vec![product_id.clone()],
                    }
                ],
            };
            
            sender.send(Message::Text(serde_json::to_string(&subscribe_msg)?)).await?;
            self.subscriptions.push(format!("level2:{}", product_id));
            
            info!("订阅Coinbase深度数据: {}", product_id);
        }
        
        Ok(())
    }

    async fn subscribe_trades(&mut self, symbol: &str) -> Result<(), Box<dyn Error>> {
        let product_id = if symbol.contains("-") {
            symbol.to_string() // 已经是Coinbase格式
        } else {
            symbol.replace("USDT", "-USD") // 转换为Coinbase格式
        };
        
        if let Some(sender) = &mut self.ws_sender {
            let subscribe_msg = CoinbaseSubscriptionRequest {
                msg_type: "subscribe".to_string(),
                channels: vec![
                    CoinbaseChannel {
                        name: "matches".to_string(),
                        product_ids: vec![product_id.clone()],
                    }
                ],
            };
            
            sender.send(Message::Text(serde_json::to_string(&subscribe_msg)?)).await?;
            self.subscriptions.push(format!("matches:{}", product_id));
            
            info!("订阅Coinbase交易数据: {}", product_id);
        }
        
        Ok(())
    }

    async fn subscribe_book_ticker(&mut self, symbol: &str) -> Result<(), Box<dyn Error>> {
        let product_id = if symbol.contains("-") {
            symbol.to_string() // 已经是Coinbase格式
        } else {
            symbol.replace("USDT", "-USD") // 转换为Coinbase格式
        };
        
        if let Some(sender) = &mut self.ws_sender {
            let subscribe_msg = CoinbaseSubscriptionRequest {
                msg_type: "subscribe".to_string(),
                channels: vec![
                    CoinbaseChannel {
                        name: "ticker".to_string(),
                        product_ids: vec![product_id.clone()],
                    }
                ],
            };
            
            sender.send(Message::Text(serde_json::to_string(&subscribe_msg)?)).await?;
            self.subscriptions.push(format!("ticker:{}", product_id));
            
            info!("订阅Coinbase最优买卖价: {}", product_id);
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
        info!("尝试重连Coinbase WebSocket");
        
        if let Err(e) = self.disconnect().await {
            warn!("断开连接时出错: {}", e);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        self.connect().await?;
        
        // 重新订阅之前的主题
        for subscription in self.subscriptions.clone() {
            let parts: Vec<&str> = subscription.split(':').collect();
            if parts.len() == 2 {
                let channel = parts[0];
                let product_id = parts[1];
                let symbol = product_id.replace("-USD", "USDT"); // 转换回原格式
                
                match channel {
                    "level2" => {
                        if let Err(e) = self.subscribe_depth(&symbol).await {
                            warn!("重新订阅深度数据失败: {}", e);
                        }
                    }
                    "matches" => {
                        if let Err(e) = self.subscribe_trades(&symbol).await {
                            warn!("重新订阅交易数据失败: {}", e);
                        }
                    }
                    "ticker" => {
                        if let Err(e) = self.subscribe_book_ticker(&symbol).await {
                            warn!("重新订阅最优买卖价失败: {}", e);
                        }
                    }
                    _ => {
                        warn!("未知的订阅频道: {}", channel);
                    }
                }
            }
        }
        
        self.stats.reconnect_attempts += 1;
        info!("Coinbase WebSocket重连成功");
        
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
        if let Some(msg_type) = message.get("type").and_then(|t| t.as_str()) {
            return msg_type == "snapshot" || msg_type == "l2update";
        }
        false
    }

    /// 判断是否为交易消息
    fn is_trade_message(&self, message: &Value) -> bool {
        if let Some(msg_type) = message.get("type").and_then(|t| t.as_str()) {
            return msg_type == "match" || msg_type == "last_match";
        }
        false
    }
} 