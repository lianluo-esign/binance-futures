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
    Signal(String),
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
    
    fn handle_book_ticker(&mut self, data: &Value) {
        // 解析bookTicker数据
        if let (Some(best_bid_str), Some(best_ask_str)) = 
            (data["b"].as_str(), data["a"].as_str()) {
            
            if let (Ok(best_bid_price), Ok(best_ask_price)) = 
                (best_bid_str.parse::<f64>(), best_ask_str.parse::<f64>()) {
                
                // 更新最优买卖价
                self.best_bid_price = Some(best_bid_price);
                self.best_ask_price = Some(best_ask_price);
                
                // 清理不合理的挂单
                let mut prices_to_clear = Vec::new();
                
                for (price, order_flow) in self.order_flows.iter_mut() {
                    let price_val = price.0;
                    
                    // 清理大于best_bid_price的ask挂单
                    if price_val > best_bid_price && order_flow.bid_ask.ask > 0.0 {
                        prices_to_clear.push((*price, "ask".to_string(), order_flow.bid_ask.ask));
                        order_flow.bid_ask.ask = 0.0;
                    }
                    
                    // 清理小于best_ask_price的bid挂单
                    if price_val < best_ask_price && order_flow.bid_ask.bid > 0.0 {
                        prices_to_clear.push((*price, "bid".to_string(), order_flow.bid_ask.bid));
                        order_flow.bid_ask.bid = 0.0;
                    }
                }
                
                // 处理清理的挂单作为撤单
                for (price, side, volume) in prices_to_clear {
                    self.detect_cancellation(price.0, &side, volume);
                }
            }
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
        
        let mut messages_processed = 0;
        // 移除消息数量限制，处理所有可用消息 - 高频交易系统优化
        
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
                                    messages_processed += 1;
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
        
        Ok(messages_processed)
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

    /// 单线程事件循环核心 - 处理所有事件
    fn tick(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. 读取WebSocket消息到事件缓冲区
        if self.websocket_manager.is_connected() {
            let _ = self.websocket_manager.read_messages(&mut self.event_buffer);
        } else if self.websocket_manager.should_reconnect() {
            self.websocket_manager.attempt_reconnect(&self.symbol);
        }
        
        // 2. 处理事件缓冲区中的事件
        self.process_events();
        
        // 3. 生成信号
        self.generate_signals();
        
        Ok(())
    }

    /// 处理事件缓冲区中的事件（非阻塞）
    fn process_events(&mut self) {
        while let Some(event) = self.event_buffer.pop() {
            match event {
                EventType::DepthUpdate(data) => {
                    self.orderbook.update(&data);
                }
                EventType::Trade(data) => {
                    self.orderbook.add_trade(&data);
                }
                EventType::BookTicker(data) => {
                    self.orderbook.handle_book_ticker(&data);
                }
                EventType::Signal(signal) => {
                    // 处理信号
                }
                EventType::WebSocketError(error) => {
                    // 处理WebSocket错误
                }
            }
        }
    }

    /// 生成信号
    fn generate_signals(&mut self) {
        let signals = self.signal_generator.check_signals(&self.orderbook);
        for signal in signals {
            self.event_buffer.push(EventType::Signal(signal));
        }
    }

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
    
    if let Some(current_price) = app.orderbook.current_price {
        let best_bid = app.orderbook.get_best_bid();
        let best_ask = app.orderbook.get_best_ask();
        
        // 获取所有价格并排序，只显示合理的价位
        let filtered_prices: Vec<f64> = app.orderbook
            .order_flows
            .iter()
            .filter(|(price, order_flow)| {
                let price_val = price.0;
                let has_valid_bid = order_flow.bid_ask.bid > 0.0 && 
                    best_bid.map_or(false, |bb| price_val <= bb);
                let has_valid_ask = order_flow.bid_ask.ask > 0.0 && 
                    best_ask.map_or(false, |ba| price_val >= ba);
                has_valid_bid || has_valid_ask
            })
            .map(|(price, _)| price.0)
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
            if let Some(order_flow) = app.orderbook.order_flows.get(&price_ordered) {
                let bid_vol = order_flow.bid_ask.bid;
                let ask_vol = order_flow.bid_ask.ask;
                
                // 获取成交量信息
                let sell_trade_vol = app.orderbook.get_trade_volume(*price, "sell");
                let buy_trade_vol = app.orderbook.get_trade_volume(*price, "buy");
                
                // 获取撤单量信息
                let bid_cancel_vol = app.orderbook.get_cancel_volume(*price, "bid");
                let ask_cancel_vol = app.orderbook.get_cancel_volume(*price, "ask");
                
                // 获取历史成交量信息
                let history_sell_trade_vol = app.orderbook.get_history_trade_volume(*price, "sell");
                let history_buy_trade_vol = app.orderbook.get_history_trade_volume(*price, "buy");

                // Bid挂单显示逻辑
                let bid_str = if bid_vol > 0.0 {
                    format!("{:.3}", bid_vol)
                } else { 
                    String::new() 
                };
                
                // Ask挂单显示逻辑
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
                
                // 撤单量显示逻辑
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
                        // 主动成交订单列
                        let total_vol = history_buy_trade_vol + history_sell_trade_vol;
                        let mut active_trade_str = String::new();
                        
                        if total_vol > 0.0 {
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
    }
    
    // 在创建表格之前调用auto_scroll
    let visible_rows_count = centered_area.height.saturating_sub(3) as usize;
    app.auto_scroll(current_price_index, visible_rows_count);

    // 应用滚动偏移
    let visible_rows: Vec<_> = rows.into_iter().skip(app.scroll_offset).collect();
    
    // 创建表格
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
            Constraint::Length(15), // History Trades
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
    
    let inner_area = block.inner(area);
    
    // 获取OrderBookData中的数据
    let (bid_ratio, ask_ratio, imbalance_signals, cancel_signals) = (
        app.orderbook.bid_volume_ratio, 
        app.orderbook.ask_volume_ratio, 
        app.orderbook.imbalance_signals.clone(),
        app.orderbook.cancel_signals.clone()
    );
    
    // 创建Text对象和Line列表
    let mut lines = Vec::new();
    
    // 添加基本信息
    lines.push(Line::from(Span::raw(format!("买单占比: {:.2}% | 卖单占比: {:.2}%", bid_ratio * 100.0, ask_ratio * 100.0))));
    
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
    
    lines.push(Line::from(Span::raw(format!("{bar}"))));
    lines.push(Line::from(Span::raw(""))); // 空行
    lines.push(Line::from(Span::raw("失衡信号:")));
    
    // 显示失衡信号
    for signal in imbalance_signals.iter().rev().take(5) {
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
    
    // 显示撤单信号
    lines.push(Line::from(Span::raw(""))); // 空行
    lines.push(Line::from(Span::raw("撤单信号:")));
    
    for signal in cancel_signals.iter().rev().take(3) {
        let time = SystemTime::UNIX_EPOCH + Duration::from_millis(signal.timestamp);
        let seconds = time.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        let hours = (seconds / 3600) % 24;
        let minutes = (seconds / 60) % 60;
        let secs = seconds % 60;
        let formatted_time = format!("{:02}:{:02}:{:02}", hours, minutes, secs);
        
        let signal_text = format!("[{}] {} 撤单: {:.2} BTC", 
            formatted_time, signal.signal_type, signal.ratio);
        lines.push(Line::from(Span::raw(signal_text)));
    }
    
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    
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

// ==================== 主函数 ====================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 移除 env_logger::init();
    
    // 读取环境变量SYMBOL，默认为BTCUSDT
    let symbol = env::var("SYMBOL").unwrap_or_else(|_| "BTCUSDT".to_string());
    
    // 验证symbol格式
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
    
    // 创建响应式应用
    let mut app = ReactiveApp::new(symbol.clone());
    
    // 初始化WebSocket连接
    if let Err(e) = app.initialize() {
        eprintln!("初始化失败: {}", e);
        // 继续运行，稍后会尝试重连
    }
    
    println!("启动单线程事件驱动系统: {}", symbol);
    
    // 主事件循环（单线程）
    loop {
        // 核心事件处理
        if let Err(e) = app.tick() {
            eprintln!("事件处理错误: {}", e);
        }
        
        // 渲染UI
        terminal.draw(|f| render_ui(f, &mut app))?;
        
        // 处理键盘输入（非阻塞）
        if crossterm::event::poll(Duration::from_millis(0))? {
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
                        KeyCode::Char('q') => break,
                        KeyCode::Char('r') => {
                            // 手动重连
                            app.websocket_manager.attempt_reconnect(&symbol);
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // 短暂休眠以避免过度占用CPU（微秒级精度）
        std::thread::sleep(Duration::from_micros(100));
    }
    
    // 清理终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    println!("程序正常退出");
    Ok(())
}