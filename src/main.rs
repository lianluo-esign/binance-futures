use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use ordered_float::OrderedFloat;
use parking_lot::Mutex;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, Cell, Row, Table, Paragraph, Wrap},
    Frame, Terminal,
};
use serde_json::Value;
use std::{
    collections::{ BTreeMap, HashMap},
    env,  // 新增：用于读取环境变量
    io,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;
// 新增：引入reqwest用于HTTP请求
use reqwest;

// 订单簿数据结构 - 基础组件
#[derive(Debug, Clone)]
struct PriceLevel {
    ask: f64,
    bid: f64,
}

#[derive(Debug, Clone)]
struct TradeRecord {
    buy_volume: f64,
    sell_volume: f64,
    timestamp: u64,
}

#[derive(Debug, Clone)]
struct CancelRecord {
    bid_cancel: f64,         // 当前实时买单撤单量
    ask_cancel: f64,         // 当前实时卖单撤单量
    timestamp: u64,          // 时间戳
}

// 新的OrderFlow结构体，整合了价格水平、交易记录和撤单记录
#[derive(Debug, Clone)]
struct OrderFlow {
    // 买卖盘数据
    bid_ask: PriceLevel,
    
    // 历史累计买单和卖单量
    history_trade_record: TradeRecord,
    
    // 实时成交订单，每过5s自动清除，用新的不断覆盖
    realtime_trade_record: TradeRecord,
    
    // 实时撤单记录
    realtime_cancel_records: CancelRecord,
    
}

impl OrderFlow {
    fn new() -> Self {
        Self {
            bid_ask: PriceLevel { bid: 0.0, ask: 0.0 },
            history_trade_record: TradeRecord { buy_volume: 0.0, sell_volume: 0.0, timestamp: 0 },
            realtime_trade_record: TradeRecord { buy_volume: 0.0, sell_volume: 0.0, timestamp: 0 },
            realtime_cancel_records: CancelRecord { bid_cancel: 0.0, ask_cancel: 0.0, timestamp: 0 },
        }
    }
}

// 添加失衡信号结构体
#[derive(Debug, Clone)]
struct ImbalanceSignal {
    timestamp: u64,
    signal_type: String, // "buy" 或 "sell"
    ratio: f64,         // 占比
}

// 添加大订单结构体
#[derive(Debug, Clone)]
struct BigOrder {
    order_type: String,  // "buy" 或 "sell"
    volume: f64,         // 订单量
    timestamp: u64,      // 时间戳
}

// 订单簿数据管理 - 使用 BTreeMap<OrderedFloat<f64>, OrderFlow>
struct OrderBookData {
    // 合并后的数据结构，使用一个BTreeMap共用价格Key
    order_flows: BTreeMap<OrderedFloat<f64>, OrderFlow>,
    current_price: Option<f64>,
    last_trade_side: Option<String>,
    trade_display_duration: u64,
    cancel_display_duration: u64,
    max_trade_records: usize,
    max_cancel_records: usize,
    
    // 注释掉市场微观结构分析器
    // microstructure_analyzer: MarketMicrostructureAnalyzer,
    
    // 新增字段
    stable_highlight_price: Option<f64>,
    stable_highlight_side: Option<String>,
    last_trade_price: Option<f64>,
    highlight_start_time: Option<u64>,
    highlight_duration: u64,
    // 新增：最后更新ID
    last_update_id: Option<u64>,
    
    // 性能优化：缓存最优买卖价格，避免重复计算
    best_bid_price: Option<f64>,
    best_ask_price: Option<f64>,
    
    // 性能优化：预分配的缓冲区，减少频繁的内存分配
    prices_to_clear_buffer: Vec<(OrderedFloat<f64>, String)>,
    cancellations_buffer: Vec<(f64, String, f64)>,
    
    // 添加OrderBook Imbalance相关字段
    bid_volume_ratio: f64,                // 买单量占比
    ask_volume_ratio: f64,                // 卖单量占比
    imbalance_signals: Vec<ImbalanceSignal>, // 失衡信号列表
    last_imbalance_check: u64,           // 上次检查失衡的时间戳
    continuous_imbalance_start: Option<u64>, // 连续失衡开始的时间戳
    current_imbalance_type: Option<String>, // 当前失衡类型 "buy" 或 "sell"
    cancel_signals: Vec<ImbalanceSignal>, // 撤单信号列表，复用ImbalanceSignal结构体
    last_cancel_check: u64,              // 上次检查撤单的时间戳
    
    // 新增：冰山订单信号列表
    iceberg_signals: Vec<ImbalanceSignal>, // 冰山订单信号列表，复用ImbalanceSignal结构体
    
    // 新增：大订单 HashMap，key为价格，value为BigOrder
    big_orders: HashMap<OrderedFloat<f64>, BigOrder>,
    last_big_order_check: u64,  // 添加这一行，用于记录上次检测大订单的时间
    active_trades_buffer: HashMap<OrderedFloat<f64>, (f64, f64)>, // 价格 -> (买单成交量, 卖单成交量)
}


impl OrderBookData {
    fn new() -> Self {
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
            
            // 初始化性能优化字段
            best_bid_price: None,
            best_ask_price: None,
            prices_to_clear_buffer: Vec::with_capacity(100), // 预分配合理容量
            cancellations_buffer: Vec::with_capacity(100),   // 预分配合理容量
            
            // 初始化OrderBook Imbalance相关字段
            bid_volume_ratio: 0.5,
            ask_volume_ratio: 0.5,
            imbalance_signals: Vec::new(),
            last_imbalance_check: 0,
            continuous_imbalance_start: None,
            current_imbalance_type: None,
            cancel_signals: Vec::new(),
            last_cancel_check: 0,
            active_trades_buffer: HashMap::new(),
            // 初始化冰山订单信号列表
            iceberg_signals: Vec::new(),
            
            // 初始化大订单 HashMap
            big_orders: HashMap::new(),
            last_big_order_check: 0,  // 初始化为0
        }
    }
    
    // 添加冰山订单信号
    fn add_iceberg_signal(&mut self, signal_type: &str, volume: f64) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64;
        
        self.iceberg_signals.push(ImbalanceSignal {
            timestamp: current_time,
            signal_type: signal_type.to_string(),
            ratio: volume, // 这里使用ratio字段存储BTC数量
        });
        
        // 限制信号列表大小，保留最近的20个信号
        if self.iceberg_signals.len() > 20 {
            self.iceberg_signals.remove(0);
        }
    }

    // 直接清理不合理挂单的方法 - 使用 BTreeMap 的范围查询优化
    fn clear_unreasonable_orders(&mut self, trade_price: f64, trade_side: &str) {
        let trade_price_ordered = OrderedFloat(trade_price);
        
        match trade_side {
            "buy" => {
                // 买单成交，清空价格小于等于成交价的所有ask挂单
                let keys_to_update: Vec<OrderedFloat<f64>> = self.order_flows
                    .range(..=trade_price_ordered)
                    .map(|(price, _)| *price)
                    .collect();
                
                for price in keys_to_update {
                    if let Some(order_flow) = self.order_flows.get_mut(&price) {
                        order_flow.bid_ask.ask = 0.0;
                    }
                }
            }
            "sell" => {
                // 卖单成交，清空价格大于等于成交价的所有bid挂单
                let keys_to_update: Vec<OrderedFloat<f64>> = self.order_flows
                    .range(trade_price_ordered..)
                    .map(|(price, _)| *price)
                    .collect();
                
                for price in keys_to_update {
                    if let Some(order_flow) = self.order_flows.get_mut(&price) {
                        order_flow.bid_ask.bid = 0.0;
                    }
                }
            }
            _ => {}
        }
    }

    fn add_trade(&mut self, data: &Value) {
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
                "buy" => entry.0 += qty_f64,  // 买单成交量
                "sell" => entry.1 += qty_f64, // 卖单成交量
                _ => {}
            }
            
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            // 获取或创建该价格的OrderFlow
            let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
            
            // 更新实时交易记录
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
            
            // 更新时间戳
            order_flow.realtime_trade_record.timestamp = current_time;
            order_flow.history_trade_record.timestamp = current_time;
        }
    }

    fn clean_old_trades(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 清理过期的实时交易记录
        for (_price, order_flow) in self.order_flows.iter_mut() {
            // 如果实时交易记录超过显示时间，则重置为0
            if current_time - order_flow.realtime_trade_record.timestamp > self.trade_display_duration {
                order_flow.realtime_trade_record.buy_volume = 0.0;
                order_flow.realtime_trade_record.sell_volume = 0.0;
            }
        }
        
        // 限制记录数量 - 如果OrderFlow数量超过限制，移除最旧的记录
        if self.order_flows.len() > self.max_trade_records {
            // 收集需要移除的键
            let to_remove = self.order_flows.len() - self.max_trade_records;
            let mut keys_to_remove = Vec::new();
            
            // 找出没有活跃数据的OrderFlow进行移除
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
            
            // 移除收集的键
            for price in keys_to_remove {
                self.order_flows.remove(&price);
            }
        }
    }

    fn detect_cancellation(&mut self, price: f64, side: &str, volume: f64) {
        let price_ordered = OrderedFloat(price);
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 获取或创建该价格的OrderFlow
        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
        
        // 更新撤单记录
        match side {
            "bid" => order_flow.realtime_cancel_records.bid_cancel += volume,
            "ask" => order_flow.realtime_cancel_records.ask_cancel += volume,
            _ => {}
        }
        
        // 更新时间戳
        order_flow.realtime_cancel_records.timestamp = current_time;
    }

    fn clean_old_cancels(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 清理过期的撤单记录
        for (_price, order_flow) in self.order_flows.iter_mut() {
            // 如果撤单记录超过显示时间，则重置为0
            if current_time - order_flow.realtime_cancel_records.timestamp > self.cancel_display_duration {
                order_flow.realtime_cancel_records.bid_cancel = 0.0;
                order_flow.realtime_cancel_records.ask_cancel = 0.0;
            }
        }
    }

    fn get_trade_volume(&self, price: f64, side: &str) -> f64 {
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

    fn get_history_trade_volume(&self, price: f64, side: &str) -> f64 {
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

    fn get_cancel_volume(&self, price: f64, side: &str) -> f64 {
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

    fn update_current_price(&mut self, price: f64) {
        self.current_price = Some(price);
    }

    fn reset_history_trade_records(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 获取当前时间的小时
        let seconds = current_time / 1000;
        let hours = (seconds / 3600) % 24;
        
        // 如果是凌晨5点，重置所有历史成交记录
        if hours == 5 {
            for (_price, order_flow) in self.order_flows.iter_mut() {
                order_flow.history_trade_record.buy_volume = 0.0;
                order_flow.history_trade_record.sell_volume = 0.0;
                order_flow.history_trade_record.timestamp = current_time;
            }
        }
    }

    fn update(&mut self, data: &Value) {
        // 清空并重用预分配的缓冲区
        self.prices_to_clear_buffer.clear();
        self.cancellations_buffer.clear();
        
        // 创建一个临时的撤单信息收集器
        let mut cancellations: Vec<(f64, String, f64)> = Vec::new();
        
        // 处理bids数组
        if let Some(bids) = data["b"].as_array() {
            // 直接获取bids中的第一个元素作为最优买价（价格最大的）
            let mut new_best_bid: Option<f64> = None;
            
            // 直接使用第一个元素作为最优买价
            if !bids.is_empty() {
                if let (Some(price_str), Some(qty_str)) = (bids[0][0].as_str(), bids[0][1].as_str()) {
                    if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        if price > 0.0 && qty > 0.0 {
                            new_best_bid = Some(price);
                        }
                    }
                }
            }
            
            // 更新OrderBookData的best_bid_price字段
            self.best_bid_price = new_best_bid;
            
            // 如果有最优买价，收集所有需要清理的价格
            if let Some(new_best_bid) = new_best_bid {
                // 收集需要清理的价格
                for (price, order_flow) in self.order_flows.iter() {
                    let price_val = price.0; // 使用.0访问OrderedFloat中的f64值
                    if price_val > new_best_bid && order_flow.bid_ask.bid > 0.0 {
                        self.prices_to_clear_buffer.push((*price, "bid".to_string()));
                    }
                }
                
                // 清理收集的价格
                for (price, side) in &self.prices_to_clear_buffer {
                    if let Some(order_flow) = self.order_flows.get_mut(price) {
                        if order_flow.bid_ask.bid > 0.0 {
                            cancellations.push((price.0, side.clone(), order_flow.bid_ask.bid)); // 使用.0访问OrderedFloat中的f64值
                            order_flow.bid_ask.bid = 0.0;
                        }
                    }
                }
            }
            
            // 然后更新bids的具体数量
            for bid in bids {
                if let (Some(price_str), Some(qty_str)) = (bid[0].as_str(), bid[1].as_str()) {
                    if let (Ok(price), Ok(qty_f64)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        let price_ordered = OrderedFloat(price);
                        
                        // 获取或创建该价格的OrderFlow
                        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                        
                        let old_bid = order_flow.bid_ask.bid;
                        
                        // 在处理bids时
                        if qty_f64 == 0.0 {
                            if order_flow.bid_ask.bid > 0.0 {
                                // 获取该价格的主动卖单成交量（消耗买单）
                                let active_sell = self.active_trades_buffer.get(&price_ordered).map_or(0.0, |v| v.1);
                                // 计算真正的撤单量 = 原挂单量 - 主动成交量
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
                                // 获取该价格的主动卖单成交量（消耗买单）
                                let active_sell = self.active_trades_buffer.get(&price_ordered).map_or(0.0, |v| v.1);
                                // 计算真正的撤单量 = 挂单减少量 - 主动成交量
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
            // 直接获取asks中的第一个元素作为最优卖价（价格最小的）
            let mut new_best_ask: Option<f64> = None;
            
            // 直接使用第一个元素作为最优卖价
            if !asks.is_empty() {
                if let (Some(price_str), Some(qty_str)) = (asks[0][0].as_str(), asks[0][1].as_str()) {
                    if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        if price > 0.0 && qty > 0.0 {
                            new_best_ask = Some(price);
                        }
                    }
                }
            }
            
            // 更新best_ask_price字段
            self.best_ask_price = new_best_ask;
            
            // 如果有最优卖价，清理所有小于最优卖价的ask挂单
            if let Some(best_ask) = new_best_ask {
                // 清空缓冲区以重用
                self.prices_to_clear_buffer.clear();
                
                // 收集需要清理的价格
                for (price, order_flow) in self.order_flows.iter() {
                    let price_val = price.0; // 使用.0访问OrderedFloat中的f64值
                    if price_val < best_ask && order_flow.bid_ask.ask > 0.0 {
                        self.prices_to_clear_buffer.push((*price, "ask".to_string()));
                    }
                }
                
                // 处理需要清理的价格
                for (price, side) in &self.prices_to_clear_buffer {
                    if let Some(order_flow) = self.order_flows.get_mut(price) {
                        if order_flow.bid_ask.ask > 0.0 {
                            cancellations.push((price.0, side.clone(), order_flow.bid_ask.ask)); // 使用.0访问OrderedFloat中的f64值
                            order_flow.bid_ask.ask = 0.0;
                        }
                    }
                }
            }
            
            // 然后更新asks的具体数量
            for ask in asks {
                if let (Some(price_str), Some(qty_str)) = (ask[0].as_str(), ask[1].as_str()) {
                    if let (Ok(price), Ok(qty_f64)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        let price_ordered = OrderedFloat(price);
                        
                        // 获取或创建该价格的OrderFlow
                        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                        
                        let old_ask = order_flow.bid_ask.ask;
                        
                        if qty_f64 == 0.0 {
                            if order_flow.bid_ask.ask > 0.0 {
                                // 获取该价格的主动买单成交量（消耗卖单）
                                let active_buy = self.active_trades_buffer.get(&price_ordered).map_or(0.0, |v| v.0);
                                // 计算真正的撤单量 = 原挂单量 - 主动成交量
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
                                // 获取该价格的主动买单成交量（消耗卖单）
                                let active_buy = self.active_trades_buffer.get(&price_ordered).map_or(0.0, |v| v.0);
                                // 计算真正的撤单量 = 挂单减少量 - 主动成交量
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
        
        // 处理收集的撤单信息
        for (price, side, volume) in cancellations {
            self.detect_cancellation(price, &side, volume);
        }
        // 清空主动成交缓冲区
        self.active_trades_buffer.clear();
        self.clean_old_trades();
        self.clean_old_cancels();
        
        // 计算买卖盘失衡 - 使用calculate_volume_ratio替代check_orderbook_imbalance
        self.calculate_volume_ratio();
        
        // 检测撤单信号
        self.detect_cancel_signal();
        
        // 检查是否需要重置历史成交记录
        self.reset_history_trade_records();
        
        // 调用检测大订单的方法，而不是在update中直接检测
        self.detect_big_orders();
    }
    
    // 检测大订单的方法 - 独立于update方法，每秒执行一次
    fn detect_big_orders(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64;
        
        // 每1000毫秒(1秒)检查一次大订单
        if current_time - self.last_big_order_check < 1000 {
            return;
        }
        
        self.last_big_order_check = current_time;
        
        // 清空之前的大订单记录
        self.big_orders.clear();
        
        // 获取最优买卖价格
        let (best_bid, best_ask) = self.get_best_bid_ask();
        let threshold = 10.0; // 阈值设为10个BTC
        
        // 遍历所有价格水平，检查是否有大于等于阈值的挂单，但排除最优价格
        for (price, order_flow) in &self.order_flows {
            let price_val = price.0; // 使用.0访问OrderedFloat中的f64值
            
            // 检查买单（排除最优买价）
            if order_flow.bid_ask.bid >= threshold {
                if let Some(best_bid_price) = best_bid {
                    if price_val != best_bid_price {
                        // 保存到big_orders HashMap
                        self.big_orders.insert(*price, BigOrder {
                            order_type: "buy".to_string(),
                            volume: order_flow.bid_ask.bid,
                            timestamp: current_time,
                        });
                    }
                } else {
                    // 保存到big_orders HashMap
                    self.big_orders.insert(*price, BigOrder {
                        order_type: "buy".to_string(),
                        volume: order_flow.bid_ask.bid,
                        timestamp: current_time,
                    });
                }
            }
            
            // 检查卖单（排除最优卖价）
            if order_flow.bid_ask.ask >= threshold {
                if let Some(best_ask_price) = best_ask {
                    if price_val != best_ask_price {
                        // 保存到big_orders HashMap
                        self.big_orders.insert(*price, BigOrder {
                            order_type: "sell".to_string(),
                            volume: order_flow.bid_ask.ask,
                            timestamp: current_time,
                        });
                    }
                } else {
                    // 保存到big_orders HashMap
                    self.big_orders.insert(*price, BigOrder {
                        order_type: "sell".to_string(),
                        volume: order_flow.bid_ask.ask,
                        timestamp: current_time,
                    });
                }
            }
        }
    }

    // 检测撤单信号
    fn detect_cancel_signal(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64;
        
        // 每500毫秒检查一次撤单信号
        if current_time - self.last_cancel_check < 500 {
            return;
        }
            
        self.last_cancel_check = current_time;
        
        // 获取最优买卖价格
        let (best_bid, best_ask) = self.get_best_bid_ask();
        
        // 检查最优买价的撤单信号
        if let Some(bid_price) = best_bid {
            let price_ordered = OrderedFloat(bid_price);
            if let Some(order_flow) = self.order_flows.get(&price_ordered) {
                let bid_volume = order_flow.bid_ask.bid;
                let bid_cancel = order_flow.realtime_cancel_records.bid_cancel;
                
                // 如果撤单量大于挂单量的90%
                if bid_volume > 0.0 && bid_cancel > bid_volume * 0.9 {
                    // 添加撤单卖出信号
                    self.add_cancel_signal("sell", bid_cancel / bid_volume);
                }
            }
        }
        
        // 检查最优卖价的撤单信号
        if let Some(ask_price) = best_ask {
            let price_ordered = OrderedFloat(ask_price);
            if let Some(order_flow) = self.order_flows.get(&price_ordered) {
                let ask_volume = order_flow.bid_ask.ask;
                let ask_cancel = order_flow.realtime_cancel_records.ask_cancel;
                
                // 如果撤单量大于挂单量的90%
                if ask_volume > 0.0 && ask_cancel > ask_volume * 0.9 {
                    // 添加撤单买入信号
                    self.add_cancel_signal("buy", ask_cancel / ask_volume);
                }
            }
        }
    }

    // 添加撤单信号
    fn add_cancel_signal(&mut self, signal_type: &str, ratio: f64) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64;
        
        self.cancel_signals.push(ImbalanceSignal {
            timestamp: current_time,
            signal_type: signal_type.to_string(),
            ratio: ratio,
        });
        
        // 限制信号列表大小，保留最近的20个信号
        if self.cancel_signals.len() > 20 {
            self.cancel_signals.remove(0);
        }
    }
    
    // 检查订单簿失衡
    fn check_orderbook_imbalance(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64;
        
        // 每500毫秒检查一次失衡
        if current_time - self.last_imbalance_check < 500 {
            return;
        }
        
        self.last_imbalance_check = current_time;
        
        // 计算买卖盘总量
        let bid_volume: f64 = self.order_flows.values().map(|of| of.bid_ask.bid).sum();
        let ask_volume: f64 = self.order_flows.values().map(|of| of.bid_ask.ask).sum();
        
        // 计算买卖盘占比
        let total_volume = bid_volume + ask_volume;
        if total_volume > 0.0 {
            self.bid_volume_ratio = bid_volume / total_volume;
            self.ask_volume_ratio = ask_volume / total_volume;
        } else {
            self.bid_volume_ratio = 0.5;
            self.ask_volume_ratio = 0.5;
        }
        
        // 检测失衡 - 如果一方占比超过70%，则认为存在失衡
        let imbalance_threshold = 0.7; // 70%
        
        let new_imbalance_type = if self.bid_volume_ratio >= imbalance_threshold {
            Some("buy".to_string())
        } else if self.ask_volume_ratio >= imbalance_threshold {
            Some("sell".to_string())
        } else {
            None
        };
        
        // 如果失衡类型发生变化
        if new_imbalance_type != self.current_imbalance_type {
            // 如果有新的失衡
            if let Some(imbalance_type) = &new_imbalance_type {
                // 记录失衡开始时间
                self.continuous_imbalance_start = Some(current_time);
                
                // 添加新的失衡信号
                let ratio = if imbalance_type == "buy" {
                    self.bid_volume_ratio
                } else {
                    self.ask_volume_ratio
                };
                
                let signal = ImbalanceSignal {
                    timestamp: current_time,
                    signal_type: imbalance_type.clone(),
                    ratio,
                };
                
                self.imbalance_signals.push(signal);
                
                // 限制信号列表大小
                if self.imbalance_signals.len() > 100 {
                    self.imbalance_signals.remove(0);
                }
            } else {
                // 失衡结束
                self.continuous_imbalance_start = None;
            }
            
            // 更新当前失衡类型
            self.current_imbalance_type = new_imbalance_type;
        }
    }
    
    // 计算最优价格上的挂单量比例
    fn calculate_volume_ratio(&mut self) {
        let (bid_volume, ask_volume) = self.get_best_volumes();
        let total_volume = bid_volume + ask_volume;
        
        if total_volume > 0.0 {
            self.bid_volume_ratio = bid_volume / total_volume;
            self.ask_volume_ratio = ask_volume / total_volume;
        } else {
            self.bid_volume_ratio = 0.5;
            self.ask_volume_ratio = 0.5;
        }
        
        // 检测失衡信号
        self.detect_imbalance_signal();
    }
    
    // 检测失衡信号
    fn detect_imbalance_signal(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64;
        
        // 每次更新时检查
        self.last_imbalance_check = current_time;
        
        // 检测是否有失衡
        let has_buy_imbalance = self.bid_volume_ratio >= 0.75;
        let has_sell_imbalance = self.ask_volume_ratio >= 0.75;
        
        match (has_buy_imbalance, has_sell_imbalance, &self.current_imbalance_type) {
            (true, false, Some(imbalance_type)) if imbalance_type == "buy" => {
                // 继续买入失衡
                if let Some(start_time) = self.continuous_imbalance_start {
                    if current_time - start_time >= 300 { // 持续300ms
                        // 添加买入失衡信号
                        self.add_imbalance_signal("buy", self.bid_volume_ratio);
                        // 重置计时器，避免连续触发
                        self.continuous_imbalance_start = Some(current_time);
                    }
                }
            },
            (false, true, Some(imbalance_type)) if imbalance_type == "sell" => {
                // 继续卖出失衡
                if let Some(start_time) = self.continuous_imbalance_start {
                    if current_time - start_time >= 300 { // 持续300ms
                        // 添加卖出失衡信号
                        self.add_imbalance_signal("sell", self.ask_volume_ratio);
                        // 重置计时器，避免连续触发
                        self.continuous_imbalance_start = Some(current_time);
                    }
                }
            },
            (true, false, None) => {
                // 开始新的买入失衡
                self.continuous_imbalance_start = Some(current_time);
                self.current_imbalance_type = Some("buy".to_string());
            },
            (true, false, Some(imbalance_type)) if imbalance_type != "buy" => {
                // 开始新的买入失衡
                self.continuous_imbalance_start = Some(current_time);
                self.current_imbalance_type = Some("buy".to_string());
            },
            (false, true, None) => {
                // 开始新的卖出失衡
                self.continuous_imbalance_start = Some(current_time);
                self.current_imbalance_type = Some("sell".to_string());
            },
            (false, true, Some(imbalance_type)) if imbalance_type != "sell" => {
                // 开始新的卖出失衡
                self.continuous_imbalance_start = Some(current_time);
                self.current_imbalance_type = Some("sell".to_string());
            },
            _ => {
                // 没有失衡或失衡结束
                self.continuous_imbalance_start = None;
                self.current_imbalance_type = None;
            }
        }
    }
    
    // 添加失衡信号
    fn add_imbalance_signal(&mut self, signal_type: &str, ratio: f64) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64;
        
        self.imbalance_signals.push(ImbalanceSignal {
            timestamp: current_time,
            signal_type: signal_type.to_string(),
            ratio: ratio,
        });
        
        // 限制信号列表大小，保留最近的20个信号
        if self.imbalance_signals.len() > 20 {
            self.imbalance_signals.remove(0);
        }
    }
    
    // 优化为O(1)时间复杂度获取最佳买价
    fn get_best_bid(&self) -> Option<f64> {
        self.best_bid_price
    }
    
    // 优化为O(1)时间复杂度获取最佳卖价
    fn get_best_ask(&self) -> Option<f64> {
        self.best_ask_price
    }
    
    // 获取最佳买卖价 - 优化为O(1)时间复杂度
    fn get_best_bid_ask(&self) -> (Option<f64>, Option<f64>) {
        (self.best_bid_price, self.best_ask_price)
    }
    
    // 获取最佳价位的挂单量
    fn get_best_volumes(&self) -> (f64, f64) {
        let (best_bid, best_ask) = self.get_best_bid_ask();
        let mut bid_volume = 0.0;
        let mut ask_volume = 0.0;
        
        if let Some(bid_price) = best_bid {
            if let Some(order_flow) = self.order_flows.get(&OrderedFloat(bid_price)) {
                bid_volume = order_flow.bid_ask.bid;
            }
        }
        
        if let Some(ask_price) = best_ask {
            if let Some(order_flow) = self.order_flows.get(&OrderedFloat(ask_price)) {
                ask_volume = order_flow.bid_ask.ask;
            }
        }
        
        (bid_volume, ask_volume)
    }

    // 新增：初始化深度数据的方法
    async fn initialize_depth_data(&mut self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 构建请求URL
        let url = format!("https://fapi.binance.com/fapi/v1/depth?symbol={}&limit=1000", symbol);
        
        // 发送HTTP请求获取深度数据
        let response = reqwest::get(&url).await?;
        let depth_data: Value = response.json().await?;
        
        // 解析最后更新ID
        if let Some(last_update_id) = depth_data["lastUpdateId"].as_u64() {
            self.last_update_id = Some(last_update_id);
        }
        
        // 先计算最优价格
        let mut best_bid_price: Option<f64> = None;
        let mut best_ask_price: Option<f64> = None;
        
        // 处理买单数据，找到最优买价
        if let Some(bids) = depth_data["bids"].as_array() {
            for bid in bids {
                if let (Some(price_str), Some(qty_str)) = (bid[0].as_str(), bid[1].as_str()) {
                    if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        if price > 0.0 && qty > 0.0 {
                            best_bid_price = Some(best_bid_price.map_or(price, |current| current.max(price)));
                            let price_ordered = OrderedFloat(price);
                            let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                            order_flow.bid_ask.bid = qty;
                        }
                    }
                }
            }
        }
        
        // 处理卖单数据，找到最优卖价
        if let Some(asks) = depth_data["asks"].as_array() {
            for ask in asks {
                if let (Some(price_str), Some(qty_str)) = (ask[0].as_str(), ask[1].as_str()) {
                    if let (Ok(price), Ok(qty)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        if price > 0.0 && qty > 0.0 {
                            best_ask_price = Some(best_ask_price.map_or(price, |current| current.min(price)));
                            let price_ordered = OrderedFloat(price);
                            let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                            order_flow.bid_ask.ask = qty;
                        }
                    }
                }
            }
        }
        
        // 更新OrderBookData的最优价格字段
        self.best_bid_price = best_bid_price;
        self.best_ask_price = best_ask_price;
        
        // 更新当前价格（取买卖盘中间价）
        if let (Some(best_bid), Some(best_ask)) = (best_bid_price, best_ask_price) {
            let mid_price = (best_bid + best_ask) / 2.0;
            self.update_current_price(mid_price);
        }
        
        log::info!("初始化深度数据完成，加载了{}个价格水平", self.order_flows.len());
        
        Ok(())
    }
}

// 应用状态
struct App {
    orderbook: Arc<Mutex<OrderBookData>>,
    scroll_offset: usize,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            orderbook: Arc::new(Mutex::new(OrderBookData::new())),
            scroll_offset: 0,
            should_quit: false,
        }
    }
    
    // 简化的自动滚动
    fn auto_scroll(&mut self, current_price_index: Option<usize>, visible_rows: usize) {
        if let Some(price_index) = current_price_index {
            let visible_start = self.scroll_offset;
            let visible_end = self.scroll_offset + visible_rows;
            
            // 检查游标是否在可见区域内
            if price_index >= visible_start && price_index < visible_end {
                let relative_position = price_index - visible_start;
                
                // 如果距离上边界或下边界3行以内，调整滚动位置让游标居中
                if relative_position <= 3 || relative_position >= visible_rows.saturating_sub(3) {
                    let center_position = visible_rows / 2;
                    self.scroll_offset = if price_index >= center_position {
                        price_index - center_position
                    } else {
                        0
                    };
                }
            } else {
                // 如果不在可见区域，立即跳转到居中位置
                let center_position = visible_rows / 2;
                self.scroll_offset = if price_index >= center_position {
                    price_index - center_position
                } else {
                    0
                };
            }
        }
    }
    
    fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }

    fn page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(10);
    }

    fn page_down(&mut self) {
        self.scroll_offset += 10;
    }

    fn home(&mut self) {
        self.scroll_offset = 0;
    }
}

// UI渲染函数 - 修改为左右布局
fn ui(f: &mut Frame, app: &mut App) {
    let size = f.size();
    
    // 创建左右布局
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70), // 订单薄占70%
            Constraint::Percentage(30), // 市场信号占30%
        ])
        .split(size);
    
    let orderbook_area = horizontal_chunks[0];
    let signal_area = horizontal_chunks[1];
    
    // 计算订单薄表格区域
    let table_width = orderbook_area.width.saturating_sub(2);
    let table_height = orderbook_area.height.saturating_sub(2);
    
    let centered_area = Rect {
        x: orderbook_area.x + 1,
        y: orderbook_area.y + 1,
        width: table_width,
        height: table_height,
    };
    
    let block = Block::default()
        .title("Binance Futures Order Book")
        .borders(Borders::ALL);
    
    // 创建表格数据和获取当前价格索引
    let mut rows = Vec::new();
    let mut current_price_index = None;
    
    // 使用作用域来限制 orderbook 的借用范围
    {
        let orderbook = app.orderbook.lock();
        
        if let Some(current_price) = orderbook.current_price {
            let best_bid = orderbook.get_best_bid();
            let best_ask = orderbook.get_best_ask();
            
            // 获取所有价格并排序，只显示合理的价位
            // 买单：价格 <= best_bid，卖单：价格 >= best_ask
            let filtered_prices: Vec<f64> = orderbook
                .order_flows
                .iter()
                .filter(|(price, order_flow)| {
                    let price_val = price.into_inner();
                    let has_valid_bid = order_flow.bid_ask.bid > 0.0 && 
                        best_bid.map_or(false, |bb| price_val <= bb);
                    let has_valid_ask = order_flow.bid_ask.ask > 0.0 && 
                        best_ask.map_or(false, |ba| price_val >= ba);
                    has_valid_bid || has_valid_ask
                })
                .map(|(price, _)| price.into_inner())
                .collect();
            
            // BTreeMap 默认是升序，我们需要降序显示
            let mut sorted_prices = filtered_prices;
            sorted_prices.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
            
            // 生成表格行
            for (i, price) in sorted_prices.iter().enumerate() {
                if (price - current_price).abs() < 0.000001 {
                    current_price_index = Some(i);
                }
                
                let price_ordered = OrderedFloat(*price);
                let order_flow = orderbook.order_flows.get(&price_ordered).unwrap();
                let bid_vol = order_flow.bid_ask.bid;
                let ask_vol = order_flow.bid_ask.ask;
                
                // 获取成交量信息
                let sell_trade_vol = orderbook.get_trade_volume(*price, "sell");
                let buy_trade_vol = orderbook.get_trade_volume(*price, "buy");
                
                // 获取撤单量信息
                let bid_cancel_vol = orderbook.get_cancel_volume(*price, "bid");
                let ask_cancel_vol = orderbook.get_cancel_volume(*price, "ask");
                
                // 获取历史成交量信息
                let history_sell_trade_vol = orderbook.get_history_trade_volume(*price, "sell");
                let history_buy_trade_vol = orderbook.get_history_trade_volume(*price, "buy");

                // Bid挂单显示逻辑：直接显示买单挂单量（过滤已在上层完成）
                let bid_str = if bid_vol > 0.0 {
                    format!("{:.3}", bid_vol)
                } else { 
                    String::new() 
                };
                
                // Ask挂单显示逻辑：直接显示卖单挂单量（过滤已在上层完成）
                let ask_str = if ask_vol > 0.0 {
                    format!("{:.3}", ask_vol)
                } else { 
                    String::new() 
                };
                
                // 成交量显示逻辑
                let sell_trade_str = if sell_trade_vol > 0.0 { 
                    format!("@{:.3}", sell_trade_vol) 
                } else { 
                    String::new() 
                };
                
                let buy_trade_str = if buy_trade_vol > 0.0 { 
                    format!("@{:.3}", buy_trade_vol) 
                } else { 
                    String::new() 
                };
                
                // 撤单量显示逻辑：直接显示撤单量（过滤已在上层完成）
                let bid_cancel_str = if bid_cancel_vol > 0.0 {
                    format!("-{:.3}", bid_cancel_vol)
                } else { 
                    String::new() 
                };
                
                let ask_cancel_str = if ask_cancel_vol > 0.0 {
                    format!("-{:.3}", ask_cancel_vol)
                } else { 
                    String::new() 
                };
                
                // 创建行
                let row = Row::new(vec![
                    Cell::from(bid_cancel_str).style(Style::default().fg(Color::Gray)),
                    Cell::from(sell_trade_str).style(Style::default().fg(Color::Red)),
                    Cell::from(bid_str).style(Style::default().fg(Color::Green)),
                    {
                        // 价格列 - 格式化为字符串显示
                        let price_str = format!("{:.2}", price);
                        let mut price_cell = Cell::from(price_str).style(Style::default().fg(Color::White));
                        if Some(i) == current_price_index {
                            if let Some(ref last_side) = orderbook.last_trade_side {
                                let highlight_color = match last_side.as_str() {
                                    "buy" => Color::Green,
                                    "sell" => Color::Red,
                                    _ => Color::White,
                                };
                                price_cell = price_cell.style(Style::default().fg(Color::Black).bg(highlight_color).add_modifier(Modifier::BOLD));
                            }
                        }
                        price_cell
                    },
                    Cell::from(ask_str).style(Style::default().fg(Color::Red)),
                    Cell::from(buy_trade_str).style(Style::default().fg(Color::Green)),
                    Cell::from(ask_cancel_str).style(Style::default().fg(Color::Gray)),
                    {
                        // 主动成交订单列 - 直接显示数字而不是条形图
                        let total_vol = history_buy_trade_vol + history_sell_trade_vol;
                        let mut active_trade_str = String::new();
                        
                        if total_vol > 0.0 {
                            // 直接显示买单和卖单的数量
                            active_trade_str = format!("买:{:.3} 卖:{:.3} 总:{:.3}", 
                                history_buy_trade_vol, 
                                history_sell_trade_vol, 
                                total_vol);
                        }
                        
                        Cell::from(active_trade_str).style(Style::default().fg(Color::White))
                    },
                ]);
                
                rows.push(row);
            }
        }
    } // orderbook 借用在这里结束
    
    // 现在可以安全地调用 auto_scroll
    let visible_rows_count = centered_area.height.saturating_sub(3) as usize;
    app.auto_scroll(current_price_index, visible_rows_count);
    
    // 应用滚动偏移
    let visible_rows: Vec<_> = rows.into_iter().skip(app.scroll_offset).collect();
    
    // 创建表格
    let table = Table::new(visible_rows)
        .header(
            Row::new(vec![
                Cell::from("Bid Cancel").style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                Cell::from("Sell Trade").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Cell::from("Bid Vol").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Cell::from("Price").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Cell::from("Ask Vol").style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Cell::from("Buy Trade").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Cell::from("Ask Cancel").style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                Cell::from("History Trades").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ])
        )
        .block(block)
        .widths(&[
            Constraint::Length(12), // Bid Cancel
            Constraint::Length(12), // Sell Trade
            Constraint::Length(12), // Bid Volume
            Constraint::Length(12), // Price
            Constraint::Length(12), // Ask Volume
            Constraint::Length(12), // Buy Trade
            Constraint::Length(12), // Ask Cancel
            Constraint::Length(60), // History Trade - 增加宽度以容纳30个字符加总量
        ]);
    
    f.render_widget(table, centered_area);
    
    // 将右侧信号区域分为三个垂直部分
    let signal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40), // Orderbook Imbalance 占40%
            Constraint::Percentage(30), // Order Momentum 占30%
            Constraint::Percentage(30), // Iceberg Orders 占30%
        ])
        .split(signal_area);
    
    let imbalance_area = signal_chunks[0];
    let momentum_area = signal_chunks[1];
    let iceberg_area = signal_chunks[2];
    
    // 渲染三个信号区域
    render_orderbook_imbalance(f, app, imbalance_area);
    render_order_momentum(f, app, momentum_area);
    render_iceberg_orders(f, app, iceberg_area);
}

