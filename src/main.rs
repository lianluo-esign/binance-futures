use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ordered_float::OrderedFloat;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame, Terminal,
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    env,
    io,
    net::TcpStream,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tungstenite::{
    client::IntoClientRequest,
    stream::MaybeTlsStream,
    Message, WebSocket,
};

use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

static APP_PTR: AtomicPtr<ReactiveApp> = AtomicPtr::new(ptr::null_mut());

// ==================== 核心数据结构 ====================

/// 高性能循环缓冲区
#[derive(Debug)]
struct RingBuffer<T> {
    buffer: Vec<Option<T>>,
    capacity: usize,
    head: usize,
    tail: usize,
    size: usize,
}

impl<T> RingBuffer<T> {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: (0..capacity).map(|_| None).collect(),
            capacity,
            head: 0,
            tail: 0,
            size: 0,
        }
    }

    fn push(&mut self, item: T) -> bool {
        if self.size == self.capacity {
            // 缓冲区满，覆盖最旧的数据
            self.head = (self.head + 1) % self.capacity;
        } else {
            self.size += 1;
        }
        
        self.buffer[self.tail] = Some(item);
        self.tail = (self.tail + 1) % self.capacity;
        true
    }

    fn pop(&mut self) -> Option<T> {
        if self.size == 0 {
            return None;
        }
        
        let item = self.buffer[self.head].take();
        self.head = (self.head + 1) % self.capacity;
        self.size -= 1;
        item
    }

    fn len(&self) -> usize {
        self.size
    }

    fn capacity(&self) -> usize {
        self.capacity
    }
}

/// 事件类型枚举
#[derive(Debug, Clone)]
enum EventType {
    DepthUpdate(Value),
    Trade(Value),
    BookTicker(Value),  // 新增
    Signal(Value),  // 修改为Value类型，保存丰富的信号数据
    WebSocketError(String),
}

/// 订单簿数据结构
#[derive(Debug, Clone)]
struct PriceLevel {
    bid: f64,
    ask: f64,
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
}

#[derive(Debug, Clone)]
struct OrderFlow {
    bid_ask: PriceLevel,
    history_trade_record: TradeRecord,
    realtime_trade_record: TradeRecord,
    realtime_cancel_records: CancelRecord,
}

impl OrderFlow {
    fn new() -> Self {
        Self {
            bid_ask: PriceLevel { bid: 0.0, ask: 0.0 },
            history_trade_record: TradeRecord { buy_volume: 0.0, sell_volume: 0.0, timestamp: 0 },
            realtime_trade_record: TradeRecord { buy_volume: 0.0, sell_volume: 0.0, timestamp: 0 },
            realtime_cancel_records: CancelRecord { bid_cancel: 0.0, ask_cancel: 0.0 },
        }
    }
}

#[derive(Debug, Clone)]
struct ImbalanceSignal {
    timestamp: u64,
    signal_type: String,
    ratio: f64,
}

#[derive(Debug, Clone)]
struct BigOrder {
    order_type: String,
    volume: f64,
    timestamp: u64,
}

/// 订单簿数据管理
struct OrderBookData {
    order_flows: BTreeMap<OrderedFloat<f64>, OrderFlow>,
    current_price: Option<f64>,
    last_trade_side: Option<String>,
    trade_display_duration: u64,
    cancel_display_duration: u64,
    max_trade_records: usize,
    max_cancel_records: usize,
    
    stable_highlight_price: Option<f64>,
    stable_highlight_side: Option<String>,
    last_trade_price: Option<f64>,
    highlight_start_time: Option<u64>,
    highlight_duration: u64,
    last_update_id: Option<u64>,
    
    best_bid_price: Option<f64>,
    best_ask_price: Option<f64>,
    prices_to_clear_buffer: Vec<(OrderedFloat<f64>, String)>,
    cancellations_buffer: Vec<(f64, String, f64)>,
    
    bid_volume_ratio: f64,
    ask_volume_ratio: f64,
    imbalance_signals: Vec<ImbalanceSignal>,
    last_imbalance_check: u64,
    continuous_imbalance_start: Option<u64>,
    current_imbalance_type: Option<String>,
    cancel_signals: Vec<ImbalanceSignal>,
    last_cancel_check: u64,
    
    iceberg_signals: Vec<ImbalanceSignal>,
    big_orders: HashMap<OrderedFloat<f64>, BigOrder>,
    last_big_order_check: u64,
    active_trades_buffer: HashMap<OrderedFloat<f64>, (f64, f64)>,
    
    // 新增：500ms比率缓冲区
    ratio_buffer: Vec<(u64, f64, f64)>, // (timestamp, bid_ratio, ask_ratio)
    buffer_window_ms: u64, // 500ms窗口
    signal_threshold: f64, // 0.75阈值
}

