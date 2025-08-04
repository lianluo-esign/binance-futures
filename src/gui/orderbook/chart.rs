/// 价格图表模块

use eframe::egui;
use egui_plot::{Plot, Line, PlotPoints, PlotPoint, GridInput, GridMark};
use std::collections::VecDeque;
use super::types::{PriceHistoryPoint, ColorScheme};

/// 图表配置
#[derive(Debug, Clone)]
pub struct ChartConfig {
    /// 图表高度
    pub height: f32,
    /// 最大数据点数
    pub max_data_points: usize,
    /// 是否显示网格
    pub show_grid: bool,
    /// 是否显示图例
    pub show_legend: bool,
    /// 颜色方案
    pub color_scheme: ColorScheme,
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            height: 300.0,
            max_data_points: 1000,
            show_grid: true,
            show_legend: true,
            color_scheme: ColorScheme::default(),
        }
    }
}

/// 价格图表组件
pub struct PriceChart {
    /// 配置
    config: ChartConfig,
    /// 价格历史数据
    price_history: VecDeque<PriceHistoryPoint>,
    /// 是否显示模态窗口
    modal_open: bool,
    /// Logo纹理
    logo_texture: Option<egui::TextureHandle>,
}

impl PriceChart {
    pub fn new(config: ChartConfig) -> Self {
        let max_data_points = config.max_data_points;
        Self {
            config,
            price_history: VecDeque::with_capacity(max_data_points),
            modal_open: false,
            logo_texture: None,
        }
    }
    
    /// 添加价格数据点
    pub fn add_price_point(&mut self, point: PriceHistoryPoint) {
        self.price_history.push_back(point);
        
        // 限制数据点数量
        while self.price_history.len() > self.config.max_data_points {
            self.price_history.pop_front();
        }
    }
    
    /// 批量添加价格数据
    pub fn add_price_points(&mut self, points: Vec<PriceHistoryPoint>) {
        for point in points {
            self.add_price_point(point);
        }
    }
    
