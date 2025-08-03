/// 订单簿工具函数模块

use std::collections::BTreeMap;
use ordered_float::OrderedFloat;
use crate::orderbook::OrderFlow;
use crate::app::ReactiveApp;
use super::types::{UnifiedOrderBookRow, AggregatedOrderFlow, SmartScrollInfo};

/// 滚动位置计算器
pub struct ScrollCalculator {
    /// 自动跟踪价格
    auto_track_price: bool,
    /// 可见行数
    visible_rows: usize,
    /// 最后计算的滚动位置
    last_scroll_position: f32,
}

impl ScrollCalculator {
    pub fn new(auto_track_price: bool, visible_rows: usize) -> Self {
        Self {
            auto_track_price,
            visible_rows,
            last_scroll_position: 0.0,
        }
    }
    
    /// 计算智能滚动位置
    pub fn calculate_smart_scroll_position(
        &mut self,
        data: &[UnifiedOrderBookRow],
        current_price: f64,
    ) -> SmartScrollInfo {
        let mut scroll_info = SmartScrollInfo::new(self.visible_rows);
        
        if !self.auto_track_price || data.is_empty() {
            scroll_info.scroll_offset = self.last_scroll_position;
            return scroll_info;
        }
        
        // 查找当前价格在数据中的位置
        let current_price_index = self.find_current_price_index(data, current_price);
        scroll_info.set_current_price_index(current_price_index);
        
        if let Some(price_index) = current_price_index {
            // 计算目标滚动位置，使当前价格居中
            let target_scroll = if price_index >= self.visible_rows / 2 {
                price_index - self.visible_rows / 2
            } else {
                0
            };
            
            // 平滑滚动
            let target_scroll_f32 = target_scroll as f32;
            let scroll_speed = 0.1; // 滚动速度
            let new_scroll = self.last_scroll_position + 
                (target_scroll_f32 - self.last_scroll_position) * scroll_speed;
            
            scroll_info.update_scroll_position(new_scroll);
            self.last_scroll_position = new_scroll;
            scroll_info.target_row = target_scroll;
        } else {
            scroll_info.scroll_offset = self.last_scroll_position;
        }
        
        scroll_info
    }
    
