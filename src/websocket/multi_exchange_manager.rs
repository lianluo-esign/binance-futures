use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use log::{info, warn, error, debug};

use super::exchange_trait::{ExchangeWebSocketManager, ExchangeConfig, ExchangeConnectionState, ExchangeStats};
use super::exchanges::okx::OkxWebSocketManager;
use super::exchanges::bybit::BybitWebSocketManager;
use super::exchanges::coinbase::CoinbaseWebSocketManager;
use super::exchanges::bitget::BitgetWebSocketManager;
use super::exchanges::bitfinex::BitfinexWebSocketManager;
use super::exchanges::gateio::GateioWebSocketManager;
use super::exchanges::mexc::MexcWebSocketManager;
use crate::events::event_types::Exchange;
use crate::events::event_bus::EventBus;

/// 支持的交易所类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExchangeType {
    Binance,
    Okx,
    Bybit,
    Coinbase,
    Bitget,
    Bitfinex,
    GateIo,
    Mexc,
}

impl ExchangeType {
    /// 转换为Exchange枚举
    pub fn to_exchange(&self) -> Exchange {
        match self {
            ExchangeType::Binance => Exchange::Binance,
            ExchangeType::Okx => Exchange::OKX,
            ExchangeType::Bybit => Exchange::Bybit,
            ExchangeType::Coinbase => Exchange::Coinbase,
            ExchangeType::Bitget => Exchange::Bitget,
            ExchangeType::Bitfinex => Exchange::Bitfinex,
            ExchangeType::GateIo => Exchange::GateIO,
            ExchangeType::Mexc => Exchange::MEXC,
        }
    }

    /// 获取交易所名称
    pub fn name(&self) -> &'static str {
        match self {
            ExchangeType::Binance => "Binance",
            ExchangeType::Okx => "OKX",
            ExchangeType::Bybit => "Bybit",
            ExchangeType::Coinbase => "Coinbase",
            ExchangeType::Bitget => "Bitget",
            ExchangeType::Bitfinex => "Bitfinex",
            ExchangeType::GateIo => "Gate.io",
            ExchangeType::Mexc => "MEXC",
        }
    }
}

/// 多交易所管理器配置
#[derive(Debug, Clone)]
pub struct MultiExchangeConfig {
    /// 启用的交易所列表
    pub enabled_exchanges: Vec<ExchangeType>,
    /// 各交易所的配置
    pub exchange_configs: HashMap<ExchangeType, ExchangeConfig>,
    /// 自动重连设置
    pub auto_reconnect: bool,
    /// 重连间隔（秒）
    pub reconnect_interval: u64,
    /// 最大重连次数
    pub max_reconnect_attempts: u32,
}

impl Default for MultiExchangeConfig {
    fn default() -> Self {
        Self {
            enabled_exchanges: vec![ExchangeType::Binance, ExchangeType::Okx],
            exchange_configs: HashMap::new(),
            auto_reconnect: true,
            reconnect_interval: 5,
            max_reconnect_attempts: 10,
        }
    }
}

/// 多交易所管理器统计信息
#[derive(Debug, Clone, Default)]
pub struct MultiExchangeStats {
    /// 总连接数
    pub total_connections: usize,
    /// 活跃连接数
    pub active_connections: usize,
    /// 总消息数
    pub total_messages: u64,
    /// 总错误数
    pub total_errors: u64,
    /// 各交易所统计
    pub exchange_stats: HashMap<ExchangeType, ExchangeStats>,
}

/// 多交易所WebSocket管理器
pub struct MultiExchangeManager {
    /// 配置
    config: MultiExchangeConfig,
    /// 交易所管理器集合
    exchanges: HashMap<ExchangeType, Box<dyn ExchangeWebSocketManager + Send + Sync>>,
    /// 事件总线
    event_bus: Arc<RwLock<EventBus>>,
    /// 统计信息
    stats: Arc<RwLock<MultiExchangeStats>>,
    /// 运行状态
    is_running: Arc<RwLock<bool>>,
}

