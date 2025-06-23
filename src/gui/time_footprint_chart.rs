use eframe::egui;
use egui_plot::{Plot, PlotPoint, Line, Points, GridMark, GridInput};
use crate::gui::time_footprint_data::TimeFootprintData;
use crate::app::ReactiveApp;
use std::collections::HashMap;
use chrono::{Utc, TimeZone};

/// 价格线数据点 - 每秒的收盘价
#[derive(Debug, Clone)]
struct PriceLinePoint {
    /// 秒级时间戳
    second_timestamp: u64,
    /// 收盘价
    close_price: f64,
}

/// Delta成交量点数据
#[derive(Debug, Clone)]
struct DeltaVolumePoint {
    /// 秒级时间戳
    second_timestamp: u64,
    /// 价格（收盘价）
    price: f64,
    /// Delta成交量 (买单量 - 卖单量)
    delta_volume: f64,
}

/// 秒级聚合数据
#[derive(Debug, Clone)]
struct SecondAggregateData {
    /// 总买单量
    total_buy_volume: f64,
    /// 总卖单量
    total_sell_volume: f64,
    /// 成交量加权价格总和
    volume_weighted_price_sum: f64,
    /// 总成交量
    total_volume: f64,
}

impl SecondAggregateData {
    fn new() -> Self {
        Self {
            total_buy_volume: 0.0,
            total_sell_volume: 0.0,
            volume_weighted_price_sum: 0.0,
            total_volume: 0.0,
        }
    }

    fn add_trade(&mut self, price: f64, buy_volume: f64, sell_volume: f64) {
        self.total_buy_volume += buy_volume;
        self.total_sell_volume += sell_volume;
        let volume = buy_volume + sell_volume;
        self.volume_weighted_price_sum += price * volume;
        self.total_volume += volume;
    }

    fn is_empty(&self) -> bool {
        self.total_volume == 0.0
    }

    fn get_volume_weighted_price(&self) -> f64 {
        if self.total_volume > 0.0 {
            self.volume_weighted_price_sum / self.total_volume
        } else {
            0.0
        }
    }

    fn get_delta_volume(&self) -> f64 {
        self.total_buy_volume - self.total_sell_volume
    }
}

/// 时间维度足迹图表组件
pub struct TimeFootprintChart {
    /// 显示的时间窗口（分钟数）
    display_window_minutes: usize,
    /// 是否自动跟随最新数据
    auto_follow: bool,
    /// 图表缩放状态
    zoom_level: f32,
    /// 颜色配置
    buy_color: egui::Color32,
    sell_color: egui::Color32,
    /// 上次更新时间（用于性能优化）
    last_update_time: std::time::Instant,
    /// 缓存的图表数据 - 按秒聚合的价格线数据
    cached_price_line_data: Vec<PriceLinePoint>,
    /// 缓存的delta成交量点数据
    cached_delta_points: Vec<DeltaVolumePoint>,
    /// 数据版本（用于检测数据变化）
    data_version: u64,
}

impl Default for TimeFootprintChart {
    fn default() -> Self {
        Self {
            display_window_minutes: 25, // 显示最近25分钟
            auto_follow: true,
            zoom_level: 1.0,
            buy_color: egui::Color32::from_rgba_unmultiplied(120, 255, 120, 180), // 半透明绿色
            sell_color: egui::Color32::from_rgba_unmultiplied(255, 120, 120, 180), // 半透明红色
            last_update_time: std::time::Instant::now(),
            cached_price_line_data: Vec::new(),
            cached_delta_points: Vec::new(),
            data_version: 0,
        }
    }
}

impl TimeFootprintChart {
    pub fn new() -> Self {
        Self::default()
    }