    /// 查找当前价格在数据中的索引
    fn find_current_price_index(&self, data: &[UnifiedOrderBookRow], current_price: f64) -> Option<usize> {
        if current_price <= 0.0 {
            return None;
        }
        
        // 二分查找最接近的价格
        let mut left = 0;
        let mut right = data.len();
        
        while left < right {
            let mid = (left + right) / 2;
            if data[mid].price < current_price {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        
        // 找到最接近的价格索引
        if left < data.len() {
            Some(left)
        } else if left > 0 {
            Some(left - 1)
        } else {
            None
        }
    }
    
    /// 检查是否为当前价格行
    pub fn is_current_price_row(&self, row_price: f64, current_price: f64) -> bool {
        if current_price <= 0.0 {
            return false;
        }
        
        let price_tolerance = 0.01; // 价格容差
        (row_price - current_price).abs() < price_tolerance
    }
    
    /// 计算中心行索引
    pub fn calculate_center_row_index(&self, data: &[UnifiedOrderBookRow], current_price: f64) -> usize {
        if data.is_empty() || current_price <= 0.0 {
            return 0;
        }
        
        // 找到最接近当前价格的行
        let mut best_index = 0;
        let mut min_diff = f64::MAX;
        
        for (i, row) in data.iter().enumerate() {
            let diff = (row.price - current_price).abs();
            if diff < min_diff {
                min_diff = diff;
                best_index = i;
            }
        }
        
        best_index
    }
    
    pub fn set_auto_track(&mut self, enabled: bool) {
        self.auto_track_price = enabled;
    }
    
    pub fn is_auto_tracking(&self) -> bool {
        self.auto_track_price
    }
}

/// 数据提取器
pub struct DataExtractor {
    /// 价格精度
    price_precision: f64,
    /// 时间窗口（秒）
    time_window_seconds: u64,
}

impl DataExtractor {
    pub fn new(price_precision: f64, time_window_seconds: u64) -> Self {
        Self {
            price_precision,
            time_window_seconds,
        }
    }
    
    /// 提取可见数据
    pub fn extract_visible_data(
        &self,
        app: &ReactiveApp,
        visible_levels: usize,
        current_price: f64,
    ) -> Vec<UnifiedOrderBookRow> {
        let order_flow = app.get_order_flow();
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // 根据价格精度决定聚合方式
        if self.price_precision >= 1.0 {
            self.extract_aggregated_data(order_flow, visible_levels, current_price, current_time)
        } else {
            self.extract_raw_data(order_flow, visible_levels, current_price, current_time)
        }
    }
    
    /// 提取原始数据（精度 < 1.0）
    fn extract_raw_data(
        &self,
        order_flow: &OrderFlow,
        visible_levels: usize,
        current_price: f64,
        current_time: u64,
    ) -> Vec<UnifiedOrderBookRow> {
        let mut visible_data = Vec::new();
        
        // 获取订单簿快照
        let snapshot = order_flow.get_snapshot();
        
        // 合并买单和卖单价格
        let mut all_prices: std::collections::BTreeSet<OrderedFloat<f64>> = 
            std::collections::BTreeSet::new();
        
        for (price, _) in &snapshot.bids {
            all_prices.insert(OrderedFloat(*price));
        }
        for (price, _) in &snapshot.asks {
            all_prices.insert(OrderedFloat(*price));
        }
        
        // 限制数据范围到可见层级
        let total_levels = visible_levels * 2 + 1; // 上下各visible_levels + 中心价格
        let prices: Vec<f64> = all_prices.into_iter()
            .map(|p| p.into_inner())
            .take(total_levels)
            .collect();
        
        for price in prices {
            let mut row = UnifiedOrderBookRow::new(price);
            
            // 设置买单和卖单深度
            if let Some(bid_amount) = snapshot.bids.get(&price) {
                row.bid_volume = *bid_amount;
            }
            if let Some(ask_amount) = snapshot.asks.get(&price) {
                row.ask_volume = *ask_amount;
            }
            
            // 计算5秒内的主动交易量
            let active_trades = order_flow.get_active_trades_in_window(
                price, 
                current_time.saturating_sub(self.time_window_seconds),
                current_time
            );
            
            row.active_buy_volume_5s = active_trades.buy_volume;
            row.active_sell_volume_5s = active_trades.sell_volume;
            row.delta = active_trades.buy_volume - active_trades.sell_volume;
            
            // 历史累计量
            let historical_trades = order_flow.get_historical_trades(price);
            row.history_buy_volume = historical_trades.total_buy_volume;
            row.history_sell_volume = historical_trades.total_sell_volume;
            
            visible_data.push(row);
        }
        
        // 按价格排序（降序）
        visible_data.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));
        
        visible_data
    }
    