impl OrderBookData {
    fn new() -> Self {
        Self {
            order_flows: BTreeMap::new(),
            current_price: None,
            last_trade_side: None,
            trade_display_duration: 3000,
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
            
            // 初始化新字段
            ratio_buffer: Vec::new(),
            buffer_window_ms: 500,
            signal_threshold: 0.75,
        }
    }

    fn update(&mut self, data: &Value) {
        let mut cancellations: Vec<(f64, String, f64)> = Vec::new();
        
        // 处理bids数组
        if let Some(bids) = data["b"].as_array() {
            // 获取最优买价（价格最高的）
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
            
            // 更新bids的具体数量
            for bid in bids {
                if let (Some(price_str), Some(qty_str)) = (bid[0].as_str(), bid[1].as_str()) {
                    if let (Ok(price), Ok(qty_f64)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        let price_ordered = OrderedFloat(price);
                        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                        order_flow.bid_ask.bid = qty_f64;
                    }
                }
            }
        }
        
        // 处理asks数组
        if let Some(asks) = data["a"].as_array() {
            // 获取最优卖价（价格最低的）
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
            
            // 更新asks的具体数量
            for ask in asks {
                if let (Some(price_str), Some(qty_str)) = (ask[0].as_str(), ask[1].as_str()) {
                    if let (Ok(price), Ok(qty_f64)) = (price_str.parse::<f64>(), qty_str.parse::<f64>()) {
                        let price_ordered = OrderedFloat(price);
                        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
                        order_flow.bid_ask.ask = qty_f64;
                    }
                }
            }
        }
    }

