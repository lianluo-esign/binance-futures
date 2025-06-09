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
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
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

// UI渲染函数 - 更新以使用 OrderedFloat
fn ui(f: &mut Frame, app: &mut App) {
    let size = f.size();
    
    // 计算居中的表格区域
    let table_width = 105;
    let table_height = size.height.saturating_sub(1);
    
    let horizontal_margin = (size.width.saturating_sub(table_width)) / 2;
    let vertical_margin = (size.height.saturating_sub(table_height)) / 10;
    
    let centered_area = Rect {
        x: horizontal_margin,
        y: vertical_margin,
        width: table_width.min(size.width),
        height: table_height.min(size.height),
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
                
                // Trade数据显示逻辑：无论价格如何跳动都要显示
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