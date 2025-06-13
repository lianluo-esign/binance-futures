use serde_json::Value;
use ordered_float::OrderedFloat;
use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub ask: f64,
    pub bid: f64,
}

#[derive(Debug, Clone)]
pub struct TradeRecord {
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct CancelRecord {
    pub bid_cancel: f64,
    pub ask_cancel: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct OrderFlow {
    pub bid_ask: PriceLevel,
    pub history_trade_record: TradeRecord,
    pub realtime_trade_record: TradeRecord,
    pub realtime_cancel_records: CancelRecord,
}

impl OrderFlow {
    pub fn new() -> Self {
        Self {
            bid_ask: PriceLevel { bid: 0.0, ask: 0.0 },
            history_trade_record: TradeRecord { buy_volume: 0.0, sell_volume: 0.0, timestamp: 0 },
            realtime_trade_record: TradeRecord { buy_volume: 0.0, sell_volume: 0.0, timestamp: 0 },
            realtime_cancel_records: CancelRecord { bid_cancel: 0.0, ask_cancel: 0.0, timestamp: 0 },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImbalanceSignal {
    pub timestamp: u64,
    pub signal_type: String,
    pub ratio: f64,
}

#[derive(Debug, Clone)]
pub struct BigOrder {
    pub order_type: String,
    pub volume: f64,
    pub timestamp: u64,
}

pub struct OrderBookData {
    pub order_flows: BTreeMap<OrderedFloat<f64>, OrderFlow>,
    pub current_price: Option<f64>,
    pub last_trade_side: Option<String>,
    pub trade_display_duration: u64,
    pub cancel_display_duration: u64,
    pub max_trade_records: usize,
    pub max_cancel_records: usize,
    pub stable_highlight_price: Option<f64>,
    pub stable_highlight_side: Option<String>,
    pub last_trade_price: Option<f64>,
    pub highlight_start_time: Option<u64>,
    pub highlight_duration: u64,
    pub last_update_id: Option<u64>,
    pub best_bid_price: Option<f64>,
    pub best_ask_price: Option<f64>,
    pub prices_to_clear_buffer: Vec<(OrderedFloat<f64>, String)>,
    pub cancellations_buffer: Vec<(f64, String, f64)>,
    pub bid_volume_ratio: f64,
    pub ask_volume_ratio: f64,
    pub imbalance_signals: Vec<ImbalanceSignal>,
    pub last_imbalance_check: u64,
    pub continuous_imbalance_start: Option<u64>,
    pub current_imbalance_type: Option<String>,
    pub cancel_signals: Vec<ImbalanceSignal>,
    pub last_cancel_check: u64,
    pub iceberg_signals: Vec<ImbalanceSignal>,
    pub big_orders: HashMap<OrderedFloat<f64>, BigOrder>,
    pub last_big_order_check: u64,
    pub active_trades_buffer: HashMap<OrderedFloat<f64>, (f64, f64)>,
}

impl OrderBookData {
    pub fn new() -> Self {
        Self {
            order_flows: BTreeMap::new(),
            current_price: None,
            last_trade_side: None,
            trade_display_duration: 10000,
            cancel_display_duration: 5000,
            max_trade_records: 1000,
            max_cancel_records: 500,
            stable_highlight_price: None,
            stable_highlight_side: None,
            last_trade_price: None,
            highlight_start_time: None,
            highlight_duration: 3000,
            last_update_id: None,
            best_bid_price: None,
            best_ask_price: None,
            prices_to_clear_buffer: Vec::with_capacity(100),
            cancellations_buffer: Vec::with_capacity(100),
            bid_volume_ratio: 0.5,
            ask_volume_ratio: 0.5,
            imbalance_signals: Vec::new(),
            last_imbalance_check: 0,
            continuous_imbalance_start: None,
            current_imbalance_type: None,
            cancel_signals: Vec::new(),
            last_cancel_check: 0,
            active_trades_buffer: HashMap::new(),
            iceberg_signals: Vec::new(),
            big_orders: HashMap::new(),
            last_big_order_check: 0,
        }
    }

    pub fn add_trade(&mut self, data: &Value) {
        if let (Some(price_str), Some(qty), Some(is_buyer_maker)) = (
            data["p"].as_str(),
            data["q"].as_str(),
            data["m"].as_bool(),
        ) {
            let price = price_str.parse::<f64>().unwrap_or(0.0);
            let price_ordered = OrderedFloat(price);
            let qty_f64 = qty.parse::<f64>().unwrap_or(0.0);
            let side = if is_buyer_maker { "sell" } else { "buy" };
            
            self.last_trade_side = Some(side.to_string());
            self.update_current_price(price);

            // 记录主动成交量，用于撤单计算
            let entry = self.active_trades_buffer.entry(price_ordered).or_insert((0.0, 0.0));
            match side {
                "buy" => entry.0 += qty_f64,
                "sell" => entry.1 += qty_f64,
                _ => {}
            }
            
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
            
            match side {
                "buy" => {
                    order_flow.realtime_trade_record.buy_volume += qty_f64;
                    order_flow.history_trade_record.buy_volume += qty_f64;
                },
                "sell" => {
                    order_flow.realtime_trade_record.sell_volume += qty_f64;
                    order_flow.history_trade_record.sell_volume += qty_f64;
                },
                _ => {}
            }
            
            order_flow.realtime_trade_record.timestamp = current_time;
            order_flow.history_trade_record.timestamp = current_time;
        }
    }

    pub fn update(&mut self, data: &Value) {
        self.prices_to_clear_buffer.clear();
        self.cancellations_buffer.clear();
        
        let mut cancellations: Vec<(f64, String, f64)> = Vec::new();
        
        // 处理bids数组
        if let Some(bids) = data["b"].as_array() {
            let mut new_best_bid: Option<f64> = None;
            
            if !bids.is_empty() {
                if let (Some(price_str), Some(qty_str)) = (bids[0][0].as_str(), bids[0][1].as_str()) {
                    if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        if price > 0.0 && qty > 0.0 {
                            new_best_bid = Some(price);
                        }
                    }
                }
            }
            
            self.best_bid_price = new_best_bid;
            
            if let Some(new_best_bid) = new_best_bid {
                for (price, order_flow) in self.order_flows.iter() {
                    let price_val = price.0;
                    if price_val > new_best_bid && order_flow.bid_ask.bid > 0.0 {
                        self.prices_to_clear_buffer.push((*price, "bid".to_string()));
                    }
                }
                
                for (price, side) in &self.prices_to_clear_buffer {
                    if let Some(order_flow) = self.order_flows.get_mut(price) {
                        if order_flow.bid_ask.bid > 0.0 {
                            cancellations.push((price.0, side.clone(), order_flow.bid_ask.bid));
                            order_flow.bid_ask.bid = 0.0;
                        }
                    }
                }
            }
            
            for bid in bids {
                if let (Some(price_str), Some(qty_str)) = (bid[0].as_str(), bid[1].as_str()) {
                    if let (Ok(price), Ok(qty_f64)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        let price_ordered = OrderedFloat(price);
                        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                        
                        if qty_f64 == 0.0 {
                            if order_flow.bid_ask.bid > 0.0 {
                                let active_sell = self.active_trades_buffer.get(&price_ordered).map_or(0.0, |v| v.1);
                                let cancel_volume = (order_flow.bid_ask.bid - active_sell).max(0.0);
                                if cancel_volume > 0.0 {
                                    cancellations.push((price, "bid".to_string(), cancel_volume));
                                }
                            }
                            order_flow.bid_ask.bid = 0.0;
                        } else {
                            let old_bid = order_flow.bid_ask.bid;
                            order_flow.bid_ask.bid = qty_f64;
                            if old_bid > qty_f64 {
                                let active_sell = self.active_trades_buffer.get(&price_ordered).map_or(0.0, |v| v.1);
                                let cancel_volume = (old_bid - qty_f64 - active_sell).max(0.0);
                                if cancel_volume > 0.0 {
                                    cancellations.push((price, "bid".to_string(), cancel_volume));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 处理asks数组
        if let Some(asks) = data["a"].as_array() {
            let mut new_best_ask: Option<f64> = None;
            
            if !asks.is_empty() {
                if let (Some(price_str), Some(qty_str)) = (asks[0][0].as_str(), asks[0][1].as_str()) {
                    if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        if price > 0.0 && qty > 0.0 {
                            new_best_ask = Some(price);
                        }
                    }
                }
            }
            
            self.best_ask_price = new_best_ask;
            
            if let Some(new_best_ask) = new_best_ask {
                for (price, order_flow) in self.order_flows.iter() {
                    let price_val = price.0;
                    if price_val < new_best_ask && order_flow.bid_ask.ask > 0.0 {
                        self.prices_to_clear_buffer.push((*price, "ask".to_string()));
                    }
                }
                
                for (price, side) in &self.prices_to_clear_buffer {
                    if let Some(order_flow) = self.order_flows.get_mut(price) {
                        if order_flow.bid_ask.ask > 0.0 {
                            cancellations.push((price.0, side.clone(), order_flow.bid_ask.ask));
                            order_flow.bid_ask.ask = 0.0;
                        }
                    }
                }
            }
            
            for ask in asks {
                if let (Some(price_str), Some(qty_str)) = (ask[0].as_str(), ask[1].as_str()) {
                    if let (Ok(price), Ok(qty_f64)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        let price_ordered = OrderedFloat(price);
                        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                        
                        if qty_f64 == 0.0 {
                            if order_flow.bid_ask.ask > 0.0 {
                                let active_buy = self.active_trades_buffer.get(&price_ordered).map_or(0.0, |v| v.0);
                                let cancel_volume = (order_flow.bid_ask.ask - active_buy).max(0.0);
                                if cancel_volume > 0.0 {
                                    cancellations.push((price, "ask".to_string(), cancel_volume));
                                }
                            }
                            order_flow.bid_ask.ask = 0.0;
                        } else {
                            let old_ask = order_flow.bid_ask.ask;
                            order_flow.bid_ask.ask = qty_f64;
                            if old_ask > qty_f64 {
                                let active_buy = self.active_trades_buffer.get(&price_ordered).map_or(0.0, |v| v.0);
                                let cancel_volume = (old_ask - qty_f64 - active_buy).max(0.0);
                                if cancel_volume > 0.0 {
                                    cancellations.push((price, "ask".to_string(), cancel_volume));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 处理检测到的撤单
        for (price, side, volume) in cancellations {
            self.detect_cancellation(price, &side, volume);
        }
        
        // 清理主动成交缓冲区
        self.active_trades_buffer.clear();
    }

    pub fn detect_cancellation(&mut self, price: f64, side: &str, volume: f64) {
        let price_ordered = OrderedFloat(price);
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
        
        match side {
            "bid" => order_flow.realtime_cancel_records.bid_cancel += volume,
            "ask" => order_flow.realtime_cancel_records.ask_cancel += volume,
            _ => {}
        }
        
        order_flow.realtime_cancel_records.timestamp = current_time;
    }

    pub fn update_current_price(&mut self, price: f64) {
        self.current_price = Some(price);
    }

    pub fn clean_old_trades(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        for (_price, order_flow) in self.order_flows.iter_mut() {
            if current_time - order_flow.realtime_trade_record.timestamp > self.trade_display_duration {
                order_flow.realtime_trade_record.buy_volume = 0.0;
                order_flow.realtime_trade_record.sell_volume = 0.0;
            }
        }
        
        if self.order_flows.len() > self.max_trade_records {
            let to_remove = self.order_flows.len() - self.max_trade_records;
            let mut keys_to_remove = Vec::new();
            
            for (price, order_flow) in &self.order_flows {
                if order_flow.bid_ask.bid == 0.0 && 
                   order_flow.bid_ask.ask == 0.0 && 
                   order_flow.realtime_trade_record.buy_volume == 0.0 && 
                   order_flow.realtime_trade_record.sell_volume == 0.0 && 
                   order_flow.realtime_cancel_records.bid_cancel == 0.0 && 
                   order_flow.realtime_cancel_records.ask_cancel == 0.0 {
                    keys_to_remove.push(*price);
                    if keys_to_remove.len() >= to_remove {
                        break;
                    }
                }
            }
            
            for price in keys_to_remove {
                self.order_flows.remove(&price);
            }
        }
    }

    pub fn clean_old_cancels(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        for (_price, order_flow) in self.order_flows.iter_mut() {
            if current_time - order_flow.realtime_cancel_records.timestamp > self.cancel_display_duration {
                order_flow.realtime_cancel_records.bid_cancel = 0.0;
                order_flow.realtime_cancel_records.ask_cancel = 0.0;
            }
        }
    }

    pub fn add_iceberg_signal(&mut self, signal_type: &str, volume: f64) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64;
        
        self.iceberg_signals.push(ImbalanceSignal {
            timestamp: current_time,
            signal_type: signal_type.to_string(),
            ratio: volume,
        });
        
        if self.iceberg_signals.len() > 20 {
            self.iceberg_signals.remove(0);
        }
    }

    pub fn get_trade_volume(&self, price: f64, side: &str) -> f64 {
        let price_ordered = OrderedFloat(price);
        if let Some(order_flow) = self.order_flows.get(&price_ordered) {
            match side {
                "buy" => order_flow.realtime_trade_record.buy_volume,
                "sell" => order_flow.realtime_trade_record.sell_volume,
                _ => 0.0,
            }
        } else {
            0.0
        }
    }

    pub fn get_history_trade_volume(&self, price: f64, side: &str) -> f64 {
        let price_ordered = OrderedFloat(price);
        if let Some(order_flow) = self.order_flows.get(&price_ordered) {
            match side {
                "buy" => order_flow.history_trade_record.buy_volume,
                "sell" => order_flow.history_trade_record.sell_volume,
                _ => 0.0,
            }
        } else {
            0.0
        }
    }

    pub fn get_cancel_volume(&self, price: f64, side: &str) -> f64 {
        let price_ordered = OrderedFloat(price);
        if let Some(order_flow) = self.order_flows.get(&price_ordered) {
            match side {
                "bid" => order_flow.realtime_cancel_records.bid_cancel,
                "ask" => order_flow.realtime_cancel_records.ask_cancel,
                _ => 0.0,
            }
        } else {
            0.0
        }
    }
}