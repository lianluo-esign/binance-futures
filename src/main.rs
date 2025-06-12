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
    
    // 新增市场微观结构分析器
    microstructure_analyzer: MarketMicrostructureAnalyzer,
    
    // 新增字段
    stable_highlight_price: Option<f64>,
    stable_highlight_side: Option<String>,
    last_trade_price: Option<f64>,
    highlight_start_time: Option<u64>,
    highlight_duration: u64,
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
            microstructure_analyzer: MarketMicrostructureAnalyzer::new(
                0.95,    // imbalance_threshold
                1.0,    // min_volume_threshold
                2.0,    // iceberg_volume_ratio
                3,      // iceberg_replenish_threshold
                1000,   // iceberg_window_ms
            ),
            stable_highlight_price: None,
            stable_highlight_side: None,
            last_trade_price: None,
            highlight_start_time: None,
            highlight_duration: 3000,
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
        
        if let Some(bids) = data["b"].as_array() {
            for bid in bids {
                if let (Some(price_str), Some(qty)) = (bid[0].as_str(), bid[1].as_str()) {
                    let price = price_str.parse::<f64>().unwrap_or(0.0);
                    let price_ordered = OrderedFloat(price);
                    let qty_f64 = qty.parse::<f64>().unwrap_or(0.0);
                    
                    // 获取或创建该价格的OrderFlow
                    let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                    
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
                    
                    // 清理同价格上的ask挂单量
                    if order_flow.bid_ask.ask > 0.0 {
                        cancellations.push((price, "ask".to_string(), order_flow.bid_ask.ask));
                        order_flow.bid_ask.ask = 0.0;
                    }
                }
            }
        }
        
        if let Some(asks) = data["a"].as_array() {
            for ask in asks {
                if let (Some(price_str), Some(qty)) = (ask[0].as_str(), ask[1].as_str()) {
                    let price = price_str.parse::<f64>().unwrap_or(0.0);
                    let price_ordered = OrderedFloat(price);
                    let qty_f64 = qty.parse::<f64>().unwrap_or(0.0);
                    
                    // 获取或创建该价格的OrderFlow
                    let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                    
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
                    
                    // 清理同价格上的bid挂单量
                    if order_flow.bid_ask.bid > 0.0 {
                        cancellations.push((price, "bid".to_string(), order_flow.bid_ask.bid));
                        order_flow.bid_ask.bid = 0.0;
                    }
                }
            }
        }
        
        // 处理收集的撤单信息
        for (price, side, volume) in cancellations {
            self.detect_cancellation(price, &side, volume);
        }
        
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
        
        self.clean_old_trades();
        self.clean_old_cancels();
        
        // 自动清理不合理的挂单数据
        self.auto_clean_unreasonable_orders();
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
        
        // // 调试信息：打印清理统计
        // if cleaned_count > 0 {
        //     eprintln!("清理了 {} 个不合理挂单，best_bid: {:.2}, best_ask: {:.2}", 
        //              cleaned_count, best_bid_price, best_ask_price);
        // }
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
        
        // 第一行：实时挂单量比率色条
        let (bid_ratio, ask_ratio) = self.microstructure_analyzer.get_current_orderbook_ratio();
        let bid_percentage = (bid_ratio * 100.0) as u32;
        let ask_percentage = (ask_ratio * 100.0) as u32;
        
        // 创建动态字符条显示 - 固定50个字符
        let total_blocks = 50; // 总字符数量固定为50个
        
        // 确保比率总和为1.0，避免浮点数精度问题
        let total_ratio = bid_ratio + ask_ratio;
        if total_ratio > 0.0 {
            let normalized_bid_ratio = bid_ratio / total_ratio;
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
        } else {
            signals.push("Waiting...".to_string());
        }
        
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
        
        if signals.len() == 1 {
            signals.push("Waiting...".to_string());
        }
        
        signals.join("\n")
    }
}


