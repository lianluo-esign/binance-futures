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
    
    // 状态跟踪
    last_best_bid: Option<f64>,
    last_best_ask: Option<f64>,
    last_bid_volume: f64,
    last_ask_volume: f64,
    
    // 检测结果存储
    detected_imbalances: Vec<LiquidityImbalance>,
    detected_icebergs: Vec<IcebergOrder>,
}


// 订单簿数据结构
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

// 订单簿数据管理 - 使用 BTreeMap<OrderedFloat<f64>, PriceLevel>
struct OrderBookData {
    price_levels: BTreeMap<OrderedFloat<f64>, PriceLevel>,
    current_price: Option<f64>,
    recent_trades: BTreeMap<OrderedFloat<f64>, TradeRecord>,
    last_trade_side: Option<String>,
    cancel_records: BTreeMap<OrderedFloat<f64>, CancelRecord>,
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
            price_levels: BTreeMap::new(),
            current_price: None,
            recent_trades: BTreeMap::new(),
            last_trade_side: None,
            cancel_records: BTreeMap::new(),
            trade_display_duration: 10000,
            cancel_display_duration: 5000,
            max_trade_records: 1000,
            max_cancel_records: 500,
            microstructure_analyzer: MarketMicrostructureAnalyzer::new(
                0.8,    // imbalance_threshold
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
                let keys_to_update: Vec<OrderedFloat<f64>> = self.price_levels
                    .range(..=trade_price_ordered)
                    .map(|(price, _)| *price)
                    .collect();
                
                for price in keys_to_update {
                    if let Some(level) = self.price_levels.get_mut(&price) {
                        level.ask = 0.0;
                    }
                }
            }
            "sell" => {
                // 卖单成交，清空价格大于等于成交价的所有bid挂单
                let keys_to_update: Vec<OrderedFloat<f64>> = self.price_levels
                    .range(trade_price_ordered..)
                    .map(|(price, _)| *price)
                    .collect();
                
                for price in keys_to_update {
                    if let Some(level) = self.price_levels.get_mut(&price) {
                        level.bid = 0.0;
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
            
            // 直接在这里清理不合理的挂单数据
            self.clear_unreasonable_orders(price, side);
            
            // 获取当前最佳买卖价和挂单量
            let (best_bid, best_ask) = self.get_best_bid_ask();
            let (bid_volume, ask_volume) = self.get_best_volumes();
            
            // 检测流动性失衡
            if let Some(imbalance) = self.microstructure_analyzer.detect_liquidity_imbalance(
                best_bid, best_ask, bid_volume, ask_volume, price, qty_f64, side
            ) {
                // println!("🚨 流动性失衡检测: {:?}", imbalance);
            }
            
            // 检测冰山订单
            if let Some(iceberg) = self.microstructure_analyzer.detect_iceberg_order(
                best_bid, best_ask, bid_volume, ask_volume, qty_f64, side
            ) {
                // println!("🧊 冰山订单检测: {:?}", iceberg);
            }
            
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            let trade = self.recent_trades.entry(price_ordered).or_insert(TradeRecord {
                buy_volume: 0.0,
                sell_volume: 0.0,
                timestamp: current_time,
            });
            
            match side {
                "buy" => trade.buy_volume += qty_f64,
                "sell" => trade.sell_volume += qty_f64,
                _ => {}
            }
            
            trade.timestamp = current_time;
        }
    }

    fn clean_old_trades(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 删除超过显示时间的成交记录
        self.recent_trades.retain(|_, trade| {
            current_time - trade.timestamp <= self.trade_display_duration
        });
        
        // 限制记录数量 - BTreeMap 天然有序，直接移除最旧的记录
        if self.recent_trades.len() > self.max_trade_records {
            let to_remove = self.recent_trades.len() - self.max_trade_records;
            let oldest_keys: Vec<OrderedFloat<f64>> = self.recent_trades
                .iter()
                .take(to_remove)
                .map(|(price, _)| *price)
                .collect();
            
            for price in oldest_keys {
                self.recent_trades.remove(&price);
            }
        }
    }

    fn detect_cancellation(&mut self, price: f64, side: &str, volume: f64) {
        let price_ordered = OrderedFloat(price);
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let cancel = self.cancel_records.entry(price_ordered).or_insert(CancelRecord {
            bid_cancel: 0.0,
            ask_cancel: 0.0,
            timestamp: current_time,
        });
        
        match side {
            "bid" => cancel.bid_cancel += volume,
            "ask" => cancel.ask_cancel += volume,
            _ => {}
        }
        
        cancel.timestamp = current_time;
    }

    fn clean_old_cancels(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 删除超过显示时间的撤单记录
        self.cancel_records.retain(|_, cancel| {
            current_time - cancel.timestamp <= self.cancel_display_duration
        });
        
        // 限制记录数量
        if self.cancel_records.len() > self.max_cancel_records {
            let to_remove = self.cancel_records.len() - self.max_cancel_records;
            let oldest_keys: Vec<OrderedFloat<f64>> = self.cancel_records
                .iter()
                .take(to_remove)
                .map(|(price, _)| *price)
                .collect();
            
            for price in oldest_keys {
                self.cancel_records.remove(&price);
            }
        }
    }

    fn get_trade_volume(&self, price: f64, side: &str) -> f64 {
        let price_ordered = OrderedFloat(price);
        if let Some(trade) = self.recent_trades.get(&price_ordered) {
            match side {
                "buy" => trade.buy_volume,
                "sell" => trade.sell_volume,
                _ => 0.0,
            }
        } else {
            0.0
        }
    }

    fn get_cancel_volume(&self, price: f64, side: &str) -> f64 {
        let price_ordered = OrderedFloat(price);
        if let Some(cancel) = self.cancel_records.get(&price_ordered) {
            match side {
                "bid" => cancel.bid_cancel,
                "ask" => cancel.ask_cancel,
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
                    
                    let old_bid = self.price_levels.get(&price_ordered)
                        .map(|level| level.bid)
                        .unwrap_or(0.0);
                    
                    let level = self.price_levels.entry(price_ordered).or_insert(PriceLevel {
                        bid: 0.0,
                        ask: 0.0,
                    });
                    
                    if qty_f64 == 0.0 {
                        if level.bid > 0.0 {
                            cancellations.push((price, "bid".to_string(), level.bid));
                        }
                        level.bid = 0.0;
                    } else {
                        level.bid = qty_f64;
                        if old_bid > qty_f64 {
                            cancellations.push((price, "bid".to_string(), old_bid - qty_f64));
                        }
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
                    
                    let old_ask = self.price_levels.get(&price_ordered)
                        .map(|level| level.ask)
                        .unwrap_or(0.0);
                    
                    let level = self.price_levels.entry(price_ordered).or_insert(PriceLevel {
                        bid: 0.0,
                        ask: 0.0,
                    });
                    
                    if qty_f64 == 0.0 {
                        if level.ask > 0.0 {
                            cancellations.push((price, "ask".to_string(), level.ask));
                        }
                        level.ask = 0.0;
                    } else {
                        level.ask = qty_f64;
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
        
        self.clean_old_trades();
        self.clean_old_cancels();
    }
    
    // 使用 BTreeMap 的优势 - O(log n) 时间复杂度获取最佳买价
    fn get_best_bid(&self) -> Option<f64> {
        self.price_levels
            .iter()
            .rev()  // 从高到低遍历
            .find(|(_, level)| level.bid > 0.0)
            .map(|(price, _)| price.into_inner())
    }
    
    // 使用 BTreeMap 的优势 - O(log n) 时间复杂度获取最佳卖价
    fn get_best_ask(&self) -> Option<f64> {
        self.price_levels
            .iter()  // 从低到高遍历
            .find(|(_, level)| level.ask > 0.0)
            .map(|(price, _)| price.into_inner())
    }
    
    // 获取最佳买卖价
    fn get_best_bid_ask(&self) -> (Option<f64>, Option<f64>) {
        let mut best_bid = None;
        let mut best_ask = None;
        
        for (price, level) in &self.price_levels {
            if level.bid > 0.0 {
                if best_bid.is_none() || price.into_inner() > best_bid.unwrap() {
                    best_bid = Some(price.into_inner());
                }
            }
            if level.ask > 0.0 {
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
            if let Some(level) = self.price_levels.get(&OrderedFloat(bid_price)) {
                bid_volume = level.bid;
            }
        }
        
        if let Some(ask_price) = best_ask {
            if let Some(level) = self.price_levels.get(&OrderedFloat(ask_price)) {
                ask_volume = level.ask;
            }
        }
        
        (bid_volume, ask_volume)
    }
    
    // 获取市场信号摘要
    fn get_market_signals(&self) -> String {
        let imbalances = self.microstructure_analyzer.get_current_imbalance_signals();
        let icebergs = self.microstructure_analyzer.get_current_iceberg_signals();
        
        let mut signals = Vec::new();
        
        for imbalance in imbalances {
            signals.push(format!(
                "{}失衡 {:.1}% (量:{:.2})",
                if imbalance.imbalance_type == "bullish" { "🟢看涨" } else { "🔴看跌" },
                imbalance.imbalance_ratio * 100.0,
                imbalance.consumed_volume
            ));
        }
        
        for iceberg in icebergs {
            signals.push(format!(
                "🧊{}冰山 {:.2} ({}次补充)",
                if iceberg.side == "bid" { "买盘" } else { "卖盘" },
                iceberg.accumulated_volume,
                iceberg.replenish_count
            ));
        }
        
        if signals.is_empty() {
            "无特殊信号".to_string()
        } else {
            signals.join("\n")
        }
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
            last_best_bid: None,
            last_best_ask: None,
            last_bid_volume: 0.0,
            last_ask_volume: 0.0,
            detected_imbalances: Vec::new(),
            detected_icebergs: Vec::new(),
        }
    }
    
    // 实时流动性失衡检测
    fn detect_liquidity_imbalance(&mut self, 
        best_bid: Option<f64>, 
        best_ask: Option<f64>,
        bid_volume: f64,
        ask_volume: f64,
        trade_price: f64,
        trade_volume: f64,
        trade_side: &str) -> Option<LiquidityImbalance> {
        
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 检查成交量是否达到最小阈值
        if trade_volume < self.min_volume_threshold {
            return None;
        }
        
        let mut imbalance_detected = None;
        
        match trade_side {
            "buy" => {
                // 主动买单检测 - 放宽价格匹配条件
                if let Some(ask) = best_ask {
                    // 检查是否为主动买单（价格大于等于best_ask）
                    if trade_price >= ask {
                        // 计算消耗比例
                        let consumption_ratio = if ask_volume > 0.0 {
                            trade_volume / ask_volume
                        } else {
                            1.0 // 如果挂单量为0，认为完全消耗
                        };
                        
                        // 降低阈值，更容易触发检测
                        if consumption_ratio > 0.3 { // 降低到30%
                            // 简化补充检测逻辑
                            let volume_change_ratio = if self.last_ask_volume > 0.0 {
                                (ask_volume - self.last_ask_volume) / self.last_ask_volume
                            } else {
                                0.0
                            };
                            
                            // 如果挂单量没有显著增加，认为存在失衡
                            if volume_change_ratio < 0.5 { // 增长不足50%
                                imbalance_detected = Some(LiquidityImbalance {
                                    timestamp: current_time,
                                    imbalance_type: "bullish".to_string(),
                                    imbalance_ratio: consumption_ratio,
                                    consumed_volume: trade_volume,
                                });
                            }
                        }
                    }
                }
            },
            "sell" => {
                // 主动卖单检测 - 放宽价格匹配条件
                if let Some(bid) = best_bid {
                    // 检查是否为主动卖单（价格小于等于best_bid）
                    if trade_price <= bid {
                        // 计算消耗比例
                        let consumption_ratio = if bid_volume > 0.0 {
                            trade_volume / bid_volume
                        } else {
                            1.0 // 如果挂单量为0，认为完全消耗
                        };
                        
                        // 降低阈值，更容易触发检测
                        if consumption_ratio > 0.3 { // 降低到30%
                            // 简化补充检测逻辑
                            let volume_change_ratio = if self.last_bid_volume > 0.0 {
                                (bid_volume - self.last_bid_volume) / self.last_bid_volume
                            } else {
                                0.0
                            };
                            
                            // 如果挂单量没有显著增加，认为存在失衡
                            if volume_change_ratio < 0.5 { // 增长不足50%
                                imbalance_detected = Some(LiquidityImbalance {
                                    timestamp: current_time,
                                    imbalance_type: "bearish".to_string(),
                                    imbalance_ratio: consumption_ratio,
                                    consumed_volume: trade_volume,
                                });
                            }
                        }
                    }
                }
            },
            _ => {}
        }
        
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
        }
        
        imbalance_detected
    }
    
    // 分析窗口中的流动性失衡
    // fn analyze_window_imbalance(&self, window: &LiquidityWindow) -> Option<LiquidityImbalance> {
    //     let total_aggressive_volume = window.aggressive_buy_volume + window.aggressive_sell_volume;
    //     
    //     if total_aggressive_volume < self.min_volume_threshold {
    //         return None;
    //     }
    //     
    //     let buy_ratio = window.aggressive_buy_volume / total_aggressive_volume;
    //     let sell_ratio = window.aggressive_sell_volume / total_aggressive_volume;
    //     
    //     // 检测强烈看涨信号 (主动买压过大)
    //     if buy_ratio > self.imbalance_threshold && 
    //        window.ask_replenish_volume < window.aggressive_buy_volume * 0.5 {
    //         return Some(LiquidityImbalance {
    //             timestamp: window.start_time,
    //             imbalance_type: "bullish".to_string(),
    //             imbalance_ratio: buy_ratio,
    //             consumed_volume: window.aggressive_buy_volume,
    //             price_level: 0.0, // 需要从上下文获取
    //         });
    //     }
    //     
    //     // 检测强烈看跌信号 (主动卖压过大)
    //     if sell_ratio > self.imbalance_threshold && 
    //        window.bid_replenish_volume < window.aggressive_sell_volume * 0.5 {
    //         return Some(LiquidityImbalance {
    //             timestamp: window.start_time,
    //             imbalance_type: "bearish".to_string(),
    //             imbalance_ratio: sell_ratio,
    //             consumed_volume: window.aggressive_sell_volume,
    //             price_level: 0.0, // 需要从上下文获取
    //         });
    //     }
    //     
    //     None
    // }
    
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
            
            // 获取所有价格并排序，过滤掉挂单量为0的层级
            // BTreeMap 已经是有序的，我们只需要过滤和收集
            let filtered_prices: Vec<f64> = orderbook
                .price_levels
                .iter()
                .filter(|(_, level)| level.ask > 0.0 || level.bid > 0.0)
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
                let level = orderbook.price_levels.get(&price_ordered).unwrap();
                let bid_vol = level.bid;
                let ask_vol = level.ask;
                
                // 获取成交量信息
                let sell_trade_vol = orderbook.get_trade_volume(*price, "sell");
                let buy_trade_vol = orderbook.get_trade_volume(*price, "buy");
                
                // 获取撤单量信息
                let bid_cancel_vol = orderbook.get_cancel_volume(*price, "bid");
                let ask_cancel_vol = orderbook.get_cancel_volume(*price, "ask");
                
                // 判断当前价格是否为best_bid或best_ask
                let is_at_best_bid = best_bid.map_or(false, |bb| (price - bb).abs() < 0.000001);
                let is_at_best_ask = best_ask.map_or(false, |ba| (price - ba).abs() < 0.000001);
                
                // Bid挂单显示逻辑
                let bid_str = if bid_vol > 0.0 {
                    if is_at_best_bid {
                        format!("{:.3}", bid_vol)
                    } else if is_at_best_ask {
                        String::new()
                    } else if *price <= current_price {
                        format!("{:.3}", bid_vol)
                    } else {
                        String::new()
                    }
                } else { 
                    String::new() 
                };
                
                // Ask挂单显示逻辑
                let ask_str = if ask_vol > 0.0 {
                    if is_at_best_ask {
                        format!("{:.3}", ask_vol)
                    } else if is_at_best_bid {
                        String::new()
                    } else if *price >= current_price {
                        format!("{:.3}", ask_vol)
                    } else {
                        String::new()
                    }
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
                
                // 撤单量显示逻辑：遵循与挂单相同的逻辑
                let bid_cancel_str = if bid_cancel_vol > 0.0 {
                    if is_at_best_bid {
                        format!("-{:.3}", bid_cancel_vol)
                    } else if is_at_best_ask {
                        String::new()
                    } else if *price <= current_price {
                        format!("-{:.3}", bid_cancel_vol)
                    } else {
                        String::new()
                    }
                } else { 
                    String::new() 
                };
                
                let ask_cancel_str = if ask_cancel_vol > 0.0 {
                    if is_at_best_ask {
                        format!("-{:.3}", ask_cancel_vol)
                    } else if is_at_best_bid {
                        String::new()
                    } else if *price >= current_price {
                        format!("-{:.3}", ask_cancel_vol)
                    } else {
                        String::new()
                    }
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
    
    // 右侧市场信号显示区域
    let signals = {
        let orderbook = app.orderbook.lock();
        orderbook.get_market_signals()
    };
    
    let signal_block = Block::default()
        .title("市场微观结构信号")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Yellow));
    
    let signal_paragraph = Paragraph::new(signals)
        .block(signal_block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    
    f.render_widget(signal_paragraph, signal_area);
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
