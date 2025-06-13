mod orderbook;
mod signal;
mod event;
mod ringbuffer;

use event::EventHandler;
use ringbuffer::{Event, EventType, TimerType, EventRingBuffer, TimerManager};
use orderbook::OrderBookData;
use signal::SignalGenerator;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
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
    io,
    time::{Duration, Instant},
    net::TcpStream,
};

use tungstenite::{connect, Message, WebSocket};
use url::Url;
use tungstenite::stream::MaybeTlsStream;

struct ReactiveApp {
    orderbook: OrderBookData,
    signal_generator: SignalGenerator,
    should_quit: bool,
    event_buffer: EventRingBuffer,
    timer_manager: TimerManager,
    websocket: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
    websocket_connected: bool,
    last_reconnect_attempt: Instant,
    should_render: bool,
    start_time: Instant,
    max_runtime: Duration,
}

impl ReactiveApp {
    fn new() -> Self {
        Self {
            orderbook: OrderBookData::new(),
            signal_generator: SignalGenerator::new(),
            should_quit: false,
            event_buffer: EventRingBuffer::new(16384), // 单一队列，16K容量
            timer_manager: TimerManager::new(),
            websocket: None,
            websocket_connected: false,
            last_reconnect_attempt: Instant::now(),
            should_render: false,
            start_time: Instant::now(),
            max_runtime: Duration::from_secs(3600), // 1小时后自动退出
        }
    }
    
    fn connect_websocket(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let url = "wss://fstream.binance.com/stream?streams=btcusdt@depth@100ms/btcusdt@aggTrade";
        
        match connect(Url::parse(url)?) {
            Ok((mut socket, _)) => {
                // 设置为非阻塞模式
                socket.get_mut().set_nonblocking(true)?;
                self.websocket = Some(socket);
                self.websocket_connected = true;
                let _ = self.event_buffer.push(Event::new(EventType::WebSocketConnected));
                println!("WebSocket connected successfully");
            },
            Err(e) => {
                println!("WebSocket connection failed: {:?}", e);
                self.websocket = None;
                self.websocket_connected = false;
            }
        }
        
        self.last_reconnect_attempt = Instant::now();
        Ok(())
    }
    
    fn handle_websocket_events(&mut self) {
        if let Some(ref mut socket) = self.websocket {
            // 非阻塞读取所有可用消息
            loop {
                match socket.read_message() {
                    Ok(msg) => {
                        if let Message::Text(text) = msg {
                            if let Ok(data) = serde_json::from_str::<Value>(&text) {
                                let _ = self.event_buffer.push(Event::new(EventType::WebSocketData(data)));
                            }
                        }
                    },
                    Err(tungstenite::Error::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // 非阻塞模式下没有数据可读，退出循环
                        break;
                    },
                    Err(e) => {
                        println!("WebSocket read error: {:?}", e);
                        self.websocket = None;
                        self.websocket_connected = false;
                        let _ = self.event_buffer.push(Event::new(EventType::WebSocketDisconnected));
                        break;
                    }
                }
            }
        }
    }
    
    fn check_runtime_limit(&mut self) {
        if self.start_time.elapsed() >= self.max_runtime {
            println!("Runtime limit reached, shutting down...");
            let _ = self.event_buffer.push(Event::new(EventType::Quit));
        }
    }
    
    fn process_websocket_data(&mut self, data: Value) {
        // 处理组合流的数据格式
        if let Some(stream) = data.get("stream").and_then(|v| v.as_str()) {
            if let Some(event_data_inner) = data.get("data") {
                if stream.contains("depth") {
                    if event_data_inner.get("e").and_then(|v| v.as_str()) == Some("depthUpdate") {
                        self.orderbook.update(event_data_inner);
                    }
                } else if stream.contains("aggTrade") {
                    if event_data_inner.get("e").and_then(|v| v.as_str()) == Some("aggTrade") {
                        self.orderbook.add_trade(event_data_inner);
                    }
                }
            }
        }
        // 兼容单流格式
        else if let Some(event_type) = data.get("e").and_then(|v| v.as_str()) {
            match event_type {
                "depthUpdate" => {
                    self.orderbook.update(&data);
                },
                "aggTrade" => {
                    self.orderbook.add_trade(&data);
                },
                _ => {}
            }
        }
    }
}

