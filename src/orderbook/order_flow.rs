use super::data_structures::*;

/// 订单流数据结构
#[derive(Debug, Clone)]
pub struct OrderFlow {
    pub bid_ask: PriceLevel,
    pub history_trade_record: TradeRecord,
    pub realtime_trade_record: TradeRecord,
    pub realtime_cancel_records: CancelRecord,
    pub realtime_increase_order: IncreaseOrder,
}

impl OrderFlow {
    pub fn new() -> Self {
        Self {
            bid_ask: PriceLevel { 
                bid: 0.0, 
                ask: 0.0, 
                timestamp: 0 
            },
            history_trade_record: TradeRecord { 
                buy_volume: 0.0, 
                sell_volume: 0.0, 
                timestamp: 0 
            },
            realtime_trade_record: TradeRecord { 
                buy_volume: 0.0, 
                sell_volume: 0.0, 
                timestamp: 0 
            },
            realtime_cancel_records: CancelRecord { 
                bid_cancel: 0.0, 
                ask_cancel: 0.0, 
                timestamp: 0 
            },
            realtime_increase_order: IncreaseOrder { 
                bid: 0.0, 
                ask: 0.0, 
                timestamp: 0 
            },
        }
    }

    /// 更新买卖价格和数量
    pub fn update_price_level(&mut self, bid: f64, ask: f64, timestamp: u64) {
        self.bid_ask.bid = bid;
        self.bid_ask.ask = ask;
        self.bid_ask.timestamp = timestamp;
    }

    /// 添加交易记录
    pub fn add_trade(&mut self, side: &str, volume: f64, timestamp: u64) {
        match side {
            "buy" => {
                self.realtime_trade_record.buy_volume += volume;
                self.history_trade_record.buy_volume += volume;
            }
            "sell" => {
                self.realtime_trade_record.sell_volume += volume;
                self.history_trade_record.sell_volume += volume;
            }
            _ => {}
        }
        
        self.realtime_trade_record.timestamp = timestamp;
        self.history_trade_record.timestamp = timestamp;
    }

    /// 添加撤单记录
    pub fn add_cancel(&mut self, side: &str, volume: f64, timestamp: u64) {
        match side {
            "bid" => {
                self.realtime_cancel_records.bid_cancel += volume;
            }
            "ask" => {
                self.realtime_cancel_records.ask_cancel += volume;
            }
            _ => {}
        }
        
        self.realtime_cancel_records.timestamp = timestamp;
    }

    /// 添加增加订单记录
    pub fn add_increase_order(&mut self, side: &str, volume: f64, timestamp: u64) {
        match side {
            "bid" => {
                self.realtime_increase_order.bid += volume;
            }
            "ask" => {
                self.realtime_increase_order.ask += volume;
            }
            _ => {}
        }
        
        self.realtime_increase_order.timestamp = timestamp;
    }

    /// 清理过期的实时交易记录
    pub fn clean_expired_trades(&mut self, current_time: u64, max_age: u64) {
        if current_time.saturating_sub(self.realtime_trade_record.timestamp) > max_age {
            self.realtime_trade_record.buy_volume = 0.0;
            self.realtime_trade_record.sell_volume = 0.0;
        }
    }

    /// 清理过期的撤单记录
    pub fn clean_expired_cancels(&mut self, current_time: u64, max_age: u64) {
        if current_time.saturating_sub(self.realtime_cancel_records.timestamp) > max_age {
            self.realtime_cancel_records.bid_cancel = 0.0;
            self.realtime_cancel_records.ask_cancel = 0.0;
        }
    }

    /// 清理过期的增加订单记录
    pub fn clean_expired_increases(&mut self, current_time: u64, max_age: u64) {
        if current_time.saturating_sub(self.realtime_increase_order.timestamp) > max_age {
            self.realtime_increase_order.bid = 0.0;
            self.realtime_increase_order.ask = 0.0;
        }
    }

