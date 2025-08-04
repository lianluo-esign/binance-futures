/// 统一订单簿主组件实现
/// 
/// 这是重构后的主要组件，整合了所有子模块的功能

use eframe::egui;
use crate::orderbook::OrderFlow;
use crate::app::ReactiveApp;
use std::time::Instant;

use super::types::{UnifiedOrderBookRow, ColorScheme, SmartScrollInfo};
use super::utils::{ScrollCalculator, DataExtractor, PerformanceTracker, PriceValidator};
use super::rendering::{TableRenderer, ColumnWidths};
use super::chart::{PriceChart, ChartConfig};
use super::popup::{PopupManager, PopupType};

/// 统一的订单簿组件 - 重构版本
pub struct UnifiedOrderBookWidget {
    // 核心配置
    auto_track_price: bool,
    time_window_seconds: u64,
    visible_price_levels: usize,
    price_precision: f64,
    
    // 渲染组件
    table_renderer: TableRenderer,
    price_chart: PriceChart,
    popup_manager: PopupManager,
    
    // 工具组件
    scroll_calculator: ScrollCalculator,
    data_extractor: DataExtractor,
    performance_tracker: PerformanceTracker,
    
    // 状态管理
    scroll_position: f32,
    last_price: f64,
    last_update_time: Instant,
    cached_visible_data: Vec<UnifiedOrderBookRow>,
    last_data_timestamp: u64,
    last_best_bid: Option<f64>,
    last_best_ask: Option<f64>,
    
    // 纹理资源
    logo_texture: Option<egui::TextureHandle>,
    binance_logo_texture: Option<egui::TextureHandle>,
    
    // UI状态
    tick_pressure_k_value: usize,
    column_widths: ColumnWidths,
}

impl Default for UnifiedOrderBookWidget {
    fn default() -> Self {
        let color_scheme = ColorScheme::default();
        let column_widths = ColumnWidths::default();
        let chart_config = ChartConfig::default();
        
        Self {
            auto_track_price: true,
            time_window_seconds: 5,
            visible_price_levels: 40,
            price_precision: 0.1,
            
            table_renderer: TableRenderer::new(color_scheme.clone(), column_widths.clone()),
            price_chart: PriceChart::new(chart_config),
            popup_manager: PopupManager::new(),
            
            scroll_calculator: ScrollCalculator::new(true, 80),
            data_extractor: DataExtractor::new(0.1, 5),
            performance_tracker: PerformanceTracker::new(100),
            
            scroll_position: 0.0,
            last_price: 0.0,
            last_update_time: Instant::now(),
            cached_visible_data: Vec::new(),
            last_data_timestamp: 0,
            last_best_bid: None,
            last_best_ask: None,
            
            logo_texture: None,
            binance_logo_texture: None,
            
            tick_pressure_k_value: 5,
            column_widths,
        }
    }
}

impl UnifiedOrderBookWidget {
    /// 创建新的组件实例
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 使用指定精度创建组件
    pub fn with_precision(precision: f64) -> Self {
        let mut widget = Self::new();
        widget.set_price_precision(precision);
        widget
    }
    
    /// 设置价格图表高度
    pub fn set_price_chart_height(&mut self, height: f32) {
        // 通话图表配置需要重构，这里暂时注释
        // self.price_chart.set_height(height);
    }
    
    /// 获取价格图表高度
    pub fn get_price_chart_height(&self) -> f32 {
        300.0 // 暂时返回固定值
    }
    
    /// 设置价格精度
    pub fn set_price_precision(&mut self, precision: f64) {
        if PriceValidator::is_valid_price(precision) {
            self.price_precision = precision;
            self.data_extractor.set_price_precision(precision);
        }
    }
    
