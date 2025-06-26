use std::collections::BTreeMap;
use ordered_float::OrderedFloat;
use serde_json::Value;

use super::data_structures::*;
use super::order_flow::OrderFlow;
use crate::gui::TimeFootprintData;

/// 订单簿管理器 - 简化版本，专注于核心功能
pub struct OrderBookManager {
    order_flows: BTreeMap<OrderedFloat<f64>, OrderFlow>,
    config: OrderBookConfig,
    stats: OrderBookStats,
    
    // 市场状态
    best_bid_price: Option<f64>,
    best_ask_price: Option<f64>,
    current_price: Option<f64>,
    
    // 快照和缓存
    bookticker_snapshot: Option<BookTickerSnapshot>,
    market_snapshot: MarketSnapshot,
    
    // 分析数据
    bid_volume_ratio: f64,
    ask_volume_ratio: f64,
    price_speed: f64,
    avg_speed: f64,
    volatility: f64,
    
    // 内部缓冲区
    tick_buffer: Vec<u64>,
    speed_history: Vec<(u64, f64)>,
    volatility_buffer: Vec<(u64, f64)>,

    // 交易高亮显示
    last_trade_price: Option<f64>,
    last_trade_side: Option<String>, // "buy" or "sell"
    last_trade_timestamp: Option<u64>,

    // 时间维度足迹数据
    time_footprint_data: TimeFootprintData,
}

impl OrderBookManager {
    pub fn new() -> Self {
        Self {
            order_flows: BTreeMap::new(),
            config: OrderBookConfig::default(),
            stats: OrderBookStats::default(),
            
            best_bid_price: None,
            best_ask_price: None,
            current_price: None,
            
            bookticker_snapshot: None,
            market_snapshot: MarketSnapshot::new(),
            
            bid_volume_ratio: 0.5,
            ask_volume_ratio: 0.5,
            price_speed: 0.0,
            avg_speed: 0.0,
            volatility: 0.0,
            
            tick_buffer: Vec::new(),
            speed_history: Vec::new(),
            volatility_buffer: Vec::new(),

            last_trade_price: None,
            last_trade_side: None,
            last_trade_timestamp: None,

            time_footprint_data: TimeFootprintData::new(30), // 30分钟滑动窗口
        }
    }

    /// 处理深度更新
    pub fn handle_depth_update(&mut self, data: &Value) {
        let current_time = self.get_current_timestamp();
        self.stats.total_depth_updates += 1;

        let mut bid_count = 0;
        let mut ask_count = 0;

        // 处理买单
        if let Some(bids) = data["b"].as_array() {
            for bid in bids {
                if let (Some(price_str), Some(qty_str)) = (bid[0].as_str(), bid[1].as_str()) {
                    if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        let price_ordered = OrderedFloat(price);
                        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                        order_flow.update_price_level(qty, 0.0, current_time);
                        bid_count += 1;

                        // 更新最优买价
                        if self.best_bid_price.map_or(true, |best| price > best) {
                            self.best_bid_price = Some(price);
                        }
                    }
                }
            }
        }