    /// 清理超过指定时间没有更新的挂单数据（价格层级数据）
    pub fn clean_expired_price_levels(&mut self, current_time: u64, max_age: u64) {
        // 检查挂单数据的时间戳，如果超过max_age（5秒）没有更新，则清除
        if current_time.saturating_sub(self.bid_ask.timestamp) > max_age {
            // 清除过期的挂单数据
            self.bid_ask.bid = 0.0;
            self.bid_ask.ask = 0.0;
            // 注意：不重置timestamp，保持原有时间戳用于后续判断
        }
    }

    /// 重置历史累计交易数据（每日UTC 0点重置）
    pub fn reset_history_trade_record(&mut self, timestamp: u64) {
        log::info!("重置历史累计交易数据 - 买单: {:.4}, 卖单: {:.4}",
                  self.history_trade_record.buy_volume,
                  self.history_trade_record.sell_volume);

        self.history_trade_record.buy_volume = 0.0;
        self.history_trade_record.sell_volume = 0.0;
        self.history_trade_record.timestamp = timestamp;
    }

    /// 检查是否为空的订单流（没有任何活跃数据）
    pub fn is_empty(&self) -> bool {
        self.bid_ask.bid == 0.0 &&
        self.bid_ask.ask == 0.0 &&
        self.realtime_trade_record.buy_volume == 0.0 &&
        self.realtime_trade_record.sell_volume == 0.0 &&
        self.realtime_cancel_records.bid_cancel == 0.0 &&
        self.realtime_cancel_records.ask_cancel == 0.0 &&
        self.realtime_increase_order.bid == 0.0 &&
        self.realtime_increase_order.ask == 0.0
    }

    /// 获取总交易量
    pub fn total_trade_volume(&self) -> f64 {
        self.realtime_trade_record.buy_volume + self.realtime_trade_record.sell_volume
    }

    /// 获取历史总交易量
    pub fn total_history_volume(&self) -> f64 {
        self.history_trade_record.buy_volume + self.history_trade_record.sell_volume
    }

    /// 获取总撤单量
    pub fn total_cancel_volume(&self) -> f64 {
        self.realtime_cancel_records.bid_cancel + self.realtime_cancel_records.ask_cancel
    }

    /// 获取总增加订单量
    pub fn total_increase_volume(&self) -> f64 {
        self.realtime_increase_order.bid + self.realtime_increase_order.ask
    }

    /// 获取买卖比例
    pub fn trade_ratio(&self) -> (f64, f64) {
        let total = self.total_trade_volume();
        if total > 0.0 {
            (
                self.realtime_trade_record.buy_volume / total,
                self.realtime_trade_record.sell_volume / total
            )
        } else {
            (0.5, 0.5)
        }
    }

    /// 获取订单流活跃度评分
    pub fn activity_score(&self) -> f64 {
        let trade_score = self.total_trade_volume() * 0.4;
        let cancel_score = self.total_cancel_volume() * 0.3;
        let increase_score = self.total_increase_volume() * 0.2;
        let price_score = if self.bid_ask.bid > 0.0 || self.bid_ask.ask > 0.0 { 10.0 } else { 0.0 };

        trade_score + cancel_score + increase_score + price_score
    }

    /// 检查是否有最近的活动或重要的历史数据
    pub fn has_recent_activity(&self, current_time: u64, max_age: u64) -> bool {
        let cutoff_time = current_time.saturating_sub(max_age);

        // 检查各种时间戳是否在时间窗口内
        self.bid_ask.timestamp >= cutoff_time ||
        self.realtime_trade_record.timestamp >= cutoff_time ||
        self.realtime_cancel_records.timestamp >= cutoff_time ||
        self.realtime_increase_order.timestamp >= cutoff_time ||
        self.history_trade_record.timestamp >= cutoff_time ||
        // 或者有非零的挂单量
        self.bid_ask.bid > 0.0 || self.bid_ask.ask > 0.0 ||
        // 或者有历史累计交易数据（重要：防止历史数据丢失）
        self.history_trade_record.buy_volume > 0.0 || self.history_trade_record.sell_volume > 0.0
    }
}

impl Default for OrderFlow {
    fn default() -> Self {
        Self::new()
    }
}