// 渲染订单簿失衡信号
fn render_orderbook_imbalance(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title("Orderbook Imbalance")
        .borders(Borders::ALL);
    
    let inner_area = block.inner(area);
    
    // 获取OrderBookData中的数据
    let (bid_ratio, ask_ratio, imbalance_signals, cancel_signals) = {
        let orderbook = app.orderbook.lock();
        (
            orderbook.bid_volume_ratio, 
            orderbook.ask_volume_ratio, 
            orderbook.imbalance_signals.clone(),
            orderbook.cancel_signals.clone()
        )
    };
    
    // 创建显示内容
    let mut content = Vec::new();
    
    // 添加比例信息
    content.push(format!("买单占比: {:.2}% | 卖单占比: {:.2}%", bid_ratio * 100.0, ask_ratio * 100.0));
    
    // 创建横向条
    let bar_width = inner_area.width.saturating_sub(2) as usize;
    let bid_bar_width = (bid_ratio * bar_width as f64) as usize;
    
    let mut bar = String::new();
    for _ in 0..bid_bar_width {
        bar.push('█');
    }
    for _ in bid_bar_width..bar_width {
        bar.push('░');
    }
    
    // content.push(bar);
    content.push(String::new()); // 空行
    
    // 添加失衡信号
    content.push("失衡信号:".to_string());
    
    // 创建Text对象和Line列表
    let mut lines = Vec::new();
    
    // 添加基本信息
    lines.push(Line::from(Span::raw(format!("买单占比: {:.2}% | 卖单占比: {:.2}%", bid_ratio * 100.0, ask_ratio * 100.0))));
    lines.push(Line::from(Span::raw(format!("{bar}"))));
    lines.push(Line::from(Span::raw(""))); // 空行
    lines.push(Line::from(Span::raw("失衡信号:")));
    
    // 显示失衡信号
    for signal in imbalance_signals.iter().rev() {
        // 使用标准库计算时间
        let time = std::time::UNIX_EPOCH + std::time::Duration::from_millis(signal.timestamp);
        let seconds = time.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let hours = (seconds / 3600) % 24;
        let minutes = (seconds / 60) % 60;
        let secs = seconds % 60;
        let formatted_time = format!("{:02}:{:02}:{:02}", hours, minutes, secs);
        
        // 创建信号文本，使用普通文字
        let signal_text = format!("[{formatted_time}] {}{:.2}%", 
            if signal.signal_type == "buy" { "失衡信号 [买入] - 占比: " } else { "失衡信号 [卖出] - 占比: " },
            signal.ratio * 100.0
        );
        
        // 使用Span::raw而不是Span::styled，不应用任何样式
        lines.push(Line::from(Span::raw(signal_text)));
    }
    
    // 添加空行和撤单信号标题
    lines.push(Line::from(Span::raw(""))); // 空行
    lines.push(Line::from(Span::raw("撤单信号:")));
    
    // 显示撤单信号
    for signal in cancel_signals.iter().rev() {
        // 使用标准库计算时间
        let time = std::time::UNIX_EPOCH + std::time::Duration::from_millis(signal.timestamp);
        let seconds = time.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let hours = (seconds / 3600) % 24;
        let minutes = (seconds / 60) % 60;
        let secs = seconds % 60;
        let formatted_time = format!("{:02}:{:02}:{:02}", hours, minutes, secs);
        
        // 创建信号文本，使用普通文字
        let signal_text = format!("[{formatted_time}] {}{:.2}%", 
            if signal.signal_type == "buy" { "撤单信号 [买入] - 占比: " } else { "撤单信号 [卖出] - 占比: " },
            signal.ratio * 100.0
        );
        
        lines.push(Line::from(Span::raw(signal_text)));
    }
    
    // 创建Text对象
    let text = Text::from(lines);
    
    // 创建段落
    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    
    f.render_widget(paragraph, area);
}