// 市场微观结构分析器
impl MarketMicrostructureAnalyzer {
    fn new(
        imbalance_threshold: f64,
        min_volume_threshold: f64,
        iceberg_volume_ratio: f64,
        iceberg_replenish_threshold: u32,
        iceberg_window_ms: u64,
    ) -> Self {
        Self {
            imbalance_threshold,
            min_volume_threshold,
            iceberg_volume_ratio,
            iceberg_replenish_threshold,
            iceberg_window_ms,
            momentum_consumption_threshold: 0.95,
            momentum_window_size: 2,
            momentum_order_size_threshold: 1.0,
            last_best_bid: None,
            last_best_ask: None,
            last_bid_volume: 0.0,
            last_ask_volume: 0.0,
            tick_history: Vec::new(),
            momentum_signals: Vec::new(),
            current_momentum_signal: None,
            consecutive_buy_count: 0,
            consecutive_sell_count: 0,
            detected_imbalances: Vec::new(),
            detected_icebergs: Vec::new(),
            current_bid_ratio: 0.5,
            current_ask_ratio: 0.5,
            current_imbalance_signal: None,
            recent_imbalance_signals: Vec::new(),
            imbalance_window_ms: 1000,  // 1秒窗口
            bullish_threshold: 0.8,     // 80%阈值
            bearish_threshold: 0.8,     // 80%阈值
            last_trend_signal: None,
            trend_signal_timestamp: None,
            trend_signal_duration_ms: 5000,  // 5秒显示时间
        }
    }
    
    // 实时流动性失衡检测 - 基于挂单量比率
    fn detect_liquidity_imbalance(&mut self, 
        best_bid: Option<f64>, 
        best_ask: Option<f64>,
        bid_volume: f64,
        ask_volume: f64,
        _trade_price: f64,
        trade_volume: f64,
        _trade_side: &str) -> Option<LiquidityImbalance> {
        
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 计算挂单量比率
        let total_volume = bid_volume + ask_volume;
        if total_volume <= 0.0 {
            return None;
        }
        
        // 更新当前比率
        self.current_bid_ratio = bid_volume / total_volume;
        self.current_ask_ratio = ask_volume / total_volume;
        
        // 检查是否触发失衡信号
        let mut imbalance_detected = None;
        
        if self.current_bid_ratio >= self.imbalance_threshold {
            // 买盘失衡（做多信号）
            imbalance_detected = Some(LiquidityImbalance {
                timestamp: current_time,
                imbalance_type: "bullish".to_string(),
                imbalance_ratio: self.current_bid_ratio,
                consumed_volume: trade_volume,
            });
        } else if self.current_ask_ratio >= self.imbalance_threshold {
            // 卖盘失衡（做空信号）
            imbalance_detected = Some(LiquidityImbalance {
                timestamp: current_time,
                imbalance_type: "bearish".to_string(),
                imbalance_ratio: self.current_ask_ratio,
                consumed_volume: trade_volume,
            });
        }
        
        // 更新当前失衡信号状态
        self.current_imbalance_signal = imbalance_detected.clone();
        
        // 更新历史状态
        self.last_best_bid = best_bid;
        self.last_best_ask = best_ask;
        self.last_bid_volume = bid_volume;
        self.last_ask_volume = ask_volume;
        
        // 如果检测到失衡，添加到记录中
        if let Some(ref imbalance) = imbalance_detected {
            self.detected_imbalances.push(imbalance.clone());
            
            // 限制记录数量，只保留最近的信号
            if self.detected_imbalances.len() > 10 {
                self.detected_imbalances.remove(0);
            }
            
            // 添加到最近1秒失衡信号统计
            self.recent_imbalance_signals.push(imbalance.clone());
        }
        
        // 清理超过时间窗口的失衡信号
        self.clean_old_imbalance_signals(current_time);
        
        // 分析最近1秒内的失衡趋势
        self.analyze_imbalance_trend();
        
        imbalance_detected
    }
    
