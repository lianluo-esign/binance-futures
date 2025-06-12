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
    widgets::{Block, Borders, Cell, Row, Table, Paragraph, Wrap},
    Frame, Terminal,
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    env,  // 新增：用于读取环境变量
    io,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;
// 新增：引入reqwest用于HTTP请求
use reqwest;

// 注释掉市场微观结构分析相关的数据结构
/*
// 流动性失衡检测结构
#[derive(Debug, Clone)]
struct LiquidityImbalance {
    timestamp: u64,
    imbalance_type: String,  // "bullish" or "bearish"
    imbalance_ratio: f64,    // 失衡百分比
    consumed_volume: f64,    // 消耗的流动性量
    // price_level: f64,        // 发生失衡的价格
}

// 订单动能检测结构
#[derive(Debug, Clone)]
struct OrderMomentum {
    timestamp: u64,
    momentum_type: String,   // "buy", "sell", "buy_positive", "sell_positive"
    trade_volume: f64,       // 主动订单成交量
    liquidity_consumed: f64, // 被动订单消耗量
    consumption_ratio: f64,  // 消耗比例
    signal_strength: f64,    // 信号强度
}

// Tick数据结构
#[derive(Debug, Clone)]
struct TickData {
    timestamp: u64,
    trade_price: f64,
    trade_volume: f64,
    trade_side: String,      // "buy" or "sell"
    best_bid: f64,
    best_ask: f64,
    bid_volume: f64,
    ask_volume: f64,
}

// 冰山订单检测结构
#[derive(Debug, Clone)]
struct IcebergOrder {
    timestamp: u64,
    side: String,            // "bid" or "ask"
    price: f64,
    accumulated_volume: f64, // 累积的冰山订单量
    replenish_count: u32,    // 补充次数
    signal_strength: f64,    // 信号强度
}

// 市场微观结构分析器
struct MarketMicrostructureAnalyzer {
    // 流动性失衡检测参数
    imbalance_threshold: f64,           // 失衡阈值 (默认 0.7 = 70%)
    min_volume_threshold: f64,          // 最小成交量阈值
    
    // 冰山订单检测参数
    iceberg_volume_ratio: f64,          // 冰山订单量比例阈值
    iceberg_replenish_threshold: u32,   // 冰山订单补充次数阈值
    iceberg_window_ms: u64,             // 冰山订单检测窗口
    
    // 订单动能检测参数
    momentum_consumption_threshold: f64, // 流动性消耗阈值 (默认 0.95 = 95%)
    momentum_window_size: usize,        // Tick窗口大小 (默认 2)
    momentum_order_size_threshold: f64, // 订单大小阈值 (默认 1.0)
    
    // 状态跟踪
    last_best_bid: Option<f64>,
    last_best_ask: Option<f64>,
    last_bid_volume: f64,
    last_ask_volume: f64,
    
    // 订单动能状态跟踪
    tick_history: Vec<TickData>,        // 最近的Tick数据
    momentum_signals: Vec<OrderMomentum>, // 动能信号历史
    current_momentum_signal: Option<OrderMomentum>, // 当前动能信号
    consecutive_buy_count: u32,         // 连续买单计数
    consecutive_sell_count: u32,        // 连续卖单计数
    
    // 检测结果存储
    detected_imbalances: Vec<LiquidityImbalance>,
    detected_icebergs: Vec<IcebergOrder>,
    
    // 新增：当前挂单量比率状态
    current_bid_ratio: f64,
    current_ask_ratio: f64,
    current_imbalance_signal: Option<LiquidityImbalance>,
    
    // 新增：最近1秒失衡信号统计
    recent_imbalance_signals: Vec<LiquidityImbalance>,  // 最近1秒内的失衡信号
    imbalance_window_ms: u64,                          // 失衡信号统计窗口（毫秒）
    bullish_threshold: f64,                            // 多头信号阈值（默认0.8 = 80%）
    bearish_threshold: f64,                            // 空头信号阈值（默认0.8 = 80%）
    last_trend_signal: Option<String>,                 // 最后的趋势信号（"bullish" 或 "bearish"）
    trend_signal_timestamp: Option<u64>,               // 趋势信号的时间戳
    trend_signal_duration_ms: u64,                     // 趋势信号显示持续时间（毫秒）
}
*/

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
    bid_cancel: f64,
    ask_cancel: f64,
    timestamp: u64,
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
    
    // 新增：最优买卖价格
    best_bid_price: Option<f64>,
    best_ask_price: Option<f64>,
}