    fn clean_old_trades(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 清理过期的实时交易记录
        for (_price, order_flow) in self.order_flows.iter_mut() {
            // 如果实时交易记录超过显示时间（1秒），则重置为0
            if current_time.saturating_sub(order_flow.realtime_trade_record.timestamp) > self.trade_display_duration {
                order_flow.realtime_trade_record.buy_volume = 0.0;
                order_flow.realtime_trade_record.sell_volume = 0.0;
            }
        }
        
        // 限制记录数量 - 如果OrderFlow数量超过限制，移除最旧的记录
        if self.order_flows.len() > self.max_trade_records {
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
            self.current_price = Some(price);
            
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
            
            match side {
                "buy" => {
                    order_flow.realtime_trade_record.buy_volume = qty_f64;
                    order_flow.history_trade_record.buy_volume += qty_f64;
                },
                "sell" => {
                    order_flow.realtime_trade_record.sell_volume = qty_f64;
                    order_flow.history_trade_record.sell_volume += qty_f64;
                },
                _ => {}
            }
            
            order_flow.realtime_trade_record.timestamp = current_time;
            order_flow.history_trade_record.timestamp = current_time;
            
            // 添加清理过期交易数据的调用
            self.clean_old_trades();
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

    fn get_best_bid(&self) -> Option<f64> {
        self.best_bid_price
    }
    
    fn get_best_ask(&self) -> Option<f64> {
        self.best_ask_price
    }
    
    fn handle_book_ticker(&mut self, data: &Value, event_buffer: &mut RingBuffer<EventType>) {
        // 解析bookTicker数据
        if let (Some(best_bid_str), Some(best_ask_str), Some(best_bid_qty_str), Some(best_ask_qty_str)) = 
            (data["b"].as_str(), data["a"].as_str(), data["B"].as_str(), data["A"].as_str()) {
            
            if let (Ok(best_bid_price), Ok(best_ask_price), Ok(best_bid_qty), Ok(best_ask_qty)) = 
                (best_bid_str.parse::<f64>(), best_ask_str.parse::<f64>(), 
                 best_bid_qty_str.parse::<f64>(), best_ask_qty_str.parse::<f64>()) {
                
                // 1. 先更新order_flow里面对应价格的挂单数据
                let best_bid_ordered = OrderedFloat(best_bid_price);
                let best_ask_ordered = OrderedFloat(best_ask_price);
                
                // 更新最优买价的挂单量
                let bid_order_flow = self.order_flows.entry(best_bid_ordered).or_insert_with(OrderFlow::new);
                bid_order_flow.bid_ask.bid = best_bid_qty;
                
                // 更新最优卖价的挂单量
                let ask_order_flow = self.order_flows.entry(best_ask_ordered).or_insert_with(OrderFlow::new);
                ask_order_flow.bid_ask.ask = best_ask_qty;
                
                // 2. 然后更新最优买卖价格
                self.best_bid_price = Some(best_bid_price);
                self.best_ask_price = Some(best_ask_price);
                
                // 3. 修正清理逻辑 - 清理不合理的挂单
                for (price, order_flow) in self.order_flows.iter_mut() {
                    let price_val = price.0;
                    
                    // 清理价格低于或等于最优买价的ask挂单（ask价格应该高于bid价格）
                    if price_val <= best_bid_price {
                        order_flow.bid_ask.ask = 0.0;
                    }
                    
                    // 清理价格高于或等于最优卖价的bid挂单（bid价格应该低于ask价格）
                    if price_val >= best_ask_price {
                        order_flow.bid_ask.bid = 0.0;
                    }
                }
                
                // 4. 传递bookTicker数据和event_buffer到calculate_volume_ratio进行计算
                self.calculate_volume_ratio(
                    Some(best_bid_price), 
                    Some(best_ask_price), 
                    Some(best_bid_qty), 
                    Some(best_ask_qty),
                    event_buffer
                );
            }
        } else {
            // 如果bookTicker数据解析失败，使用默认计算方式
            self.calculate_volume_ratio(None, None, None, None, event_buffer);
        }
    }

    // 添加计算多空占比的函数
    fn calculate_volume_ratio(&mut self, best_bid_price: Option<f64>, best_ask_price: Option<f64>, best_bid_qty: Option<f64>, best_ask_qty: Option<f64>, event_buffer: &mut RingBuffer<EventType>) {
        let mut total_bid_volume = 0.0;
        let mut total_ask_volume = 0.0;
        
        // 如果有bookTicker数据，优先使用最优买卖价的数量
        if let (Some(bid_qty), Some(ask_qty)) = (best_bid_qty, best_ask_qty) {
            total_bid_volume = bid_qty;
            total_ask_volume = ask_qty;
        } else {
            // 否则基于当前orderflow数据计算总的买卖挂单量
            for (price, order_flow) in &self.order_flows {
                // 只计算有效的挂单量（大于0的）
                if order_flow.bid_ask.bid > 0.0 {
                    total_bid_volume += order_flow.bid_ask.bid;
                }
                if order_flow.bid_ask.ask > 0.0 {
                    total_ask_volume += order_flow.bid_ask.ask;
                }
            }
        }
        
        let total_volume = total_bid_volume + total_ask_volume;
        
        if total_volume > 0.0 {
            self.bid_volume_ratio = total_bid_volume / total_volume;
            self.ask_volume_ratio = total_ask_volume / total_volume;
            
            // 获取当前时间戳
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            // 添加当前比率到缓冲区
            self.ratio_buffer.push((current_time, self.bid_volume_ratio, self.ask_volume_ratio));
            
            // 清理超过500ms的旧数据
            self.ratio_buffer.retain(|(timestamp, _, _)| {
                current_time.saturating_sub(*timestamp) <= self.buffer_window_ms
            });
            
            // 检查500ms内是否所有比率都超过阈值
            if !self.ratio_buffer.is_empty() && self.ratio_buffer.len() >= 3 {
                // 检查多头信号：500ms内所有bid_ratio都 >= 0.75
                let all_bull_signals = self.ratio_buffer.iter().all(|(_, bid_ratio, _)| {
                    *bid_ratio >= self.signal_threshold
                });
                
                if all_bull_signals {
                    // 计算平均比率
                    let avg_ratio: f64 = self.ratio_buffer.iter()
                        .map(|(_, bid_ratio, _)| bid_ratio)
                        .sum::<f64>() / self.ratio_buffer.len() as f64;
                    
                    // 创建结构化的信号数据
                    let signal_data = serde_json::json!({
                        "signal_type": "imbalance",
                        "direction": "bull",
                        "timestamp": current_time,
                        "ratio": avg_ratio,
                        "description": "多头失衡(500ms累计)",
                        "window_ms": self.buffer_window_ms,
                        "sample_count": self.ratio_buffer.len(),
                        "bid_volume": total_bid_volume,
                        "ask_volume": total_ask_volume,
                        "total_volume": total_volume,
                        "best_bid_price": best_bid_price,
                        "best_ask_price": best_ask_price
                    });
                    
                    // 推送到事件缓冲区
                    event_buffer.push(EventType::Signal(signal_data));
                    
                    // 清空缓冲区，避免重复信号
                    self.ratio_buffer.clear();
                }
                
                // 检查空头信号：500ms内所有ask_ratio都 >= 0.75
                let all_bear_signals = self.ratio_buffer.iter().all(|(_, _, ask_ratio)| {
                    *ask_ratio >= self.signal_threshold
                });
                
                if all_bear_signals {
                    // 计算平均比率
                    let avg_ratio: f64 = self.ratio_buffer.iter()
                        .map(|(_, _, ask_ratio)| ask_ratio)
                        .sum::<f64>() / self.ratio_buffer.len() as f64;
                    
                    // 创建结构化的信号数据
                    let signal_data = serde_json::json!({
                        "signal_type": "imbalance",
                        "direction": "bear",
                        "timestamp": current_time,
                        "ratio": avg_ratio,
                        "description": "空头失衡(500ms累计)",
                        "window_ms": self.buffer_window_ms,
                        "sample_count": self.ratio_buffer.len(),
                        "bid_volume": total_bid_volume,
                        "ask_volume": total_ask_volume,
                        "total_volume": total_volume,
                        "best_bid_price": best_bid_price,
                        "best_ask_price": best_ask_price
                    });
                    
                    // 推送到事件缓冲区
                    event_buffer.push(EventType::Signal(signal_data));
                    
                    // 清空缓冲区，避免重复信号
                    self.ratio_buffer.clear();
                }
            }
        } else {
            // 如果没有挂单数据，保持50:50的比例
            self.bid_volume_ratio = 0.5;
            self.ask_volume_ratio = 0.5;
        }
    }
    
    fn detect_cancellation(&mut self, price: f64, side: &str, volume: f64) {
        let price_ordered = OrderedFloat(price);
        let order_flow = self.order_flows.entry(price_ordered).or_insert_with(OrderFlow::new);
        
        match side {
            "bid" => {
                order_flow.realtime_cancel_records.bid_cancel += volume;
            }
            "ask" => {
                order_flow.realtime_cancel_records.ask_cancel += volume;
            }
            _ => {}
        }
    }
}

/// 信号生成器
struct SignalGenerator {
    last_check_time: Instant,
}

impl SignalGenerator {
    fn new() -> Self {
        Self {
            last_check_time: Instant::now(),
        }
    }

    fn check_signals(&mut self, orderbook: &OrderBookData) -> Vec<String> {
        let mut signals = Vec::new();
        
        // 检查失衡信号
        if orderbook.bid_volume_ratio > 0.7 {
            signals.push("买单失衡信号".to_string());
        } else if orderbook.ask_volume_ratio > 0.7 {
            signals.push("卖单失衡信号".to_string());
        }
        
        // 检查大订单信号
        if !orderbook.big_orders.is_empty() {
            signals.push("大订单信号".to_string());
        }
        
        signals
    }
}

/// WebSocket 连接管理器（非阻塞）
struct WebSocketManager {
    socket: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
    connected: bool,
  
    reconnect_attempts: u32,
    max_reconnect_attempts: u32,
}

impl WebSocketManager {
    fn new() -> Self {
        Self {
            socket: None,
            connected: false,
        
            reconnect_attempts: 0,
            max_reconnect_attempts: 5,
        }
    }

    fn connect(&mut self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        let symbol_lower = symbol.to_lowercase();
        let url_string = format!(
            "wss://fstream.binance.com/stream?streams={}@depth20@100ms/{}@aggTrade/{}@bookTicker",
            symbol_lower, symbol_lower, symbol_lower
        );
        
        let request = url_string.into_client_request()?;
        let (socket, _) = tungstenite::client::connect(request)?;
        
        // 设置非阻塞模式
        let stream = socket.get_ref();
        match stream {
            MaybeTlsStream::Plain(tcp_stream) => {
                tcp_stream.set_nonblocking(true)?;
            }
            MaybeTlsStream::NativeTls(tls_stream) => {
                tls_stream.get_ref().set_nonblocking(true)?;
            }
            _ => {}
        }
        
        self.socket = Some(socket);
        self.connected = true;
        self.reconnect_attempts = 0;
        
        Ok(())
    }

    fn read_messages(&mut self, event_buffer: &mut RingBuffer<EventType>) -> Result<usize, Box<dyn std::error::Error>> {
        if !self.connected || self.socket.is_none() {
            return Ok(0);
        }
        
        if let Some(ref mut socket) = self.socket {
            loop {
                match socket.read() {
                    Ok(Message::Text(text)) => {
                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            if let Some(stream) = data["stream"].as_str() {
                                if let Some(event_data) = data["data"].as_object() {
                                    let event_value = serde_json::Value::Object(event_data.clone());
                                    
                                    let event = if stream.contains("depth") {
                                        EventType::DepthUpdate(event_value)
                                    } else if stream.contains("aggTrade") {
                                        EventType::Trade(event_value)
                                    } else if stream.contains("bookTicker") {
                                        EventType::BookTicker(event_value)
                                    } else {
                                        continue;
                                    };
                                    
                                    event_buffer.push(event);
                                   
                                }
                            }
                        }
                    }
                    Ok(Message::Ping(payload)) => {
                        // 响应 ping
                        let _ = socket.send(Message::Pong(payload));
                    }
                    Ok(Message::Close(_)) => {
                        self.connected = false;
                        break;
                    }
                    Err(tungstenite::Error::Io(ref e)) if e.kind() == io::ErrorKind::WouldBlock => {
                        // 非阻塞模式下没有数据可读
                        break;
                    }
                    Err(e) => {
                        event_buffer.push(EventType::WebSocketError(format!("WebSocket error: {}", e)));
                        self.connected = false;
                        break;
                    }
                    _ => {}
                }
            }
        }
        Ok(0)
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn should_reconnect(&self) -> bool {
        !self.connected && self.reconnect_attempts < self.max_reconnect_attempts
    }

    fn attempt_reconnect(&mut self, symbol: &str) {
        if self.should_reconnect() {
            self.reconnect_attempts += 1;
            if let Err(e) = self.connect(symbol) {
                eprintln!("重连失败 (尝试 {}): {}", self.reconnect_attempts, e);
            } else {
                println!("重连成功！");
            }
        }
    }
}

/// 响应式应用主结构（单线程）
struct ReactiveApp {
    event_buffer: RingBuffer<EventType>,
    orderbook: OrderBookData,
    signal_generator: SignalGenerator,
    websocket_manager: WebSocketManager,
    scroll_offset: usize,
    auto_scroll: bool,
    last_update: Instant,
    symbol: String,
}

impl ReactiveApp {
    fn new(symbol: String) -> Self {
        Self {
            event_buffer: RingBuffer::new(10000), // 10K事件缓冲区
            orderbook: OrderBookData::new(),
            signal_generator: SignalGenerator::new(),
            websocket_manager: WebSocketManager::new(),
            scroll_offset: 0,
            auto_scroll: true,
            last_update: Instant::now(),
            symbol,
        }
    }

    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 连接WebSocket
        self.websocket_manager.connect(&self.symbol)?;
        println!("WebSocket 连接成功: {}", self.symbol);
        Ok(())
    }

    /// 处理所有事件
    fn process_events(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. 读取WebSocket消息到事件缓冲区
        if self.websocket_manager.is_connected() {
            let _ = self.websocket_manager.read_messages(&mut self.event_buffer);
        } else if self.websocket_manager.should_reconnect() {
            self.websocket_manager.attempt_reconnect(&self.symbol);
        }
        
        // 2. 处理事件缓冲区中的事件
        self.handle_events();
        
        // 3. 生成信号
        // self.generate_signals();
        
        Ok(())
    }

    /// 处理事件缓冲区中的事件（非阻塞）
    fn handle_events(&mut self) {
        // 这里的while循环是为了加速处理evebt buffer当中存在的事件 清空buffer然后进入下一轮 避免消息阻塞
        while let Some(event) = self.event_buffer.pop() {
            match event {
                EventType::DepthUpdate(data) => {
                    self.orderbook.update(&data);
                }
                EventType::Trade(data) => {
                    self.orderbook.add_trade(&data);
                }
                EventType::BookTicker(data) => {
                    self.orderbook.handle_book_ticker(&data, &mut self.event_buffer);
                }
                EventType::Signal(signal_data) => {
                    // 处理信号事件，从事件缓冲区读取数据并填充到imbalance_signals列表
                    if let Some(signal_type) = signal_data["signal_type"].as_str() {
                        match signal_type {
                            "imbalance" => {
                                // 解析失衡信号数据
                                if let (Some(timestamp), Some(ratio), Some(description)) = (
                                    signal_data["timestamp"].as_u64(),
                                    signal_data["ratio"].as_f64(),
                                    signal_data["description"].as_str()
                                ) {
                                    // 检查是否为重复信号（去重逻辑）
                                    let is_duplicate = self.orderbook.imbalance_signals.iter().any(|existing| {
                                        // 检查最近5秒内是否有相同类型和相似比率的信号
                                        let time_diff = timestamp.saturating_sub(existing.timestamp);
                                        let ratio_diff = (ratio - existing.ratio).abs();
                                        
                                        time_diff < 5000 && // 5秒内
                                        existing.signal_type == description && // 相同类型
                                        ratio_diff < 0.01 // 比率差异小于1%
                                    });
                                    
                                    // 只有非重复信号才添加
                                    if !is_duplicate {
                                        // 创建ImbalanceSignal并添加到列表
                                        let imbalance_signal = ImbalanceSignal {
                                            timestamp,
                                            signal_type: description.to_string(),
                                            ratio,
                                        };
                                        
                                        // 添加到orderbook的imbalance_signals列表
                                        self.orderbook.imbalance_signals.push(imbalance_signal);
                                        
                                        // 限制列表长度为50（减少内存使用）
                                        if self.orderbook.imbalance_signals.len() > 50 {
                                            self.orderbook.imbalance_signals.remove(0);
                                        }
                                    }
                                }
                            }
                            "cancel" => {
                                // 撤单信号的去重处理
                                if let (Some(timestamp), Some(ratio), Some(description)) = (
                                    signal_data["timestamp"].as_u64(),
                                    signal_data["ratio"].as_f64(),
                                    signal_data["description"].as_str()
                                ) {
                                    // 检查撤单信号重复
                                    let is_duplicate = self.orderbook.cancel_signals.iter().any(|existing| {
                                        let time_diff = timestamp.saturating_sub(existing.timestamp);
                                        let ratio_diff = (ratio - existing.ratio).abs();
                                        
                                        time_diff < 3000 && // 3秒内
                                        existing.signal_type == description && // 相同类型
                                        ratio_diff < 0.1 // 比率差异小于0.1
                                    });
                                    
                                    if !is_duplicate {
                                        let cancel_signal = ImbalanceSignal {
                                            timestamp,
                                            signal_type: description.to_string(),
                                            ratio,
                                        };
                                        
                                        self.orderbook.cancel_signals.push(cancel_signal);
                                        
                                        // 限制撤单信号列表长度为30
                                        if self.orderbook.cancel_signals.len() > 30 {
                                            self.orderbook.cancel_signals.remove(0);
                                        }
                             
                                    }
                                }
                            }
                            _ => {
                                // 其他类型信号的处理
                            }
                        }
                    }
                }
                EventType::WebSocketError(error) => {
                    // 处理WebSocket错误
                }
            }
        }
    }

    /// 生成信号
    // fn generate_signals(&mut self) {
    //     let signals = self.signal_generator.check_signals(&self.orderbook);
    //     for signal in signals {
    //         self.event_buffer.push(EventType::Signal(signal));
    //     }
    // }

    fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
        self.auto_scroll = false;
    }

