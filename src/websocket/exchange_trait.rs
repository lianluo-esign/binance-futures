use async_trait::async_trait;
use serde_json::Value;
use std::error::Error;
use std::time::Duration;
use tokio::sync::mpsc;
use crate::events::Event;

/// 交易所WebSocket配置
#[derive(Debug, Clone)]
pub struct ExchangeConfig {
    pub exchange_name: String,
    pub symbol: String,
    pub testnet: bool,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub reconnect_interval: Duration,
    pub heartbeat_interval: Duration,
    pub max_reconnect_attempts: u32,
}

impl Default for ExchangeConfig {
    fn default() -> Self {
        Self {
            exchange_name: "BTCUSDT".to_string(),
            symbol: "BTCUSDT".to_string(),
            testnet: false,
            api_key: None,
            api_secret: None,
            reconnect_interval: Duration::from_secs(5),
            heartbeat_interval: Duration::from_secs(30),
            max_reconnect_attempts: 5,
        }
    }
}

/// 交易所WebSocket连接状态
#[derive(Debug, Clone, PartialEq)]
pub enum ExchangeConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Authenticating,
    Authenticated,
    Subscribing,
    Subscribed,
    Reconnecting,
    Failed(String),
}

/// 标准化的市场数据结构
#[derive(Debug, Clone)]
pub struct StandardizedMarketData {
    pub exchange: String,
    pub data_type: String,  // "depth" 或 "trade"
    pub data: Value,        // 原始数据
    pub timestamp: u64,
}

/// 交易所WebSocket管理器trait
#[async_trait]
pub trait ExchangeWebSocketManager: Send + Sync {
    /// 获取交易所名称
    fn exchange_name(&self) -> &str;

    /// 连接到WebSocket
    async fn connect(&mut self) -> Result<(), Box<dyn Error>>;

    /// 断开连接
    async fn disconnect(&mut self) -> Result<(), Box<dyn Error>>;

    /// 订阅深度数据
    async fn subscribe_depth(&mut self, symbol: &str) -> Result<(), Box<dyn Error>>;

    /// 订阅成交数据
    async fn subscribe_trades(&mut self, symbol: &str) -> Result<(), Box<dyn Error>>;

    /// 订阅最优买卖价
    async fn subscribe_book_ticker(&mut self, symbol: &str) -> Result<(), Box<dyn Error>>;

    /// 读取消息
    async fn read_messages(&mut self) -> Result<Vec<Value>, Box<dyn Error>>;

    /// 发送心跳
    async fn send_heartbeat(&mut self) -> Result<(), Box<dyn Error>>;

    /// 获取连接状态
    fn get_connection_state(&self) -> ExchangeConnectionState;

    /// 是否已连接
    fn is_connected(&self) -> bool {
        matches!(
            self.get_connection_state(),
            ExchangeConnectionState::Connected
                | ExchangeConnectionState::Authenticated
                | ExchangeConnectionState::Subscribed
        )
    }

    /// 是否需要重连
    fn should_reconnect(&self) -> bool;

    /// 尝试重连
    async fn attempt_reconnect(&mut self) -> Result<(), Box<dyn Error>>;

    /// 获取统计信息
    fn get_stats(&self) -> ExchangeStats;

    /// 订阅BTCUSDT永续合约数据（便捷方法）- 只订阅depth和trades
    async fn subscribe_btcusdt_perpetual(&mut self) -> Result<(), Box<dyn Error>> {
        self.subscribe_depth("BTCUSDT").await?;
        self.subscribe_trades("BTCUSDT").await?;
        // 根据todolist要求，只订阅depth和trades，不订阅book_ticker
        Ok(())
    }

    /// 重连（包装attempt_reconnect）
    async fn reconnect(&mut self) -> Result<(), Box<dyn Error>> {
        self.attempt_reconnect().await
    }