impl MultiExchangeManager {
    /// 创建新的多交易所管理器
    pub fn new(config: MultiExchangeConfig, event_bus: Arc<RwLock<EventBus>>) -> Self {
        Self {
            config,
            exchanges: HashMap::new(),
            event_bus,
            stats: Arc::new(RwLock::new(MultiExchangeStats::default())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// 初始化所有交易所连接
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("开始初始化多交易所管理器...");

        for exchange_type in &self.config.enabled_exchanges {
            if let Some(exchange_config) = self.config.exchange_configs.get(exchange_type) {
                match self.create_exchange_manager(*exchange_type, exchange_config.clone()).await {
                    Ok(manager) => {
                        self.exchanges.insert(*exchange_type, manager);
                        info!("成功初始化 {} 交易所连接", exchange_type.name());
                    }
                    Err(e) => {
                        error!("初始化 {} 交易所连接失败: {}", exchange_type.name(), e);
                        // 继续初始化其他交易所，不因单个失败而停止
                    }
                }
            } else {
                warn!("未找到 {} 交易所的配置", exchange_type.name());
            }
        }

        self.update_stats().await;
        info!("多交易所管理器初始化完成，共初始化 {} 个交易所", self.exchanges.len());
        Ok(())
    }

    /// 创建交易所管理器
    async fn create_exchange_manager(
        &self,
        exchange_type: ExchangeType,
        config: ExchangeConfig,
    ) -> Result<Box<dyn ExchangeWebSocketManager + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
        match exchange_type {
            ExchangeType::Okx => {
                let manager = OkxWebSocketManager::new(config);
                Ok(Box::new(manager))
            }
            ExchangeType::Bybit => {
                let manager = BybitWebSocketManager::new(config);
                Ok(Box::new(manager))
            }
            ExchangeType::Binance => {
                // TODO: 实现Binance管理器
                Err("Binance WebSocket管理器尚未实现".into())
            }
            ExchangeType::Coinbase => {
                let manager = CoinbaseWebSocketManager::new(config);
                Ok(Box::new(manager))
            }
            ExchangeType::Bitget => {
                let manager = BitgetWebSocketManager::new();
                Ok(Box::new(manager))
            }
            ExchangeType::Bitfinex => {
                let manager = BitfinexWebSocketManager::new();
                Ok(Box::new(manager))
            }
            ExchangeType::GateIo => {
                let manager = GateioWebSocketManager::new();
                Ok(Box::new(manager))
            }
            ExchangeType::Mexc => {
                let manager = MexcWebSocketManager::new();
                Ok(Box::new(manager))
            }
        }
    }

    /// 启动所有交易所连接
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("启动多交易所WebSocket连接...");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        let mut success_count = 0;
        let mut error_count = 0;

        for (exchange_type, manager) in &mut self.exchanges {
            match manager.connect().await {
                Ok(()) => {
                    info!("成功连接到 {} 交易所", exchange_type.name());
                    success_count += 1;
                }
                Err(e) => {
                    error!("连接到 {} 交易所失败: {}", exchange_type.name(), e);
                    error_count += 1;
                }
            }
        }

        self.update_stats().await;
        info!("多交易所连接启动完成，成功: {}, 失败: {}", success_count, error_count);

        if success_count > 0 {
            Ok(())
        } else {
            Err("所有交易所连接都失败".into())
        }
    }

    /// 停止所有交易所连接
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("停止多交易所WebSocket连接...");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        for (exchange_type, manager) in &mut self.exchanges {
            match manager.disconnect().await {
                Ok(()) => {
                    info!("成功断开 {} 交易所连接", exchange_type.name());
                }
                Err(e) => {
                    error!("断开 {} 交易所连接失败: {}", exchange_type.name(), e);
                }
            }
        }