        // 处理卖单
        if let Some(asks) = data["a"].as_array() {
            for ask in asks {
                if let (Some(price_str), Some(qty_str)) = (ask[0].as_str(), ask[1].as_str()) {
                    if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        let price_ordered = OrderedFloat(price);
                        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                        order_flow.update_price_level(0.0, qty, current_time);
                        ask_count += 1;

                        // 更新最优卖价
                        if self.best_ask_price.map_or(true, |best| price < best) {
                            self.best_ask_price = Some(price);
                        }
                    }
                }
            }
        }

        // 调试信息已移除

        self.update_market_snapshot();
    }

    /// 处理交易数据
    pub fn handle_trade(&mut self, data: &Value) {
        let current_time = self.get_current_timestamp();
        self.stats.total_trades += 1;
        
        if let (Some(price_str), Some(qty_str), Some(is_buyer_maker)) = (
            data["p"].as_str(),
            data["q"].as_str(),
            data["m"].as_bool(),
        ) {
            if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                let side = if is_buyer_maker { "sell" } else { "buy" };
                
                // 更新当前价格
                self.current_price = Some(price);

                // 更新交易高亮信息
                self.last_trade_price = Some(price);
                self.last_trade_side = Some(side.to_string());
                self.last_trade_timestamp = Some(current_time);

                // 更新订单流
                let price_ordered = OrderedFloat(price);
                let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                order_flow.add_trade(side, qty, current_time);

                // 更新时间维度足迹数据
                self.time_footprint_data.add_trade(price, side, qty, current_time);

                // 计算价格速度和波动率
                self.calculate_price_speed(current_time);
                self.calculate_volatility(current_time, price);

                self.update_market_snapshot();
            }
        }
    }

    /// 处理BookTicker数据
    pub fn handle_book_ticker(&mut self, data: &Value) {
        let current_time = self.get_current_timestamp();
        self.stats.total_book_ticker_updates += 1;

        if let (Some(best_bid_str), Some(best_ask_str), Some(best_bid_qty_str), Some(best_ask_qty_str)) =
            (data["b"].as_str(), data["a"].as_str(), data["B"].as_str(), data["A"].as_str()) {

            if let (Ok(best_bid_price), Ok(best_ask_price), Ok(best_bid_qty), Ok(best_ask_qty)) =
                (best_bid_str.parse::<f64>(), best_ask_str.parse::<f64>(),
                 best_bid_qty_str.parse::<f64>(), best_ask_qty_str.parse::<f64>()) {

                // 更新最优价格
                self.best_bid_price = Some(best_bid_price);
                self.best_ask_price = Some(best_ask_price);

                // 清理非法的挂单数据
                self.clean_invalid_orders(best_bid_price, best_ask_price);

                // 创建快照
                self.bookticker_snapshot = Some(BookTickerSnapshot {
                    best_bid_price,
                    best_ask_price,
                    best_bid_qty,
                    best_ask_qty,
                    timestamp: current_time,
                });

                // 计算多空比例
                self.calculate_volume_ratio(best_bid_qty, best_ask_qty);

                self.update_market_snapshot();
            }
        }
    }

    /// 清理不合理的挂单数据
    fn clean_invalid_orders(&mut self, best_bid_price: f64, best_ask_price: f64) {
        // println!("DEBUG: clean_invalid_orders - best_bid: {:.2}, best_ask: {:.2}",
                // best_bid_price, best_ask_price);

        let mut bid_cleared = 0;
        let mut ask_cleared = 0;
        let total_flows = self.order_flows.len();

        // 更保守的清理策略：只清理明显不合理的数据
        for (price, order_flow) in self.order_flows.iter_mut() {
            let price_val = price.0;

            // 只清理明显违反市场规则的数据，添加缓冲区
            let spread = best_ask_price - best_bid_price;
            let buffer = spread * 0.1; // 10%的缓冲区

            // 处理asks时：只清除明显高于best_ask_price的bid挂单
            if price_val > best_ask_price + buffer && order_flow.bid_ask.bid > 0.0 {
                // println!("DEBUG: clearing bid at price {:.2} (best_ask: {:.2})", price_val, best_ask_price);
                order_flow.bid_ask.bid = 0.0;
                bid_cleared += 1;
            }

            // 处理bids时：只清除明显低于best_bid_price的ask挂单
            if price_val < best_bid_price - buffer && order_flow.bid_ask.ask > 0.0 {
                // println!("DEBUG: clearing ask at price {:.2} (best_bid: {:.2})", price_val, best_bid_price);
                order_flow.bid_ask.ask = 0.0;
                ask_cleared += 1;
            }
        }

        // println!("DEBUG: cleaned {} bid orders, {} ask orders from {} total flows",
                // bid_cleared, ask_cleared, total_flows);

        // 定期清理过期的交易数据，但保留挂单数据
        let current_time = self.get_current_timestamp();
        for (_, order_flow) in self.order_flows.iter_mut() {
            order_flow.clean_expired_trades(current_time, 5000); // 5秒
            order_flow.clean_expired_cancels(current_time, 5000); // 5秒
            order_flow.clean_expired_increases(current_time, 5000); // 5秒
        }
    }

    /// 计算价格速度
    fn calculate_price_speed(&mut self, timestamp: u64) {
        self.tick_buffer.push(timestamp);
        
        // 清理超过窗口的旧数据
        let cutoff_time = timestamp.saturating_sub(self.config.speed_window_ms);
        self.tick_buffer.retain(|&ts| ts >= cutoff_time);
        
        // 计算当前速度
        self.price_speed = self.tick_buffer.len() as f64;
        
        // 记录历史速度
        self.speed_history.push((timestamp, self.price_speed));
        
        // 清理历史数据
        let avg_cutoff_time = timestamp.saturating_sub(self.config.avg_speed_window_ms);
        self.speed_history.retain(|&(ts, _)| ts >= avg_cutoff_time);
        
        // 计算平均速度
        if !self.speed_history.is_empty() {
            let total_speed: f64 = self.speed_history.iter().map(|&(_, speed)| speed).sum();
            self.avg_speed = total_speed / self.speed_history.len() as f64;
        }
    }

    /// 计算波动率
    fn calculate_volatility(&mut self, timestamp: u64, price: f64) {
        if price <= 0.0 {
            return;
        }
        
        self.volatility_buffer.push((timestamp, price));
        
        // 清理超过窗口的旧数据
        let cutoff_time = timestamp.saturating_sub(self.config.volatility_window_ms);
        self.volatility_buffer.retain(|&(ts, _)| ts >= cutoff_time);
        
        // 计算波动率
        if self.volatility_buffer.len() >= 2 {
            let mut returns = Vec::new();
            for i in 1..self.volatility_buffer.len() {
                let prev_price = self.volatility_buffer[i-1].1;
                let curr_price = self.volatility_buffer[i].1;
                
                if prev_price > 0.0 && curr_price > 0.0 {
                    let log_return = (curr_price / prev_price).ln();
                    if !log_return.is_nan() && !log_return.is_infinite() {
                        returns.push(log_return);
                    }
                }
            }
            
            if !returns.is_empty() {
                let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
                let variance: f64 = returns.iter()
                    .map(|&r| (r - mean).powi(2))
                    .sum::<f64>() / returns.len() as f64;
                
                if variance >= 0.0 && !variance.is_nan() && !variance.is_infinite() {
                    self.volatility = variance.sqrt() * 100000.0;
                }
            }
        }
    }

    /// 计算多空比例
    fn calculate_volume_ratio(&mut self, bid_qty: f64, ask_qty: f64) {
        let total_volume = bid_qty + ask_qty;
        if total_volume > 0.0 {
            self.bid_volume_ratio = bid_qty / total_volume;
            self.ask_volume_ratio = ask_qty / total_volume;
        }
    }

    /// 更新市场快照
    fn update_market_snapshot(&mut self) {
        self.market_snapshot = MarketSnapshot {
            timestamp: self.get_current_timestamp(),
            best_bid_price: self.best_bid_price,
            best_ask_price: self.best_ask_price,
            current_price: self.current_price,
            bid_volume_ratio: self.bid_volume_ratio,
            ask_volume_ratio: self.ask_volume_ratio,
            price_speed: self.price_speed,
            avg_speed: self.avg_speed,
            volatility: self.volatility,
            tick_price_diff_volatility: 0.0, // 简化版本暂不实现
        };
        
        self.stats.last_update_time = self.market_snapshot.timestamp;
    }

    /// 获取当前时间戳
    fn get_current_timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    // 公共接口方法
    pub fn get_market_snapshot(&self) -> MarketSnapshot {
        self.market_snapshot.clone()
    }

    pub fn get_stats(&self) -> OrderBookStats {
        self.stats.clone()
    }

    pub fn get_order_flows(&self) -> &BTreeMap<OrderedFloat<f64>, OrderFlow> {
        &self.order_flows
    }

    pub fn get_best_prices(&self) -> (Option<f64>, Option<f64>) {
        (self.best_bid_price, self.best_ask_price)
    }

    pub fn get_current_price(&self) -> Option<f64> {
        self.current_price
    }

    pub fn get_volume_ratios(&self) -> (f64, f64) {
        (self.bid_volume_ratio, self.ask_volume_ratio)
    }

    /// 获取最近交易高亮信息
    pub fn get_last_trade_highlight(&self) -> (Option<f64>, Option<String>, Option<u64>) {
        (self.last_trade_price, self.last_trade_side.clone(), self.last_trade_timestamp)
    }

    /// 检查交易高亮是否应该显示（基于时间）
    pub fn should_show_trade_highlight(&self, max_age_ms: u64) -> bool {
        if let Some(timestamp) = self.last_trade_timestamp {
            let current_time = self.get_current_timestamp();
            current_time.saturating_sub(timestamp) <= max_age_ms
        } else {
            false
        }
    }

    /// 获取时间维度足迹数据
    pub fn get_time_footprint_data(&self) -> &TimeFootprintData {
        &self.time_footprint_data
    }



    /// 定期清理过期数据
    pub fn cleanup_expired_data(&mut self) {
        let current_time = self.get_current_timestamp();

        // 清理5秒内的实时交易数据
        for (_, order_flow) in self.order_flows.iter_mut() {
            order_flow.clean_expired_trades(current_time, 5000); // 5秒
            order_flow.clean_expired_cancels(current_time, 5000); // 5秒
            order_flow.clean_expired_increases(current_time, 5000); // 5秒

            // 新增：清理超过5秒没有更新的挂单数据（1美元精度聚合的深度订单薄数据）
            order_flow.clean_expired_price_levels(current_time, 1000); // 1秒
        }

        // 清理空的或过期的订单流条目
        self.order_flows.retain(|_, order_flow| {
            // 保留有挂单数据或最近有活动的条目
            order_flow.bid_ask.bid > 0.0 ||
            order_flow.bid_ask.ask > 0.0 ||
            order_flow.has_recent_activity(current_time, 60000) // 保留60秒内有活动的
        });
    }

    pub fn clear(&mut self) {
        self.order_flows.clear();
        self.best_bid_price = None;
        self.best_ask_price = None;
        self.current_price = None;
        self.bookticker_snapshot = None;
        self.market_snapshot = MarketSnapshot::new();
    }
}

impl Default for OrderBookManager {
    fn default() -> Self {
        Self::new()
    }
}