impl OrderFlow {
    fn new() -> Self {
        Self {
            bid_ask: PriceLevel { bid: 0.0, ask: 0.0 },
            history_trade_record: TradeRecord { buy_volume: 0.0, sell_volume: 0.0, timestamp: 0 },
            realtime_trade_record: TradeRecord { buy_volume: 0.0, sell_volume: 0.0, timestamp: 0 },
            realtime_cancel_records: CancelRecord { bid_cancel: 0.0, ask_cancel: 0.0, timestamp: 0 },
            best_bid_price: None,
            best_ask_price: None,
        }
    }
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
            // 注释掉微观结构分析器初始化
            /*
            microstructure_analyzer: MarketMicrostructureAnalyzer::new(
                0.95,    // imbalance_threshold
                1.0,    // min_volume_threshold
                2.0,    // iceberg_volume_ratio
                3,      // iceberg_replenish_threshold
                1000,   // iceberg_window_ms
            ),
            */
            stable_highlight_price: None,
            stable_highlight_side: None,
            last_trade_price: None,
            highlight_start_time: None,
            highlight_duration: 3000,
            last_update_id: None,
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
            
            // 注释掉市场微观结构分析调用
            /*
            // 获取当前最佳买卖价和挂单量
            let (best_bid, best_ask) = self.get_best_bid_ask();
            let (bid_volume, ask_volume) = self.get_best_volumes();
            
            // 检测流动性失衡
            if let Some(_imbalance) = self.microstructure_analyzer.detect_liquidity_imbalance(
                best_bid, best_ask, bid_volume, ask_volume, price, qty_f64, side
            ) {
                // println!("🚨 流动性失衡检测: {:?}", _imbalance);
            }
            
            // 检测冰山订单
            if let Some(_iceberg) = self.microstructure_analyzer.detect_iceberg_order(
                best_bid, best_ask, bid_volume, ask_volume, qty_f64, side
            ) {
                // println!("🧊 冰山订单检测: {:?}", _iceberg);
            }
            
            // 检测订单动能
            if let (Some(best_bid_price), Some(best_ask_price)) = (best_bid, best_ask) {
                if let Some(_momentum) = self.microstructure_analyzer.detect_order_momentum(
                    price, qty_f64, side, best_bid_price, best_ask_price, bid_volume, ask_volume
                ) {
                    // println!("⚡ 订单动能检测: {:?}", _momentum);
                }
            }
            */
            
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