    /// 主要显示函数
    pub fn show(&mut self, ui: &mut egui::Ui, app: &ReactiveApp) {
        let render_start_time = Instant::now();
        
        // 检查是否需要更新数据
        let should_update = self.should_update_data(app);
        
        if should_update {
            self.update_data(app);
        }
        
        // 渲染主要界面
        self.render_main_interface(ui, app);
        
        // 渲染弹出窗口
        self.popup_manager.render_popups(ui.ctx());
        
        // 记录性能
        let render_time = render_start_time.elapsed();
        self.performance_tracker.record_render_time(render_time);
    }
    
    /// 检查是否需要更新数据
    fn should_update_data(&self, _app: &ReactiveApp) -> bool {
        let now = Instant::now();
        let update_interval = std::time::Duration::from_millis(500); // 减少到2 FPS更新频率
        
        // 只依赖时间间隔控制，避免过于频繁的更新
        now.duration_since(self.last_update_time) >= update_interval
    }
    
    /// 更新数据
    fn update_data(&mut self, app: &ReactiveApp) {
        // 简化实现 - 使用模拟数据
        // TODO: 实现真实的数据提取逻辑
        
        // 获取当前价格 - 使用市场快照
        let market_snapshot = app.get_market_snapshot();
        let current_price = market_snapshot.current_price.unwrap_or(50000.0);
        
        // 更新价格历史
        if current_price > 0.0 && current_price != self.last_price {
            // self.price_chart.update_price_history(current_price, 0.0, "unknown".to_string());
            self.last_price = current_price;
        }
        
        // 提取可见数据
        self.cached_visible_data = self.data_extractor.extract_visible_data(
            app,
            self.visible_price_levels,
            current_price,
        );
        
        // 添加调试信息 - 减少日志频率
        static mut LAST_LOG_TIME: Option<Instant> = None;
        let now = Instant::now();
        unsafe {
            if LAST_LOG_TIME.map_or(true, |last| now.duration_since(last) > std::time::Duration::from_secs(5)) {
                log::info!("提取到 {} 行订单簿数据, 当前价格: {}", self.cached_visible_data.len(), current_price);
                if !self.cached_visible_data.is_empty() {
                    log::info!("第一行价格: {}, 最后一行价格: {}", 
                              self.cached_visible_data[0].price, 
                              self.cached_visible_data.last().unwrap().price);
                }
                LAST_LOG_TIME = Some(now);
            }
        }
        
        // 更新最佳买卖价 - 使用模拟数据
        self.last_best_bid = Some(current_price - 1.0);
        self.last_best_ask = Some(current_price + 1.0);
        
        // 更新时间戳
        self.last_data_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_update_time = Instant::now();
    }
    
    /// 渲染主要界面
    fn render_main_interface(&mut self, ui: &mut egui::Ui, app: &ReactiveApp) {
        ui.vertical(|ui| {
            // 渲染头部
            self.render_header(ui, app);
            
            ui.separator();
            
            // 渲染主要内容区域
            ui.horizontal(|ui| {
                // 左侧：订单簿表格
                ui.vertical(|ui| {
                    self.render_orderbook_table(ui, app);
                });
                
                ui.separator();
                
                // 右侧：价格图表和控制面板
                ui.vertical(|ui| {
                    self.render_side_panel(ui);
                });
            });
        });
    }
    