    // 冰山订单检测
    fn detect_iceberg_order(&mut self,
        best_bid: Option<f64>,
        best_ask: Option<f64>,
        bid_volume: f64,
        ask_volume: f64,
        trade_volume: f64,
        trade_side: &str) -> Option<IcebergOrder> {
        
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 检测买盘冰山订单 (在best_bid持续补充)
        if let Some(bid_price) = best_bid {
            if trade_side == "sell" && bid_volume > self.last_bid_volume {
                let replenish_volume = bid_volume - self.last_bid_volume;
                
                // 检查是否满足冰山订单条件
                if replenish_volume > trade_volume * self.iceberg_volume_ratio {
                    // 查找或创建冰山订单记录
                    let mut found_iceberg = false;
                    for iceberg in &mut self.detected_icebergs {
                        if iceberg.side == "bid" && 
                           (iceberg.price - bid_price).abs() < 0.000001 &&
                           current_time - iceberg.timestamp < self.iceberg_window_ms {
                            iceberg.accumulated_volume += replenish_volume;
                            iceberg.replenish_count += 1;
                            iceberg.timestamp = current_time;
                            iceberg.signal_strength = iceberg.accumulated_volume / (current_time - iceberg.timestamp + 1) as f64;
                            found_iceberg = true;
                            
                            if iceberg.replenish_count >= self.iceberg_replenish_threshold {
                                return Some(iceberg.clone());
                            }
                            break;
                        }
                    }
                    
                    if !found_iceberg {
                        let new_iceberg = IcebergOrder {
                            timestamp: current_time,
                            side: "bid".to_string(),
                            price: bid_price,
                            accumulated_volume: replenish_volume,
                            replenish_count: 1,
                            signal_strength: replenish_volume,
                        };
                        self.detected_icebergs.push(new_iceberg);
                    }
                }
            }
        }
        
        // 检测卖盘冰山订单 (在best_ask持续补充)
        if let Some(ask_price) = best_ask {
            if trade_side == "buy" && ask_volume > self.last_ask_volume {
                let replenish_volume = ask_volume - self.last_ask_volume;
                
                if replenish_volume > trade_volume * self.iceberg_volume_ratio {
                    let mut found_iceberg = false;
                    for iceberg in &mut self.detected_icebergs {
                        if iceberg.side == "ask" && 
                           (iceberg.price - ask_price).abs() < 0.000001 &&
                           current_time - iceberg.timestamp < self.iceberg_window_ms {
                            iceberg.accumulated_volume += replenish_volume;
                            iceberg.replenish_count += 1;
                            iceberg.timestamp = current_time;
                            iceberg.signal_strength = iceberg.accumulated_volume / (current_time - iceberg.timestamp + 1) as f64;
                            found_iceberg = true;
                            
                            if iceberg.replenish_count >= self.iceberg_replenish_threshold {
                                return Some(iceberg.clone());
                            }
                            break;
                        }
                    }
                    
                    if !found_iceberg {
                        let new_iceberg = IcebergOrder {
                            timestamp: current_time,
                            side: "ask".to_string(),
                            price: ask_price,
                            accumulated_volume: replenish_volume,
                            replenish_count: 1,
                            signal_strength: replenish_volume,
                        };
                        self.detected_icebergs.push(new_iceberg);
                    }
                }
            }
        }
        
        // 清理过期的冰山订单记录
        self.detected_icebergs.retain(|iceberg| {
            current_time - iceberg.timestamp < self.iceberg_window_ms * 2
        });
        
        None
    }
    