    fn update(&mut self, data: &Value) {
        // 收集需要处理的撤单信息
        let mut cancellations = Vec::new();
        
        // 处理bids数组
        if let Some(bids) = data["b"].as_array() {
            // 先获取bids中的最优价格（价格最大的）
            let mut best_bid_price: Option<f64> = None;
            for bid in bids {
                if let Some(price_str) = bid[0].as_str() {
                    let price = price_str.parse::<f64>().unwrap_or(0.0);
                    if price > 0.0 {
                        if let Some(qty_str) = bid[1].as_str() {
                            let qty = qty_str.parse::<f64>().unwrap_or(0.0);
                            if qty > 0.0 {
                                best_bid_price = Some(best_bid_price.map_or(price, |current| current.max(price)));
                            }
                        }
                    }
                }
            }
            
            // 更新所有OrderFlow的best_bid_price
            for (_, order_flow) in self.order_flows.iter_mut() {
                order_flow.best_bid_price = best_bid_price;
            }
            
            // 如果有最优买价，清理所有大于最优买价的bid挂单
            if let Some(best_bid) = best_bid_price {
                let prices_to_clear: Vec<OrderedFloat<f64>> = self.order_flows
                    .iter()
                    .filter(|(price, order_flow)| {
                        price.into_inner() > best_bid && order_flow.bid_ask.bid > 0.0
                    })
                    .map(|(price, _)| *price)
                    .collect();
                
                for price in prices_to_clear {
                    if let Some(order_flow) = self.order_flows.get_mut(&price) {
                        if order_flow.bid_ask.bid > 0.0 {
                            cancellations.push((price.into_inner(), "bid".to_string(), order_flow.bid_ask.bid));
                            order_flow.bid_ask.bid = 0.0;
                        }
                    }
                }
            }
            
            // 然后更新bids的具体数量
            for bid in bids {
                if let (Some(price_str), Some(qty)) = (bid[0].as_str(), bid[1].as_str()) {
                    let price = price_str.parse::<f64>().unwrap_or(0.0);
                    let price_ordered = OrderedFloat(price);
                    let qty_f64 = qty.parse::<f64>().unwrap_or(0.0);
                    
                    // 获取或创建该价格的OrderFlow
                    let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                    order_flow.best_bid_price = best_bid_price;
                    
                    let old_bid = order_flow.bid_ask.bid;
                    
                    if qty_f64 == 0.0 {
                        if order_flow.bid_ask.bid > 0.0 {
                            cancellations.push((price, "bid".to_string(), order_flow.bid_ask.bid));
                        }
                        order_flow.bid_ask.bid = 0.0;
                    } else {
                        order_flow.bid_ask.bid = qty_f64;
                        if old_bid > qty_f64 {
                            cancellations.push((price, "bid".to_string(), old_bid - qty_f64));
                        }
                    }
                }
            }
        }
        
        // 处理asks数组
        if let Some(asks) = data["a"].as_array() {
            // 先获取asks中的最优价格（价格最小的）
            let mut best_ask_price: Option<f64> = None;
            for ask in asks {
                if let Some(price_str) = ask[0].as_str() {
                    let price = price_str.parse::<f64>().unwrap_or(0.0);
                    if price > 0.0 {
                        if let Some(qty_str) = ask[1].as_str() {
                            let qty = qty_str.parse::<f64>().unwrap_or(0.0);
                            if qty > 0.0 {
                                best_ask_price = Some(best_ask_price.map_or(price, |current| current.min(price)));
                            }
                        }
                    }
                }
            }
            
            // 更新所有OrderFlow的best_ask_price
            for (_, order_flow) in self.order_flows.iter_mut() {
                order_flow.best_ask_price = best_ask_price;
            }
            
            // 如果有最优卖价，清理所有小于最优卖价的ask挂单
            if let Some(best_ask) = best_ask_price {
                let prices_to_clear: Vec<OrderedFloat<f64>> = self.order_flows
                    .iter()
                    .filter(|(price, order_flow)| {
                        price.into_inner() < best_ask && order_flow.bid_ask.ask > 0.0
                    })
                    .map(|(price, _)| *price)
                    .collect();
                
                for price in prices_to_clear {
                    if let Some(order_flow) = self.order_flows.get_mut(&price) {
                        if order_flow.bid_ask.ask > 0.0 {
                            cancellations.push((price.into_inner(), "ask".to_string(), order_flow.bid_ask.ask));
                            order_flow.bid_ask.ask = 0.0;
                        }
                    }
                }
            }
            
            // 然后更新asks的具体数量
            for ask in asks {
                if let (Some(price_str), Some(qty)) = (ask[0].as_str(), ask[1].as_str()) {
                    let price = price_str.parse::<f64>().unwrap_or(0.0);
                    let price_ordered = OrderedFloat(price);
                    let qty_f64 = qty.parse::<f64>().unwrap_or(0.0);
                    
                    // 获取或创建该价格的OrderFlow
                    let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                    order_flow.best_ask_price = best_ask_price;
                    
                    let old_ask = order_flow.bid_ask.ask;
                    
                    if qty_f64 == 0.0 {
                        if order_flow.bid_ask.ask > 0.0 {
                            cancellations.push((price, "ask".to_string(), order_flow.bid_ask.ask));
                        }
                        order_flow.bid_ask.ask = 0.0;
                    } else {
                        order_flow.bid_ask.ask = qty_f64;
                        if old_ask > qty_f64 {
                            cancellations.push((price, "ask".to_string(), old_ask - qty_f64));
                        }
                    }
                }
            }
        }
        
        // 处理收集的撤单信息
        for (price, side, volume) in cancellations {
            self.detect_cancellation(price, &side, volume);
        }
        
        // 注释掉流动性不平衡检测
        /*
        // 在更新完订单簿后，立即计算挂单量比率
        if let (Some(best_bid), Some(best_ask)) = (self.get_best_bid(), self.get_best_ask()) {
            let (bid_volume, ask_volume) = self.get_best_volumes();
            
            // 调用失衡检测（不依赖交易，纯粹基于挂单量）
            self.microstructure_analyzer.detect_liquidity_imbalance(
                Some(best_bid),
                Some(best_ask),
                bid_volume,
                ask_volume,
                0.0,  // 无交易价格
                0.0,  // 无交易量
                ""    // 无交易方向
            );
        }
        */
        
        self.clean_old_trades();
        self.clean_old_cancels();
        
        // 自动清理不合理的挂单数据
        // self.auto_clean_unreasonable_orders();
    }
    