    fn scroll_down(&mut self) {
        self.scroll_offset += 1;
        self.auto_scroll = false;
    }

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

    fn get_buffer_status(&self) -> (usize, usize) {
        (self.event_buffer.len(), self.event_buffer.capacity())
    }
}

// ==================== UI渲染函数 ====================
fn render_ui(f: &mut Frame, app: &mut ReactiveApp) {
    let size = f.area();
    
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
    
    render_orderbook(f, app, orderbook_area);
    render_signals(f, app, signal_area);
}

fn render_orderbook(f: &mut Frame, app: &mut ReactiveApp, area: Rect) {
    let (buffer_len, buffer_cap) = app.get_buffer_status();
    let connection_status = if app.websocket_manager.is_connected() {
        "已连接"
    } else {
        "断开连接"
    };
    
    let title = format!("Binance Futures Order Book - {} | 缓冲区: {}/{} | 状态: {}", 
        app.symbol, buffer_len, buffer_cap, connection_status);
    
    // 计算订单薄表格区域
    let table_width = area.width.saturating_sub(2);
    let table_height = area.height.saturating_sub(2);
    
    let centered_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: table_width,
        height: table_height,
    };
    
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL);
    
    // 创建表格数据和获取当前价格索引
    let mut rows = Vec::new();
    let mut current_price_index = None;
    
    // 直接获取orderbook中的所有价格数据，不做任何过滤
    let all_prices: Vec<f64> = app.orderbook
        .order_flows
        .keys()
        .map(|price| price.0)
        .collect();
    
    if !all_prices.is_empty() {
        // 按价格降序排列
        let mut sorted_prices = all_prices;
        sorted_prices.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        
        // 为每个价格生成表格行
        for (i, price) in sorted_prices.iter().enumerate() {
            // 检查是否为当前价格
            if let Some(current_price) = app.orderbook.current_price {
                if (price - current_price).abs() < 0.000001 {
                    current_price_index = Some(i);
                }
            }
            
            let price_ordered = OrderedFloat(*price);
            if let Some(order_flow) = app.orderbook.order_flows.get(&price_ordered) {
                // 直接使用orderbook中的实时数据，不做任何条件判断或过滤
                let bid_vol = order_flow.bid_ask.bid;
                let ask_vol = order_flow.bid_ask.ask;
                
                // 获取所有相关数据
                let sell_trade_vol = app.orderbook.get_trade_volume(*price, "sell");
                let buy_trade_vol = app.orderbook.get_trade_volume(*price, "buy");
                let bid_cancel_vol = app.orderbook.get_cancel_volume(*price, "bid");
                let ask_cancel_vol = app.orderbook.get_cancel_volume(*price, "ask");
                let history_sell_trade_vol = app.orderbook.get_history_trade_volume(*price, "sell");
                let history_buy_trade_vol = app.orderbook.get_history_trade_volume(*price, "buy");

                // 构建显示字符串（只有数值大于0才显示，否则显示空字符串）
                let bid_str = if bid_vol > 0.0 { format!("{:.3}", bid_vol) } else { String::new() };
                let ask_str = if ask_vol > 0.0 { format!("{:.3}", ask_vol) } else { String::new() };
                let sell_trade_str = if sell_trade_vol > 0.0 { format!("@{:.3}", sell_trade_vol) } else { String::new() };
                let buy_trade_str = if buy_trade_vol > 0.0 { format!("@{:.3}", buy_trade_vol) } else { String::new() };
                let bid_cancel_str = if bid_cancel_vol > 0.0 { format!("-{:.3}", bid_cancel_vol) } else { String::new() };
                let ask_cancel_str = if ask_cancel_vol > 0.0 { format!("-{:.3}", ask_cancel_vol) } else { String::new() };
                
                // 创建行 - 显示所有价位，不做任何过滤
                let row = Row::new(vec![
                    Cell::from(bid_cancel_str).style(Style::default().fg(Color::Gray)),
                    Cell::from(sell_trade_str).style(Style::default().fg(Color::Red)),
                    Cell::from(bid_str).style(Style::default().fg(Color::Green)),
                    {
                        let price_str = format!("{:.2}", price);
                        let mut price_cell = Cell::from(price_str).style(Style::default().fg(Color::White));
                        if Some(i) == current_price_index {
                            if let Some(ref last_side) = app.orderbook.last_trade_side {
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
                        let total_vol = history_buy_trade_vol + history_sell_trade_vol;
                        let active_trade_str = if total_vol > 0.0 {
                            format!("B:{:.3} S:{:.3} T:{:.3}", 
                                history_buy_trade_vol, 
                                history_sell_trade_vol, 
                                total_vol)
                        } else {
                            String::new()
                        };
                        Cell::from(active_trade_str).style(Style::default().fg(Color::White))
                    },
                ]);
                
                rows.push(row);
            }
        }
    }
    
    // 如果orderbook完全没有数据，显示等待状态
    if rows.is_empty() {
        let status_message = if app.websocket_manager.is_connected() {
            "连接正常，等待订单薄数据..."
        } else {
            "WebSocket连接断开，尝试重连中..."
        };
        
        let empty_row = Row::new(vec![
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(status_message).style(Style::default().fg(Color::Yellow)),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ]);
        rows.push(empty_row);
    }
    
    // 应用滚动逻辑
    let visible_rows_count = centered_area.height.saturating_sub(3) as usize;
    app.auto_scroll(current_price_index, visible_rows_count);
    let visible_rows: Vec<_> = rows.into_iter().skip(app.scroll_offset).collect();
    
    // 创建并渲染表格
    let table = Table::new(
        visible_rows,
        [
            Constraint::Length(10), // Bid Cancel
            Constraint::Length(10), // Sell Trade
            Constraint::Length(10), // Bid Vol
            Constraint::Length(12), // Price
            Constraint::Length(10), // Ask Vol
            Constraint::Length(10), // Buy Trade
            Constraint::Length(10), // Ask Cancel
            Constraint::Length(25), // History Trades
        ]
    )
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
    .block(block);
    
    f.render_widget(table, centered_area);
}

fn render_signals(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    // 将右侧信号区域分为三个垂直部分
    let signal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40), // Orderbook Imbalance 占40%
            Constraint::Percentage(30), // Order Momentum 占30%
            Constraint::Percentage(30), // Iceberg Orders 占30%
        ])
        .split(area);
    
    let imbalance_area = signal_chunks[0];
    let momentum_area = signal_chunks[1];
    let iceberg_area = signal_chunks[2];
    
    // 渲染三个信号区域
    render_orderbook_imbalance(f, app, imbalance_area);
    render_order_momentum(f, app, momentum_area);
    render_iceberg_orders(f, app, iceberg_area);
}

