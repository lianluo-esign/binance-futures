use ordered_float::OrderedFloat;
use std::collections::{BTreeMap, HashMap};

/// 价格级别数据结构
#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub bid: f64,
    pub ask: f64,
    pub timestamp: u64,
}

/// 交易记录数据结构
#[derive(Debug, Clone)]
pub struct TradeRecord {
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub timestamp: u64,
}

/// 撤单记录数据结构
#[derive(Debug, Clone)]
pub struct CancelRecord {
    pub bid_cancel: f64,
    pub ask_cancel: f64,
    pub timestamp: u64,
}

/// 增加订单数据结构
#[derive(Debug, Clone)]
pub struct IncreaseOrder {
    pub bid: f64,
    pub ask: f64,
    pub timestamp: u64,
}

/// 不平衡信号数据结构
#[derive(Debug, Clone)]
pub struct ImbalanceSignal {
    pub timestamp: u64,
    pub signal_type: String,
    pub ratio: f64,
}

/// 大订单数据结构
#[derive(Debug, Clone)]
pub struct BigOrder {
    pub order_type: String,
    pub volume: f64,
    pub timestamp: u64,
}

/// 订单冲击信号数据结构
#[derive(Debug, Clone)]
pub struct OrderImpactSignal {
    pub timestamp: u64,
    pub direction: String,  // "buy" 或 "sell"
    pub trade_price: f64,
    pub trade_quantity: f64,
    pub best_price: f64,    // 对应的最优买价或卖价
    pub best_quantity: f64, // 对应的最优买量或卖量
    pub impact_ratio: f64,  // 冲击比率
    pub description: String,
}

/// BookTicker快照数据结构
#[derive(Debug, Clone)]
pub struct BookTickerSnapshot {
    pub best_bid_price: f64,
    pub best_ask_price: f64,
    pub best_bid_qty: f64,
    pub best_ask_qty: f64,
    pub timestamp: u64,
}

/// 订单簿配置
#[derive(Debug, Clone)]
pub struct OrderBookConfig {
    pub trade_display_duration: u64,
    pub cancel_display_duration: u64,
    pub max_trade_records: usize,
    pub max_cancel_records: usize,
    pub highlight_duration: u64,
    pub buffer_window_ms: u64,
    pub signal_threshold: f64,
    pub speed_window_ms: u64,
    pub avg_speed_window_ms: u64,
    pub volatility_window_ms: u64,
    pub tick_price_diff_window_size: usize,
}

impl Default for OrderBookConfig {
    fn default() -> Self {
        Self {
            trade_display_duration: 3000,
            cancel_display_duration: 5000,
            max_trade_records: 1000,
            max_cancel_records: 500,
            highlight_duration: 3000,
            buffer_window_ms: 500,
            signal_threshold: 0.75,
            speed_window_ms: 100,
            avg_speed_window_ms: 5000,
            volatility_window_ms: 60000,
            tick_price_diff_window_size: 10,
        }
    }
}

/// 订单簿统计信息
#[derive(Debug, Clone, Default)]
pub struct OrderBookStats {
    pub total_depth_updates: u64,
    pub total_trades: u64,
    pub total_book_ticker_updates: u64,
    pub total_signals_generated: u64,
    pub current_latency: i64,
    pub avg_processing_time: f64,
    pub last_update_time: u64,
}

/// 市场数据快照
#[derive(Debug, Clone)]
pub struct MarketSnapshot {
    pub timestamp: u64,
    pub best_bid_price: Option<f64>,
    pub best_ask_price: Option<f64>,
    pub current_price: Option<f64>,
    pub bid_volume_ratio: f64,
    pub ask_volume_ratio: f64,
    pub price_speed: f64,
    pub avg_speed: f64,
    pub volatility: f64,
    pub tick_price_diff_volatility: f64,
}

impl MarketSnapshot {
    pub fn new() -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            timestamp,
            best_bid_price: None,
            best_ask_price: None,
            current_price: None,
            bid_volume_ratio: 0.5,
            ask_volume_ratio: 0.5,
            price_speed: 0.0,
            avg_speed: 0.0,
            volatility: 0.0,
            tick_price_diff_volatility: 0.0,
        }
    }

    pub fn spread(&self) -> Option<f64> {
        if let (Some(bid), Some(ask)) = (self.best_bid_price, self.best_ask_price) {
            Some(ask - bid)
        } else {
            None
        }
    }

    pub fn spread_percentage(&self) -> Option<f64> {
        if let (Some(bid), Some(ask)) = (self.best_bid_price, self.best_ask_price) {
            Some((ask - bid) / bid * 100.0)
        } else {
            None
        }
    }

    pub fn mid_price(&self) -> Option<f64> {
        if let (Some(bid), Some(ask)) = (self.best_bid_price, self.best_ask_price) {
            Some((bid + ask) / 2.0)
        } else {
            None
        }
    }
}
