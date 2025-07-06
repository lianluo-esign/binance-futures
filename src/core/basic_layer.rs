use std::collections::{HashMap, VecDeque, BTreeMap};
use std::time::{SystemTime, UNIX_EPOCH};
use ordered_float::OrderedFloat;
use serde_json::Value;

use crate::orderbook::{OrderFlow, OrderBookConfig, OrderBookStats};
use crate::events::event_types::{Event, EventType};

/// 交易数据结构
#[derive(Debug, Clone)]
pub struct TradeData {
    pub timestamp: u64,
    pub price: f64,
    pub quantity: f64,
    pub side: String, // "buy" or "sell"
    pub exchange: String,
}

/// 订单簿数据结构
#[derive(Debug, Clone)]
pub struct OrderBookData {
    pub timestamp: u64,
    pub bids: Vec<(f64, f64)>, // (price, quantity)
    pub asks: Vec<(f64, f64)>, // (price, quantity)
    pub exchange: String,
}

/// 单个交易所的数据管理器
#[derive(Debug)]
pub struct ExchangeDataManager {
    /// 交易所名称
    pub exchange_name: String,
    
    /// 订单簿数据 - 使用现有的OrderFlow结构
    pub order_flows: BTreeMap<OrderedFloat<f64>, OrderFlow>,
    
    /// 成交数据滑动窗口 - 最多10000条
    pub trades_window: VecDeque<TradeData>,
    
    /// 最优买卖价
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    
    /// 统计信息
    pub stats: OrderBookStats,
    
    /// 配置
    pub config: OrderBookConfig,
    
    /// 最后更新时间
    pub last_update: u64,
}

impl ExchangeDataManager {
    pub fn new(exchange_name: String) -> Self {
        Self {
            exchange_name,
            order_flows: BTreeMap::new(),
            trades_window: VecDeque::with_capacity(10000),
            best_bid: None,
            best_ask: None,
            stats: OrderBookStats::default(),
            config: OrderBookConfig::default(),
            last_update: 0,
        }
    }

    /// 处理深度数据更新
    pub fn handle_depth_update(&mut self, data: &Value) {
        let current_time = self.get_current_timestamp();
        self.stats.total_depth_updates += 1;
        self.last_update = current_time;

        // 处理买单
        if let Some(bids) = data.get("bids").and_then(|b| b.as_array()) {
            for bid in bids {
                if let (Some(price_val), Some(qty_val)) = (bid.get(0), bid.get(1)) {
                    if let (Ok(price), Ok(qty)) = (
                        price_val.as_str().unwrap_or("0").parse::<f64>(),
                        qty_val.as_str().unwrap_or("0").parse::<f64>()
                    ) {
                        let price_ordered = OrderedFloat(price);
                        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                        order_flow.update_price_level(qty, 0.0, current_time);

                        // 更新最优买价
                        if self.best_bid.map_or(true, |best| price > best) {
                            self.best_bid = Some(price);
                        }
                    }
                }
            }
        }

        // 处理卖单
        if let Some(asks) = data.get("asks").and_then(|a| a.as_array()) {
            for ask in asks {
                if let (Some(price_val), Some(qty_val)) = (ask.get(0), ask.get(1)) {
                    if let (Ok(price), Ok(qty)) = (
                        price_val.as_str().unwrap_or("0").parse::<f64>(),
                        qty_val.as_str().unwrap_or("0").parse::<f64>()
                    ) {
                        let price_ordered = OrderedFloat(price);
                        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                        order_flow.update_price_level(0.0, qty, current_time);

                        // 更新最优卖价
                        if self.best_ask.map_or(true, |best| price < best) {
                            self.best_ask = Some(price);
                        }
                    }
                }
            }
        }
    }

