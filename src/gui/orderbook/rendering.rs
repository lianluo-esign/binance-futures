/// 订单簿渲染模块

use eframe::egui;
use super::types::{UnifiedOrderBookRow, ColorScheme};

/// 表格渲染器
pub struct TableRenderer {
    /// 颜色方案
    color_scheme: ColorScheme,
    /// 列宽设置
    column_widths: ColumnWidths,
    /// 条形图渲染器
    bar_renderer: BarRenderer,
}

/// 列宽配置
#[derive(Debug, Clone)]
pub struct ColumnWidths {
    pub price: f32,
    pub bids_asks: f32,
    pub buy: f32,
    pub sell: f32,
    pub delta: f32,
}

impl Default for ColumnWidths {
    fn default() -> Self {
        Self {
            price: 80.0,
            bids_asks: 200.0,
            buy: 100.0,
            sell: 100.0,
            delta: 80.0,
        }
    }
}

impl TableRenderer {
    pub fn new(color_scheme: ColorScheme, column_widths: ColumnWidths) -> Self {
        Self {
            color_scheme,
            column_widths,
            bar_renderer: BarRenderer::new(),
        }
    }
    
    /// 渲染统一表格
    pub fn render_unified_table(
        &mut self,
        ui: &mut egui::Ui,
        data: &[UnifiedOrderBookRow],
        current_price: f64,
        scroll_position: f32,
        visible_rows: usize,
    ) {
        // 添加调试信息 - 减少日志频率
        static mut LAST_RENDER_LOG_TIME: Option<std::time::Instant> = None;
        let now = std::time::Instant::now();
        unsafe {
            if LAST_RENDER_LOG_TIME.map_or(true, |last| now.duration_since(last) > std::time::Duration::from_secs(10)) {
                log::info!("渲染表格: {} 行数据, 滚动位置: {}, 可见行: {}", data.len(), scroll_position, visible_rows);
                LAST_RENDER_LOG_TIME = Some(now);
            }
        }
        
        // 如果没有数据，显示提示信息
        if data.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.heading("📊 订单簿数据");
                ui.label("正在加载数据...");
            });
            return;
        }
        
        // 简单的数据显示作为备用方案
        if data.len() > 0 {
            ui.separator();
            ui.heading(format!("📈 当前价格: {:.2}", current_price));
            ui.label(format!("📊 数据行数: {}", data.len()));
            ui.separator();
        }
        
        use egui_extras::{Column, TableBuilder};
        
        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::exact(self.column_widths.price))      // 价格
            .column(Column::exact(self.column_widths.bids_asks))  // 买单/卖单
            .column(Column::exact(self.column_widths.buy))        // 买量
            .column(Column::exact(self.column_widths.sell))       // 卖量
            .column(Column::exact(self.column_widths.delta))      // Delta
            .scroll_to_row(scroll_position as usize, None);
        
        table.header(25.0, |mut header| {
            header.col(|ui| { ui.heading("价格"); });
            header.col(|ui| { ui.heading("买单/卖单"); });
            header.col(|ui| { ui.heading("买量"); });
            header.col(|ui| { ui.heading("卖量"); });
            header.col(|ui| { ui.heading("Delta"); });
        }).body(|mut body| {
            let start_idx = scroll_position as usize;
            let end_idx = (start_idx + visible_rows).min(data.len());
            
            for (i, row) in data[start_idx..end_idx].iter().enumerate() {
                let is_current_price = self.is_current_price_row(row.price, current_price);
                
                body.row(25.0, |mut table_row| {
                    self.render_table_row(&mut table_row, row, is_current_price);
                });
            }
        });
    }
    
    /// 渲染表格行
    fn render_table_row(
        &self,
        row: &mut egui_extras::TableRow,
        data: &UnifiedOrderBookRow,
        is_current_price: bool,
    ) {
        // 价格列
        row.col(|ui| {
            self.render_price_cell(ui, data.price, is_current_price);
        });
        
        // 买单/卖单列
        row.col(|ui| {
            self.render_bid_ask_cell(ui, data);
        });
        
        // 买量列
        row.col(|ui| {
            self.render_volume_cell(ui, data.active_buy_volume_5s, true);
        });
        
        // 卖量列
        row.col(|ui| {
            self.render_volume_cell(ui, data.active_sell_volume_5s, false);
        });
        
        // Delta列
        row.col(|ui| {
            self.render_delta_cell(ui, data.delta);
        });
    }
    
    /// 渲染价格单元格
    fn render_price_cell(&self, ui: &mut egui::Ui, price: f64, is_current_price: bool) {
        if is_current_price {
            // 当前价格高亮显示
            let rect = ui.available_rect_before_wrap();
            ui.painter().rect_filled(
                rect,
                egui::Rounding::same(2.0),
                self.color_scheme.current_price_bg,
            );
            ui.colored_label(self.color_scheme.current_price_text, format!("{:.2}", price));
        } else {
            ui.label(format!("{:.2}", price));
        }
    }
    
    /// 渲染买单/卖单单元格
    fn render_bid_ask_cell(&self, ui: &mut egui::Ui, data: &UnifiedOrderBookRow) {
        ui.horizontal(|ui| {
            // 买单量条形图
            if data.bid_volume > 0.0 {
                let width = self.bar_renderer.calculate_bar_width(
                    data.bid_volume, 
                    data.bid_volume.max(data.ask_volume)
                );
                self.bar_renderer.draw_background_bar(
                    ui, 
                    width, 
                    self.apply_fade(self.color_scheme.bid_color, data.bid_fade_alpha)
                );
                ui.colored_label(self.color_scheme.bid_color, format!("{:.3}", data.bid_volume));
            }
            
            ui.separator();
            
            // 卖单量条形图
            if data.ask_volume > 0.0 {
                let width = self.bar_renderer.calculate_bar_width(
                    data.ask_volume, 
                    data.bid_volume.max(data.ask_volume)
                );
                self.bar_renderer.draw_background_bar(
                    ui, 
                    width, 
                    self.apply_fade(self.color_scheme.ask_color, data.ask_fade_alpha)
                );
                ui.colored_label(self.color_scheme.ask_color, format!("{:.3}", data.ask_volume));
            }
        });
    }
    
    /// 渲染成交量单元格
    fn render_volume_cell(&self, ui: &mut egui::Ui, volume: f64, is_buy: bool) {
        if volume > 0.0 {
            let color = if is_buy { 
                self.color_scheme.bid_color 
            } else { 
                self.color_scheme.ask_color 
            };
            ui.colored_label(color, format!("{:.1}", volume));
        }
    }
    
    /// 渲染Delta单元格
    fn render_delta_cell(&self, ui: &mut egui::Ui, delta: f64) {
        if delta.abs() > 0.01 {
            let color = if delta > 0.0 {
                self.color_scheme.positive_delta
            } else {
                self.color_scheme.negative_delta
            };
            ui.colored_label(color, format!("{:+.1}", delta));
        } else {
            ui.colored_label(self.color_scheme.neutral, "0.0");
        }
    }
    
    /// 检查是否为当前价格行
    fn is_current_price_row(&self, row_price: f64, current_price: f64) -> bool {
        if current_price <= 0.0 {
            return false;
        }
        (row_price - current_price).abs() < 0.01
    }
    
    /// 应用淡出效果
    fn apply_fade(&self, color: egui::Color32, alpha: f32) -> egui::Color32 {
        let [r, g, b, _] = color.to_array();
        egui::Color32::from_rgba_premultiplied(r, g, b, (255.0 * alpha) as u8)
    }
    
    /// 渲染Logo
    pub fn render_logo(&self, ui: &mut egui::Ui, texture: &egui::TextureHandle, header_height: f32) {
        let logo_size = header_height * 0.8;
        let image = egui::Image::new(texture)
            .fit_to_exact_size(egui::Vec2::splat(logo_size))
            .rounding(egui::Rounding::same(4.0));
        
        ui.add(image);
    }
    
    pub fn set_color_scheme(&mut self, scheme: ColorScheme) {
        self.color_scheme = scheme;
    }
    
    pub fn set_column_widths(&mut self, widths: ColumnWidths) {
        self.column_widths = widths;
    }
}

