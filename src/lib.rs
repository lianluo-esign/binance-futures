// 外部依赖
extern crate lazy_static;

// 核心模块
pub mod core;
pub mod events;
pub mod handlers;
pub mod orderbook;
pub mod websocket;
pub mod app;
pub mod monitoring;
pub mod gui;
pub mod audio;

// 重新导出主要类型
pub use core::RingBuffer;
pub use events::{Event, EventType, EventBus, EventDispatcher};
pub use orderbook::{OrderBookManager, OrderFlow, MarketSnapshot};
pub use websocket::{WebSocketManager, WebSocketConnection};
pub use app::ReactiveApp;
pub use gui::TradingGUI;

/// 库的版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 初始化日志系统 - 禁用控制台输出以避免干扰UI
pub fn init_logging() {
    // 对于字符界面应用，我们需要将日志重定向到文件而不是控制台
    use std::fs::OpenOptions;

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("flow_sight.log")
        .unwrap_or_else(|_| {
            // 如果无法创建日志文件，就完全禁用日志
            std::fs::File::create("/dev/null").unwrap()
        });

    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .filter_level(log::LevelFilter::Info) // 记录信息、警告和错误
        .init();
}

/// 库的配置结构
#[derive(Debug, Clone)]
pub struct Config {
    pub symbol: String,
    pub event_buffer_size: usize,
    pub max_reconnect_attempts: u32,
    pub log_level: String,

    // 订单簿显示配置
    pub max_visible_rows: usize,    // 最大可见行数
    pub price_precision: f64,       // 价格精度聚合参数（USD增量）
}

impl Default for Config {
    fn default() -> Self {
        Self {
            symbol: "BTCUSDT".to_string(),
            event_buffer_size: 10000,
            max_reconnect_attempts: 5,
            log_level: "info".to_string(),
            max_visible_rows: 3000,     // 默认最大可见行数为3000
            price_precision: 0.01,      // 默认价格精度为0.01 USD (1分)
        }
    }
}

impl Config {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            ..Default::default()
        }
    }

    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.event_buffer_size = size;
        self
    }

    pub fn with_max_reconnects(mut self, max: u32) -> Self {
        self.max_reconnect_attempts = max;
        self
    }

    pub fn with_log_level(mut self, level: String) -> Self {
        self.log_level = level;
        self
    }

    pub fn with_max_visible_rows(mut self, rows: usize) -> Self {
        self.max_visible_rows = rows;
        self
    }

    pub fn with_price_precision(mut self, precision: f64) -> Self {
        self.price_precision = precision;
        self
    }
}