    /// 处理成交数据
    pub fn handle_trade_update(&mut self, data: &Value) {
        let current_time = self.get_current_timestamp();
        self.stats.total_trades += 1;
        self.last_update = current_time;

        if let (Some(price_val), Some(qty_val), Some(side_val)) = (
            data.get("price").or_else(|| data.get("p")),
            data.get("quantity").or_else(|| data.get("q")).or_else(|| data.get("size")).or_else(|| data.get("sz")),
            data.get("side").or_else(|| data.get("S")).or_else(|| data.get("m"))
        ) {
            if let (Ok(price), Ok(quantity)) = (
                price_val.as_str().unwrap_or("0").parse::<f64>(),
                qty_val.as_str().unwrap_or("0").parse::<f64>()
            ) {
                let side = if let Some(side_str) = side_val.as_str() {
                    match side_str.to_lowercase().as_str() {
                        "buy" | "b" | "bid" => "buy",
                        "sell" | "s" | "ask" => "sell",
                        _ => {
                            // 对于一些交易所，可能使用布尔值表示方向
                            if side_val.as_bool().unwrap_or(false) {
                                "sell" // true表示卖方主动成交
                            } else {
                                "buy"  // false表示买方主动成交
                            }
                        }
                    }
                } else {
                    "unknown"
                };

                // 创建成交数据
                let trade_data = TradeData {
                    timestamp: current_time,
                    price,
                    quantity,
                    side: side.to_string(),
                    exchange: self.exchange_name.clone(),
                };

                // 添加到滑动窗口
                self.add_trade_to_window(trade_data);

                // 更新订单簿中的成交记录
                let price_ordered = OrderedFloat(price);
                let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                
                order_flow.add_trade(side, quantity, current_time);
            }
        }
    }

    /// 处理最优买卖价数据
    pub fn handle_book_ticker_update(&mut self, data: &Value) {
        let current_time = self.get_current_timestamp();
        self.stats.total_book_ticker_updates += 1;
        self.last_update = current_time;

        if let (Some(bid_price_val), Some(ask_price_val)) = (
            data.get("bidPrice").or_else(|| data.get("b")).or_else(|| data.get("best_bid")),
            data.get("askPrice").or_else(|| data.get("a")).or_else(|| data.get("best_ask"))
        ) {
            if let (Ok(bid_price), Ok(ask_price)) = (
                bid_price_val.as_str().unwrap_or("0").parse::<f64>(),
                ask_price_val.as_str().unwrap_or("0").parse::<f64>()
            ) {
                self.best_bid = Some(bid_price);
                self.best_ask = Some(ask_price);
            }
        }
    }

    /// 添加成交数据到滑动窗口
    fn add_trade_to_window(&mut self, trade_data: TradeData) {
        // 如果窗口已满，移除最旧的数据
        if self.trades_window.len() >= 10000 {
            self.trades_window.pop_front();
        }
        
        self.trades_window.push_back(trade_data);
    }

    /// 获取当前时间戳
    fn get_current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// 获取指定时间范围内的成交数据
    pub fn get_trades_in_range(&self, start_time: u64, end_time: u64) -> Vec<&TradeData> {
        self.trades_window
            .iter()
            .filter(|trade| trade.timestamp >= start_time && trade.timestamp <= end_time)
            .collect()
    }

    /// 获取最近N条成交数据
    pub fn get_recent_trades(&self, count: usize) -> Vec<&TradeData> {
        self.trades_window
            .iter()
            .rev()
            .take(count)
            .collect()
    }

    /// 获取订单簿快照
    pub fn get_orderbook_snapshot(&self) -> OrderBookData {
        let mut bids = Vec::new();
        let mut asks = Vec::new();

        for (price, order_flow) in &self.order_flows {
            let price_val = price.into_inner();
            if order_flow.bid_ask.bid > 0.0 {
                bids.push((price_val, order_flow.bid_ask.bid));
            }
            if order_flow.bid_ask.ask > 0.0 {
                asks.push((price_val, order_flow.bid_ask.ask));
            }
        }

        // 排序：买单从高到低，卖单从低到高
        bids.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        asks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        OrderBookData {
            timestamp: self.last_update,
            bids,
            asks,
            exchange: self.exchange_name.clone(),
        }
    }

