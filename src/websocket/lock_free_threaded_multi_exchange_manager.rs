use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;
use serde::{Deserialize, Serialize};
use log::{info, warn, error, debug};

use super::exchange_trait::{ExchangeWebSocketManager, ExchangeConfig, ExchangeConnectionState, ExchangeStats, StandardizedMarketData};
use super::exchanges::okx::OkxWebSocketManager;
use super::exchanges::bybit::BybitWebSocketManager;
use super::exchanges::coinbase::CoinbaseWebSocketManager;
use super::exchanges::bitget::BitgetWebSocketManager;
use super::exchanges::bitfinex::BitfinexWebSocketManager;
use super::exchanges::gateio::GateioWebSocketManager;
use super::exchanges::mexc::MexcWebSocketManager;
use crate::events::event_types::{Event, EventType, Exchange};
use crate::events::lock_free_event_bus::LockFreeEventBus;

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

/// 无锁线程化多交易所配置
#[derive(Debug, Clone)]
pub struct LockFreeThreadedMultiExchangeConfig {
    /// 启用的交易所列表
    pub enabled_exchanges: Vec<ExchangeType>,
    /// 各交易所的配置
    pub exchange_configs: HashMap<ExchangeType, ExchangeConfig>,
    /// 事件缓冲区大小
    pub event_buffer_size: usize,
    /// 数据处理批次大小
    pub batch_size: usize,
    /// 数据处理间隔（毫秒）
    pub processing_interval_ms: u64,
}

impl Default for LockFreeThreadedMultiExchangeConfig {
    fn default() -> Self {
        Self {
            enabled_exchanges: vec![
                ExchangeType::Okx,
                ExchangeType::Bybit,
                ExchangeType::Bitget,
                ExchangeType::Bitfinex,
                ExchangeType::GateIo,
                ExchangeType::Mexc,
            ],
            exchange_configs: HashMap::new(),
            event_buffer_size: 50000,  // 更大的缓冲区以支持高频数据
            batch_size: 200,           // 更大的批次以提高吞吐量
            processing_interval_ms: 1, // 1毫秒处理间隔
        }
    }
}

/// 交易所线程信息
#[derive(Debug)]
pub struct ExchangeThreadInfo {
    pub exchange_type: ExchangeType,
    pub handle: JoinHandle<()>,
    pub data_sender: mpsc::UnboundedSender<StandardizedMarketData>,
    pub stats: Arc<RwLock<ExchangeStats>>,
}

/// 无锁线程化多交易所管理器
/// 
/// 这个管理器使用LockFreeEventBus来避免锁竞争，
/// 特别适合高频交易数据处理场景
pub struct LockFreeThreadedMultiExchangeManager {
    config: LockFreeThreadedMultiExchangeConfig,
    exchange_threads: HashMap<ExchangeType, ExchangeThreadInfo>,
    event_bus: Arc<LockFreeEventBus>,
    data_receiver: Option<mpsc::UnboundedReceiver<StandardizedMarketData>>,
    data_sender: mpsc::UnboundedSender<StandardizedMarketData>,
    running: Arc<RwLock<bool>>,
    processor_handle: Option<JoinHandle<()>>,
}

impl LockFreeThreadedMultiExchangeManager {
    /// 创建新的无锁线程化多交易所管理器
    pub fn new(config: LockFreeThreadedMultiExchangeConfig, event_bus: Arc<LockFreeEventBus>) -> Self {
        let (data_sender, data_receiver) = mpsc::unbounded_channel();
        
        Self {
            config,
            exchange_threads: HashMap::new(),
            event_bus,
            data_receiver: Some(data_receiver),
            data_sender,
            running: Arc::new(RwLock::new(false)),
            processor_handle: None,
        }
    }

    /// 启动所有交易所的独立线程
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("启动无锁线程化多交易所管理器...");
        
        *self.running.write().await = true;

        // 启动数据处理线程
        self.start_data_processor().await?;

        // 为每个启用的交易所启动独立线程
        let enabled_exchanges = self.config.enabled_exchanges.clone();
        for exchange_type in enabled_exchanges {
            if let Some(config) = self.config.exchange_configs.get(&exchange_type) {
                self.start_exchange_thread(exchange_type, config.clone()).await?;
            } else {
                warn!("未找到 {} 的配置，使用默认配置", exchange_type.name());
                let default_config = ExchangeConfig::default();
                self.start_exchange_thread(exchange_type, default_config).await?;
            }
        }

