/// 订单簿相关数据类型定义

use serde::{Deserialize, Serialize};

/// 智能滚动信息
#[derive(Debug, Clone)]
pub struct SmartScrollInfo {
    /// 滚动偏移量
    pub scroll_offset: f32,
    /// 当前价格在数据中的索引
    pub current_price_index: Option<usize>,
    /// 目标行索引
    pub target_row: usize,
    /// 可见行数
    pub visible_rows: usize,
}

impl SmartScrollInfo {
    pub fn new(visible_rows: usize) -> Self {
        Self {
            scroll_offset: 0.0,
            current_price_index: None,
            target_row: visible_rows / 2,
            visible_rows,
        }
    }
    
    pub fn update_scroll_position(&mut self, new_offset: f32) {
        self.scroll_offset = new_offset;
    }
    
    pub fn set_current_price_index(&mut self, index: Option<usize>) {
        self.current_price_index = index;
    }
}

/// 统一订单簿行数据结构
#[derive(Debug, Clone)]
pub struct UnifiedOrderBookRow {
    pub price: f64,
    pub bid_volume: f64,           // 买单深度
    pub ask_volume: f64,           // 卖单深度
    pub active_buy_volume_5s: f64, // 5秒内主动买单累计
    pub active_sell_volume_5s: f64,// 5秒内主动卖单累计
    pub history_buy_volume: f64,   // 历史累计主动买单量
    pub history_sell_volume: f64,  // 历史累计主动卖单量
    pub delta: f64,                // 主动订单delta (买单量 - 卖单量)
    // 淡出动画支持
    pub bid_fade_alpha: f32,       // bid淡出透明度 (0.0 = 完全透明, 1.0 = 完全不透明)
    pub ask_fade_alpha: f32,       // ask淡出透明度 (0.0 = 完全透明, 1.0 = 完全不透明)
}

impl UnifiedOrderBookRow {
    pub fn new(price: f64) -> Self {
        Self {
            price,
            bid_volume: 0.0,
            ask_volume: 0.0,
            active_buy_volume_5s: 0.0,
            active_sell_volume_5s: 0.0,
            history_buy_volume: 0.0,
            history_sell_volume: 0.0,
            delta: 0.0,
            bid_fade_alpha: 1.0,
            ask_fade_alpha: 1.0,
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.bid_volume == 0.0 && self.ask_volume == 0.0 &&
        self.active_buy_volume_5s == 0.0 && self.active_sell_volume_5s == 0.0
    }
    
    pub fn total_volume(&self) -> f64 {
        self.bid_volume + self.ask_volume
    }
    
    pub fn total_active_volume(&self) -> f64 {
        self.active_buy_volume_5s + self.active_sell_volume_5s
    }
    
    pub fn update_fade_alpha(&mut self, delta_time: f32, fade_speed: f32) {
        if self.bid_fade_alpha > 0.0 {
            self.bid_fade_alpha = (self.bid_fade_alpha - delta_time * fade_speed).max(0.0);
        }
        if self.ask_fade_alpha > 0.0 {
            self.ask_fade_alpha = (self.ask_fade_alpha - delta_time * fade_speed).max(0.0);
        }
    }
}

/// 聚合订单流数据结构（用于1美元级别聚合）
#[derive(Debug, Clone)]
pub struct AggregatedOrderFlow {
    pub bid_volume: f64,           // 聚合买单深度
    pub ask_volume: f64,           // 聚合卖单深度
    pub active_buy_volume_5s: f64, // 聚合5秒内主动买单累计
    pub active_sell_volume_5s: f64,// 聚合5秒内主动卖单累计
    pub history_buy_volume: f64,   // 聚合历史累计主动买单量
    pub history_sell_volume: f64,  // 聚合历史累计主动卖单量
    // 淡出动画支持
    pub bid_fade_alpha: f32,       // bid淡出透明度 (0.0 = 完全透明, 1.0 = 完全不透明)
    pub ask_fade_alpha: f32,       // ask淡出透明度 (0.0 = 完全透明, 1.0 = 完全不透明)
}

impl AggregatedOrderFlow {
    pub fn new() -> Self {
        Self {
            bid_volume: 0.0,
            ask_volume: 0.0,
            active_buy_volume_5s: 0.0,
            active_sell_volume_5s: 0.0,
            history_buy_volume: 0.0,
            history_sell_volume: 0.0,
            bid_fade_alpha: 1.0,  // 默认完全不透明
            ask_fade_alpha: 1.0,  // 默认完全不透明
        }
    }
    