    // 获取当前流动性失衡状态
    fn get_current_imbalance_signals(&self) -> Vec<&LiquidityImbalance> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        self.detected_imbalances
            .iter()
            .filter(|imbalance| current_time - imbalance.timestamp < 5000) // 5秒内的信号
            .collect()
    }
    
    // 获取当前冰山订单信号
    fn get_current_iceberg_signals(&self) -> Vec<&IcebergOrder> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        self.detected_icebergs
            .iter()
            .filter(|iceberg| {
                current_time - iceberg.timestamp < self.iceberg_window_ms &&
                iceberg.replenish_count >= self.iceberg_replenish_threshold
            })
            .collect()
    }
    
    // 新增：获取当前挂单量比率
    fn get_current_orderbook_ratio(&self) -> (f64, f64) {
        (self.current_bid_ratio, self.current_ask_ratio)
    }
    
    // 新增：获取当前失衡信号
    fn get_current_imbalance_signal(&self) -> Option<&LiquidityImbalance> {
        self.current_imbalance_signal.as_ref()
    }
    
    // 清理超过时间窗口的失衡信号
    fn clean_old_imbalance_signals(&mut self, current_time: u64) {
        self.recent_imbalance_signals.retain(|signal| {
            current_time - signal.timestamp <= self.imbalance_window_ms
        });
    }
    
    // 分析最近1秒内的失衡趋势
    fn analyze_imbalance_trend(&mut self) {
        if self.recent_imbalance_signals.is_empty() {
            return;
        }
        
        let total_signals = self.recent_imbalance_signals.len();
        let bullish_count = self.recent_imbalance_signals.iter()
            .filter(|signal| signal.imbalance_type == "bullish")
            .count();
        let bearish_count = total_signals - bullish_count;
        
        let bullish_ratio = bullish_count as f64 / total_signals as f64;
        let bearish_ratio = bearish_count as f64 / total_signals as f64;
        
        // 判断是否达到80%阈值
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
            
        if bullish_ratio >= self.bullish_threshold {
            if self.last_trend_signal.as_ref() != Some(&"bullish".to_string()) {
                self.trend_signal_timestamp = Some(current_time);
            }
            self.last_trend_signal = Some("bullish".to_string());
        } else if bearish_ratio >= self.bearish_threshold {
            if self.last_trend_signal.as_ref() != Some(&"bearish".to_string()) {
                self.trend_signal_timestamp = Some(current_time);
            }
            self.last_trend_signal = Some("bearish".to_string());
        }
    }
    
    // 获取最近的趋势信号（检查5秒过期）
    fn get_trend_signal(&mut self) -> Option<String> {
        if let (Some(_), Some(timestamp)) = (&self.last_trend_signal, self.trend_signal_timestamp) {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
                
            // 检查信号是否已过期（5秒）
            if current_time - timestamp > self.trend_signal_duration_ms {
                self.last_trend_signal = None;
                self.trend_signal_timestamp = None;
                return None;
            }
            
            self.last_trend_signal.clone()
        } else {
            None
        }
    }
    
    // 订单动能检测 - 监控主动订单对被动订单的瞬时消耗
    fn detect_order_momentum(&mut self, 
        trade_price: f64,
        trade_volume: f64,
        trade_side: &str,
        best_bid: f64,
        best_ask: f64,
        bid_volume: f64,
        ask_volume: f64) -> Option<OrderMomentum> {
        
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 创建当前tick数据
        let current_tick = TickData {
            timestamp: current_time,
            trade_price,
            trade_volume,
            trade_side: trade_side.to_string(),
            best_bid,
            best_ask,
            bid_volume,
            ask_volume,
        };
        
        // 添加到历史记录
        self.tick_history.push(current_tick.clone());
        
        // 保持窗口大小
        if self.tick_history.len() > self.momentum_window_size {
            self.tick_history.remove(0);
        }
        
        // 需要至少2个tick才能进行分析
        if self.tick_history.len() < 2 {
            return None;
        }
        
        let previous_tick = &self.tick_history[self.tick_history.len() - 2];
        let current_tick = &self.tick_history[self.tick_history.len() - 1];
        
        let mut momentum_detected = None;
        
        match current_tick.trade_side.as_str() {
            "buy" => {
                // 主动买单，检查best ask的流动性消耗
                if previous_tick.ask_volume > 0.0 {
                    let consumption_ratio = 1.0 - (current_tick.ask_volume / previous_tick.ask_volume);
                    
                    if consumption_ratio >= self.momentum_consumption_threshold && current_tick.trade_volume >= self.momentum_order_size_threshold {
                        // 检测到买单冲击
                        self.consecutive_buy_count += 1;
                        self.consecutive_sell_count = 0;
                        
                        let momentum_type = if self.consecutive_buy_count >= 2 {
                            "buy_positive".to_string()
                        } else {
                            "buy".to_string()
                        };
                        
                        momentum_detected = Some(OrderMomentum {
                            timestamp: current_time,
                            momentum_type,
                            trade_volume: current_tick.trade_volume,
                            liquidity_consumed: previous_tick.ask_volume - current_tick.ask_volume,
                            consumption_ratio,
                            signal_strength: consumption_ratio,
                        });
                    }
                }
            },
            "sell" => {
                // 主动卖单，检查best bid的流动性消耗
                if previous_tick.bid_volume > 0.0 {
                    let consumption_ratio = 1.0 - (current_tick.bid_volume / previous_tick.bid_volume);
                    
                    if consumption_ratio >= self.momentum_consumption_threshold && current_tick.trade_volume >= self.momentum_order_size_threshold {
                        // 检测到卖单冲击
                        self.consecutive_sell_count += 1;
                        self.consecutive_buy_count = 0;
                        
                        let momentum_type = if self.consecutive_sell_count >= 2 {
                            "sell_positive".to_string()
                        } else {
                            "sell".to_string()
                        };
                        
                        momentum_detected = Some(OrderMomentum {
                            timestamp: current_time,
                            momentum_type,
                            trade_volume: current_tick.trade_volume,
                            liquidity_consumed: previous_tick.bid_volume - current_tick.bid_volume,
                            consumption_ratio,
                            signal_strength: consumption_ratio,
                        });
                    }
                }
            },
            _ => {}
        }
        
        // 更新当前动能信号
        self.current_momentum_signal = momentum_detected.clone();
        
        // 如果检测到动能，添加到历史记录
        if let Some(ref momentum) = momentum_detected {
            self.momentum_signals.push(momentum.clone());
            
            // 限制历史记录数量
            if self.momentum_signals.len() > 20 {
                self.momentum_signals.remove(0);
            }
        }
        
        momentum_detected
    }
    
    // 获取当前动能信号 - 3秒后自动消失
    fn get_current_momentum_signal(&self) -> Option<&OrderMomentum> {
        if let Some(ref signal) = self.current_momentum_signal {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            // 检查信号是否超过3秒（3000毫秒）
            if current_time - signal.timestamp <= 3000 {
                Some(signal)
            } else {
                None
            }
        } else {
            None
        }
    }
    
    // 获取最近的动能信号
    fn get_recent_momentum_signals(&self) -> Vec<&OrderMomentum> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        self.momentum_signals
            .iter()
            .filter(|momentum| current_time - momentum.timestamp < 10000) // 10秒内的信号
            .collect()
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
                let is_at_best_bid = best_bid.map_or(false, |bb| (price - bb).abs() < 0.000001);
                let is_at_best_ask = best_ask.map_or(false, |ba| (price - ba).abs() < 0.000001);
                
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
    
    // 渲染三个信号区域
    render_orderbook_imbalance(f, app, imbalance_area);
    render_order_momentum(f, app, momentum_area);
    render_iceberg_orders(f, app, iceberg_area);
}