        info!("所有交易所线程启动完成，共启动 {} 个交易所", self.exchange_threads.len());
        Ok(())
    }

    /// 启动单个交易所的独立线程
    async fn start_exchange_thread(
        &mut self,
        exchange_type: ExchangeType,
        config: ExchangeConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("启动 {} 交易所线程...", exchange_type.name());

        let (thread_sender, mut thread_receiver) = mpsc::unbounded_channel::<StandardizedMarketData>();
        let main_sender = self.data_sender.clone();
        let stats = Arc::new(RwLock::new(ExchangeStats::default()));
        let stats_clone = stats.clone();
        let running = self.running.clone();

        // 创建交易所管理器
        let mut manager = self.create_exchange_manager(exchange_type, config).await?;

        // 启动交易所WebSocket线程
        let ws_handle = {
            let exchange_name = exchange_type.name().to_string();
            tokio::spawn(async move {
                info!("[{}] WebSocket线程启动", exchange_name);
                
                if let Err(e) = manager.start_with_event_sender(thread_sender).await {
                    error!("[{}] WebSocket线程异常退出: {}", exchange_name, e);
                } else {
                    info!("[{}] WebSocket线程正常退出", exchange_name);
                }
            })
        };

        // 启动数据转发线程
        let forward_handle = {
            let exchange_name = exchange_type.name().to_string();
            tokio::spawn(async move {
                info!("[{}] 数据转发线程启动", exchange_name);
                
                while *running.read().await {
                    match thread_receiver.recv().await {
                        Some(data) => {
                            // 更新统计信息
                            {
                                let mut stats_guard = stats_clone.write().await;
                                stats_guard.total_messages_received += 1;
                                stats_guard.last_message_time = Some(data.timestamp);
                            }

                            // 转发到主数据处理器
                            if let Err(_) = main_sender.send(data) {
                                error!("[{}] 转发数据到主处理器失败", exchange_name);
                                break;
                            }
                        }
                        None => {
                            debug!("[{}] 数据接收通道关闭", exchange_name);
                            break;
                        }
                    }
                }
                
                info!("[{}] 数据转发线程退出", exchange_name);
            })
        };

        // 将线程信息保存到管理器中
        self.exchange_threads.insert(exchange_type, ExchangeThreadInfo {
            exchange_type,
            handle: ws_handle,
            data_sender: self.data_sender.clone(),
            stats,
        });

        info!("[{}] 交易所线程启动成功", exchange_type.name());
        Ok(())
    }

    /// 启动数据处理线程（无锁版本）
    async fn start_data_processor(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut receiver = self.data_receiver.take()
            .ok_or("数据接收器已被使用")?;
        
        let event_bus = self.event_bus.clone();
        let running = self.running.clone();
        let batch_size = self.config.batch_size;
        let processing_interval = std::time::Duration::from_millis(self.config.processing_interval_ms);

        let handle = tokio::spawn(async move {
            info!("无锁数据处理线程启动");
            let mut batch = Vec::with_capacity(batch_size);
            let mut last_process_time = std::time::Instant::now();

            while *running.read().await {
                // 收集批次数据或等待超时
                let mut collected = false;
                
                // 尝试快速收集一批数据
                while batch.len() < batch_size {
                    match receiver.try_recv() {
                        Ok(data) => {
                            batch.push(data);
                            collected = true;
                        }
                        Err(mpsc::error::TryRecvError::Empty) => break,
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            warn!("数据接收通道断开");
                            return;
                        }
                    }
                }

                // 如果没有收集到数据，等待一个数据或超时
                if !collected {
                    match tokio::time::timeout(processing_interval, receiver.recv()).await {
                        Ok(Some(data)) => {
                            batch.push(data);
                            collected = true;
                        }
                        Ok(None) => {
                            debug!("数据处理接收通道关闭");
                            break;
                        }
                        Err(_) => {
                            // 超时，检查是否有部分数据需要处理
                        }
                    }
                }

                // 处理批次数据或定期处理
                let now = std::time::Instant::now();
                if !batch.is_empty() && (batch.len() >= batch_size || now.duration_since(last_process_time) >= processing_interval) {
                    Self::process_data_batch_lock_free(&event_bus, &mut batch).await;
                    last_process_time = now;
                }
            }

            // 处理剩余的数据
            if !batch.is_empty() {
                Self::process_data_batch_lock_free(&event_bus, &mut batch).await;
            }

            info!("无锁数据处理线程退出");
        });

        self.processor_handle = Some(handle);
        Ok(())
    }

    /// 处理数据批次（无锁版本）
    async fn process_data_batch_lock_free(
        event_bus: &Arc<LockFreeEventBus>,
        batch: &mut Vec<StandardizedMarketData>,
    ) {
        if batch.is_empty() {
            return;
        }

        debug!("处理数据批次，大小: {}", batch.len());

        let mut events = Vec::with_capacity(batch.len());
        
        for data in batch.drain(..) {
            // 将标准化数据转换为事件
            let event_type = match data.data_type.as_str() {
                "depth" => EventType::DepthUpdate(data.data),
                "trade" => EventType::Trade(data.data),
                _ => continue,
            };

            let event = Event::new_with_exchange(
                event_type,
                "lock_free_websocket_manager".to_string(),
                data.exchange,
            );

            events.push(event);
        }

        // 批量发布事件到无锁事件总线
        if !events.is_empty() {
            let published = event_bus.publish_batch(events);
            debug!("成功发布 {} 个事件到无锁事件总线", published);
        }
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
            ExchangeType::Binance => {
                Err("Binance WebSocket管理器尚未实现".into())
            }
        }
    }

    /// 停止所有交易所线程
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("停止无锁线程化多交易所管理器...");
        
        *self.running.write().await = false;

        // 等待所有交易所线程完成
        for (exchange_type, thread_info) in self.exchange_threads.drain() {
            info!("等待 {} 线程完成...", exchange_type.name());
            if let Err(e) = thread_info.handle.await {
                error!("等待 {} 线程完成时出错: {:?}", exchange_type.name(), e);
            }
        }

        // 等待数据处理线程完成
        if let Some(handle) = self.processor_handle.take() {
            info!("等待数据处理线程完成...");
            if let Err(e) = handle.await {
                error!("等待数据处理线程完成时出错: {:?}", e);
            }
        }

        info!("所有线程已停止");
        Ok(())
    }

    /// 获取所有交易所的统计信息
    pub async fn get_all_stats(&self) -> HashMap<ExchangeType, ExchangeStats> {
        let mut all_stats = HashMap::new();
        
        for (exchange_type, thread_info) in &self.exchange_threads {
            let stats = thread_info.stats.read().await.clone();
            all_stats.insert(*exchange_type, stats);
        }
        
        all_stats
    }

    /// 获取运行状态
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// 添加交易所配置
    pub fn add_exchange_config(&mut self, exchange_type: ExchangeType, config: ExchangeConfig) {
        self.config.exchange_configs.insert(exchange_type, config);
        if !self.config.enabled_exchanges.contains(&exchange_type) {
            self.config.enabled_exchanges.push(exchange_type);
        }
    }

    /// 获取活跃的交易所数量
    pub fn active_exchanges_count(&self) -> usize {
        self.exchange_threads.len()
    }

    /// 获取事件总线统计信息
    pub fn get_event_bus_stats(&self) -> crate::events::lock_free_event_bus::EventBusStats {
        self.event_bus.stats()
    }

    /// 获取事件总线缓冲区使用情况
    pub fn get_event_bus_usage(&self) -> (usize, usize) {
        (self.event_bus.pending_events(), self.event_bus.capacity())
    }

    /// 处理事件总线中的待处理事件
    pub fn process_pending_events(&self, max_events: usize) -> usize {
        self.event_bus.process_events(max_events)
    }
}

