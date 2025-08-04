// TODO: 暂时注释掉egui相关代码，专注于终端UI优化
// use eframe::egui;
use std::collections::HashMap;
use std::time::Instant;

// use crate::app::ReactiveApp;
// use crate::Config;
// use crate::orderbook::MarketSnapshot;

/// Windows GUI应用程序（暂时禁用）
pub struct TradingGUI {
    market_data: HashMap<String, f64>,
    last_update: Instant,
}

impl TradingGUI {
    pub fn new() -> Self {
        Self {
            market_data: HashMap::new(),
            last_update: Instant::now(),
        }
    }

    // TODO: 重新启用egui功能时取消注释
    /*
    pub fn new(config: Config) -> Self {
        let mut app = ReactiveApp::new(config);
        
        // 初始化应用程序
        if let Err(e) = app.initialize() {
            eprintln!("Failed to initialize app: {}", e);
        }
        
        Self {
            app,
            last_update: Instant::now(),
            update_interval: Duration::from_millis(100), // 10 FPS
            show_settings: false,
            show_stats: false,
        }
    }

    pub fn update_market_data(&mut self, snapshot: &MarketSnapshot) {
        if let Some(bid) = snapshot.best_bid_price {
            self.market_data.insert("best_bid".to_string(), bid);
        }
        if let Some(ask) = snapshot.best_ask_price {
            self.market_data.insert("best_ask".to_string(), ask);
        }
        if let Some(price) = snapshot.current_price {
            self.market_data.insert("current_price".to_string(), price);
        }
        
        self.last_update = Instant::now();
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let options = eframe::NativeOptions::default();
        eframe::run_native("Trading GUI", options, Box::new(|_cc| Box::new(TradingGUI::new())))
    }
    */
}

impl Default for TradingGUI {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: 重新启用egui功能时取消注释
/*
impl eframe::App for TradingGUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // GUI实现代码...
    }
    
    fn on_close_event(&mut self) -> bool {
        // 清理资源
        self.app.stop();
        true
    }
}

impl Drop for TradingGUI {
    fn drop(&mut self) {
        self.app.stop();
    }
}
*/