        self.update_stats().await;
        info!("多交易所连接停止完成");
        Ok(())
    }

    /// 订阅BTCUSDT永续合约数据
    pub async fn subscribe_btcusdt_perpetual(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("订阅所有交易所的BTCUSDT永续合约数据...");

        for (exchange_type, manager) in &mut self.exchanges {
            match manager.subscribe_btcusdt_perpetual().await {
                Ok(()) => {
                    info!("成功订阅 {} 交易所的BTCUSDT永续合约数据", exchange_type.name());
                }
                Err(e) => {
                    error!("订阅 {} 交易所的BTCUSDT永续合约数据失败: {}", exchange_type.name(), e);
                }
            }
        }

        Ok(())
    }

    /// 处理所有交易所的消息
    pub async fn process_messages(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let is_running = *self.is_running.read().await;
        if !is_running {
            return Ok(());
        }

        for (exchange_type, manager) in &mut self.exchanges {
            match manager.read_messages().await {
                Ok(messages) => {
                    // 处理接收到的消息
                    for message in messages {
                        debug!("收到 {} 交易所消息: {:?}", exchange_type.name(), message);
                        // TODO: 将消息发送到事件总线
                    }
                }
                Err(e) => {
                    debug!("处理 {} 交易所消息时出错: {}", exchange_type.name(), e);
                    // 记录错误但继续处理其他交易所
                }
            }
        }

        Ok(())
    }

    /// 获取连接状态
    pub async fn get_connection_states(&self) -> HashMap<ExchangeType, ExchangeConnectionState> {
        let mut states = HashMap::new();
        
        for (exchange_type, manager) in &self.exchanges {
            states.insert(*exchange_type, manager.get_connection_state());
        }
        
        states
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> MultiExchangeStats {
        self.stats.read().await.clone()
    }

    /// 更新统计信息
    async fn update_stats(&self) {
        let mut stats = self.stats.write().await;
        
        stats.total_connections = self.exchanges.len();
        stats.active_connections = 0;
        stats.total_messages = 0;
        stats.total_errors = 0;
        stats.exchange_stats.clear();

        for (exchange_type, manager) in &self.exchanges {
            let exchange_stats = manager.get_stats();
            
            if manager.get_connection_state() == ExchangeConnectionState::Connected {
                stats.active_connections += 1;
            }
            
            stats.total_messages += exchange_stats.total_messages_received;
            stats.total_errors += exchange_stats.connection_errors;
            stats.exchange_stats.insert(*exchange_type, exchange_stats);
        }
    }

    /// 重连指定交易所
    pub async fn reconnect_exchange(&mut self, exchange_type: ExchangeType) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(manager) = self.exchanges.get_mut(&exchange_type) {
            info!("尝试重连 {} 交易所...", exchange_type.name());
            
            match manager.reconnect().await {
                Ok(()) => {
                    info!("成功重连 {} 交易所", exchange_type.name());
                    self.update_stats().await;
                    Ok(())
                }
                                    Err(e) => {
                        error!("重连 {} 交易所失败: {}", exchange_type.name(), e);
                        Err(format!("重连 {} 交易所失败: {}", exchange_type.name(), e).into())
                    }
            }
        } else {
            Err(format!("未找到 {} 交易所的管理器", exchange_type.name()).into())
        }
    }

    /// 重连所有断开的交易所
    pub async fn reconnect_all(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("检查并重连所有断开的交易所...");
        
        let mut reconnect_count = 0;
        let mut error_count = 0;

        for (exchange_type, manager) in &mut self.exchanges {
            let state = manager.get_connection_state();
            
            if state != ExchangeConnectionState::Connected {
                match manager.reconnect().await {
                    Ok(()) => {
                        info!("成功重连 {} 交易所", exchange_type.name());
                        reconnect_count += 1;
                    }
                    Err(e) => {
                        error!("重连 {} 交易所失败: {}", exchange_type.name(), e);
                        error_count += 1;
                    }
                }
            }
        }

        self.update_stats().await;
        info!("重连完成，成功: {}, 失败: {}", reconnect_count, error_count);
        Ok(())
    }

    /// 检查是否有交易所需要重连
    pub async fn check_health(&self) -> Vec<ExchangeType> {
        let mut unhealthy_exchanges = Vec::new();
        
        for (exchange_type, manager) in &self.exchanges {
            let state = manager.get_connection_state();
            if state != ExchangeConnectionState::Connected {
                unhealthy_exchanges.push(*exchange_type);
            }
        }
        
        unhealthy_exchanges
    }

    /// 获取运行状态
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// 添加交易所配置
    pub fn add_exchange_config(&mut self, exchange_type: ExchangeType, config: ExchangeConfig) {
        self.config.exchange_configs.insert(exchange_type, config);
        if !self.config.enabled_exchanges.contains(&exchange_type) {
            self.config.enabled_exchanges.push(exchange_type);
        }
    }

    /// 移除交易所
    pub async fn remove_exchange(&mut self, exchange_type: ExchangeType) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(mut manager) = self.exchanges.remove(&exchange_type) {
            if let Err(e) = manager.disconnect().await {
                error!("断开 {} 交易所连接失败: {}", exchange_type.name(), e);
            }
            info!("成功移除 {} 交易所", exchange_type.name());
        }
        
        self.config.enabled_exchanges.retain(|&x| x != exchange_type);
        self.config.exchange_configs.remove(&exchange_type);
        self.update_stats().await;
        Ok(())
    }
}

/// 多交易所管理器构建器
pub struct MultiExchangeManagerBuilder {
    config: MultiExchangeConfig,
}

impl MultiExchangeManagerBuilder {
    pub fn new() -> Self {
        Self {
            config: MultiExchangeConfig::default(),
        }
    }

    pub fn with_exchanges(mut self, exchanges: Vec<ExchangeType>) -> Self {
        self.config.enabled_exchanges = exchanges;
        self
    }

    pub fn with_exchange_config(mut self, exchange_type: ExchangeType, config: ExchangeConfig) -> Self {
        self.config.exchange_configs.insert(exchange_type, config);
        self
    }

    pub fn with_auto_reconnect(mut self, enabled: bool) -> Self {
        self.config.auto_reconnect = enabled;
        self
    }

    pub fn with_reconnect_interval(mut self, interval: u64) -> Self {
        self.config.reconnect_interval = interval;
        self
    }

    pub fn with_max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.config.max_reconnect_attempts = attempts;
        self
    }

    pub fn build(self, event_bus: Arc<RwLock<EventBus>>) -> MultiExchangeManager {
        MultiExchangeManager::new(self.config, event_bus)
    }
}

impl Default for MultiExchangeManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
} 