// 渲染订单簿失衡信号
fn render_orderbook_imbalance(f: &mut Frame, app: &mut App, area: Rect) {
    let signals = {
        let mut orderbook = app.orderbook.lock();
        orderbook.get_market_signals()
    };
    
    let block = Block::default()
        .title("📊 Orderbook Imbalance")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Green));
    
    let paragraph = Paragraph::new(signals)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    
    f.render_widget(paragraph, area);
}

// 渲染订单动能信号（占位符）
fn render_order_momentum(f: &mut Frame, app: &mut App, area: Rect) {
    let signals = {
        let orderbook = app.orderbook.lock();
        let current_momentum = orderbook.microstructure_analyzer.get_current_momentum_signal();
        let recent_signals = orderbook.microstructure_analyzer.get_recent_momentum_signals();
        
        let mut signal_lines = Vec::new();
        
        // 显示当前动能信号（3秒内有效）
        if let Some(momentum) = current_momentum {
            let signal_text = match momentum.momentum_type.as_str() {
                "buy" => format!("🟢 Buy Orders({:.2}) Momentum", momentum.trade_volume),
                "sell" => format!("🔴 Sell Orders({:.2}) Momentum", momentum.trade_volume),
                "buy_positive" => format!("🟢🟢 Buy Positive Momentum ({:.2})", momentum.trade_volume),
                "sell_positive" => format!("🔴🔴 Sell Positive Momentum ({:.2})", momentum.trade_volume),
                _ => format!("⚡ Unknown Momentum"),
            };
            
            signal_lines.push(signal_text);
            signal_lines.push(format!("消耗比例: {:.1}%", momentum.consumption_ratio * 100.0));
            signal_lines.push(format!("流动性消耗: {:.2}", momentum.liquidity_consumed));
        }
        
        // 显示历史信号（每个信号换行显示）
        if !recent_signals.is_empty() {
            if !signal_lines.is_empty() {
                signal_lines.push("".to_string());
            }
            
            // 显示最近的5个信号，每个信号一行
            for signal in recent_signals.iter().rev().take(5) {
                let signal_text = match signal.momentum_type.as_str() {
                    "buy" => format!("🟢 买单冲击 ({:.2})", signal.trade_volume),
                    "sell" => format!("🔴 卖单冲击 ({:.2})", signal.trade_volume),
                    "buy_positive" => format!("🟢🟢 买单积极 ({:.2})", signal.trade_volume),
                    "sell_positive" => format!("🔴🔴 卖单积极 ({:.2})", signal.trade_volume),
                    _ => format!("⚡ 未知信号 ({:.2})", signal.trade_volume),
                };
                signal_lines.push(signal_text);
            }
        }
        
        signal_lines.join("\n")
    };
    
    let block = Block::default()
        .title("⚡ Order Momentum")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Blue));
    
    let paragraph = Paragraph::new(signals)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    
    f.render_widget(paragraph, area);
}

// 渲染冰山订单信号（占位符）
fn render_iceberg_orders(f: &mut Frame, app: &mut App, area: Rect) {
    let signals = {
        let orderbook = app.orderbook.lock();
        let icebergs = orderbook.microstructure_analyzer.get_current_iceberg_signals();
        
        if icebergs.is_empty() {
            "暂无冰山订单检测".to_string()
        } else {
            icebergs.iter()
                .map(|iceberg| {
                    format!(
                        "🧊{}冰山 {:.2} ({}次补充)",
                        if iceberg.side == "bid" { "买盘" } else { "卖盘" },
                        iceberg.accumulated_volume,
                        iceberg.replenish_count
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    };
    
    let block = Block::default()
        .title("🧊 Iceberg Orders")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    
    let paragraph = Paragraph::new(signals)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    
    f.render_widget(paragraph, area);
}

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
