/// 聚合深度数据管理器
/// 
/// 使用BTreeMap按价格排序保存深度数据，价格精度为1美元
/// 实现高效的深度数据聚合和缓存，直接为GUI提供排序后的数据

use std::collections::BTreeMap;
use crate::events::{Event, EventType};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

/// 聚合价格级别数据（按1美元精度聚合）
#[derive(Debug, Clone)]
pub struct AggregatedPriceLevel {
    /// 价格（整数美元）
    pub price: i64,
    /// 买单总量
    pub bid_volume: f64,
    /// 卖单总量
    pub ask_volume: f64,
    /// 最后更新时间
    pub last_update: u64,
    /// 买单数量
    pub bid_count: u32,
    /// 卖单数量
    pub ask_count: u32,
}

impl AggregatedPriceLevel {
    pub fn new(price: i64) -> Self {
        Self {
            price,
            bid_volume: 0.0,
            ask_volume: 0.0,
            last_update: 0,
            bid_count: 0,
            ask_count: 0,
        }
    }

    /// 更新买单数据
    pub fn update_bid(&mut self, volume: f64, timestamp: u64) {
        if volume > 0.0 {
            self.bid_volume = volume;
            self.bid_count = 1; // 简化处理，实际应该累加订单数
        } else {
            self.bid_volume = 0.0;
            self.bid_count = 0;
        }
        self.last_update = timestamp;
    }

    /// 更新卖单数据
    pub fn update_ask(&mut self, volume: f64, timestamp: u64) {
        if volume > 0.0 {
            self.ask_volume = volume;
            self.ask_count = 1; // 简化处理，实际应该累加订单数
        } else {
            self.ask_volume = 0.0;
            self.ask_count = 0;
        }
        self.last_update = timestamp;
    }

    /// 检查价格级别是否为空
    pub fn is_empty(&self) -> bool {
        self.bid_volume == 0.0 && self.ask_volume == 0.0
    }

    /// 获取总成交量
    pub fn total_volume(&self) -> f64 {
        self.bid_volume + self.ask_volume
    }
}

/// 聚合深度数据管理器
pub struct AggregatedDepthManager {
    /// 价格级别数据，按价格排序（BTreeMap自动排序）
    price_levels: BTreeMap<i64, AggregatedPriceLevel>,
    /// 最佳买价
    best_bid: Option<i64>,
    /// 最佳卖价
    best_ask: Option<i64>,
    /// 当前交易对
    symbol: String,
    /// 最后更新时间
    last_update: u64,
    /// 价格精度（以美元为单位）
    price_precision: f64,
}

impl AggregatedDepthManager {
    pub fn new(symbol: String) -> Self {
        Self {
            price_levels: BTreeMap::new(),
            best_bid: None,
            best_ask: None,
            symbol,
            last_update: 0,
            price_precision: 1.0, // 1美元精度
        }
    }

    /// 将浮点价格转换为整数价格（按1美元精度）
    fn price_to_level(&self, price: f64) -> i64 {
        (price / self.price_precision).round() as i64
    }

    /// 将整数价格级别转换回浮点价格
    fn level_to_price(&self, level: i64) -> f64 {
        level as f64 * self.price_precision
    }

    /// 处理深度更新事件
    pub fn handle_depth_update(&mut self, event: &Event) -> bool {
        if let EventType::DepthUpdate(data) = &event.event_type {
            self.process_depth_data(data, event.timestamp)
        } else {
            false
        }
    }

    /// 处理BookTicker事件
    pub fn handle_book_ticker(&mut self, event: &Event) -> bool {
        if let EventType::BookTicker(data) = &event.event_type {
            self.process_book_ticker_data(data, event.timestamp)
        } else {
            false
        }
    }

    /// 处理深度数据
    fn process_depth_data(&mut self, data: &Value, timestamp: u64) -> bool {
        // 解析binance深度数据格式
        // {
        //   "e": "depthUpdate",
        //   "E": 1672515782136,
        //   "s": "BTCUSDT",
        //   "U": 400900217,
        //   "u": 400900218,
        //   "b": [["43638.46000000", "7.47400000"]],
        //   "a": [["43638.47000000", "8.11700000"]]
        // }

        let mut updated = false;

        // 处理买单数据
        if let Some(bids) = data["b"].as_array() {
            for bid in bids {
                if let Some(bid_array) = bid.as_array() {
                    if bid_array.len() >= 2 {
                        if let (Some(price_str), Some(volume_str)) = 
                            (bid_array[0].as_str(), bid_array[1].as_str()) {
                            if let (Ok(price), Ok(volume)) = 
                                (price_str.parse::<f64>(), volume_str.parse::<f64>()) {
                                self.update_bid_level(price, volume, timestamp);
                                updated = true;
                            }
                        }
                    }
                }
            }
        }

        // 处理卖单数据
        if let Some(asks) = data["a"].as_array() {
            for ask in asks {
                if let Some(ask_array) = ask.as_array() {
                    if ask_array.len() >= 2 {
                        if let (Some(price_str), Some(volume_str)) = 
                            (ask_array[0].as_str(), ask_array[1].as_str()) {
                            if let (Ok(price), Ok(volume)) = 
                                (price_str.parse::<f64>(), volume_str.parse::<f64>()) {
                                self.update_ask_level(price, volume, timestamp);
                                updated = true;
                            }
                        }
                    }
                }
            }
        }

        if updated {
            self.last_update = timestamp;
            self.update_best_prices();
        }

        updated
    }