    // 使用 BTreeMap 的优势 - O(log n) 时间复杂度获取最佳买价
    fn get_best_bid(&self) -> Option<f64> {
        self.order_flows
            .iter()
            .rev()  // 从高到低遍历
            .find(|(_, order_flow)| order_flow.bid_ask.bid > 0.0)
            .map(|(price, _)| price.into_inner())
    }
    
    // 使用 BTreeMap 的优势 - O(log n) 时间复杂度获取最佳卖价
    fn get_best_ask(&self) -> Option<f64> {
        self.order_flows
            .iter()  // 从低到高遍历
            .find(|(_, order_flow)| order_flow.bid_ask.ask > 0.0)
            .map(|(price, _)| price.into_inner())
    }
    
    // 自动清理不合理的挂单数据
    fn auto_clean_unreasonable_orders(&mut self) {
        let best_bid = self.get_best_bid();
        let best_ask = self.get_best_ask();
        
        // 如果没有最佳买价或卖价，则不进行清理
        if best_bid.is_none() || best_ask.is_none() {
            return;
        }
        
        let best_bid_price = best_bid.unwrap();
        let best_ask_price = best_ask.unwrap();
        
        // 收集需要清理的价格
        let mut prices_to_clean = Vec::new();
        
        for (price, order_flow) in &self.order_flows {
            let price_val = price.into_inner();
            
            // 检查买单挂单：价格大于best_bid的买单挂单需要清理（不合理）
            if order_flow.bid_ask.bid > 0.0 && price_val > best_bid_price {
                prices_to_clean.push((price_val, "bid"));
            }
            
            // 检查卖单挂单：价格小于best_ask的卖单挂单需要清理（不合理）
            if order_flow.bid_ask.ask > 0.0 && price_val < best_ask_price {
                prices_to_clean.push((price_val, "ask"));
            }
        }
        
        // 执行清理
        let mut cleaned_count = 0;
        for (price, side) in prices_to_clean {
            let price_ordered = OrderedFloat(price);
            if let Some(order_flow) = self.order_flows.get_mut(&price_ordered) {
                match side {
                    "bid" => {
                        if order_flow.bid_ask.bid > 0.0 {
                            order_flow.bid_ask.bid = 0.0;
                            cleaned_count += 1;
                        }
                    },
                    "ask" => {
                        if order_flow.bid_ask.ask > 0.0 {
                            order_flow.bid_ask.ask = 0.0;
                            cleaned_count += 1;
                        }
                    },
                    _ => {}
                }
            }
        }
    }
    