    /// 更新价格历史
    pub fn update_price_history(&mut self, current_price: f64, volume: f64, side: String) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        
        let point = PriceHistoryPoint::new(timestamp, current_price, volume, side);
        self.add_price_point(point);
    }
    
    /// 显示价格图表（嵌入式）
    pub fn show_embedded(&mut self, ui: &mut egui::Ui) {
        if self.price_history.is_empty() {
            ui.label("暂无价格数据");
            return;
        }
        
        let plot = Plot::new("price_chart")
            .height(self.config.height)
            .show_axes([true, true])
            .show_grid(self.config.show_grid)
            .allow_zoom(true)
            .allow_drag(true)
            .allow_scroll(true)
            .include_x(self.get_time_range().0)
            .include_x(self.get_time_range().1)
            .include_y(self.get_price_range().0)
            .include_y(self.get_price_range().1);
        
        plot.show(ui, |plot_ui| {
            self.render_price_line(plot_ui);
            self.render_volume_bars(plot_ui);
            self.render_current_price_marker(plot_ui);
        });
    }
    
    /// 显示价格图表模态窗口
    pub fn show_modal(&mut self, ctx: &egui::Context) {
        if !self.modal_open {
            return;
        }
        
        let mut modal_open = self.modal_open;
        let result = egui::Window::new("价格图表")
            .open(&mut modal_open)
            .default_size(egui::Vec2::new(800.0, 600.0))
            .resizable(true)
            .show(ctx, |ui| {
                self.render_chart_controls(ui);
                ui.separator();
                self.render_detailed_chart(ui);
            });
        self.modal_open = modal_open;
    }
    
    /// 渲染价格线
    fn render_price_line(&self, plot_ui: &mut egui_plot::PlotUi) {
        let points: PlotPoints = self.price_history
            .iter()
            .map(|point| [point.timestamp, point.price])
            .collect();
        
        let line = Line::new(points)
            .color(self.config.color_scheme.bid_color)
            .width(2.0)
            .name("价格");
        
        plot_ui.line(line);
    }
    
    /// 渲染成交量柱状图
    fn render_volume_bars(&self, plot_ui: &mut egui_plot::PlotUi) {
        // 简化实现：将成交量数据转换为柱状图
        for (i, point) in self.price_history.iter().enumerate() {
            let color = if point.is_buy() {
                self.config.color_scheme.bid_color
            } else {
                self.config.color_scheme.ask_color
            };
            
            // 创建简单的成交量指示
            let volume_points = vec![
                [point.timestamp, point.price - point.volume * 0.1],
                [point.timestamp, point.price + point.volume * 0.1],
            ];
            
            let volume_line = Line::new(PlotPoints::from(volume_points))
                .color(color.gamma_multiply(0.5))
                .width(1.0);
            
            plot_ui.line(volume_line);
        }
    }
    
    /// 渲染当前价格标记
    fn render_current_price_marker(&self, plot_ui: &mut egui_plot::PlotUi) {
        if let Some(latest_point) = self.price_history.back() {
            let marker_points = vec![
                [latest_point.timestamp, latest_point.price]
            ];
            
            let marker = egui_plot::Points::new(PlotPoints::from(marker_points))
                .color(self.config.color_scheme.current_price_bg)
                .radius(8.0)
                .name("当前价格");
            
            plot_ui.points(marker);
            
            // 添加Logo标记（如果有纹理）
            if let Some(_texture) = &self.logo_texture {
                // 在当前价格位置显示Logo
                // 注意：egui_plot 不直接支持纹理，这里是概念性实现
            }
        }
    }
    
    /// 渲染详细图表
    fn render_detailed_chart(&mut self, ui: &mut egui::Ui) {
        let plot = Plot::new("detailed_price_chart")
            .height(400.0)
            .show_axes([true, true])
            .show_grid(true)
            .allow_zoom(true)
            .allow_drag(true)
            .allow_scroll(true)
            .legend(egui_plot::Legend::default());
        
        plot.show(ui, |plot_ui| {
            self.render_price_line(plot_ui);
            self.render_volume_bars(plot_ui);
            self.render_current_price_marker(plot_ui);
            self.render_trade_signals(plot_ui);
        });
    }
    
    /// 渲染图表控制面板
    fn render_chart_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("图表控制:");
            
            if ui.button("清除数据").clicked() {
                self.price_history.clear();
            }
            
            if ui.button("重置缩放").clicked() {
                // 重置缩放逻辑
            }
            
            ui.checkbox(&mut self.config.show_grid, "显示网格");
            ui.checkbox(&mut self.config.show_legend, "显示图例");
            
            ui.separator();
            
            ui.label(format!("数据点: {}/{}", 
                self.price_history.len(), 
                self.config.max_data_points
            ));
        });
    }
    
    /// 渲染交易信号
    fn render_trade_signals(&self, plot_ui: &mut egui_plot::PlotUi) {
        // 简化的交易信号渲染
        let mut buy_signals = Vec::new();
        let mut sell_signals = Vec::new();
        
        for point in &self.price_history {
            if point.volume > 1000.0 { // 大额交易阈值
                if point.is_buy() {
                    buy_signals.push([point.timestamp, point.price]);
                } else {
                    sell_signals.push([point.timestamp, point.price]);
                }
            }
        }
        
        if !buy_signals.is_empty() {
            let buy_points = egui_plot::Points::new(PlotPoints::from(buy_signals))
                .color(self.config.color_scheme.positive_delta)
                .radius(6.0)
                .shape(egui_plot::MarkerShape::Up)
                .name("大额买单");
            plot_ui.points(buy_points);
        }
        
        if !sell_signals.is_empty() {
            let sell_points = egui_plot::Points::new(PlotPoints::from(sell_signals))
                .color(self.config.color_scheme.negative_delta)
                .radius(6.0)
                .shape(egui_plot::MarkerShape::Down)
                .name("大额卖单");
            plot_ui.points(sell_points);
        }
    }
    
    /// 时间网格间隔器
    fn time_grid_spacer(&self, input: GridInput) -> Vec<GridMark> {
        let mut marks = Vec::new();
        
        // 简化的时间网格
        let range = input.bounds.1 - input.bounds.0;
        let step = if range < 3600.0 {
            300.0 // 5分钟
        } else if range < 86400.0 {
            3600.0 // 1小时
        } else {
            86400.0 // 1天
        };
        
        let mut time = (input.bounds.0 / step).ceil() * step;
        while time <= input.bounds.1 {
            marks.push(GridMark {
                value: time,
                step_size: step,
            });
            time += step;
        }
        
        marks
    }
    
    /// 价格网格间隔器
    fn price_grid_spacer(&self, input: GridInput) -> Vec<GridMark> {
        let mut marks = Vec::new();
        
        // 1美元间隔的价格网格
        let range = input.bounds.1 - input.bounds.0;
        let step = if range < 10.0 {
            0.1
        } else if range < 100.0 {
            1.0
        } else {
            10.0
        };
        
        let mut price = (input.bounds.0 / step).ceil() * step;
        while price <= input.bounds.1 {
            marks.push(GridMark {
                value: price,
                step_size: step,
            });
            price += step;
        }
        
        marks
    }
    
    /// 获取时间范围
    fn get_time_range(&self) -> (f64, f64) {
        if self.price_history.is_empty() {
            return (0.0, 1.0);
        }
        
        let min_time = self.price_history.front().unwrap().timestamp;
        let max_time = self.price_history.back().unwrap().timestamp;
        
        (min_time, max_time)
    }
    
    /// 获取价格范围
    fn get_price_range(&self) -> (f64, f64) {
        if self.price_history.is_empty() {
            return (0.0, 1.0);
        }
        
        let mut min_price = f64::MAX;
        let mut max_price = f64::MIN;
        
        for point in &self.price_history {
            min_price = min_price.min(point.price);
            max_price = max_price.max(point.price);
        }
        
        // 添加5%的边距
        let margin = (max_price - min_price) * 0.05;
        (min_price - margin, max_price + margin)
    }
    
    /// 设置Logo纹理
    pub fn set_logo_texture(&mut self, texture: egui::TextureHandle) {
        self.logo_texture = Some(texture);
    }
    
    /// 打开模态窗口
    pub fn open_modal(&mut self) {
        self.modal_open = true;
    }
    
    /// 关闭模态窗口
    pub fn close_modal(&mut self) {
        self.modal_open = false;
    }
    
    /// 检查模态窗口是否打开
    pub fn is_modal_open(&self) -> bool {
        self.modal_open
    }
    
    /// 获取数据点数量
    pub fn data_point_count(&self) -> usize {
        self.price_history.len()
    }
    
    /// 清除历史数据
    pub fn clear_history(&mut self) {
        self.price_history.clear();
    }
    
    /// 设置最大数据点数
    pub fn set_max_data_points(&mut self, max_points: usize) {
        self.config.max_data_points = max_points;
        self.price_history = VecDeque::with_capacity(max_points);
    }
    
    /// 导出数据为CSV格式
    pub fn export_to_csv(&self) -> String {
        let mut csv = String::from("timestamp,price,volume,side\n");
        
        for point in &self.price_history {
            csv.push_str(&format!("{},{},{},{}\n", 
                point.timestamp, point.price, point.volume, point.side));
        }
        
        csv
    }
}