use eframe::egui;
use egui_plot::{Plot, PlotPoint, Line, Points, GridMark, GridInput};
use crate::orderbook::TimeFootprintData;
use crate::app::ReactiveApp;
use std::collections::HashMap;
use chrono::{Utc, TimeZone};

/// Price line data point - closing price per second
#[derive(Debug, Clone)]
struct PriceLinePoint {
    /// Second-level timestamp
    second_timestamp: u64,
    /// Closing price
    close_price: f64,
}

/// Delta volume point data
#[derive(Debug, Clone)]
struct DeltaVolumePoint {
    /// Second-level timestamp
    second_timestamp: u64,
    /// Price (closing price)
    price: f64,
    /// Delta volume (buy volume - sell volume)
    delta_volume: f64,
}

/// Second-level aggregate data
#[derive(Debug, Clone)]
struct SecondAggregateData {
    /// Total buy volume
    total_buy_volume: f64,
    /// Total sell volume
    total_sell_volume: f64,
    /// Volume-weighted price sum
    volume_weighted_price_sum: f64,
    /// Total volume
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

/// Time dimension footprint chart component
pub struct TimeFootprintChart {
    /// Display time window (minutes)
    display_window_minutes: usize,
    /// Whether to auto-follow latest data
    auto_follow: bool,
    /// Chart zoom state
    zoom_level: f32,
    /// Color configuration
    buy_color: egui::Color32,
    sell_color: egui::Color32,
    /// Last update time (for performance optimization)
    last_update_time: std::time::Instant,
    /// Cached chart data - second-aggregated price line data
    cached_price_line_data: Vec<PriceLinePoint>,
    /// Cached delta volume point data
    cached_delta_points: Vec<DeltaVolumePoint>,
    /// Data version (for detecting data changes)
    data_version: u64,
}

impl Default for TimeFootprintChart {
    fn default() -> Self {
        Self {
            display_window_minutes: 25, // Display last 25 minutes
            auto_follow: true,
            zoom_level: 1.0,
            buy_color: egui::Color32::from_rgba_unmultiplied(120, 255, 120, 180), // Semi-transparent green
            sell_color: egui::Color32::from_rgba_unmultiplied(255, 120, 120, 180), // Semi-transparent red
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

    /// Render time dimension footprint chart
    pub fn show(&mut self, ui: &mut egui::Ui, app: &ReactiveApp) {
        // Get time dimension data
        let time_footprint_data = app.get_orderbook_manager().get_time_footprint_data();
        
        // Check if cached data needs updating
        let current_data_version = time_footprint_data.total_trades_processed;
        if current_data_version != self.data_version || 
           self.last_update_time.elapsed().as_millis() > 100 { // 100ms update interval
            self.update_cached_data(time_footprint_data);
            self.data_version = current_data_version;
            self.last_update_time = std::time::Instant::now();
        }

        // Create chart, configure fixed grid spacing and custom formatters
        let plot = Plot::new("time_footprint_chart")
            .legend(egui_plot::Legend::default().position(egui_plot::Corner::LeftTop))
            .show_axes([true, true])
            .show_grid([false, false])
            .allow_zoom(true)
            .allow_drag(true)
            .allow_scroll(true)
            .auto_bounds([true, true].into())
            .x_axis_label("Time")
            .y_axis_label("Price (USD)")
            .width(ui.available_width())
            .height(ui.available_height())
            // X-axis: fixed 30-second spacing, custom time formatting
            .x_grid_spacer(Self::time_grid_spacer)
            .x_axis_formatter(Self::format_time_axis)
            // Y-axis: fixed $50 spacing, custom price formatting
            .y_grid_spacer(Self::price_grid_spacer)
            .y_axis_formatter(Self::format_price_axis);

        plot.show(ui, |plot_ui| {
            // If no data, show prompt
            if self.cached_price_line_data.is_empty() {
                // Show no data prompt
                let center_point = PlotPoint::new(0.0, 50000.0);
                plot_ui.text(
                    egui_plot::Text::new(center_point, "No Trade Data Available")
                        .color(egui::Color32::GRAY)
                );
                return;
            }

            // Render price line
            self.render_price_line(plot_ui);

            // Render price reference lines
            self.render_price_reference_lines(plot_ui, app);
        });

        // Render control panel
        self.render_control_panel(ui);
    }

    /// Update cached chart data - second-aggregated price line and delta volume
    fn update_cached_data(&mut self, time_footprint_data: &TimeFootprintData) {
        self.cached_price_line_data.clear();
        self.cached_delta_points.clear();

        // Get recent data
        let recent_data = time_footprint_data.get_recent_data(self.display_window_minutes);

        // Aggregate data by second
        let mut second_data: HashMap<u64, SecondAggregateData> = HashMap::new();

        for minute_data in recent_data {
            // Split minute data into 60 seconds
            for second_offset in 0..60 {
                let second_timestamp = minute_data.minute_timestamp + (second_offset * 1000);

                // Simulate per-second data (in real applications, there should be actual second-level data)
                let mut second_aggregate = SecondAggregateData::new();

                // Evenly distribute minute-level data across 60 seconds
                for price_level in minute_data.get_sorted_price_levels() {
                    if price_level.get_total_volume() > 0.0 {
                        // Simple even distribution (in real applications, there should be more precise timestamps)
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

        // Generate price line data and delta volume points
        let mut sorted_timestamps: Vec<u64> = second_data.keys().cloned().collect();
        sorted_timestamps.sort();

        for timestamp in sorted_timestamps {
            if let Some(aggregate) = second_data.get(&timestamp) {
                // Price line point (use volume-weighted average price as closing price)
                let close_price = aggregate.get_volume_weighted_price();
                self.cached_price_line_data.push(PriceLinePoint {
                    second_timestamp: timestamp,
                    close_price,
                });

                // Delta volume point
                let delta_volume = aggregate.get_delta_volume();
                if delta_volume.abs() > 0.01 { // Only show meaningful delta
                    self.cached_delta_points.push(DeltaVolumePoint {
                        second_timestamp: timestamp,
                        price: close_price,
                        delta_volume,
                    });
                }
            }
        }
    }

    /// Render price line
    fn render_price_line(&self, plot_ui: &mut egui_plot::PlotUi) {
        if self.cached_price_line_data.len() < 2 {
            return;
        }

        // Build price line point collection
        let price_points: Vec<[f64; 2]> = self.cached_price_line_data
            .iter()
            .map(|point| [
                self.timestamp_to_plot_x(point.second_timestamp),
                point.close_price
            ])
            .collect();

        // Create price line
        let price_line = Line::new(price_points)
            .color(egui::Color32::WHITE)
            .width(2.0)
            .name("Price Line");

        plot_ui.line(price_line);
    }

    /// Render delta volume points
    fn render_delta_volume_points(&self, plot_ui: &mut egui_plot::PlotUi) {
        if self.cached_delta_points.is_empty() {
            return;
        }

        // Calculate maximum delta volume for point size scaling
        let max_abs_delta = self.cached_delta_points
            .iter()
            .map(|p| p.delta_volume.abs())
            .fold(0.0, f64::max);

        if max_abs_delta == 0.0 {
            return;
        }

        // Handle positive delta (buy advantage) and negative delta (sell advantage) separately
        let mut positive_points = Vec::new();
        let mut negative_points = Vec::new();

        for point in &self.cached_delta_points {
            let x = self.timestamp_to_plot_x(point.second_timestamp);
            let y = point.price;

            // Calculate point radius (based on absolute delta volume)
            let normalized_delta = point.delta_volume.abs() / max_abs_delta;
            let radius = (normalized_delta * 10.0).max(2.0); // Minimum radius 2, maximum radius 10

            if point.delta_volume > 0.0 {
                // Positive delta - buy advantage, use green
                positive_points.push([x, y]);
            } else if point.delta_volume < 0.0 {
                // Negative delta - sell advantage, use red
                negative_points.push([x, y]);
            }
        }

        // Render positive delta points (buy advantage)
        if !positive_points.is_empty() {
            let positive_points_plot = Points::new(positive_points)
                .color(self.buy_color)
                .radius(5.0) // Fixed radius, can be adjusted as needed
                .name("Buy Advantage");
            plot_ui.points(positive_points_plot);
        }

        // Render negative delta points (sell advantage)
        if !negative_points.is_empty() {
            let negative_points_plot = Points::new(negative_points)
                .color(self.sell_color)
                .radius(5.0) // Fixed radius, can be adjusted as needed
                .name("Sell Advantage");
            plot_ui.points(negative_points_plot);
        }
    }

    /// Render price reference lines
    fn render_price_reference_lines(&self, plot_ui: &mut egui_plot::PlotUi, app: &ReactiveApp) {
        let snapshot = app.get_market_snapshot();
        
        // Current price line
        if let Some(current_price) = snapshot.current_price {
            let time_range = self.get_time_range();
            if let Some((start_time, end_time)) = time_range {
                let current_price_line = Line::new(vec![
                    [start_time, current_price],
                    [end_time, current_price],
                ])
                .color(egui::Color32::YELLOW)
                .width(2.0)
                .name("Current Price");
                
                plot_ui.line(current_price_line);
            }
        }
        
        // Best bid/ask price lines
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
                .name("Best Bid");
                
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
                .name("Best Ask");
                
                plot_ui.line(ask_line);
            }
        }
    }

    /// Render control panel
    fn render_control_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Display Window:");
            ui.add(egui::Slider::new(&mut self.display_window_minutes, 10..=60)
                .suffix(" minutes"));

            ui.separator();

            ui.checkbox(&mut self.auto_follow, "Auto Follow");

            ui.separator();

            if ui.button("Reset Zoom").clicked() {
                self.zoom_level = 1.0;
            }

            ui.separator();

            ui.label("Display: Per-second closing price line chart");
        });
    }

    /// Convert timestamp to chart X coordinate
    fn timestamp_to_plot_x(&self, timestamp: u64) -> f64 {
        // Convert millisecond timestamp to seconds
        (timestamp / 1000) as f64
    }

    /// Get current display time range
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

    /// X-axis time grid spacer - fixed 30-second spacing
    fn time_grid_spacer(input: GridInput) -> Vec<GridMark> {
        let mut marks = Vec::new();

        // Fixed 30-second spacing
        let step_size = 30.0; // 30 seconds corresponds to 30.0 units

        // Calculate start and end second markers, round down and up to multiples of 30
        let start_second = ((input.bounds.0 / 30.0).floor() as i64) * 30;
        let end_second = ((input.bounds.1 / 30.0).ceil() as i64) * 30;

        // Generate grid marks every 30 seconds
        let mut second = start_second;
        while second <= end_second {
            let value = second as f64;
            if value >= input.bounds.0 && value <= input.bounds.1 {
                marks.push(GridMark {
                    value,
                    step_size,
                });
            }
            second += 30; // Increment by 30 seconds each time
        }

        marks
    }

    /// Y-axis price grid spacer - fixed $50 spacing
    fn price_grid_spacer(input: GridInput) -> Vec<GridMark> {
        let mut marks = Vec::new();

        // Fixed $50 spacing
        let step_size = 50.0;

        // Calculate start and end price markers, round down and up to multiples of 50
        let start_price = ((input.bounds.0 / 50.0).floor() as i64) * 50;
        let end_price = ((input.bounds.1 / 50.0).ceil() as i64) * 50;

        // Generate grid marks every $50
        let mut price = start_price;
        while price <= end_price {
            let value = price as f64;
            if value >= input.bounds.0 && value <= input.bounds.1 {
                marks.push(GridMark {
                    value,
                    step_size,
                });
            }
            price += 50; // Increment by $50 each time
        }

        marks
    }

    /// X-axis time formatter - display as HH:MM:SS format
    fn format_time_axis(mark: GridMark, _axis_index: usize, _range: &std::ops::RangeInclusive<f64>) -> String {
        // Convert seconds back to timestamp
        let second_timestamp = (mark.value as u64) * 1000; // Convert to millisecond timestamp

        // Convert to UTC time
        let datetime = Utc.timestamp_millis_opt(second_timestamp as i64)
            .single()
            .unwrap_or_else(|| Utc::now());

        // Format as HH:MM:SS
        datetime.format("%H:%M:%S").to_string()
    }

    /// Y-axis price formatter - display as integer price
    fn format_price_axis(mark: GridMark, _axis_index: usize, _range: &std::ops::RangeInclusive<f64>) -> String {
        // Display as integer price, e.g., 101480, 101481, 101482
        format!("{:.0}", mark.value)
    }
}