    // 获取最佳买卖价
    fn get_best_bid_ask(&self) -> (Option<f64>, Option<f64>) {
        let mut best_bid = None;
        let mut best_ask = None;
        
        for (price, order_flow) in &self.order_flows {
            if order_flow.bid_ask.bid > 0.0 {
                if best_bid.is_none() || price.into_inner() > best_bid.unwrap() {
                    best_bid = Some(price.into_inner());
                }
            }
            if order_flow.bid_ask.ask > 0.0 {
                if best_ask.is_none() || price.into_inner() < best_ask.unwrap() {
                    best_ask = Some(price.into_inner());
                }
            }
        }
        
        (best_bid, best_ask)
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
    
    // 获取市场信号摘要
    fn get_market_signals(&mut self) -> String {
        let mut signals = Vec::new();
        
        // 获取买卖盘总量
        let bid_volume: f64 = self.order_flows.values().map(|of| of.bid_ask.bid).sum();
        let ask_volume: f64 = self.order_flows.values().map(|of| of.bid_ask.ask).sum();
        
        // 计算比率
        let ratio = if ask_volume > 0.0 { bid_volume / ask_volume } else { 1.0 };
        let bid_percentage = (bid_volume / (bid_volume + ask_volume) * 100.0) as u32;
        let ask_percentage = 100 - bid_percentage;
        
        // 创建动态字符条显示 - 固定50个字符
        let total_blocks = 50; // 总字符数量固定为50个
        
        // 确保比率总和为1.0，避免浮点数精度问题
        let normalized_bid_ratio = bid_volume / (bid_volume + ask_volume);
        let green_blocks = (normalized_bid_ratio * total_blocks as f64).round() as usize;
        let red_blocks = total_blocks - green_blocks;
        
        // 构建字符条：使用不同字符表示买卖盘
        let bid_bar = "▓".repeat(green_blocks);  // 买盘用深色块
        let ask_bar = "░".repeat(red_blocks);    // 卖盘用浅色块
        
        // 组合显示
        let char_bar = format!(
            "[{}{}] BID:{}% ASK:{}%",
            bid_bar,      // 买盘部分
            ask_bar,      // 卖盘部分
            bid_percentage,
            ask_percentage
        );
        
        signals.push(char_bar);
        
        // 基本信息
        signals.push(format!("当前价格: {:.2}", self.current_price.unwrap_or(0.0)));
        signals.push(format!("买卖盘比: {:.2}", ratio));
        signals.push(format!("买盘总量: {:.2}", bid_volume));
        signals.push(format!("卖盘总量: {:.2}", ask_volume));
        
        // 注释掉微观结构信号
        /*
        // 第二行：失衡信号（如果有）
        if let Some(current_signal) = self.microstructure_analyzer.get_current_imbalance_signal() {
            let signal_text = if current_signal.imbalance_type == "bullish" {
                format!("🟢Imbalance Buy Signal (BID{}%)", bid_percentage)
            } else {
                format!("🔴Imbalance Sell Signal (ASK{}%)", ask_percentage)
            };
            signals.push(signal_text);
        }
        
        // 第三行：最近1秒趋势信号（如果有）
        if let Some(trend_signal) = self.microstructure_analyzer.get_trend_signal() {
            let trend_text = if trend_signal == "bullish" {
                "\x1b[32m📈 1秒趋势: 多头信号 (80%+)\x1b[0m".to_string()  // 绿色
            } else {
                "\x1b[31m📉 1秒趋势: 空头信号 (80%+)\x1b[0m".to_string()  // 红色
            };
            signals.push(trend_text);
        }
        
        // 添加其他信号（冰山订单等）
        let icebergs = self.microstructure_analyzer.get_current_iceberg_signals();
        
        for iceberg in icebergs {
            signals.push(format!(
                "🧊{}冰山 {:.2} ({}次补充)",
                if iceberg.side == "bid" { "买盘" } else { "卖盘" },
                iceberg.accumulated_volume,
                iceberg.replenish_count
            ));
        }
        */
        
        signals.join("\n")
    }
    
    // 获取显示信号（用于UI）
    fn get_display_signals(&self) -> String {
        // 获取买卖盘总量
        let bid_volume: f64 = self.order_flows.values().map(|of| of.bid_ask.bid).sum();
        let ask_volume: f64 = self.order_flows.values().map(|of| of.bid_ask.ask).sum();
        
        // 计算比率
        let ratio = if ask_volume > 0.0 { bid_volume / ask_volume } else { 1.0 };
        
        // 创建买卖盘比例的可视化表示
        let max_bar_length = 20;
        let normalized_ratio = ratio.min(5.0) / 5.0;  // 将比率限制在0-5之间，然后归一化到0-1
        let bar_length = (normalized_ratio * max_bar_length as f64) as usize;
        
        let mut bar = String::new();
        for _ in 0..bar_length {
            bar.push('█');
        }
        for _ in bar_length..max_bar_length {
            bar.push('░');
        }
        
        // 基本信息
        let mut signals = vec![
            format!("当前价格: {:.2}", self.current_price.unwrap_or(0.0)),
            format!("买卖盘比: {:.2} | {}", ratio, bar),
            format!("买盘总量: {:.2}", bid_volume),
            format!("卖盘总量: {:.2}", ask_volume),
        ];
        
        signals.join("\n")
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
                    let price = price_str.parse::<f64>().unwrap_or(0.0);
                    let qty = qty_str.parse::<f64>().unwrap_or(0.0);
                    
                    if price > 0.0 && qty > 0.0 {
                        best_bid_price = Some(best_bid_price.map_or(price, |current| current.max(price)));
                        let price_ordered = OrderedFloat(price);
                        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                        order_flow.bid_ask.bid = qty;
                    }
                }
            }
        }
        
        // 处理卖单数据，找到最优卖价
        if let Some(asks) = depth_data["asks"].as_array() {
            for ask in asks {
                if let (Some(price_str), Some(qty_str)) = (ask[0].as_str(), ask[1].as_str()) {
                    let price = price_str.parse::<f64>().unwrap_or(0.0);
                    let qty = qty_str.parse::<f64>().unwrap_or(0.0);
                    
                    if price > 0.0 && qty > 0.0 {
                        best_ask_price = Some(best_ask_price.map_or(price, |current| current.min(price)));
                        let price_ordered = OrderedFloat(price);
                        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                        order_flow.bid_ask.ask = qty;
                    }
                }
            }
        }
        
        // 更新所有OrderFlow的最优价格
        for (_, order_flow) in self.order_flows.iter_mut() {
            order_flow.best_bid_price = best_bid_price;
            order_flow.best_ask_price = best_ask_price;
        }
        
        // 更新当前价格（取买卖盘中间价）
        if let (Some(best_bid), Some(best_ask)) = (best_bid_price, best_ask_price) {
            let mid_price = (best_bid + best_ask) / 2.0;
            self.update_current_price(mid_price);
        }
        
        log::info!("初始化深度数据完成，加载了{}个价格水平", self.order_flows.len());
        
        Ok(())
    }
}