/// 无锁线程化多交易所管理器构建器
pub struct LockFreeThreadedMultiExchangeManagerBuilder {
    config: LockFreeThreadedMultiExchangeConfig,
}

impl LockFreeThreadedMultiExchangeManagerBuilder {
    pub fn new() -> Self {
        Self {
            config: LockFreeThreadedMultiExchangeConfig::default(),
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

    pub fn with_event_buffer_size(mut self, size: usize) -> Self {
        self.config.event_buffer_size = size;
        self
    }

    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.config.batch_size = size;
        self
    }

    pub fn with_processing_interval_ms(mut self, interval_ms: u64) -> Self {
        self.config.processing_interval_ms = interval_ms;
        self
    }

    pub fn build(self, event_bus: Arc<LockFreeEventBus>) -> LockFreeThreadedMultiExchangeManager {
        LockFreeThreadedMultiExchangeManager::new(self.config, event_bus)
    }
}

impl Default for LockFreeThreadedMultiExchangeManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 创建预配置的无锁线程化多交易所管理器
pub fn create_lock_free_threaded_manager(
    event_bus_capacity: usize,
) -> (LockFreeThreadedMultiExchangeManager, Arc<LockFreeEventBus>) {
    let event_bus = Arc::new(LockFreeEventBus::new(event_bus_capacity));
    let manager = LockFreeThreadedMultiExchangeManagerBuilder::new()
        .with_event_buffer_size(event_bus_capacity)
        .build(event_bus.clone());
    
    (manager, event_bus)
} 