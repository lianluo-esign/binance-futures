// TODO: 暂时注释掉egui相关代码，专注于终端UI优化
// use eframe::egui;
use std::collections::HashMap;
use std::time::Instant;

// use crate::app::ReactiveApp;
// use crate::Config;
// use crate::orderbook::MarketSnapshot;

/// Windows GUI应用程序（暂时禁用）
pub struct TradingGUI {
    #[allow(dead_code)]
    market_data: HashMap<String, f64>,
    #[allow(dead_code)]
    last_update: Instant,
}

impl TradingGUI {
    pub fn new() -> Self {
        Self {
            market_data: HashMap::new(),
            last_update: Instant::now(),
        }
    }

}

impl Default for TradingGUI {
    fn default() -> Self {
        Self::new()
    }
}