impl EventHandler for ReactiveApp {
    fn handle_event(&mut self, event: Event) -> Result<(), Box<dyn std::error::Error>> {
        match event.event_type {
            EventType::WebSocketData(data) => {
                self.process_websocket_data(data);
            },
            EventType::WebSocketConnected => {
                self.websocket_connected = true;
                println!("WebSocket connection established");
            },
            EventType::WebSocketDisconnected => {
                self.websocket_connected = false;
                println!("WebSocket connection lost");
            },
            EventType::Timer(timer_type) => {
                match timer_type {
                    TimerType::SignalGeneration => {
                        self.signal_generator.generate_signals(&mut self.orderbook);
                    },
                    TimerType::DataCleanup => {
                        self.orderbook.clean_old_trades();
                        self.orderbook.clean_old_cancels();
                    },
                    TimerType::Reconnect => {
                        if !self.websocket_connected {
                            let _ = self.connect_websocket();
                        }
                    }
                }
            },
            EventType::Render => {
                self.should_render = true;
            },
            EventType::Quit => {
                self.should_quit = true;
            },
        }
        Ok(())
    }
}

fn render_ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(f.size());

    render_orderbook(f, chunks[0], app);
    render_signals(f, chunks[1], app);
}

fn render_orderbook(f: &mut Frame, area: Rect, app: &App) {
    let mut rows = Vec::new();
    
    // 获取当前最优买卖价
    let best_bid = app.orderbook.best_bid_price.unwrap_or(0.0);
    let best_ask = app.orderbook.best_ask_price.unwrap_or(0.0);
    
    // 显示订单簿数据
    for (price, order_flow) in app.orderbook.order_flows.iter().take(20) {
        let price_val = price.0;
        
        // 设置价格颜色
        let price_style = if price_val == best_bid {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else if price_val == best_ask {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        
        rows.push(Row::new(vec![
            Cell::from(format!("{:.2}", order_flow.bid_ask.bid)).style(Style::default().fg(Color::Green)),
            Cell::from(format!("{:.2}", price_val)).style(price_style),
            Cell::from(format!("{:.2}", order_flow.bid_ask.ask)).style(Style::default().fg(Color::Red)),
            Cell::from(format!("{:.2}", order_flow.realtime_trade_record.buy_volume)).style(Style::default().fg(Color::Cyan)),
            Cell::from(format!("{:.2}", order_flow.realtime_trade_record.sell_volume)).style(Style::default().fg(Color::Magenta)),
            Cell::from(format!("{:.2}", order_flow.realtime_cancel_records.bid_cancel)).style(Style::default().fg(Color::Yellow)),
            Cell::from(format!("{:.2}", order_flow.realtime_cancel_records.ask_cancel)).style(Style::default().fg(Color::Yellow)),
        ]));
    }

    let table = Table::new(rows)
        .header(Row::new(vec!["Bid", "Price", "Ask", "Buy Vol", "Sell Vol", "Bid Cancel", "Ask Cancel"]))
        .block(Block::default().borders(Borders::ALL).title("Order Book"))
        .widths(&[
            Constraint::Percentage(14),
            Constraint::Percentage(14),
            Constraint::Percentage(14),
            Constraint::Percentage(14),
            Constraint::Percentage(14),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
        ]);

    f.render_widget(table, area);
}

fn render_signals(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    // 失衡信号
    let imbalance_text = format!(
        "Imbalance Signals: {}\nBid Ratio: {:.2}%\nAsk Ratio: {:.2}%\nRecent: {}",
        app.orderbook.imbalance_signals.len(),
        app.orderbook.bid_volume_ratio * 100.0,
        app.orderbook.ask_volume_ratio * 100.0,
        app.orderbook.imbalance_signals.last()
            .map(|s| format!("{} ({:.2})", s.signal_type, s.ratio))
            .unwrap_or_else(|| "None".to_string())
    );
    
    let imbalance_paragraph = Paragraph::new(imbalance_text)
        .block(Block::default().borders(Borders::ALL).title("Imbalance"))
        .wrap(Wrap { trim: true });
    f.render_widget(imbalance_paragraph, chunks[0]);

    // 大订单信号
    let big_orders_text = format!(
        "Big Orders: {}\nActive: {}\nLast: {}",
        app.orderbook.big_orders.len(),
        app.orderbook.big_orders.values().count(),
        app.orderbook.big_orders.values().last()
            .map(|o| format!("{} {:.2}", o.order_type, o.volume))
            .unwrap_or_else(|| "None".to_string())
    );
    
    let big_orders_paragraph = Paragraph::new(big_orders_text)
        .block(Block::default().borders(Borders::ALL).title("Big Orders"))
        .wrap(Wrap { trim: true });
    f.render_widget(big_orders_paragraph, chunks[1]);

    // 冰山订单和撤单信号
    let other_signals_text = format!(
        "Iceberg Signals: {}\nCancel Signals: {}\nLast Cancel: {}",
        app.orderbook.iceberg_signals.len(),
        app.orderbook.cancel_signals.len(),
        app.orderbook.cancel_signals.last()
            .map(|s| format!("{} ({:.2})", s.signal_type, s.ratio))
            .unwrap_or_else(|| "None".to_string())
    );
    
    let other_signals_paragraph = Paragraph::new(other_signals_text)
        .block(Block::default().borders(Borders::ALL).title("Other Signals"))
        .wrap(Wrap { trim: true });
    f.render_widget(other_signals_paragraph, chunks[2]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = ReactiveApp::new();
    
    // 初始连接 WebSocket
    app.connect_websocket()?;

    // 创建事件轮询器
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1024);
    
    println!("Starting reactive trading system...");
    
    // 主事件循环 - 完全事件驱动，基于单一RingBuffer
    loop {
        // 1. 检查运行时间限制
        app.check_runtime_limit();
        
        // 2. 检查定时器并推送定时器事件到RingBuffer
        app.timer_manager.check_and_push_events(&app.event_buffer);
        
        // 3. 非阻塞处理WebSocket事件
        app.handle_websocket_events();
        
        // 4. 处理RingBuffer中的所有事件（FIFO顺序）
        let mut events_processed = 0;
        const MAX_EVENTS_PER_CYCLE: usize = 2000; // 增加单次处理事件数量
        
        while let Some(event) = app.event_buffer.pop() {
            app.handle_event(event)?;
            events_processed += 1;
            
            if app.should_quit {
                break;
            }
            
            // 限制单次循环处理的事件数量，保证响应性
            if events_processed >= MAX_EVENTS_PER_CYCLE {
                break;
            }
        }
        
        if app.should_quit {
            break;
        }
        
        // 5. 渲染 UI（仅在需要时）
        if app.should_render {
            terminal.draw(|f| render_ui(f, &app))?;
            app.should_render = false;
        }
        
        // 6. 如果没有事件需要处理，等待下一个超时或I/O事件
        if app.event_buffer.is_empty() {
            let timeout = app.timer_manager.next_timeout();
            poll.poll(&mut events, Some(timeout))?;
        }
        
        // 7. 监控RingBuffer使用情况
        if app.event_buffer.len() > app.event_buffer.capacity() * 8 / 10 {
            println!("Warning: Event buffer usage high: {}/{}", 
                app.event_buffer.len(), app.event_buffer.capacity());
        }
    }

    // 清理
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    
    println!("Trading system shutdown complete");

    Ok(())
}