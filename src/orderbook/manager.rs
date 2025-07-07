use std::collections::{BTreeMap, HashSet};
use ordered_float::OrderedFloat;
use serde_json::Value;

use super::data_structures::*;
use super::order_flow::OrderFlow;
use crate::gui::TimeFootprintData;
use crate::audio::play_tick_pressure_sound;

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
    realized_volatility: f64,    // 高频波动率（Realized Volatility）
    jump_signal: f64,            // 价格跳跃信号（Jump）
    
    // 内部缓冲区
    tick_buffer: Vec<u64>,
    speed_history: Vec<(u64, f64)>,
    volatility_buffer: Vec<(u64, f64)>,
    price_returns: std::collections::VecDeque<f64>,  // 价格收益率序列，用于计算高频波动率
    price_history: std::collections::VecDeque<(u64, f64)>, // 价格历史数据，用于跳跃检测
    rv_history: std::collections::VecDeque<(u64, f64)>, // RV历史数据，用于线型图显示

    // 交易高亮显示
    last_trade_price: Option<f64>,
    last_trade_side: Option<String>, // "buy" or "sell"
    last_trade_timestamp: Option<u64>,
    last_trade_volume: Option<f64>, // 最新交易量

    // 时间维度足迹数据
    time_footprint_data: TimeFootprintData,

    // 当前Trade Imbalance值（基于最近10笔交易）
    current_trade_imbalance: f64,

    // ΔTick Pressure检测相关（同时用于Trade Imbalance计算）
    tick_pressure_window: std::collections::VecDeque<TickData>, // Tick数据滑动窗口
    tick_pressure_signals: std::collections::VecDeque<String>, // 信号文本滑动窗口，容量512
    tick_pressure_k_value: usize, // 连续K笔设置，默认7
    tick_pressure_signal_capacity: usize, // 信号窗口容量，默认512

    // 历史数据重置跟踪
    last_history_reset_date: u32, // 存储上次重置的UTC日期（YYYYMMDD格式）
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
            realized_volatility: 0.0,    // 高频波动率（Realized Volatility）
            jump_signal: 0.0,            // 价格跳跃信号（Jump）
            
            tick_buffer: Vec::new(),
            speed_history: Vec::new(),
            volatility_buffer: Vec::new(),
            price_returns: std::collections::VecDeque::new(),  // 价格收益率序列，用于计算高频波动率
            price_history: std::collections::VecDeque::new(), // 价格历史数据，用于跳跃检测
            rv_history: std::collections::VecDeque::new(), // RV历史数据，用于线型图显示

            last_trade_price: None,
            last_trade_side: None,
            last_trade_timestamp: None,
            last_trade_volume: None,

            time_footprint_data: TimeFootprintData::new(30), // 30分钟滑动窗口
            current_trade_imbalance: 0.0,

            // 初始化ΔTick Pressure相关字段（同时用于Trade Imbalance计算）
            tick_pressure_window: std::collections::VecDeque::new(),
            tick_pressure_signals: std::collections::VecDeque::new(),
            tick_pressure_k_value: 5, // 默认3笔
            tick_pressure_signal_capacity: 512, // 默认512容量

            last_history_reset_date: 0, // 初始化为0，表示还未重置过
        }
    }

    /// 处理深度更新
    pub fn handle_depth_update(&mut self, data: &Value) {
        let current_time = self.get_current_timestamp();
        self.stats.total_depth_updates += 1;

        let mut bid_count = 0;
        let mut ask_count = 0;
        
        // 收集本次更新的价格层级
        let mut updated_prices = HashSet::new();

        // 处理买单
        if let Some(bids) = data["b"].as_array() {
            for bid in bids {
                if let (Some(price_str), Some(qty_str)) = (bid[0].as_str(), bid[1].as_str()) {
                    if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        let price_ordered = OrderedFloat(price);
                        updated_prices.insert(price_ordered);
                        
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
                        updated_prices.insert(price_ordered);
                        
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

        // 根据配置决定是否清理未在本次更新中出现的深度数据
        if self.config.keep_only_updated_depth {
            self.clean_non_updated_depth_data(&updated_prices, current_time);
        }

        // 重新计算基于整个订单簿的多空比例
        self.calculate_volume_ratio(0.0, 0.0);

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
                // 跳过无效的交易数据
                if price <= 0.0 || qty <= 0.0 {
                    return;
                }

                let side = if is_buyer_maker { "sell" } else { "buy" };

                // 更新当前价格
                self.current_price = Some(price);

                // 更新交易高亮信息
                self.last_trade_price = Some(price);
                self.last_trade_side = Some(side.to_string());
                self.last_trade_timestamp = Some(current_time);
                self.last_trade_volume = Some(qty);

                // 更新订单流
                let price_ordered = OrderedFloat(price);
                let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                order_flow.add_trade(side, qty, current_time);

                // 更新时间维度足迹数据
                self.time_footprint_data.add_trade(price, side, qty, current_time);

                // 更新ΔTick Pressure检测（同时更新Trade Imbalance）
                self.update_tick_pressure_detection(current_time, price, qty, side == "buy");

                // 计算价格速度和波动率
                self.calculate_price_speed(current_time);
                self.calculate_volatility(current_time, price);

                // 计算高频波动率和价格跳跃
                self.calculate_realized_volatility(current_time, price);
                self.calculate_jump_signal(current_time, price);

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

    /// 清理未在本次深度更新中出现的挂单数据（优化版本，支持淡出动画）
    fn clean_non_updated_depth_data(&mut self, updated_prices: &HashSet<OrderedFloat<f64>>, _current_time: u64) {
        // 限制清理频率，避免每次更新都清理
        static mut LAST_CLEANUP_TIME: u64 = 0;
        let current_time = self.get_current_timestamp();
        
        unsafe {
            if current_time.saturating_sub(LAST_CLEANUP_TIME) < self.config.depth_cleanup_interval_ms {
                return;
            }
            LAST_CLEANUP_TIME = current_time;
        }

        let mut bid_fade_started = 0;
        let mut ask_fade_started = 0;
        let mut prices_to_remove = Vec::new();

        // 批量处理，减少单次操作的影响
        let mut batch_count = 0;
        let max_batch_size = self.config.max_cleanup_batch_size;

        // 遍历所有价格层级，处理未更新的深度数据
        for (price, order_flow) in self.order_flows.iter_mut() {
            if batch_count >= max_batch_size {
                break; // 限制单次处理数量
            }

            // 如果这个价格层级在本次更新中没有出现，启动淡出动画
            if !updated_prices.contains(price) {
                // 启动bid淡出动画
                if order_flow.bid_ask.bid > 0.0 && order_flow.bid_ask.bid_fade_start_time.is_none() {
                    order_flow.start_bid_fade(current_time);
                    bid_fade_started += 1;
                }
                
                // 启动ask淡出动画
                if order_flow.bid_ask.ask > 0.0 && order_flow.bid_ask.ask_fade_start_time.is_none() {
                    order_flow.start_ask_fade(current_time);
                    ask_fade_started += 1;
                }
                
                batch_count += 1;
            }

            // 检查淡出动画是否完成，如果完成则清理数据
            let (bid_fade_complete, ask_fade_complete) = order_flow.is_fade_complete(current_time);
            
            if bid_fade_complete && order_flow.bid_ask.bid > 0.0 {
                order_flow.bid_ask.bid = 0.0;
                order_flow.bid_ask.bid_fade_start_time = None;
            }
            
            if ask_fade_complete && order_flow.bid_ask.ask > 0.0 {
                order_flow.bid_ask.ask = 0.0;
                order_flow.bid_ask.ask_fade_start_time = None;
            }
            
            // 如果这个订单流现在完全为空，标记为待删除
            if order_flow.is_empty() {
                prices_to_remove.push(*price);
            }
        }

        // 限制删除操作的数量，避免一次性删除太多导致卡顿
        let max_delete_count = (max_batch_size / 2).max(5); // 最多删除批量大小的一半，最少5个
        if prices_to_remove.len() > max_delete_count {
            prices_to_remove.truncate(max_delete_count);
        }

        // 删除完全为空的价格层级
        for price in prices_to_remove {
            self.order_flows.remove(&price);
        }

        if bid_fade_started > 0 || ask_fade_started > 0 {
            log::debug!("启动深度数据淡出动画: {}个bid, {}个ask", bid_fade_started, ask_fade_started);
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

    /// 计算高频波动率（Realized Volatility）
    /// 基于5秒窗口的高频价格收益率的方差计算
    fn calculate_realized_volatility(&mut self, timestamp: u64, price: f64) {
        if price <= 0.0 {
            return;
        }

        // 添加价格到历史记录
        self.price_history.push_back((timestamp, price));
        
        // 保持价格历史窗口大小（5秒窗口）
        let window_ms = 5000; // 5秒 = 5000毫秒
        let cutoff_time = timestamp.saturating_sub(window_ms);
        while let Some(&(ts, _)) = self.price_history.front() {
            if ts < cutoff_time {
                self.price_history.pop_front();
            } else {
                break;
            }
        }

        // 计算价格收益率 - 每个新价格都计算收益率
        if self.price_history.len() >= 2 {
            let (_, prev_price) = self.price_history[self.price_history.len() - 2];
            if prev_price > 0.0 {
                // 使用对数收益率，更适合高频数据
                let log_return = (price / prev_price).ln() * 10000.0; // 放大10000倍以便观察
                self.price_returns.push_back(log_return);
                
                // 调试输出
                if self.price_returns.len() % 10 == 0 { // 每10个数据点输出一次
                    log::debug!("RV计算(5s): 价格 {:.2} -> {:.2}, 收益率 {:.4}, 历史长度 {}", 
                               prev_price, price, log_return, self.price_returns.len());
                }
            }
        }

        // 保持收益率序列长度 - 5秒窗口，保持最近500个点
        while self.price_returns.len() > 500 {
            self.price_returns.pop_front();
        }

        // 计算高频波动率（基于收益率的标准差）- 降低最小样本要求
        if self.price_returns.len() >= 5 {
            let mean: f64 = self.price_returns.iter().sum::<f64>() / self.price_returns.len() as f64;
            let variance: f64 = self.price_returns.iter()
                .map(|&r| (r - mean).powi(2))
                .sum::<f64>() / (self.price_returns.len() - 1) as f64; // 使用样本方差
            
            if variance >= 0.0 && !variance.is_nan() && !variance.is_infinite() {
                let new_volatility = variance.sqrt();
                self.realized_volatility = new_volatility;
                
                // 记录RV历史数据用于线型图显示
                self.rv_history.push_back((timestamp, new_volatility));
                
                // 保持RV历史数据长度（最近5分钟的数据，约300个点）
                while self.rv_history.len() > 300 {
                    self.rv_history.pop_front();
                }
                
                // 调试输出
                if self.price_returns.len() % 20 == 0 { // 每20个数据点输出一次
                    log::debug!("RV更新(5s): 波动率 {:.4}, 样本数 {}, 均值 {:.4}, 方差 {:.4}", 
                               new_volatility, self.price_returns.len(), mean, variance);
                }
            }
        }
    }

    /// 计算价格跳跃信号（Jump Detection）
    /// 基于价格变化的Z-score检测异常跳跃
    fn calculate_jump_signal(&mut self, _timestamp: u64, price: f64) {
        if price <= 0.0 || self.price_returns.len() < 30 {
            return;
        }

        // 计算最近的价格变化
        if let Some(&last_return) = self.price_returns.back() {
            // 计算历史收益率的统计特征
            let returns_vec: Vec<f64> = self.price_returns.iter().cloned().collect();
            let mean: f64 = returns_vec.iter().sum::<f64>() / returns_vec.len() as f64;
            let variance: f64 = returns_vec.iter()
                .map(|&r| (r - mean).powi(2))
                .sum::<f64>() / returns_vec.len() as f64;
            
            if variance > 0.0 && !variance.is_nan() && !variance.is_infinite() {
                let std_dev = variance.sqrt();
                
                // 计算Z-score
                let z_score = (last_return - mean) / std_dev;
                
                // 跳跃检测：如果Z-score超过阈值（2.5标准差），认为是跳跃
                let jump_threshold = 2.5;
                if z_score.abs() > jump_threshold {
                    self.jump_signal = z_score.abs();
                } else {
                    // 逐渐衰减跳跃信号
                    self.jump_signal = (self.jump_signal * 0.95).max(0.0);
                }
            }
        }
    }

    /// 计算多空比例 - 基于整个订单簿的所有挡位挂单数据
    fn calculate_volume_ratio(&mut self, _best_bid_qty: f64, _best_ask_qty: f64) {
        // 累计所有挡位的买单和卖单数量
        let mut total_bid_volume = 0.0;
        let mut total_ask_volume = 0.0;
        
        // 遍历所有价格层级，累计所有有效的挂单数据
        for (_, order_flow) in &self.order_flows {
            // 只计算有效的挂单量（大于0的）
            if order_flow.bid_ask.bid > 0.0 {
                total_bid_volume += order_flow.bid_ask.bid;
            }
            if order_flow.bid_ask.ask > 0.0 {
                total_ask_volume += order_flow.bid_ask.ask;
            }
        }
        
        // 计算比例
        let total_volume = total_bid_volume + total_ask_volume;
        if total_volume > 0.0 {
            self.bid_volume_ratio = total_bid_volume / total_volume;
            self.ask_volume_ratio = total_ask_volume / total_volume;
        } else {
            // 如果没有挂单数据，保持默认比例
            self.bid_volume_ratio = 0.5;
            self.ask_volume_ratio = 0.5;
        }
        
        // 调试输出（可选）
        if total_volume > 0.0 {
            log::debug!("OBI计算: 总买单={:.2}, 总卖单={:.2}, 买单比例={:.1}%, 卖单比例={:.1}%", 
                       total_bid_volume, total_ask_volume, 
                       self.bid_volume_ratio * 100.0, self.ask_volume_ratio * 100.0);
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
            realized_volatility: self.realized_volatility,
            jump_signal: self.jump_signal,
        };
        
        self.stats.last_update_time = self.market_snapshot.timestamp;
    }

    /// 获取当前时间戳
    pub fn get_current_timestamp(&self) -> u64 {
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

    /// 获取详细的OBI数据：(买单比例, 卖单比例, 总买单量, 总卖单量, 总挂单量)
    pub fn get_detailed_volume_ratios(&self) -> (f64, f64, f64, f64, f64) {
        let mut total_bid_volume = 0.0;
        let mut total_ask_volume = 0.0;
        
        // 遍历所有价格层级，累计所有有效的挂单数据
        for (_, order_flow) in &self.order_flows {
            if order_flow.bid_ask.bid > 0.0 {
                total_bid_volume += order_flow.bid_ask.bid;
            }
            if order_flow.bid_ask.ask > 0.0 {
                total_ask_volume += order_flow.bid_ask.ask;
            }
        }
        
        let total_volume = total_bid_volume + total_ask_volume;
        let bid_ratio = if total_volume > 0.0 { total_bid_volume / total_volume } else { 0.5 };
        let ask_ratio = if total_volume > 0.0 { total_ask_volume / total_volume } else { 0.5 };
        
        (bid_ratio, ask_ratio, total_bid_volume, total_ask_volume, total_volume)
    }

    /// 获取最近交易高亮信息
    pub fn get_last_trade_highlight(&self) -> (Option<f64>, Option<String>, Option<u64>, Option<f64>) {
        (self.last_trade_price, self.last_trade_side.clone(), self.last_trade_timestamp, self.last_trade_volume)
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

        // 检查并在UTC 0点重置历史累计交易数据
        self.check_and_reset_history_data();

        // 清理5秒内的实时交易数据
        for (_, order_flow) in self.order_flows.iter_mut() {
            order_flow.clean_expired_trades(current_time, 5000); // 5秒
            order_flow.clean_expired_cancels(current_time, 5000); // 5秒
            order_flow.clean_expired_increases(current_time, 5000); // 5秒

            // 新增：清理超过5秒没有更新的挂单数据（1美元精度聚合的深度订单薄数据）
            order_flow.clean_expired_price_levels(current_time, 500); // 500毫秒 = 0.5秒
        }

        // 清理空的或过期的订单流条目
        self.order_flows.retain(|_, order_flow| {
            // 保留有挂单数据或最近有活动的条目
            order_flow.bid_ask.bid > 0.0 ||
            order_flow.bid_ask.ask > 0.0 ||
            order_flow.has_recent_activity(current_time, 60000) // 保留60秒内有活动的
        });
    }

    /// 检查并在UTC 0点重置历史累计交易数据
    pub fn check_and_reset_history_data(&mut self) {
        let current_time = self.get_current_timestamp();
        let current_date = self.get_utc_date_from_timestamp(current_time);

        // 如果日期发生变化（跨越UTC 0点），重置历史数据
        if self.last_history_reset_date != current_date {
            log::info!("检测到UTC日期变化: {} -> {}, 开始重置历史累计交易数据",
                      self.last_history_reset_date, current_date);

            let mut reset_count = 0;
            let mut total_buy_volume = 0.0;
            let mut total_sell_volume = 0.0;

            // 重置所有价格层级的历史累计数据
            for (price, order_flow) in self.order_flows.iter_mut() {
                if order_flow.history_trade_record.buy_volume > 0.0 ||
                   order_flow.history_trade_record.sell_volume > 0.0 {
                    total_buy_volume += order_flow.history_trade_record.buy_volume;
                    total_sell_volume += order_flow.history_trade_record.sell_volume;
                    order_flow.reset_history_trade_record(current_time);
                    reset_count += 1;
                }
            }

            // 更新重置日期
            self.last_history_reset_date = current_date;

            log::info!("历史累计交易数据重置完成: 重置了{}个价格层级, 总买单量: {:.4}, 总卖单量: {:.4}",
                      reset_count, total_buy_volume, total_sell_volume);
        }
    }

    /// 从时间戳获取UTC日期（YYYYMMDD格式）
    fn get_utc_date_from_timestamp(&self, timestamp: u64) -> u32 {
        // 将毫秒时间戳转换为秒
        let timestamp_secs = timestamp / 1000;

        // 计算自1970年1月1日以来的天数
        let days_since_epoch = timestamp_secs / (24 * 60 * 60);

        // 1970年1月1日是星期四，从这里开始计算
        // 简化的日期计算（不考虑闰年的复杂情况，但对于日期变化检测足够准确）
        let mut year = 1970;
        let mut remaining_days = days_since_epoch;

        // 粗略计算年份
        while remaining_days >= 365 {
            let days_in_year = if self.is_leap_year(year) { 366 } else { 365 };
            if remaining_days >= days_in_year {
                remaining_days -= days_in_year;
                year += 1;
            } else {
                break;
            }
        }

        // 计算月份和日期
        let days_in_months = if self.is_leap_year(year) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };

        let mut month = 1;
        for &days_in_month in &days_in_months {
            if remaining_days >= days_in_month {
                remaining_days -= days_in_month;
                month += 1;
            } else {
                break;
            }
        }

        let day = remaining_days + 1;

        // 返回YYYYMMDD格式
        (year * 10000 + month * 100 + day) as u32
    }

    /// 判断是否为闰年
    fn is_leap_year(&self, year: u64) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    /// 基于最近10笔交易更新Trade Imbalance
    fn update_trade_imbalance_from_ticks(&mut self) {
        // 取最近10笔交易
        let recent_trades: Vec<&TickData> = self.tick_pressure_window
            .iter()
            .rev()
            .take(10)
            .collect();

        if recent_trades.is_empty() {
            self.current_trade_imbalance = 0.0;
            return;
        }

        // 计算Trade Imbalance: TI = (#BuyTrades - #SellTrades) / TotalTrades
        let mut buy_count = 0;
        let mut sell_count = 0;

        for trade in &recent_trades {
            if trade.is_buy {
                buy_count += 1;
            } else {
                sell_count += 1;
            }
        }

        let total_trades = buy_count + sell_count;
        if total_trades > 0 {
            self.current_trade_imbalance = (buy_count as f64 - sell_count as f64) / total_trades as f64;
        } else {
            self.current_trade_imbalance = 0.0;
        }
    }

    /// 获取当前Trade Imbalance值
    pub fn get_trade_imbalance(&self) -> f64 {
        self.current_trade_imbalance
    }

    pub fn clear(&mut self) {
        self.order_flows.clear();
        self.best_bid_price = None;
        self.best_ask_price = None;
        self.current_price = None;
        self.bookticker_snapshot = None;
        self.market_snapshot = MarketSnapshot::new();
        self.current_trade_imbalance = 0.0;
        self.tick_pressure_window.clear();
        self.tick_pressure_signals.clear();
    }

    /// 更新ΔTick Pressure检测
    fn update_tick_pressure_detection(&mut self, timestamp: u64, price: f64, volume: f64, is_buy: bool) {
        // 过滤掉无效的volume数据
        if volume <= 0.0 {
            return;
        }

        // 添加新的Tick数据
        let tick_data = TickData {
            timestamp,
            price,
            volume,
            is_buy,
        };
        self.tick_pressure_window.push_back(tick_data);

        // 保持窗口大小，保留足够的数据用于两个模块的检测
        // 需要保留更多数据：ΔTick Pressure需要K*2，Trade Imbalance需要10笔
        let max_needed = (self.tick_pressure_k_value * 2).max(10);
        while self.tick_pressure_window.len() > max_needed {
            self.tick_pressure_window.pop_front();
        }

        // 更新Trade Imbalance（基于最近10笔交易）
        self.update_trade_imbalance_from_ticks();

        // 检测连续K笔成交量方向一致且价位递增/递减
        if self.tick_pressure_window.len() >= self.tick_pressure_k_value {
            self.detect_tick_pressure_signal(timestamp);
        }
    }

    /// 检测ΔTick Pressure信号
    fn detect_tick_pressure_signal(&mut self, timestamp: u64) {
        let window_len = self.tick_pressure_window.len();
        if window_len < self.tick_pressure_k_value {
            return;
        }

        // 检查最近K笔交易
        let recent_ticks: Vec<&TickData> = self.tick_pressure_window
            .iter()
            .rev()
            .take(self.tick_pressure_k_value)
            .collect();

        // 检查方向一致性
        let first_is_buy = recent_ticks[0].is_buy;
        let direction_consistent = recent_ticks.iter().all(|tick| tick.is_buy == first_is_buy);

        if !direction_consistent {
            return;
        }

        // 检查价格递增/递减
        let mut price_trend_consistent = true;
        let mut is_ascending = true;

        if recent_ticks.len() >= 2 {
            // 确定趋势方向（基于前两笔交易）
            is_ascending = recent_ticks[1].price < recent_ticks[0].price;

            // 检查所有交易是否符合趋势
            for i in 1..recent_ticks.len() {
                let current_price = recent_ticks[i - 1].price;
                let prev_price = recent_ticks[i].price;

                if is_ascending && current_price <= prev_price {
                    price_trend_consistent = false;
                    break;
                } else if !is_ascending && current_price >= prev_price {
                    price_trend_consistent = false;
                    break;
                }
            }
        }

        if !price_trend_consistent {
            return;
        }

        // 生成信号
        let direction_str = if is_ascending { "Up" } else { "Down" };
        let side_str = if first_is_buy { "Buy" } else { "Sell" };
        let total_volume: f64 = recent_ticks.iter().map(|tick| tick.volume).sum();

        // 如果总量为0，跳过信号生成
        if total_volume <= 0.0 {
            return;
        }

        let price_start = recent_ticks.last().unwrap().price;
        let price_end = recent_ticks.first().unwrap().price;
        let price_change = ((price_end - price_start) / price_start * 100.0).abs();

        // 判断信号类型
        let signal_type = if total_volume >= 10.0 && price_change >= 0.05 {
            "Ignition Detection"
        } else {
            "Momentum Follow"
        };

        // 格式化时间
        let time_str = self.format_timestamp(timestamp);

        let signal_text = format!(
            "[{}] {} - {} {} {} ticks Price {:.2}->{:.2} Volume {:.4}",
            time_str,
            signal_type,
            side_str,
            direction_str,
            self.tick_pressure_k_value, // 使用K值设置
            price_start,
            price_end,
            total_volume
        );

        // 播放音效
        play_tick_pressure_sound(first_is_buy);

        // 添加到信号窗口
        self.tick_pressure_signals.push_front(signal_text);

        // 维护信号窗口容量
        while self.tick_pressure_signals.len() > self.tick_pressure_signal_capacity {
            self.tick_pressure_signals.pop_back();
        }
    }

    /// 格式化时间戳为可读格式
    fn format_timestamp(&self, timestamp: u64) -> String {
        use std::time::{SystemTime, UNIX_EPOCH, Duration};

        let system_time = UNIX_EPOCH + Duration::from_millis(timestamp);
        let datetime = chrono::DateTime::<chrono::Utc>::from(system_time);
        datetime.format("%H:%M:%S%.3f").to_string()
    }

    /// 获取ΔTick Pressure信号列表
    pub fn get_tick_pressure_signals(&self) -> &std::collections::VecDeque<String> {
        &self.tick_pressure_signals
    }

    /// 设置ΔTick Pressure的K值
    pub fn set_tick_pressure_k_value(&mut self, k: usize) {
        self.tick_pressure_k_value = k.max(3).min(20); // 限制在3-20之间
    }

    /// 获取当前K值设置
    pub fn get_tick_pressure_k_value(&self) -> usize {
        self.tick_pressure_k_value
    }

    /// 设置是否只保留当前更新的深度数据
    pub fn set_keep_only_updated_depth(&mut self, enabled: bool) {
        self.config.keep_only_updated_depth = enabled;
        log::info!("订单簿深度数据保留模式已{}：{}", 
                  if enabled { "启用" } else { "禁用" },
                  if enabled { "只保留当前更新的数据" } else { "保留所有历史数据" });
    }

    /// 获取当前深度数据保留模式
    pub fn get_keep_only_updated_depth(&self) -> bool {
        self.config.keep_only_updated_depth
    }

    /// 设置深度数据清理参数
    pub fn set_depth_cleanup_params(&mut self, interval_ms: u64, batch_size: usize) {
        self.config.depth_cleanup_interval_ms = interval_ms;
        self.config.max_cleanup_batch_size = batch_size;
        log::info!("深度数据清理参数已更新: 间隔{}ms, 批量大小{}", interval_ms, batch_size);
    }

    /// 获取深度数据清理参数
    pub fn get_depth_cleanup_params(&self) -> (u64, usize) {
        (self.config.depth_cleanup_interval_ms, self.config.max_cleanup_batch_size)
    }

    /// 获取RV历史数据用于线型图显示
    pub fn get_rv_history(&self) -> &std::collections::VecDeque<(u64, f64)> {
        &self.rv_history
    }
}

impl Default for OrderBookManager {
    fn default() -> Self {
        Self::new()
    }
}