// 渲染订单簿失衡信号
fn render_orderbook_imbalance(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let block = Block::default()
        .title("Orderbook Imbalance")
        .borders(Borders::ALL);
    
    // 获取OrderBookData中的数据
    let (bid_ratio, ask_ratio, imbalance_signals) = (
        app.orderbook.bid_volume_ratio, 
        app.orderbook.ask_volume_ratio, 
        app.orderbook.imbalance_signals.clone()
    );
    
    // 创建Text对象和Line列表
    let mut lines = Vec::new();
    
    // 添加基本信息
    let basic_info = format!("买单占比: {:.2}% | 卖单占比: {:.2}%", bid_ratio * 100.0, ask_ratio * 100.0);
    lines.push(Line::from(Span::raw(basic_info)));
    
    // 创建横向条
    let bar_width = 60; // 固定宽度
    let bid_bar_width = (bid_ratio * bar_width as f64) as usize;
    
    let mut bar = String::new();
    for _ in 0..bid_bar_width {
        bar.push('█');
    }
    for _ in bid_bar_width..bar_width {
        bar.push('░');
    }
    
    lines.push(Line::from(Span::raw(bar)));
    lines.push(Line::from(Span::raw(""))); // 空行
    
    // 显示失衡信号 - 让框架自动处理行数
    for signal in imbalance_signals.iter().rev().take(20) { // 显示最近20个信号，让框架自动裁剪
        let time = SystemTime::UNIX_EPOCH + Duration::from_millis(signal.timestamp);
        let seconds = time.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let hours = (seconds / 3600) % 24;
        let minutes = (seconds / 60) % 60;
        let secs = seconds % 60;
        let formatted_time = format!("{:02}:{:02}:{:02}", hours, minutes, secs);
        
        let signal_text = format!("[{}] {} 失衡: {:.2}%", 
            formatted_time, signal.signal_type, signal.ratio * 100.0);
        
        lines.push(Line::from(Span::raw(signal_text)));
    }
    
    // 创建Text并渲染 - 让Paragraph自动处理换行和裁剪
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((0, 0)); // 可以添加滚动功能
    
    f.render_widget(paragraph, area);
}