// 渲染订单动能信号
fn render_order_momentum(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title("Order Momentum")
        .borders(Borders::ALL);
    
    let text = "Order Momentum Signal";
    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    
    f.render_widget(paragraph, area);
}

// 渲染冰山订单信号
fn render_iceberg_orders(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title("大订单")
        .borders(Borders::ALL);
    
    let inner_area = block.inner(area);
    
    // 获取大订单信息
    let big_orders = {
        let orderbook = app.orderbook.lock();
        orderbook.big_orders.clone()
    };
    
    // 创建Text对象和Line列表
    let mut lines = Vec::new();
    
    // 将大订单按BTC数量排序（从大到小）
    let mut orders: Vec<_> = big_orders.iter().collect();
    orders.sort_by(|a, b| b.1.volume.partial_cmp(&a.1.volume).unwrap_or(std::cmp::Ordering::Equal));
    
    // 显示大订单信息
    for (price, order) in orders.iter().take(20) { // 限制显示最大的20条
        // 使用标准库计算时间
        let time = std::time::UNIX_EPOCH + std::time::Duration::from_millis(order.timestamp);
        let seconds = time.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let hours = (seconds / 3600) % 24;
        let minutes = (seconds / 60) % 60;
        let secs = seconds % 60;
        let formatted_time = format!("{:02}:{:02}:{:02}", hours, minutes, secs);
        
        // 创建信号文本
        let signal_text = format!("[{formatted_time}] 价格:{} 大订单挂单{} {:.2} 个 BTC", 
            price,
            if order.order_type == "buy" { "买入" } else { "卖出" },
            order.volume
        );
        
        lines.push(Line::from(Span::raw(signal_text)));
    }
    
    // 创建Text对象
    let text = Text::from(lines);
    
    // 创建段落
    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    
    f.render_widget(paragraph, area);
}