    /// 渲染时间维度足迹图表
    pub fn show(&mut self, ui: &mut egui::Ui, app: &ReactiveApp) {
        // 获取时间维度数据
        let time_footprint_data = app.get_orderbook_manager().get_time_footprint_data();
        
        // 检查是否需要更新缓存数据
        let current_data_version = time_footprint_data.total_trades_processed;
        if current_data_version != self.data_version || 
           self.last_update_time.elapsed().as_millis() > 100 { // 100ms更新间隔
            self.update_cached_data(time_footprint_data);
            self.data_version = current_data_version;
            self.last_update_time = std::time::Instant::now();
        }

        // 创建图表，配置固定的网格间距和自定义格式化器
        let plot = Plot::new("time_footprint_chart")
            .legend(egui_plot::Legend::default().position(egui_plot::Corner::LeftTop))
            .show_axes([true, true])
            .show_grid([true, true])
            .allow_zoom(true)
            .allow_drag(true)
            .allow_scroll(true)
            .auto_bounds([true, true].into())
            .x_axis_label("时间")
            .y_axis_label("价格 (USD)")
            .width(ui.available_width())
            .height(ui.available_height())
            // X轴：固定30秒间距，自定义时间格式化
            .x_grid_spacer(Self::time_grid_spacer)
            .x_axis_formatter(Self::format_time_axis)
            // Y轴：固定50美元间距，自定义价格格式化
            .y_grid_spacer(Self::price_grid_spacer)
            .y_axis_formatter(Self::format_price_axis);

        plot.show(ui, |plot_ui| {
            // 如果没有数据，显示提示
            if self.cached_price_line_data.is_empty() {
                // 显示无数据提示
                let center_point = PlotPoint::new(0.0, 50000.0);
                plot_ui.text(
                    egui_plot::Text::new(center_point, "暂无交易数据")
                        .color(egui::Color32::GRAY)
                );
                return;
            }

            // 渲染价格线
            self.render_price_line(plot_ui);

            // 渲染价格参考线
            self.render_price_reference_lines(plot_ui, app);
        });

        // 渲染控制面板
        self.render_control_panel(ui);
    }

    /// 更新缓存的图表数据 - 按秒聚合价格线和delta成交量
    fn update_cached_data(&mut self, time_footprint_data: &TimeFootprintData) {
        self.cached_price_line_data.clear();
        self.cached_delta_points.clear();

        // 获取最近的数据
        let recent_data = time_footprint_data.get_recent_data(self.display_window_minutes);

        // 按秒聚合数据
        let mut second_data: HashMap<u64, SecondAggregateData> = HashMap::new();

        for minute_data in recent_data {
            // 将分钟数据拆分为60秒
            for second_offset in 0..60 {
                let second_timestamp = minute_data.minute_timestamp + (second_offset * 1000);

                // 模拟每秒的数据（实际应用中应该有真实的秒级数据）
                let mut second_aggregate = SecondAggregateData::new();

                // 将分钟级数据平均分配到60秒中
                for price_level in minute_data.get_sorted_price_levels() {
                    if price_level.get_total_volume() > 0.0 {
                        // 简单平均分配（实际应用中应该有更精确的时间戳）
                        let buy_volume = price_level.buy_volume / 60.0;
                        let sell_volume = price_level.sell_volume / 60.0;

                        if buy_volume > 0.0 || sell_volume > 0.0 {
                            second_aggregate.add_trade(price_level.price, buy_volume, sell_volume);
                        }
                    }
                }

                if !second_aggregate.is_empty() {
                    second_data.insert(second_timestamp, second_aggregate);
                }
            }
        }

        // 生成价格线数据和delta成交量点
        let mut sorted_timestamps: Vec<u64> = second_data.keys().cloned().collect();
        sorted_timestamps.sort();

        for timestamp in sorted_timestamps {
            if let Some(aggregate) = second_data.get(&timestamp) {
                // 价格线点（使用成交量加权平均价格作为收盘价）
                let close_price = aggregate.get_volume_weighted_price();
                self.cached_price_line_data.push(PriceLinePoint {
                    second_timestamp: timestamp,
                    close_price,
                });

                // Delta成交量点
                let delta_volume = aggregate.get_delta_volume();
                if delta_volume.abs() > 0.01 { // 只显示有意义的delta
                    self.cached_delta_points.push(DeltaVolumePoint {
                        second_timestamp: timestamp,
                        price: close_price,
                        delta_volume,
                    });
                }
            }
        }
    }