/// 条形图渲染器
pub struct BarRenderer {
    max_bar_width: f32,
}

impl BarRenderer {
    pub fn new() -> Self {
        Self {
            max_bar_width: 80.0,
        }
    }
    
    /// 绘制水平条形图
    pub fn draw_horizontal_bar(&self, ui: &mut egui::Ui, width: f32, color: egui::Color32) {
        let rect = ui.available_rect_before_wrap();
        let bar_height = rect.height() * 0.6;
        let bar_rect = egui::Rect::from_min_size(
            rect.min + egui::Vec2::new(0.0, (rect.height() - bar_height) / 2.0),
            egui::Vec2::new(width, bar_height),
        );
        
        ui.painter().rect_filled(bar_rect, egui::Rounding::same(2.0), color);
    }
    
    /// 绘制背景条形图
    pub fn draw_background_bar(&self, ui: &mut egui::Ui, width: f32, color: egui::Color32) {
        let rect = ui.available_rect_before_wrap();
        let bar_height = rect.height() * 0.3;
        
        // 背景条形图
        let bg_rect = egui::Rect::from_min_size(
            rect.min + egui::Vec2::new(0.0, (rect.height() - bar_height) / 2.0),
            egui::Vec2::new(width.min(rect.width()), bar_height),
        );
        
        ui.painter().rect_filled(
            bg_rect,
            egui::Rounding::same(1.0),
            color.gamma_multiply(0.3), // 半透明背景
        );
    }
    
