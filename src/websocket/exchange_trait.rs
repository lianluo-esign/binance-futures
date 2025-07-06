use async_trait::async_trait;
use serde_json::Value;
use std::error::Error;

/// 交易所WebSocket配置
#[derive(Debug, Clone)]
pub struct ExchangeConfig {
    pub exchange_name: String,
    pub symbol: String,
    pub testnet: bool,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
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

    /// 订阅BTCUSDT永续合约数据（便捷方法）
    async fn subscribe_btcusdt_perpetual(&mut self) -> Result<(), Box<dyn Error>> {
        self.subscribe_depth("BTCUSDT").await?;
        self.subscribe_trades("BTCUSDT").await?;
        self.subscribe_book_ticker("BTCUSDT").await?;
        Ok(())
    }

    /// 重连（包装attempt_reconnect）
    async fn reconnect(&mut self) -> Result<(), Box<dyn Error>> {
        self.attempt_reconnect().await
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