use std::collections::BTreeMap;
use ordered_float::OrderedFloat;

/// 时间维度足迹图表的数据结构

/// 单个分钟内的价格层级数据
#[derive(Debug, Clone)]
pub struct MinutePriceLevel {
    /// 价格（1美元精度，向上取整）
    pub price: f64,
    /// 主动买单累计量
    pub buy_volume: f64,
    /// 主动卖单累计量
    pub sell_volume: f64,
    /// 最后更新时间戳
    pub last_update: u64,
}

impl MinutePriceLevel {
    pub fn new(price: f64) -> Self {
        Self {
            price,
            buy_volume: 0.0,
            sell_volume: 0.0,
            last_update: 0,
        }
    }

    /// 添加交易数据
    pub fn add_trade(&mut self, side: &str, volume: f64, timestamp: u64) {
        match side {
            "buy" => self.buy_volume += volume,
            "sell" => self.sell_volume += volume,
            _ => {}
        }
        self.last_update = timestamp;
    }

    /// 获取净买卖差额
    pub fn get_delta(&self) -> f64 {
        self.buy_volume - self.sell_volume
    }

    /// 获取总成交量
    pub fn get_total_volume(&self) -> f64 {
        self.buy_volume + self.sell_volume
    }
}

/// 单个分钟的聚合数据
#[derive(Debug, Clone)]
pub struct MinuteData {
    /// 分钟时间戳（分钟开始时间，精确到分钟）
    pub minute_timestamp: u64,
    /// 该分钟内的价格层级数据 (价格 -> 数据)
    pub price_levels: BTreeMap<OrderedFloat<f64>, MinutePriceLevel>,
    /// 该分钟是否已完成（用于优化）
    pub is_complete: bool,
}

impl MinuteData {
    pub fn new(minute_timestamp: u64) -> Self {
        Self {
            minute_timestamp,
            price_levels: BTreeMap::new(),
            is_complete: false,
        }
    }

    /// 添加交易数据到对应的价格层级
    pub fn add_trade(&mut self, price: f64, side: &str, volume: f64, timestamp: u64) {
        // 价格向上取整到1美元精度
        let aggregated_price = price.ceil();
        let price_key = OrderedFloat(aggregated_price);
        
        let price_level = self.price_levels
            .entry(price_key)
            .or_insert_with(|| MinutePriceLevel::new(aggregated_price));
        
        price_level.add_trade(side, volume, timestamp);
    }

    /// 标记该分钟为完成状态
    pub fn mark_complete(&mut self) {
        self.is_complete = true;
    }

    /// 获取该分钟的所有价格层级，按价格排序
    pub fn get_sorted_price_levels(&self) -> Vec<&MinutePriceLevel> {
        self.price_levels
            .values()
            .collect::<Vec<_>>()
    }

    /// 获取该分钟的价格范围
    pub fn get_price_range(&self) -> Option<(f64, f64)> {
        if self.price_levels.is_empty() {
            return None;
        }
        
        let min_price = self.price_levels.keys().next().unwrap().0;
        let max_price = self.price_levels.keys().next_back().unwrap().0;
        Some((min_price, max_price))
    }
}

/// 时间维度足迹图表的主数据管理器
#[derive(Debug, Clone)]
pub struct TimeFootprintData {
    /// 分钟级数据存储 (分钟时间戳 -> 分钟数据)
    pub minute_data: BTreeMap<u64, MinuteData>,
    /// 滑动窗口大小（分钟数）
    pub window_size_minutes: usize,
    /// 当前分钟时间戳
    pub current_minute: u64,
    /// 数据统计
    pub total_trades_processed: u64,
    pub total_minutes: usize,
}

impl TimeFootprintData {
    pub fn new(window_size_minutes: usize) -> Self {
        Self {
            minute_data: BTreeMap::new(),
            window_size_minutes,
            current_minute: 0,
            total_trades_processed: 0,
            total_minutes: 0,
        }
    }