    /// 计算条形图宽度
    pub fn calculate_bar_width(&self, value: f64, max_value: f64) -> f32 {
        if max_value <= 0.0 {
            return 0.0;
        }
        
        let ratio = (value / max_value).min(1.0).max(0.0);
        ratio as f32 * self.max_bar_width
    }
    
    /// 设置最大条形图宽度
    pub fn set_max_bar_width(&mut self, width: f32) {
        self.max_bar_width = width.max(10.0); // 最小宽度限制
    }
    
    /// 绘制渐变条形图
    pub fn draw_gradient_bar(
        &self, 
        ui: &mut egui::Ui, 
        width: f32, 
        start_color: egui::Color32, 
        end_color: egui::Color32
    ) {
        let rect = ui.available_rect_before_wrap();
        let bar_height = rect.height() * 0.6;
        let bar_rect = egui::Rect::from_min_size(
            rect.min + egui::Vec2::new(0.0, (rect.height() - bar_height) / 2.0),
            egui::Vec2::new(width, bar_height),
        );
        
        // 简化的渐变效果（使用中间色）
        let mid_color = egui::Color32::from_rgba_premultiplied(
            ((start_color.r() as u16 + end_color.r() as u16) / 2) as u8,
            ((start_color.g() as u16 + end_color.g() as u16) / 2) as u8,
            ((start_color.b() as u16 + end_color.b() as u16) / 2) as u8,
            ((start_color.a() as u16 + end_color.a() as u16) / 2) as u8,
        );
        
        ui.painter().rect_filled(bar_rect, egui::Rounding::same(2.0), mid_color);
    }
}

/// 网格渲染器
pub struct GridRenderer {
    color_scheme: ColorScheme,
}

impl GridRenderer {
    pub fn new(color_scheme: ColorScheme) -> Self {
        Self { color_scheme }
    }
    
    /// 绘制价格网格线
    pub fn draw_price_grid(&self, ui: &mut egui::Ui, price_levels: &[f64]) {
        let rect = ui.available_rect_before_wrap();
        
        for (i, _price) in price_levels.iter().enumerate() {
            let y = rect.min.y + (i as f32 * 25.0); // 假设每行25像素高
            if y < rect.max.y {
                ui.painter().hline(
                    rect.min.x..=rect.max.x,
                    y,
                    egui::Stroke::new(0.5, self.color_scheme.grid_lines),
                );
            }
        }
    }
    
    /// 绘制当前价格指示线
    pub fn draw_current_price_line(&self, ui: &mut egui::Ui, y_position: f32) {
        let rect = ui.available_rect_before_wrap();
        
        ui.painter().hline(
            rect.min.x..=rect.max.x,
            y_position,
            egui::Stroke::new(2.0, self.color_scheme.current_price_bg),
        );
    }
}