// 渲染订单动量信号
fn render_order_momentum(f: &mut Frame, _app: &ReactiveApp, area: Rect) {
    let block = Block::default()
        .title("Order Momentum")
        .borders(Borders::ALL);
    
    let text = Text::from("订单动量分析");
    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(Color::White));
    
    f.render_widget(paragraph, area);
}

// 渲染冰山订单信号
fn render_iceberg_orders(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    let block = Block::default()
        .title("Iceberg Orders")
        .borders(Borders::ALL);
    
    let mut lines = Vec::new();
    
    // 显示大订单信号
    let mut orders: Vec<_> = app.orderbook.big_orders.iter().collect();
    orders.sort_by(|a, b| b.1.volume.partial_cmp(&a.1.volume).unwrap_or(std::cmp::Ordering::Equal));
    
    for (price, order) in orders.iter().take(8) {
        let time = SystemTime::UNIX_EPOCH + Duration::from_millis(order.timestamp);
        let seconds = time.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let hours = (seconds / 3600) % 24;
        let minutes = (seconds / 60) % 60;
        let secs = seconds % 60;
        let formatted_time = format!("{:02}:{:02}:{:02}", hours, minutes, secs);
        
        let signal_text = format!("[{}] {:.2} {} {:.2} BTC", 
            formatted_time, price.0,
            if order.order_type == "buy" { "买入" } else { "卖出" },
            order.volume);
        lines.push(Line::from(Span::raw(signal_text)));
    }
    
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    
    f.render_widget(paragraph, area);
}