    /// 处理BookTicker数据
    fn process_book_ticker_data(&mut self, data: &Value, timestamp: u64) -> bool {
        // {
        //   "u":400900218,
        //   "s":"BTCUSDT",
        //   "b":"43638.46000000",
        //   "B":"7.47400000",
        //   "a":"43638.47000000",
        //   "A":"8.11700000"
        // }

        let mut updated = false;

        // 处理最佳买价
        if let (Some(bid_price_str), Some(bid_volume_str)) = 
            (data["b"].as_str(), data["B"].as_str()) {
            if let (Ok(price), Ok(volume)) = 
                (bid_price_str.parse::<f64>(), bid_volume_str.parse::<f64>()) {
                self.update_bid_level(price, volume, timestamp);
                updated = true;
            }
        }

        // 处理最佳卖价
        if let (Some(ask_price_str), Some(ask_volume_str)) = 
            (data["a"].as_str(), data["A"].as_str()) {
            if let (Ok(price), Ok(volume)) = 
                (ask_price_str.parse::<f64>(), ask_volume_str.parse::<f64>()) {
                self.update_ask_level(price, volume, timestamp);
                updated = true;
            }
        }

        if updated {
            self.last_update = timestamp;
            self.update_best_prices();
        }

        updated
    }

    /// 更新买单价格级别
    fn update_bid_level(&mut self, price: f64, volume: f64, timestamp: u64) {
        let level = self.price_to_level(price);
        let price_level = self.price_levels.entry(level).or_insert_with(|| AggregatedPriceLevel::new(level));
        price_level.update_bid(volume, timestamp);

        // 如果成交量为0，则删除该价格级别
        if price_level.is_empty() {
            self.price_levels.remove(&level);
        }
    }

    /// 更新卖单价格级别
    fn update_ask_level(&mut self, price: f64, volume: f64, timestamp: u64) {
        let level = self.price_to_level(price);
        let price_level = self.price_levels.entry(level).or_insert_with(|| AggregatedPriceLevel::new(level));
        price_level.update_ask(volume, timestamp);

        // 如果成交量为0，则删除该价格级别
        if price_level.is_empty() {
            self.price_levels.remove(&level);
        }
    }

    /// 更新最佳买卖价
    fn update_best_prices(&mut self) {
        // 找到最佳买价（最高的有买单的价格）
        self.best_bid = self.price_levels
            .iter()
            .rev() // 从高到低遍历
            .find(|(_, level)| level.bid_volume > 0.0)
            .map(|(price, _)| *price);

        // 找到最佳卖价（最低的有卖单的价格）
        self.best_ask = self.price_levels
            .iter() // 从低到高遍历
            .find(|(_, level)| level.ask_volume > 0.0)
            .map(|(price, _)| *price);
    }

    /// 获取按价格排序的可见深度数据（用于GUI显示）
    /// 返回格式：卖单在上（价格从高到低），买单在下（价格从高到低）
    pub fn get_visible_depth(&self, max_levels: usize) -> Vec<AggregatedPriceLevel> {
        let mut result = Vec::new();

        // 获取最佳买卖价作为参考点
        let best_bid = self.best_bid.unwrap_or(50000); // 默认50000
        let best_ask = self.best_ask.unwrap_or(50001); // 默认50001

        // 计算显示范围
        let mid_price = (best_bid + best_ask) / 2;
        let half_levels = max_levels / 2;

        // 获取显示范围内的价格级别（从高到低）
        let start_level = mid_price + half_levels as i64;
        let end_level = mid_price - half_levels as i64;

        for level in (end_level..=start_level).rev() {
            if let Some(price_level) = self.price_levels.get(&level) {
                result.push(price_level.clone());
            } else {
                // 创建空的价格级别用于显示
                result.push(AggregatedPriceLevel::new(level));
            }
        }

        result
    }

    /// 获取最佳买价
    pub fn get_best_bid(&self) -> Option<f64> {
        self.best_bid.map(|level| self.level_to_price(level))
    }

    /// 获取最佳卖价
    pub fn get_best_ask(&self) -> Option<f64> {
        self.best_ask.map(|level| self.level_to_price(level))
    }

    /// 获取价差
    pub fn get_spread(&self) -> Option<f64> {
        match (self.get_best_bid(), self.get_best_ask()) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }

    /// 获取总价格级别数量
    pub fn get_total_levels(&self) -> usize {
        self.price_levels.len()
    }

    /// 清理过期的价格级别
    pub fn cleanup_expired_levels(&mut self, max_age_ms: u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.price_levels.retain(|_, level| {
            now.saturating_sub(level.last_update) <= max_age_ms
        });

        // 重新计算最佳价格
        self.update_best_prices();
    }

    /// 获取最后更新时间
    pub fn get_last_update(&self) -> u64 {
        self.last_update
    }

    /// 获取当前交易对
    pub fn get_symbol(&self) -> &str {
        &self.symbol
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{Event, EventType};
    use serde_json::json;

    #[test]
    fn test_aggregated_depth_manager() {
        let mut manager = AggregatedDepthManager::new("BTCUSDT".to_string());

        // 创建测试深度更新事件
        let depth_data = json!({
            "e": "depthUpdate",
            "s": "BTCUSDT",
            "b": [["50000.50", "1.5"], ["49999.75", "2.0"]],
            "a": [["50001.25", "1.8"], ["50002.00", "2.5"]]
        });

        let event = Event::new(
            EventType::DepthUpdate(depth_data),
            "test".to_string()
        );

        // 处理事件
        assert!(manager.handle_depth_update(&event));

        // 检查最佳价格
        assert_eq!(manager.get_best_bid(), Some(50000.0)); // 聚合到50000
        assert_eq!(manager.get_best_ask(), Some(50001.0)); // 聚合到50001
        assert_eq!(manager.get_spread(), Some(1.0));

        // 获取可见深度
        let visible_depth = manager.get_visible_depth(10);
        assert!(!visible_depth.is_empty());
    }
}