    /// 获取分钟级时间戳（去除秒和毫秒）
    fn get_minute_timestamp(timestamp: u64) -> u64 {
        // 将毫秒时间戳转换为分钟时间戳
        (timestamp / 60000) * 60000
    }

    /// 添加交易数据
    pub fn add_trade(&mut self, price: f64, side: &str, volume: f64, timestamp: u64) {
        let minute_timestamp = Self::get_minute_timestamp(timestamp);
        
        // 检查是否是新的分钟
        if minute_timestamp != self.current_minute {
            // 标记上一分钟为完成状态
            if let Some(prev_minute_data) = self.minute_data.get_mut(&self.current_minute) {
                prev_minute_data.mark_complete();
            }
            
            self.current_minute = minute_timestamp;
            self.total_minutes += 1;
        }

        // 获取或创建当前分钟的数据
        let minute_data = self.minute_data
            .entry(minute_timestamp)
            .or_insert_with(|| MinuteData::new(minute_timestamp));

        // 添加交易数据
        minute_data.add_trade(price, side, volume, timestamp);
        self.total_trades_processed += 1;

        // 维护滑动窗口
        self.maintain_sliding_window();
    }

    /// 维护滑动窗口，移除过旧的数据
    fn maintain_sliding_window(&mut self) {
        if self.minute_data.len() > self.window_size_minutes {
            // 计算需要保留的最早时间戳
            let cutoff_timestamp = self.current_minute - (self.window_size_minutes as u64 * 60000);
            
            // 移除过旧的数据
            let keys_to_remove: Vec<u64> = self.minute_data
                .keys()
                .filter(|&&timestamp| timestamp < cutoff_timestamp)
                .cloned()
                .collect();
            
            for key in keys_to_remove {
                self.minute_data.remove(&key);
            }
        }
    }

    /// 获取指定时间范围内的数据
    pub fn get_data_in_range(&self, start_minute: u64, end_minute: u64) -> Vec<&MinuteData> {
        self.minute_data
            .range(start_minute..=end_minute)
            .map(|(_, data)| data)
            .collect()
    }

    /// 获取最近N分钟的数据
    pub fn get_recent_data(&self, minutes: usize) -> Vec<&MinuteData> {
        let start_timestamp = if self.current_minute >= (minutes as u64 * 60000) {
            self.current_minute - (minutes as u64 * 60000)
        } else {
            0
        };
        
        self.get_data_in_range(start_timestamp, self.current_minute)
    }

    /// 获取所有数据，按时间排序
    pub fn get_all_data_sorted(&self) -> Vec<&MinuteData> {
        self.minute_data
            .values()
            .collect()
    }

    /// 获取价格范围（所有数据中的最小和最大价格）
    pub fn get_overall_price_range(&self) -> Option<(f64, f64)> {
        let mut min_price = f64::MAX;
        let mut max_price = f64::MIN;
        let mut found_data = false;

        for minute_data in self.minute_data.values() {
            if let Some((min, max)) = minute_data.get_price_range() {
                min_price = min_price.min(min);
                max_price = max_price.max(max);
                found_data = true;
            }
        }

        if found_data {
            Some((min_price, max_price))
        } else {
            None
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> (usize, u64, usize) {
        (
            self.minute_data.len(),
            self.total_trades_processed,
            self.total_minutes,
        )
    }
}

/// 用于图表渲染的数据点
#[derive(Debug, Clone)]
pub struct ChartDataPoint {
    /// X轴：分钟时间戳
    pub minute_timestamp: u64,
    /// Y轴：价格
    pub price: f64,
    /// 买单量
    pub buy_volume: f64,
    /// 卖单量
    pub sell_volume: f64,
    /// 净差额
    pub delta: f64,
}

impl ChartDataPoint {
    pub fn from_minute_price_level(minute_timestamp: u64, price_level: &MinutePriceLevel) -> Self {
        Self {
            minute_timestamp,
            price: price_level.price,
            buy_volume: price_level.buy_volume,
            sell_volume: price_level.sell_volume,
            delta: price_level.get_delta(),
        }
    }
}