// UI线程函数，采用与主线程相同的调用方式
fn ui_thread() {
    // 设置UI终端
    let mut terminal = setup_ui_terminal().expect("Failed to setup UI terminal");
    
    loop {
        // 每50ms刷新一次UI
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        // 直接通过原子指针访问app实例（无锁）
        let app_ptr = APP_PTR.load(Ordering::Acquire);
        if !app_ptr.is_null() {
            unsafe {
                let app = &mut *app_ptr;
                // 采用与主线程相同的调用方式
                let _ = terminal.draw(|f| render_ui(f, app));
            }
        }
        
        // 处理UI事件（退出等）
        if crossterm::event::poll(Duration::from_millis(0)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = crossterm::event::read() {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Up => {
                            unsafe {
                                let app = &mut *app_ptr;
                                app.scroll_up();
                            }
                        },
                        KeyCode::Down => {
                            unsafe {
                                let app = &mut *app_ptr;
                                app.scroll_down();
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
    }
    
    cleanup_ui_terminal(terminal);
}

// 设置UI终端
fn setup_ui_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

// 清理UI终端
fn cleanup_ui_terminal(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) {
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = terminal.show_cursor();
}

// ==================== 主函数 ====================
// 基于ringbuffer的纯粹的单线程无锁事件驱动架构的低延迟高频交易系统
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let disable_ui = false;  // 控制UI界面是否显示
    // 读取环境变量SYMBOL，默认为BTCUSDT
    let symbol = env::var("SYMBOL").unwrap_or_else(|_| "BTCUSDT".to_string());
    
    // 验证symbol格式
    if symbol.is_empty() {
        eprintln!("Error: SYMBOL cannot be empty");
        std::process::exit(1);
    }
    
    // 创建响应式应用
    let mut app = ReactiveApp::new(symbol.clone());
    // 设置全局app指针
    APP_PTR.store(&mut app as *mut ReactiveApp, Ordering::Release);

    // 初始化WebSocket连接
    if let Err(e) = app.initialize() {
        eprintln!("初始化失败: {}", e);
        return Err(e);
    }

    // 启动UI线程（如果启用UI）
    if !disable_ui {
        std::thread::spawn(|| {
            ui_thread();
        });
    }
    
    // 主事件循环
    loop {
        // 核心事件处理（保持原有逻辑不变）
        if let Err(e) = app.process_events() {
            eprintln!("事件处理错误: {}", e);
        }
    }
}