    /// 标准化深度数据
    fn standardize_depth_data(&self, raw_data: &Value) -> StandardizedMarketData {
        StandardizedMarketData {
            exchange: self.exchange_name().to_string(),
            data_type: "depth".to_string(),
            data: raw_data.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    /// 标准化交易数据
    fn standardize_trade_data(&self, raw_data: &Value) -> StandardizedMarketData {
        StandardizedMarketData {
            exchange: self.exchange_name().to_string(),
            data_type: "trade".to_string(),
            data: raw_data.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    /// 启动独立线程管理WebSocket连接
    async fn start_with_event_sender(
        &mut self,
        event_sender: mpsc::UnboundedSender<StandardizedMarketData>,
    ) -> Result<(), Box<dyn Error>> {
        // 默认实现，各交易所可以覆盖此方法
        self.connect().await?;
        self.subscribe_btcusdt_perpetual().await?;
        
        // 启动消息处理循环
        loop {
            let exchange_name = self.exchange_name().to_string();
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
                            log::error!("[{}] 发送标准化数据到事件总线失败", exchange_name);
                            break;
                        }
                    }
                }
                Err(e) => {
                    log::error!("[{}] 读取消息失败: {}", exchange_name, e);
                }
            }
            
            // 在match块外检查重连，确保没有错误值跨越await
            if should_reconnect {
                if let Err(reconnect_error) = self.attempt_reconnect().await {
                    log::error!("[{}] 重连失败: {}", exchange_name, reconnect_error);
                    break;
                }
            }
            
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        Ok(())
    }

    /// 判断是否为深度消息（需要各交易所实现）
    fn is_depth_message(&self, message: &Value) -> bool {
        // 默认实现，各交易所应该覆盖此方法
        false
    }

    /// 判断是否为交易消息（需要各交易所实现）
    fn is_trade_message(&self, message: &Value) -> bool {
        // 默认实现，各交易所应该覆盖此方法
        false
    }
}

/// 交易所WebSocket统计信息
#[derive(Debug, Clone, Default)]
pub struct ExchangeStats {
    pub total_messages_received: u64,
    pub total_bytes_received: u64,
    pub parse_errors: u64,
    pub connection_errors: u64,
    pub subscription_errors: u64,
    pub reconnect_attempts: u32,
    pub last_message_time: Option<u64>,
    pub connection_start_time: Option<u64>,
}

/// 统一的市场数据格式
#[derive(Debug, Clone)]
pub struct UnifiedMarketData {
    pub exchange: String,
    pub symbol: String,
    pub timestamp: u64,
    pub data_type: MarketDataType,
    pub data: Value,
}

/// 市场数据类型
#[derive(Debug, Clone, PartialEq)]
pub enum MarketDataType {
    Depth,
    Trade,
    BookTicker,
    Ticker,
}

/// 合约规格信息（用于合约张数转换）
#[derive(Debug, Clone)]
pub struct ContractSpec {
    pub exchange: String,
    pub symbol: String,
    pub contract_size: f64,      // 每张合约的BTC数量
    pub tick_size: f64,          // 最小价格变动
    pub lot_size: f64,           // 最小数量变动
    pub is_inverse: bool,        // 是否为反向合约
    pub is_linear: bool,         // 是否为正向合约
}

impl ContractSpec {
    /// 将合约张数转换为BTC数量
    pub fn contracts_to_btc(&self, contracts: f64, price: f64) -> f64 {
        if self.is_linear {
            // 正向合约：数量 = 张数 * 合约面值
            contracts * self.contract_size
        } else if self.is_inverse {
            // 反向合约：数量 = 张数 * 合约面值 / 价格
            contracts * self.contract_size / price
        } else {
            // 币本位合约（如Binance）：直接是BTC数量
            contracts
        }
    }

    /// 将BTC数量转换为合约张数
    pub fn btc_to_contracts(&self, btc_amount: f64, price: f64) -> f64 {
        if self.is_linear {
            // 正向合约
            btc_amount / self.contract_size
        } else if self.is_inverse {
            // 反向合约
            btc_amount * price / self.contract_size
        } else {
            // 币本位合约
            btc_amount
        }
    }
} 