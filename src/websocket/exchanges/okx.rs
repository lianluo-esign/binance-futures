use async_trait::async_trait;
use serde_json::{json, Value};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tungstenite::{Message, WebSocket};
use std::net::TcpStream;
use tungstenite::stream::MaybeTlsStream;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::websocket::exchange_trait::{
    ExchangeWebSocketManager, ExchangeConfig, ExchangeConnectionState, 
    ExchangeStats, ContractSpec, StandardizedMarketData
};

/// OKX WebSocket管理器
pub struct OkxWebSocketManager {
    config: ExchangeConfig,
    socket: Option<Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>>,
    state: ExchangeConnectionState,
    stats: ExchangeStats,
    contract_spec: ContractSpec,
}

impl OkxWebSocketManager {
    pub fn new(config: ExchangeConfig) -> Self {
        // OKX BTCUSDT永续合约规格
        let contract_spec = ContractSpec {
            exchange: "okx".to_string(),
            symbol: "BTC-USDT-SWAP".to_string(),
            contract_size: 0.01,  // 每张合约0.01 BTC
            tick_size: 0.1,       // 最小价格变动0.1 USDT
            lot_size: 1.0,        // 最小交易1张合约
            is_inverse: false,    // 正向合约
            is_linear: true,      // 线性合约
        };

        Self {
            config,
            socket: None,
            state: ExchangeConnectionState::Disconnected,
            stats: ExchangeStats::default(),
            contract_spec,
        }
    }

    /// 构建WebSocket URL
    fn build_url(&self) -> String {
        if self.config.testnet {
            "wss://wspap.okx.com:8443/ws/v5/public".to_string()
        } else {
            "wss://ws.okx.com:8443/ws/v5/public".to_string()
        }
    }

    /// 构建订阅消息
    fn build_subscribe_message(&self, channel: &str, inst_id: &str) -> Value {
        json!({
            "op": "subscribe",
            "args": [{
                "channel": channel,
                "instId": inst_id
            }]
        })
    }

    /// 解析OKX消息格式
    fn parse_okx_message(&self, msg: &Value) -> Result<Value, Box<dyn Error>> {
        // OKX的消息格式通常是：
        // {
        //   "arg": { "channel": "trades", "instId": "BTC-USDT-SWAP" },
        //   "data": [...]
        // }
        
        if let Some(arg) = msg.get("arg") {
            if let Some(channel) = arg.get("channel").and_then(|v| v.as_str()) {
                let mut result = json!({
                    "exchange": "okx",
                    "channel": channel,
                    "symbol": self.config.symbol.clone(),
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64
                });

                match channel {
                    "trades" => {
                        result["type"] = json!("trade");
                        result["data"] = self.parse_trades_data(msg)?;
                    }
                    "books5" | "books-l2-tbt" => {
                        result["type"] = json!("depth");
                        result["data"] = self.parse_depth_data(msg)?;
                    }
                    "bbo-tbt" => {
                        result["type"] = json!("book_ticker");
                        result["data"] = self.parse_book_ticker_data(msg)?;
                    }
                    _ => {
                        result["type"] = json!("unknown");
                        result["raw_data"] = msg.clone();
                    }
                }

                return Ok(result);
            }
        }

        // 处理其他类型的消息（如订阅确认）
        Ok(msg.clone())
    }

    /// 解析交易数据
    fn parse_trades_data(&self, msg: &Value) -> Result<Value, Box<dyn Error>> {
        if let Some(data) = msg.get("data").and_then(|v| v.as_array()) {
            let mut trades = Vec::new();
            
            for trade in data {
                // OKX交易数据格式：
                // {
                //   "instId": "BTC-USDT-SWAP",
                //   "tradeId": "123456",
                //   "px": "30000.0",      // 价格
                //   "sz": "10",           // 合约张数
                //   "side": "buy",        // 买卖方向
                //   "ts": "1597026383085" // 时间戳
                // }
                
                let price = trade.get("px")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                
                let contracts = trade.get("sz")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                
                // 将合约张数转换为BTC数量
                let btc_amount = self.contract_spec.contracts_to_btc(contracts, price);
                
                trades.push(json!({
                    "trade_id": trade.get("tradeId").and_then(|v| v.as_str()).unwrap_or(""),
                    "price": price,
                    "amount": btc_amount,  // BTC数量
                    "contracts": contracts, // 原始合约张数
                    "side": trade.get("side").and_then(|v| v.as_str()).unwrap_or(""),
                    "timestamp": trade.get("ts").and_then(|v| v.as_str()).unwrap_or("")
                }));
            }
            
            return Ok(json!(trades));
        }
        
        Ok(json!([]))
    }