    /// 渲染头部
    fn render_header(&mut self, ui: &mut egui::Ui, app: &ReactiveApp) {
        ui.horizontal(|ui| {
            // Logo
            if let Some(ref texture) = self.logo_texture {
                self.table_renderer.render_logo(ui, texture, 32.0);
            }
            
            ui.separator();
            
            // 标题
            ui.heading("订单簿");
            
            ui.separator();
            
            // 当前价格信息
            if let Some(bid) = self.last_best_bid {
                ui.colored_label(egui::Color32::GREEN, format!("买: {:.2}", bid));
            }
            
            if let Some(ask) = self.last_best_ask {
                ui.colored_label(egui::Color32::RED, format!("卖: {:.2}", ask));
            }
            
            if let (Some(bid), Some(ask)) = (self.last_best_bid, self.last_best_ask) {
                let spread = ask - bid;
                ui.colored_label(egui::Color32::YELLOW, format!("价差: {:.2}", spread));
            }
            
            // 性能信息
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let fps = self.performance_tracker.fps();
                let color = if fps >= 30.0 {
                    egui::Color32::GREEN
                } else if fps >= 15.0 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };
                ui.colored_label(color, format!("FPS: {:.1}", fps));
                
                ui.separator();
                
                // 控制按钮
                if ui.button("📊").on_hover_text("价格图表").clicked() {
                    self.price_chart.open_modal();
                }
                
                if ui.button("📈").on_hover_text("交易信号").clicked() {
                    self.popup_manager.open_popup(PopupType::TradingSignal);
                }
                
                if ui.button("⚙").on_hover_text("设置").clicked() {
                    self.popup_manager.open_popup(PopupType::Settings);
                }
            });
        });
    }
    
    /// 渲染订单簿表格
    fn render_orderbook_table(&mut self, ui: &mut egui::Ui, app: &ReactiveApp) {
        if self.cached_visible_data.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("暂无数据");
            });
            return;
        }
        
        let market_snapshot = app.get_market_snapshot();
        let current_price = market_snapshot.current_price.unwrap_or(50000.0);
        
        // 计算滚动位置
        if self.auto_track_price {
            let scroll_info = self.scroll_calculator.calculate_smart_scroll_position(
                &self.cached_visible_data,
                current_price,
            );
            self.scroll_position = scroll_info.scroll_offset;
        }
        
        // 渲染表格
        self.table_renderer.render_unified_table(
            ui,
            &self.cached_visible_data,
            current_price,
            self.scroll_position,
            self.visible_price_levels * 2,
        );
    }
    
    /// 渲染侧边面板
    fn render_side_panel(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // 控制面板
            ui.group(|ui| {
                ui.label("控制面板");
                
                ui.checkbox(&mut self.auto_track_price, "自动跟踪价格");
                
                ui.horizontal(|ui| {
                    ui.label("价格精度:");
                    let mut precision = self.price_precision;
                    if ui.add(egui::Slider::new(&mut precision, 0.01..=10.0)
                        .logarithmic(true)
                        .text("USD")).changed() {
                        self.set_price_precision(precision);
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("时间窗口:");
                    ui.add(egui::Slider::new(&mut self.time_window_seconds, 1..=60)
                        .suffix("秒"));
                });
                
                ui.horizontal(|ui| {
                    ui.label("可见层级:");
                    ui.add(egui::Slider::new(&mut self.visible_price_levels, 10..=100));
                });
            });
            
            ui.add_space(10.0);
            
            // 迷你价格图表
            ui.group(|ui| {
                ui.label("价格走势");
                self.price_chart.show_embedded(ui);
            });
            
            ui.add_space(10.0);
            
            // 统计信息
            ui.group(|ui| {
                ui.label("统计信息");
                ui.label(format!("数据点: {}", self.cached_visible_data.len()));
                ui.label(format!("FPS: {:.1}", self.performance_tracker.fps()));
                ui.label(format!("平均渲染时间: {:.2}ms", 
                    self.performance_tracker.average_render_time().as_millis()));
            });
        });
    }
    
    /// 加载Logo纹理
    fn load_logo(&mut self, ctx: &egui::Context) {
        if self.logo_texture.is_some() {
            return;
        }
        
        match self.load_image_from_path(ctx, "assets/logo.png") {
            Ok(texture) => {
                self.logo_texture = Some(texture);
            }
            Err(e) => {
                log::warn!("加载Logo失败: {}", e);
            }
        }
    }
    
    /// 加载币安Logo纹理
    fn load_binance_logo(&mut self, ctx: &egui::Context) {
        if self.binance_logo_texture.is_some() {
            return;
        }
        
        match self.load_image_from_path(ctx, "assets/binance_logo.png") {
            Ok(texture) => {
                self.binance_logo_texture = Some(texture.clone());
                self.price_chart.set_logo_texture(texture);
            }
            Err(e) => {
                log::warn!("加载币安Logo失败: {}", e);
            }
        }
    }
    
    /// 从路径加载图片
    fn load_image_from_path(&self, ctx: &egui::Context, path: &str) -> Result<egui::TextureHandle, Box<dyn std::error::Error>> {
        let image = image::open(path)?;
        let rgba_image = image.to_rgba8();
        let dimensions = rgba_image.dimensions();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [dimensions.0 as usize, dimensions.1 as usize],
            &rgba_image,
        );
        
        Ok(ctx.load_texture(path, color_image, egui::TextureOptions::default()))
    }
    
    /// 处理键盘输入
    pub fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.popup_manager.open_popup(PopupType::Help);
        }
        
        if ctx.input(|i| i.key_pressed(egui::Key::F2)) {
            self.popup_manager.open_popup(PopupType::Settings);
        }
        
        if ctx.input(|i| i.key_pressed(egui::Key::F3)) {
            self.popup_manager.open_popup(PopupType::TradingSignal);
        }
        
        if ctx.input(|i| i.key_pressed(egui::Key::F4)) {
            self.popup_manager.open_popup(PopupType::QuantitativeBacktest);
        }
        
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.scroll_calculator.set_auto_track(!self.scroll_calculator.is_auto_tracking());
        }
        
        // 价格精度调整 - 使用新的API
        let scroll_delta = ctx.input(|i| i.raw_scroll_delta.y);
        if scroll_delta != 0.0 {
            let factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
            let new_precision = (self.price_precision * factor).clamp(0.01, 10.0);
            self.set_price_precision(new_precision);
        }
    }
    
    /// 获取组件状态摘要
    pub fn get_status_summary(&self) -> String {
        format!(
            "订单簿组件 - 数据点: {}, FPS: {:.1}, 精度: {:.2}, 自动跟踪: {}",
            self.cached_visible_data.len(),
            self.performance_tracker.fps(),
            self.price_precision,
            self.auto_track_price
        )
    }
    
    /// 导出配置
    pub fn export_config(&self) -> serde_json::Value {
        serde_json::json!({
            "auto_track_price": self.auto_track_price,
            "time_window_seconds": self.time_window_seconds,
            "visible_price_levels": self.visible_price_levels,
            "price_precision": self.price_precision,
            "tick_pressure_k_value": self.tick_pressure_k_value,
            "column_widths": {
                "price": self.column_widths.price,
                "bids_asks": self.column_widths.bids_asks,
                "buy": self.column_widths.buy,
                "sell": self.column_widths.sell,
                "delta": self.column_widths.delta,
            }
        })
    }
    
    /// 导入配置
    pub fn import_config(&mut self, config: serde_json::Value) {
        if let Ok(auto_track) = serde_json::from_value::<bool>(config["auto_track_price"].clone()) {
            self.auto_track_price = auto_track;
            self.scroll_calculator.set_auto_track(auto_track);
        }
        
        if let Ok(time_window) = serde_json::from_value::<u64>(config["time_window_seconds"].clone()) {
            self.time_window_seconds = time_window;
            self.data_extractor.set_time_window(time_window);
        }
        
        if let Ok(levels) = serde_json::from_value::<usize>(config["visible_price_levels"].clone()) {
            self.visible_price_levels = levels;
        }
        
        if let Ok(precision) = serde_json::from_value::<f64>(config["price_precision"].clone()) {
            self.set_price_precision(precision);
        }
        
        // 导入列宽设置
        if let Some(widths) = config["column_widths"].as_object() {
            if let Ok(price_width) = serde_json::from_value::<f32>(widths["price"].clone()) {
                self.column_widths.price = price_width;
            }
            // ... 其他列宽类似处理
        }
    }
}