    /// 渲染价格线
    fn render_price_line(&self, plot_ui: &mut egui_plot::PlotUi) {
        if self.cached_price_line_data.len() < 2 {
            return;
        }

        // 构建价格线的点集合
        let price_points: Vec<[f64; 2]> = self.cached_price_line_data
            .iter()
            .map(|point| [
                self.timestamp_to_plot_x(point.second_timestamp),
                point.close_price
            ])
            .collect();

        // 创建价格线
        let price_line = Line::new(price_points)
            .color(egui::Color32::WHITE)
            .width(2.0)
            .name("价格线");

        plot_ui.line(price_line);
    }

    /// 渲染delta成交量点
    fn render_delta_volume_points(&self, plot_ui: &mut egui_plot::PlotUi) {
        if self.cached_delta_points.is_empty() {
            return;
        }

        // 计算最大delta成交量用于缩放点的大小
        let max_abs_delta = self.cached_delta_points
            .iter()
            .map(|p| p.delta_volume.abs())
            .fold(0.0, f64::max);

        if max_abs_delta == 0.0 {
            return;
        }

        // 分别处理正delta（买单优势）和负delta（卖单优势）
        let mut positive_points = Vec::new();
        let mut negative_points = Vec::new();

        for point in &self.cached_delta_points {
            let x = self.timestamp_to_plot_x(point.second_timestamp);
            let y = point.price;

            // 计算点的半径（基于delta成交量的绝对值）
            let normalized_delta = point.delta_volume.abs() / max_abs_delta;
            let radius = (normalized_delta * 10.0).max(2.0); // 最小半径2，最大半径10

            if point.delta_volume > 0.0 {
                // 正delta - 买单优势，使用绿色
                positive_points.push([x, y]);
            } else if point.delta_volume < 0.0 {
                // 负delta - 卖单优势，使用红色
                negative_points.push([x, y]);
            }
        }

        // 渲染正delta点（买单优势）
        if !positive_points.is_empty() {
            let positive_points_plot = Points::new(positive_points)
                .color(self.buy_color)
                .radius(5.0) // 固定半径，后续可以根据需要调整
                .name("买单优势");
            plot_ui.points(positive_points_plot);
        }

        // 渲染负delta点（卖单优势）
        if !negative_points.is_empty() {
            let negative_points_plot = Points::new(negative_points)
                .color(self.sell_color)
                .radius(5.0) // 固定半径，后续可以根据需要调整
                .name("卖单优势");
            plot_ui.points(negative_points_plot);
        }
    }

    /// 渲染价格参考线
    fn render_price_reference_lines(&self, plot_ui: &mut egui_plot::PlotUi, app: &ReactiveApp) {
        let snapshot = app.get_market_snapshot();
        
        // 当前价格线
        if let Some(current_price) = snapshot.current_price {
            let time_range = self.get_time_range();
            if let Some((start_time, end_time)) = time_range {
                let current_price_line = Line::new(vec![
                    [start_time, current_price],
                    [end_time, current_price],
                ])
                .color(egui::Color32::YELLOW)
                .width(2.0)
                .name("当前价格");
                
                plot_ui.line(current_price_line);
            }
        }
        
        // 最优买卖价线
        if let Some(best_bid) = snapshot.best_bid_price {
            let time_range = self.get_time_range();
            if let Some((start_time, end_time)) = time_range {
                let bid_line = Line::new(vec![
                    [start_time, best_bid],
                    [end_time, best_bid],
                ])
                .color(self.buy_color.gamma_multiply(1.5))
                .width(1.0)
                .style(egui_plot::LineStyle::Dashed { length: 5.0 })
                .name("最优买价");
                
                plot_ui.line(bid_line);
            }
        }
        
        if let Some(best_ask) = snapshot.best_ask_price {
            let time_range = self.get_time_range();
            if let Some((start_time, end_time)) = time_range {
                let ask_line = Line::new(vec![
                    [start_time, best_ask],
                    [end_time, best_ask],
                ])
                .color(self.sell_color.gamma_multiply(1.5))
                .width(1.0)
                .style(egui_plot::LineStyle::Dashed { length: 5.0 })
                .name("最优卖价");
                
                plot_ui.line(ask_line);
            }
        }
    }