// 注释掉MarketMicrostructureAnalyzer实现
/*
impl MarketMicrostructureAnalyzer {
    // ... 实现代码 ...
}
*/

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
                
                // 判断当前价格是否为best_bid或best_ask
                // let is_at_best_bid = best_bid.map_or(false, |bb| (price - bb).abs() < 0.000001);
                // let is_at_best_ask = best_ask.map_or(false, |ba| (price - ba).abs() < 0.000001);
                
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
                    format!("+{:.3}", sell_trade_vol) 
                } else { 
                    String::new() 
                };
                
                let buy_trade_str = if buy_trade_vol > 0.0 { 
                    format!("+{:.3}", buy_trade_vol) 
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
        ]);
    
    f.render_widget(table, centered_area);
    
    // 渲染市场信号区域
    let signals = {
        let orderbook = app.orderbook.lock();
        orderbook.get_display_signals()
    };
    
    let signal_block = Block::default()
        .title("Market Signals")
        .borders(Borders::ALL);
    
    let signal_paragraph = Paragraph::new(signals)
        .block(signal_block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    
    f.render_widget(signal_paragraph, signal_area);
    
    // 注释掉其他信号区域渲染
    /*
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
    */
}

// 注释掉其他渲染函数
/*
// 渲染订单簿失衡信号
fn render_orderbook_imbalance(f: &mut Frame, app: &mut App, area: Rect) {
    // ... 实现代码 ...
}

// 渲染订单动能信号（占位符）
fn render_order_momentum(f: &mut Frame, app: &mut App, area: Rect) {
    // ... 实现代码 ...
}

// 渲染冰山订单信号（占位符）
fn render_iceberg_orders(f: &mut Frame, app: &mut App, area: Rect) {
    // ... 实现代码 ...
}
*/



// WebSocket消息处理 - 修改为接受symbol参数
async fn handle_websocket_messages(orderbook: Arc<Mutex<OrderBookData>>, symbol: String) -> Result<(), Box<dyn std::error::Error>> {
    // 将symbol转换为小写用于WebSocket URL
    let symbol_lower = symbol.to_lowercase();
    
    // 动态构建WebSocket URL
    let url_string = format!(
        "wss://fstream.binance.com/stream?streams={}@depth20@100ms/{}@aggTrade",
        symbol_lower, symbol_lower
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