    pub fn add(&mut self, other: &AggregatedOrderFlow) {
        self.bid_volume += other.bid_volume;
        self.ask_volume += other.ask_volume;
        self.active_buy_volume_5s += other.active_buy_volume_5s;
        self.active_sell_volume_5s += other.active_sell_volume_5s;
        self.history_buy_volume += other.history_buy_volume;
        self.history_sell_volume += other.history_sell_volume;
        
        // 取最大的透明度值
        self.bid_fade_alpha = self.bid_fade_alpha.max(other.bid_fade_alpha);
        self.ask_fade_alpha = self.ask_fade_alpha.max(other.ask_fade_alpha);
    }
    
    pub fn reset(&mut self) {
        self.bid_volume = 0.0;
        self.ask_volume = 0.0;
        self.active_buy_volume_5s = 0.0;
        self.active_sell_volume_5s = 0.0;
        self.history_buy_volume = 0.0;
        self.history_sell_volume = 0.0;
        self.bid_fade_alpha = 1.0;
        self.ask_fade_alpha = 1.0;
    }
}

impl Default for AggregatedOrderFlow {
    fn default() -> Self {
        Self::new()
    }
}

/// 价格历史数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceHistoryPoint {
    pub timestamp: f64,
    pub price: f64,
    pub volume: f64,
    pub side: String,
}

impl PriceHistoryPoint {
    pub fn new(timestamp: f64, price: f64, volume: f64, side: String) -> Self {
        Self {
            timestamp,
            price,
            volume,
            side,
        }
    }
    
    pub fn is_buy(&self) -> bool {
        self.side == "buy" || self.side == "BUY"
    }
    
    pub fn is_sell(&self) -> bool {
        self.side == "sell" || self.side == "SELL"
    }
}

/// 颜色配置
#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub bid_color: eframe::egui::Color32,
    pub ask_color: eframe::egui::Color32,
    pub current_price_bg: eframe::egui::Color32,
    pub current_price_text: eframe::egui::Color32,
    pub positive_delta: eframe::egui::Color32,
    pub negative_delta: eframe::egui::Color32,
    pub neutral: eframe::egui::Color32,
    pub background: eframe::egui::Color32,
    pub grid_lines: eframe::egui::Color32,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            bid_color: eframe::egui::Color32::from_rgb(0, 150, 0),      // 绿色
            ask_color: eframe::egui::Color32::from_rgb(200, 0, 0),      // 红色
            current_price_bg: eframe::egui::Color32::from_rgb(255, 215, 0), // 金色
            current_price_text: eframe::egui::Color32::BLACK,
            positive_delta: eframe::egui::Color32::from_rgb(0, 200, 0), // 亮绿
            negative_delta: eframe::egui::Color32::from_rgb(255, 0, 0), // 亮红
            neutral: eframe::egui::Color32::GRAY,
            background: eframe::egui::Color32::from_rgb(32, 32, 32),    // 深灰背景
            grid_lines: eframe::egui::Color32::from_rgb(64, 64, 64),    // 网格线
        }
    }
}

impl ColorScheme {
    pub fn light_theme() -> Self {
        Self {
            bid_color: eframe::egui::Color32::from_rgb(0, 100, 0),
            ask_color: eframe::egui::Color32::from_rgb(150, 0, 0),
            current_price_bg: eframe::egui::Color32::from_rgb(255, 235, 59),
            current_price_text: eframe::egui::Color32::BLACK,
            positive_delta: eframe::egui::Color32::from_rgb(0, 150, 0),
            negative_delta: eframe::egui::Color32::from_rgb(200, 0, 0),
            neutral: eframe::egui::Color32::DARK_GRAY,
            background: eframe::egui::Color32::WHITE,
            grid_lines: eframe::egui::Color32::LIGHT_GRAY,
        }
    }
    
    pub fn dark_theme() -> Self {
        Self::default()
    }
}