    /// 渲染控制面板
    fn render_control_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("显示窗口:");
            ui.add(egui::Slider::new(&mut self.display_window_minutes, 10..=60)
                .suffix(" 分钟"));

            ui.separator();

            ui.checkbox(&mut self.auto_follow, "自动跟随");

            ui.separator();

            if ui.button("重置缩放").clicked() {
                self.zoom_level = 1.0;
            }

            ui.separator();

            ui.label("显示: 每秒收盘价线型图");
        });
    }

    /// 将时间戳转换为图表X坐标
    fn timestamp_to_plot_x(&self, timestamp: u64) -> f64 {
        // 将毫秒时间戳转换为秒
        (timestamp / 1000) as f64
    }

    /// 获取当前显示的时间范围
    fn get_time_range(&self) -> Option<(f64, f64)> {
        if self.cached_price_line_data.is_empty() {
            return None;
        }

        let timestamps: Vec<u64> = self.cached_price_line_data.iter()
            .map(|p| p.second_timestamp)
            .collect();

        let min_timestamp = *timestamps.iter().min()?;
        let max_timestamp = *timestamps.iter().max()?;

        Some((
            self.timestamp_to_plot_x(min_timestamp),
            self.timestamp_to_plot_x(max_timestamp)
        ))
    }

    /// X轴时间网格间距器 - 固定30秒间距
    fn time_grid_spacer(input: GridInput) -> Vec<GridMark> {
        let mut marks = Vec::new();

        // 固定30秒间距
        let step_size = 30.0; // 30秒对应30.0单位

        // 计算起始和结束的秒标记，向下和向上取整到30的倍数
        let start_second = ((input.bounds.0 / 30.0).floor() as i64) * 30;
        let end_second = ((input.bounds.1 / 30.0).ceil() as i64) * 30;

        // 生成每30秒的网格标记
        let mut second = start_second;
        while second <= end_second {
            let value = second as f64;
            if value >= input.bounds.0 && value <= input.bounds.1 {
                marks.push(GridMark {
                    value,
                    step_size,
                });
            }
            second += 30; // 每次增加30秒
        }

        marks
    }

    /// Y轴价格网格间距器 - 固定50美元间距
    fn price_grid_spacer(input: GridInput) -> Vec<GridMark> {
        let mut marks = Vec::new();

        // 固定50美元间距
        let step_size = 50.0;

        // 计算起始和结束的价格标记，向下和向上取整到50的倍数
        let start_price = ((input.bounds.0 / 50.0).floor() as i64) * 50;
        let end_price = ((input.bounds.1 / 50.0).ceil() as i64) * 50;

        // 生成每50美元的网格标记
        let mut price = start_price;
        while price <= end_price {
            let value = price as f64;
            if value >= input.bounds.0 && value <= input.bounds.1 {
                marks.push(GridMark {
                    value,
                    step_size,
                });
            }
            price += 50; // 每次增加50美元
        }

        marks
    }

    /// X轴时间格式化器 - 显示为 HH:MM:SS 格式
    fn format_time_axis(mark: GridMark, _axis_index: usize, _range: &std::ops::RangeInclusive<f64>) -> String {
        // 将秒数转换回时间戳
        let second_timestamp = (mark.value as u64) * 1000; // 转换为毫秒时间戳

        // 转换为UTC时间
        let datetime = Utc.timestamp_millis_opt(second_timestamp as i64)
            .single()
            .unwrap_or_else(|| Utc::now());

        // 格式化为 HH:MM:SS
        datetime.format("%H:%M:%S").to_string()
    }

    /// Y轴价格格式化器 - 显示为整数价格
    fn format_price_axis(mark: GridMark, _axis_index: usize, _range: &std::ops::RangeInclusive<f64>) -> String {
        // 显示为整数价格，例如 101480, 101481, 101482
        format!("{:.0}", mark.value)
    }
}