// WebSocket消息处理 - 修改为接受symbol参数
async fn handle_websocket_messages(orderbook: Arc<Mutex<OrderBookData>>, symbol: String) -> Result<(), Box<dyn std::error::Error>> {
    // 将symbol转换为小写用于WebSocket URL
    let symbol_lower = symbol.to_lowercase();
  
    let testnet_baseurl = "fstream.binancefuture.com";
    let live_baseurl = "fstream.binance.com";

    // 动态构建WebSocket URL
    let url_string = format!(
        "wss://{}/stream?streams={}@depth20@100ms/{}@aggTrade",
        live_baseurl, symbol_lower, symbol_lower
    );
    
    let url = Url::parse(&url_string)?;
    let (ws_stream, _) = connect_async(url).await?;
    let (_, mut read) = ws_stream.split();
    
    while let Some(msg) = read.next().await {
        match msg? {
            Message::Text(text) => {
                if let Ok(data) = serde_json::from_str::<Value>(&text) {
                    if let Some(stream) = data["stream"].as_str() {
                        if let Some(event_data) = data["data"].as_object() {
                            let event_value = serde_json::Value::Object(event_data.clone());
                            
                            if stream.contains("depth") {
                                let mut orderbook_guard = orderbook.lock();
                                orderbook_guard.update(&event_value);
                            } else if stream.contains("aggTrade") {
                                let mut orderbook_guard = orderbook.lock();
                                orderbook_guard.add_trade(&event_value);
                            }
                        }
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    // 读取环境变量SYMBOL，默认为BTCUSDT
    let symbol = env::var("SYMBOL").unwrap_or_else(|_| "BTCUSDT".to_string());
    
    // 验证symbol格式（基本验证）
    if symbol.is_empty() {
        eprintln!("Error: SYMBOL cannot be empty");
        std::process::exit(1);
    }
    
    // 设置终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // 创建应用状态
    let mut app = App::new();
    
    // 初始化深度数据
    {
        let mut orderbook_guard = app.orderbook.lock();
        println!("正在初始化深度数据...");
        if let Err(e) = orderbook_guard.initialize_depth_data(&symbol).await {
            eprintln!("初始化深度数据失败: {}", e);
            // 继续执行，不中断程序
        } else {
            println!("深度数据初始化完成！");
        }
    }
    
    let orderbook_clone = app.orderbook.clone();
    let symbol_clone = symbol.clone();
    
    // 启动WebSocket处理任务
    tokio::spawn(async move {
        if let Err(e) = handle_websocket_messages(orderbook_clone, symbol_clone).await {
            log::error!("WebSocket error: {}", e);
        }
    });
    
    // 主事件循环
    let timeout = Duration::from_millis(0);
    loop {
        terminal.draw(|f| ui(f, &mut app))?;
        
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                        if let KeyCode::Char('c') = key.code {
                            break;
                        }
                    }
                    
                    match key.code {
                        KeyCode::Up => app.scroll_up(),
                        KeyCode::Down => app.scroll_down(),
                        KeyCode::PageUp => app.page_up(),
                        KeyCode::PageDown => app.page_down(),
                        KeyCode::Home => app.home(),
                        KeyCode::Char('q') => break,
                        _ => {}
                    }
                }
            }
        }
        
        if app.should_quit {
            break;
        }
    }
    
    // 清理终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    Ok(())
}