    /// 提取聚合数据（精度 >= 1.0）
    fn extract_aggregated_data(
        &self,
        order_flow: &OrderFlow,
        visible_levels: usize,
        current_price: f64,
        current_time: u64,
    ) -> Vec<UnifiedOrderBookRow> {
        let mut aggregated_map: BTreeMap<i64, AggregatedOrderFlow> = BTreeMap::new();
        let snapshot = order_flow.get_snapshot();
        
        // 聚合买单数据
        for (price, amount) in &snapshot.bids {
            let bucket = self.price_to_bucket(*price);
            let entry = aggregated_map.entry(bucket).or_insert_with(AggregatedOrderFlow::new);
            entry.bid_volume += amount;
        }
        
        // 聚合卖单数据
        for (price, amount) in &snapshot.asks {
            let bucket = self.price_to_bucket(*price);
            let entry = aggregated_map.entry(bucket).or_insert_with(AggregatedOrderFlow::new);
            entry.ask_volume += amount;
        }
        
        // 聚合主动交易数据
        let active_trades = order_flow.get_all_active_trades_in_window(
            current_time.saturating_sub(self.time_window_seconds),
            current_time
        );
        
        for trade in active_trades {
            let bucket = self.price_to_bucket(trade.price);
            if let Some(entry) = aggregated_map.get_mut(&bucket) {
                if trade.is_buy {
                    entry.active_buy_volume_5s += trade.volume;
                } else {
                    entry.active_sell_volume_5s += trade.volume;
                }
            }
        }
        
        // 转换为UnifiedOrderBookRow
        let mut visible_data = Vec::new();
        for (bucket, flow) in aggregated_map {
            let price = self.bucket_to_price(bucket);
            let mut row = UnifiedOrderBookRow::new(price);
            
            row.bid_volume = flow.bid_volume;
            row.ask_volume = flow.ask_volume;
            row.active_buy_volume_5s = flow.active_buy_volume_5s;
            row.active_sell_volume_5s = flow.active_sell_volume_5s;
            row.history_buy_volume = flow.history_buy_volume;
            row.history_sell_volume = flow.history_sell_volume;
            row.delta = flow.active_buy_volume_5s - flow.active_sell_volume_5s;
            row.bid_fade_alpha = flow.bid_fade_alpha;
            row.ask_fade_alpha = flow.ask_fade_alpha;
            
            visible_data.push(row);
        }
        
        // 限制到可见层级并排序
        visible_data.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));
        visible_data.truncate(visible_levels * 2);
        
        visible_data
    }
    
    /// 将价格转换为桶索引（用于聚合）
    fn price_to_bucket(&self, price: f64) -> i64 {
        (price / self.price_precision).floor() as i64
    }
    
    /// 将桶索引转换回价格
    fn bucket_to_price(&self, bucket: i64) -> f64 {
        bucket as f64 * self.price_precision
    }
    
    pub fn set_price_precision(&mut self, precision: f64) {
        self.price_precision = precision;
    }
    
    pub fn set_time_window(&mut self, seconds: u64) {
        self.time_window_seconds = seconds;
    }
}

/// 价格验证工具
pub struct PriceValidator;

impl PriceValidator {
    /// 验证价格是否有效
    pub fn is_valid_price(price: f64) -> bool {
        price > 0.0 && price.is_finite()
    }
    
    /// 验证价格范围
    pub fn is_price_in_range(price: f64, min_price: f64, max_price: f64) -> bool {
        Self::is_valid_price(price) && price >= min_price && price <= max_price
    }
    
    /// 标准化价格精度
    pub fn normalize_price(price: f64, precision: f64) -> f64 {
        if precision <= 0.0 {
            return price;
        }
        (price / precision).round() * precision
    }
    
    /// 计算价格差异百分比
    pub fn calculate_price_change_percent(old_price: f64, new_price: f64) -> f64 {
        if old_price <= 0.0 {
            return 0.0;
        }
        ((new_price - old_price) / old_price) * 100.0
    }
}

/// 性能统计工具
pub struct PerformanceTracker {
    render_times: std::collections::VecDeque<std::time::Duration>,
    max_samples: usize,
}

impl PerformanceTracker {
    pub fn new(max_samples: usize) -> Self {
        Self {
            render_times: std::collections::VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }
    
    pub fn record_render_time(&mut self, duration: std::time::Duration) {
        self.render_times.push_back(duration);
        if self.render_times.len() > self.max_samples {
            self.render_times.pop_front();
        }
    }
    
    pub fn average_render_time(&self) -> std::time::Duration {
        if self.render_times.is_empty() {
            return std::time::Duration::ZERO;
        }
        
        let total: std::time::Duration = self.render_times.iter().sum();
        total / self.render_times.len() as u32
    }
    
    pub fn max_render_time(&self) -> std::time::Duration {
        self.render_times.iter().max().copied().unwrap_or(std::time::Duration::ZERO)
    }
    
    pub fn min_render_time(&self) -> std::time::Duration {
        self.render_times.iter().min().copied().unwrap_or(std::time::Duration::ZERO)
    }
    
    pub fn fps(&self) -> f32 {
        let avg = self.average_render_time();
        if avg.is_zero() {
            return 0.0;
        }
        1.0 / avg.as_secs_f32()
    }
}