    /// 清理过期数据
    pub fn cleanup_expired_data(&mut self, max_age_ms: u64) {
        let current_time = self.get_current_timestamp();
        let cutoff_time = current_time.saturating_sub(max_age_ms);

        // 清理过期的成交数据
        while let Some(front) = self.trades_window.front() {
            if front.timestamp < cutoff_time {
                self.trades_window.pop_front();
            } else {
                break;
            }
        }

        // 清理过期的订单簿数据
        let mut prices_to_remove = Vec::new();
        for (price, order_flow) in &self.order_flows {
            if order_flow.bid_ask.timestamp < cutoff_time {
                prices_to_remove.push(*price);
            }
        }

        for price in prices_to_remove {
            self.order_flows.remove(&price);
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> &OrderBookStats {
        &self.stats
    }

    /// 获取成交数据窗口大小
    pub fn get_trades_count(&self) -> usize {
        self.trades_window.len()
    }

    /// 获取订单簿价格层级数量
    pub fn get_price_levels_count(&self) -> usize {
        self.order_flows.len()
    }
}

/// BasicLayer - 基础数据层管理器
#[derive(Debug)]
pub struct BasicLayer {
    /// 各交易所的数据管理器
    exchange_managers: HashMap<String, ExchangeDataManager>,
    
    /// 支持的交易所列表
    supported_exchanges: Vec<String>,
    
    /// 全局配置
    config: BasicLayerConfig,
    
    /// 最后更新时间
    last_update: u64,
}

/// BasicLayer配置
#[derive(Debug, Clone)]
pub struct BasicLayerConfig {
    /// 成交数据窗口大小
    pub trades_window_size: usize,
    
    /// 数据过期时间（毫秒）
    pub data_expire_time_ms: u64,
    
    /// 自动清理间隔（毫秒）
    pub cleanup_interval_ms: u64,
    
    /// 最后清理时间
    pub last_cleanup: u64,
}

impl Default for BasicLayerConfig {
    fn default() -> Self {
        Self {
            trades_window_size: 10000,
            data_expire_time_ms: 24 * 60 * 60 * 1000, // 24小时
            cleanup_interval_ms: 60 * 1000, // 1分钟
            last_cleanup: 0,
        }
    }
}

impl BasicLayer {
    pub fn new() -> Self {
        let supported_exchanges = vec![
            "binance".to_string(),
            "okx".to_string(),
            "bybit".to_string(),
            "coinbase".to_string(),
            "bitget".to_string(),
            "bitfinex".to_string(),
            "gateio".to_string(),
            "mexc".to_string(),
        ];

        let mut exchange_managers = HashMap::new();
        for exchange in &supported_exchanges {
            exchange_managers.insert(
                exchange.clone(),
                ExchangeDataManager::new(exchange.clone())
            );
        }

        Self {
            exchange_managers,
            supported_exchanges,
            config: BasicLayerConfig::default(),
            last_update: 0,
        }
    }

    /// 处理事件数据
    pub fn handle_event(&mut self, event: &Event) {
        let exchange_name = event.exchange.clone();
        
        // 确保交易所管理器存在
        if !self.exchange_managers.contains_key(&exchange_name) {
            self.exchange_managers.insert(
                exchange_name.clone(),
                ExchangeDataManager::new(exchange_name.clone())
            );
        }

        if let Some(manager) = self.exchange_managers.get_mut(&exchange_name) {
            match &event.event_type {
                EventType::DepthUpdate(data) => {
                    manager.handle_depth_update(data);
                }
                EventType::Trade(data) => {
                    manager.handle_trade_update(data);
                }
                EventType::BookTicker(data) => {
                    manager.handle_book_ticker_update(data);
                }
                EventType::TickPrice(data) => {
                    // 处理价格变动事件
                    manager.handle_trade_update(data);
                }
                _ => {
                    // 其他事件类型暂不处理
                }
            }
        }

        self.last_update = self.get_current_timestamp();
        
        // 定期清理过期数据
        self.cleanup_if_needed();
    }

    /// 获取指定交易所的数据管理器
    pub fn get_exchange_manager(&self, exchange: &str) -> Option<&ExchangeDataManager> {
        self.exchange_managers.get(exchange)
    }

    /// 获取指定交易所的数据管理器（可变引用）
    pub fn get_exchange_manager_mut(&mut self, exchange: &str) -> Option<&mut ExchangeDataManager> {
        self.exchange_managers.get_mut(exchange)
    }

    /// 获取所有交易所的订单簿快照
    pub fn get_all_orderbook_snapshots(&self) -> HashMap<String, OrderBookData> {
        let mut snapshots = HashMap::new();
        
        for (exchange, manager) in &self.exchange_managers {
            snapshots.insert(exchange.clone(), manager.get_orderbook_snapshot());
        }
        
        snapshots
    }

    /// 获取所有交易所的最近成交数据
    pub fn get_all_recent_trades(&self, count: usize) -> HashMap<String, Vec<&TradeData>> {
        let mut trades = HashMap::new();
        
        for (exchange, manager) in &self.exchange_managers {
            trades.insert(exchange.clone(), manager.get_recent_trades(count));
        }
        
        trades
    }

    /// 获取支持的交易所列表
    pub fn get_supported_exchanges(&self) -> &Vec<String> {
        &self.supported_exchanges
    }

    /// 获取活跃的交易所列表（有数据的交易所）
    pub fn get_active_exchanges(&self) -> Vec<String> {
        self.exchange_managers
            .iter()
            .filter(|(_, manager)| manager.get_trades_count() > 0 || manager.get_price_levels_count() > 0)
            .map(|(exchange, _)| exchange.clone())
            .collect()
    }

    /// 获取指定交易所的统计信息
    pub fn get_exchange_stats(&self, exchange: &str) -> Option<&OrderBookStats> {
        self.exchange_managers.get(exchange).map(|manager| manager.get_stats())
    }

    /// 获取全局统计信息
    pub fn get_global_stats(&self) -> BasicLayerStats {
        let mut total_trades = 0;
        let mut total_depth_updates = 0;
        let mut total_book_ticker_updates = 0;
        let mut active_exchanges = 0;

        for manager in self.exchange_managers.values() {
            let stats = manager.get_stats();
            total_trades += stats.total_trades;
            total_depth_updates += stats.total_depth_updates;
            total_book_ticker_updates += stats.total_book_ticker_updates;
            
            if manager.get_trades_count() > 0 || manager.get_price_levels_count() > 0 {
                active_exchanges += 1;
            }
        }

        BasicLayerStats {
            total_exchanges: self.exchange_managers.len(),
            active_exchanges,
            total_trades,
            total_depth_updates,
            total_book_ticker_updates,
            last_update: self.last_update,
        }
    }

    /// 定期清理过期数据
    fn cleanup_if_needed(&mut self) {
        let current_time = self.get_current_timestamp();
        
        if current_time.saturating_sub(self.config.last_cleanup) >= self.config.cleanup_interval_ms {
            for manager in self.exchange_managers.values_mut() {
                manager.cleanup_expired_data(self.config.data_expire_time_ms);
            }
            
            self.config.last_cleanup = current_time;
        }
    }

    /// 强制清理所有过期数据
    pub fn force_cleanup(&mut self) {
        for manager in self.exchange_managers.values_mut() {
            manager.cleanup_expired_data(self.config.data_expire_time_ms);
        }
        
        self.config.last_cleanup = self.get_current_timestamp();
    }

    /// 获取当前时间戳
    fn get_current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

/// BasicLayer统计信息
#[derive(Debug, Clone)]
pub struct BasicLayerStats {
    pub total_exchanges: usize,
    pub active_exchanges: usize,
    pub total_trades: u64,
    pub total_depth_updates: u64,
    pub total_book_ticker_updates: u64,
    pub last_update: u64,
} 