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
        // let order_flow = app.get_order_flow(); // 方法不存在，使用模拟数据
        let order_flow: Option<()> = None; // 模拟空数据
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // 创建模拟的订单簿数据用于测试GUI
        self.create_mock_orderbook_data(visible_levels, current_price)
    }
    
    /// 提取原始数据（精度 < 1.0）
    fn extract_raw_data(
        &self,
        _order_flow: Option<()>, // 模拟类型
        _visible_levels: usize,
        _current_price: f64,
        _current_time: u64,
    ) -> Vec<UnifiedOrderBookRow> {
        // 返回模拟数据 - 实际实现需要真实的OrderFlow类型
        Vec::new()
    }
    
    /// 提取聚合数据（精度 >= 1.0）
    fn extract_aggregated_data(
        &self,
        _order_flow: &OrderFlow,
        _visible_levels: usize,
        _current_price: f64,
        _current_time: u64,
    ) -> Vec<UnifiedOrderBookRow> {
        // TODO: 实现真实的聚合逻辑，需要OrderFlow::get_snapshot()和get_all_active_trades_in_window()方法
        // 暂时返回空数据，避免编译错误
        Vec::new()
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
    
    /// 创建模拟订单簿数据用于GUI测试
    fn create_mock_orderbook_data(&self, visible_levels: usize, current_price: f64) -> Vec<UnifiedOrderBookRow> {
        let mut rows = Vec::new();
        let spread = current_price * 0.001; // 0.1% 价差
        
        // 生成买单数据 (绿色，在当前价格下方)
        for i in 0..visible_levels/2 {
            let price = current_price - spread * (i + 1) as f64;
            let size = 100.0 + (i * 20) as f64; // 递增的订单大小
            let cumulative_size = size * (i + 1) as f64;
            
            rows.push(UnifiedOrderBookRow {
                price,
                bid_volume: size,
                ask_volume: 0.0,
                active_buy_volume_5s: size * 0.3, // 30%的主动买单
                active_sell_volume_5s: 0.0,
                history_buy_volume: cumulative_size,
                history_sell_volume: 0.0,
                delta: size * 0.3, // 正delta表示买压
                bid_fade_alpha: 1.0,
                ask_fade_alpha: 0.3,
            });
        }
        
        // 生成卖单数据 (红色，在当前价格上方)
        for i in 0..visible_levels/2 {
            let price = current_price + spread * (i + 1) as f64;
            let size = 80.0 + (i * 15) as f64; // 递增的订单大小
            let cumulative_size = size * (i + 1) as f64;
            
            rows.push(UnifiedOrderBookRow {
                price,
                bid_volume: 0.0,
                ask_volume: size,
                active_buy_volume_5s: 0.0,
                active_sell_volume_5s: size * 0.4, // 40%的主动卖单
                history_buy_volume: 0.0,
                history_sell_volume: cumulative_size,
                delta: -size * 0.4, // 负delta表示卖压
                bid_fade_alpha: 0.3,
                ask_fade_alpha: 1.0,
            });
        }
        
        // 按价格排序（卖单在上，买单在下）
        rows.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));
        
        rows
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