    /// 解析深度数据
    fn parse_depth_data(&self, msg: &Value) -> Result<Value, Box<dyn Error>> {
        if let Some(data) = msg.get("data").and_then(|v| v.as_array()) {
            if let Some(depth) = data.first() {
                // OKX深度数据格式：
                // {
                //   "asks": [["30100.0", "5", "0", "1"]],  // [价格, 合约张数, 0, 订单数]
                //   "bids": [["30000.0", "10", "0", "2"]], 
                //   "ts": "1597026383085"
                // }
                
                let mut asks = Vec::new();
                let mut bids = Vec::new();
                
                // 处理卖单
                if let Some(ask_data) = depth.get("asks").and_then(|v| v.as_array()) {
                    for ask in ask_data {
                        if let Some(ask_arr) = ask.as_array() {
                            if ask_arr.len() >= 2 {
                                let price = ask_arr[0].as_str()
                                    .and_then(|s| s.parse::<f64>().ok())
                                    .unwrap_or(0.0);
                                let contracts = ask_arr[1].as_str()
                                    .and_then(|s| s.parse::<f64>().ok())
                                    .unwrap_or(0.0);
                                
                                let btc_amount = self.contract_spec.contracts_to_btc(contracts, price);
                                
                                asks.push(json!({
                                    "price": price,
                                    "amount": btc_amount,
                                    "contracts": contracts
                                }));
                            }
                        }
                    }
                }
                
                // 处理买单
                if let Some(bid_data) = depth.get("bids").and_then(|v| v.as_array()) {
                    for bid in bid_data {
                        if let Some(bid_arr) = bid.as_array() {
                            if bid_arr.len() >= 2 {
                                let price = bid_arr[0].as_str()
                                    .and_then(|s| s.parse::<f64>().ok())
                                    .unwrap_or(0.0);
                                let contracts = bid_arr[1].as_str()
                                    .and_then(|s| s.parse::<f64>().ok())
                                    .unwrap_or(0.0);
                                
                                let btc_amount = self.contract_spec.contracts_to_btc(contracts, price);
                                
                                bids.push(json!({
                                    "price": price,
                                    "amount": btc_amount,
                                    "contracts": contracts
                                }));
                            }
                        }
                    }
                }
                
                return Ok(json!({
                    "asks": asks,
                    "bids": bids,
                    "timestamp": depth.get("ts").and_then(|v| v.as_str()).unwrap_or("")
                }));
            }
        }
        
        Ok(json!({"asks": [], "bids": []}))
    }

    /// 解析最优买卖价数据
    fn parse_book_ticker_data(&self, msg: &Value) -> Result<Value, Box<dyn Error>> {
        if let Some(data) = msg.get("data").and_then(|v| v.as_array()) {
            if let Some(ticker) = data.first() {
                // OKX BBO数据格式：
                // {
                //   "instId": "BTC-USDT-SWAP",
                //   "bidPx": "30000.0",   // 最优买价
                //   "bidSz": "10",        // 最优买量（合约张数）
                //   "askPx": "30100.0",   // 最优卖价
                //   "askSz": "5",         // 最优卖量（合约张数）
                //   "ts": "1597026383085"
                // }
                
                let bid_price = ticker.get("bidPx")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                
                let ask_price = ticker.get("askPx")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                
                let bid_contracts = ticker.get("bidSz")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                
                let ask_contracts = ticker.get("askSz")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                
                // 转换为BTC数量
                let bid_amount = self.contract_spec.contracts_to_btc(bid_contracts, bid_price);
                let ask_amount = self.contract_spec.contracts_to_btc(ask_contracts, ask_price);
                
                return Ok(json!({
                    "bid_price": bid_price,
                    "bid_amount": bid_amount,
                    "bid_contracts": bid_contracts,
                    "ask_price": ask_price,
                    "ask_amount": ask_amount,
                    "ask_contracts": ask_contracts,
                    "timestamp": ticker.get("ts").and_then(|v| v.as_str()).unwrap_or("")
                }));
            }
        }
        
        Ok(json!({}))
    }
}

#[async_trait]
impl ExchangeWebSocketManager for OkxWebSocketManager {
    fn exchange_name(&self) -> &str {
        "okx"
    }

    async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        self.state = ExchangeConnectionState::Connecting;
        
        let url = self.build_url();
        log::info!("[OKX] 正在连接到: {}", url);
        
        // 同步连接
        let (socket, response) = tungstenite::connect(&url)?;
        
        log::info!("[OKX] WebSocket连接响应状态: {}", response.status());
        
        self.socket = Some(Arc::new(Mutex::new(socket)));
        self.state = ExchangeConnectionState::Connected;
        self.stats.connection_start_time = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        );
        
        log::info!("[OKX] WebSocket连接成功");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(socket) = self.socket.take() {
            let mut socket_guard = socket.lock().await;
            socket_guard.close(None)?;
        }
        
        self.state = ExchangeConnectionState::Disconnected;
        log::info!("[OKX] WebSocket连接已断开");
        Ok(())
    }

    async fn subscribe_depth(&mut self, symbol: &str) -> Result<(), Box<dyn Error>> {
        let subscribe_msg = self.build_subscribe_message("books5", symbol);
        self.send_message(subscribe_msg).await?;
        log::info!("[OKX] 已订阅深度数据: {}", symbol);
        Ok(())
    }

    async fn subscribe_trades(&mut self, symbol: &str) -> Result<(), Box<dyn Error>> {
        let subscribe_msg = self.build_subscribe_message("trades", symbol);
        self.send_message(subscribe_msg).await?;
        log::info!("[OKX] 已订阅交易数据: {}", symbol);
        Ok(())
    }

    async fn subscribe_book_ticker(&mut self, symbol: &str) -> Result<(), Box<dyn Error>> {
        let subscribe_msg = self.build_subscribe_message("bbo-tbt", symbol);
        self.send_message(subscribe_msg).await?;
        log::info!("[OKX] 已订阅最优买卖价: {}", symbol);
        Ok(())
    }

    async fn read_messages(&mut self) -> Result<Vec<Value>, Box<dyn Error>> {
        let mut messages = Vec::new();
        
        if let Some(socket) = &self.socket {
            let mut socket_guard = socket.lock().await;
            
            // 非阻塞读取
            match socket_guard.read_message() {
                Ok(msg) => {
                    match msg {
                        tungstenite::Message::Text(text) => {
                            self.stats.total_messages_received += 1;
                            self.stats.total_bytes_received += text.len() as u64;
                            
                            if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                                let processed = self.parse_okx_message(&parsed)?;
                                messages.push(processed);
                            } else {
                                log::warn!("[OKX] 解析消息失败: {}", text);
                            }
                        }
                        tungstenite::Message::Ping(data) => {
                            socket_guard.write_message(tungstenite::Message::Pong(data))?;
                        }
                        tungstenite::Message::Close(_) => {
                            self.state = ExchangeConnectionState::Disconnected;
                        }
                        _ => {}
                    }
                }
                Err(tungstenite::Error::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // 非阻塞模式下的正常情况
                }
                Err(e) => {
                    log::error!("[OKX] 读取消息错误: {}", e);
                    return Err(e.into());
                }
            }
        }
        
        Ok(messages)
    }

    async fn send_heartbeat(&mut self) -> Result<(), Box<dyn Error>> {
        let ping_msg = json!({"op": "ping"});
        self.send_message(ping_msg).await
    }

    fn should_reconnect(&self) -> bool {
        matches!(self.state, ExchangeConnectionState::Disconnected | ExchangeConnectionState::Failed(_))
    }

    async fn attempt_reconnect(&mut self) -> Result<(), Box<dyn Error>> {
        log::info!("[OKX] 尝试重连...");
        self.connect().await
    }

    fn get_connection_state(&self) -> ExchangeConnectionState {
        self.state.clone()
    }

    fn get_stats(&self) -> ExchangeStats {
        self.stats.clone()
    }

    /// 判断是否为深度消息
    fn is_depth_message(&self, message: &Value) -> bool {
        if let Some(arg) = message.get("arg") {
            if let Some(channel) = arg.get("channel").and_then(|c| c.as_str()) {
                return channel == "books5" || channel == "books-l2-tbt";
            }
        }
        false
    }

    /// 判断是否为交易消息
    fn is_trade_message(&self, message: &Value) -> bool {
        if let Some(arg) = message.get("arg") {
            if let Some(channel) = arg.get("channel").and_then(|c| c.as_str()) {
                return channel == "trades";
            }
        }
        false
    }

    /// 启动独立线程管理WebSocket连接
    async fn start_with_event_sender(
        &mut self,
        event_sender: mpsc::UnboundedSender<StandardizedMarketData>,
    ) -> Result<(), Box<dyn Error>> {
        log::info!("[OKX] 启动独立线程管理");
        
        // 连接到WebSocket
        self.connect().await?;
        
        // 只订阅depth和trades数据
        self.subscribe_depth("BTC-USDT-SWAP").await?;
        self.subscribe_trades("BTC-USDT-SWAP").await?;
        
        // 启动消息处理循环
        loop {
            let should_reconnect = self.should_reconnect();
            
            match self.read_messages().await {
                Ok(messages) => {
                    for message in messages {
                        // 根据消息内容判断类型并标准化
                        let standardized_data = if self.is_depth_message(&message) {
                            self.standardize_depth_data(&message)
                        } else if self.is_trade_message(&message) {
                            self.standardize_trade_data(&message)
                        } else {
                            continue; // 跳过不需要的消息类型
                        };
                        
                        // 发送到事件总线
                        if let Err(_) = event_sender.send(standardized_data) {
                            log::error!("[OKX] 发送标准化数据到事件总线失败");
                            break;
                        }
                    }
                }
                Err(e) => {
                    log::error!("[OKX] 读取消息失败: {}", e);
                }
            }
            
            // 在match块外检查重连，确保没有错误值跨越await
            if should_reconnect {
                if let Err(reconnect_error) = self.attempt_reconnect().await {
                    log::error!("[OKX] 重连失败: {}", reconnect_error);
                    break;
                }
            }
            
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        Ok(())
    }
}

impl OkxWebSocketManager {
    /// 发送消息到WebSocket
    async fn send_message(&mut self, msg: Value) -> Result<(), Box<dyn Error>> {
        if let Some(socket) = &self.socket {
            let mut socket_guard = socket.lock().await;
            let text = msg.to_string();
            socket_guard.write_message(tungstenite::Message::Text(text))?;
            log::debug!("[OKX] 发送消息: {}", msg);
        }
        Ok(())
    }
} 