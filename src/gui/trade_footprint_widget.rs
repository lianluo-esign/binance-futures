use eframe::egui;
use crate::orderbook::OrderFlow;
use crate::app::ReactiveApp;
use std::collections::BTreeMap;
use ordered_float::OrderedFloat;

/// Active trade order history footprint data component
pub struct TradeFootprintWidget {
    /// Scroll position
    scroll_position: f32,
    /// Auto track current price
    auto_track_price: bool,
    /// Display price range
    price_range: f64,
    /// Bar chart maximum width
    max_bar_width: f32,
}

impl Default for TradeFootprintWidget {
    fn default() -> Self {
        Self {
            scroll_position: 0.0,
            auto_track_price: true,
            price_range: 100.0, // Display $100 range above and below current price
            max_bar_width: 200.0,
        }
    }
}

impl TradeFootprintWidget {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render trade footprint component - responsive full-screen layout
    pub fn show(&mut self, ui: &mut egui::Ui, app: &ReactiveApp) {
        // Get available space
        let available_rect = ui.available_rect_before_wrap();

        ui.allocate_ui_with_layout(
            available_rect.size(),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                // Title and control buttons - fixed height
                ui.horizontal(|ui| {
                    ui.heading("Trade Footprint Data");
                    ui.separator();

                    // Auto track price toggle
                    ui.checkbox(&mut self.auto_track_price, "Auto Track Price");

                    // Price range control
                    ui.label("Price Range:");
                    ui.add(egui::Slider::new(&mut self.price_range, 10.0..=500.0).suffix("$"));

                    // Reset view button
                    if ui.button("Reset View").clicked() {
                        self.scroll_position = 0.0;
                    }
                });

                ui.separator();

                // Get order flow data
                let order_flows = app.get_orderbook_manager().get_order_flows();
                let snapshot = app.get_market_snapshot();
                let current_price = snapshot.current_price.unwrap_or(50000.0);

                // Filter data within price range
                let filtered_data = self.filter_price_range(&order_flows, current_price);

                // Calculate table available height (total height - header height - separator height)
                let header_height = 60.0; // Estimated header height
                let table_height = available_rect.height() - header_height;

                // Create table to display footprint data - occupy remaining space
                self.render_footprint_table(ui, &filtered_data, current_price, table_height);
            },
        );
    }

    /// Filter data within price range
    fn filter_price_range(
        &self,
        order_flows: &BTreeMap<OrderedFloat<f64>, OrderFlow>,
        current_price: f64,
    ) -> Vec<FootprintRow> {
        let mut rows = Vec::new();
        let price_min = current_price - self.price_range / 2.0;
        let price_max = current_price + self.price_range / 2.0;

        // Sort by price from high to low, only take data within price range
        let mut sorted_flows: Vec<_> = order_flows
            .iter()
            .filter(|(price, _)| {
                let price_val = price.0;
                price_val >= price_min && price_val <= price_max
            })
            .collect();
        sorted_flows.sort_by(|a, b| b.0.cmp(a.0));

        for (price, order_flow) in sorted_flows {
            let price_val = price.0;

            // Get historical trade data
            let buy_volume = order_flow.history_trade_record.buy_volume;
            let sell_volume = order_flow.history_trade_record.sell_volume;
            let delta = buy_volume - sell_volume;

            // Only display price levels with trading activity
            if buy_volume > 0.0 || sell_volume > 0.0 {
                rows.push(FootprintRow {
                    price: price_val,
                    buy_volume,
                    sell_volume,
                    delta,
                });
            }
        }

        rows
    }

    /// Render footprint table - responsive height
    fn render_footprint_table(
        &mut self,
        ui: &mut egui::Ui,
        data: &[FootprintRow],
        current_price: f64,
        table_height: f32,
    ) {
        use egui_extras::{Column, TableBuilder};

        // Calculate maximum trade volume for bar chart scaling
        let max_volume = data
            .iter()
            .map(|row| row.buy_volume.max(row.sell_volume))
            .fold(0.0, f64::max);

        if max_volume == 0.0 {
            ui.label("No Trade Data Available");
            return;
        }

        // Use unique UI area to avoid ID conflicts
        ui.push_id("footprint_table_container", |ui| {
            // Get available width
            let available_width = ui.available_width();
            let column_width = available_width / 4.0; // 4 columns evenly distributed

            // Set table to occupy available space
            ui.allocate_ui_with_layout(
                egui::Vec2::new(available_width, table_height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // Create table
                    let table = TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::exact(column_width)) // Price
                    .column(Column::exact(column_width)) // Buy volume + bar chart
                    .column(Column::exact(column_width)) // Sell volume + bar chart
                    .column(Column::exact(column_width)) // Delta
                    .max_scroll_height(table_height - 30.0) // Subtract header height
                    .vscroll(true);

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Price");
                });
                header.col(|ui| {
                    ui.strong("Buy Volume");
                });
                header.col(|ui| {
                    ui.strong("Sell Volume");
                });
                header.col(|ui| {
                    ui.strong("Delta");
                });
            })
            .body(|mut body| {
                for row in data {
                    body.row(25.0, |mut row_ui| {
                        // Price column - highlight near current price
                        row_ui.col(|ui| {
                            let price_diff = (row.price - current_price).abs();
                            let is_near_current = price_diff < 1.0; // Within $1 range

                            if is_near_current {
                                ui.colored_label(egui::Color32::YELLOW, format!("{:.2}", row.price))
                                    .on_hover_text("Near current price");
                            } else {
                                ui.label(format!("{:.2}", row.price));
                            }
                        });

                        // Buy volume column + bar chart
                        row_ui.col(|ui| {
                            ui.horizontal(|ui| {
                                // Text display
                                ui.colored_label(egui::Color32::GREEN, format!("{:.4}", row.buy_volume));
                                
                                // Bar chart
                                if row.buy_volume > 0.0 {
                                    let bar_width = (row.buy_volume / max_volume * self.max_bar_width as f64) as f32;
                                    self.draw_horizontal_bar(ui, bar_width, egui::Color32::GREEN);
                                }
                            });
                        });

                        // Sell volume column + bar chart
                        row_ui.col(|ui| {
                            ui.horizontal(|ui| {
                                // Text display
                                ui.colored_label(egui::Color32::RED, format!("{:.4}", row.sell_volume));
                                
                                // Bar chart
                                if row.sell_volume > 0.0 {
                                    let bar_width = (row.sell_volume / max_volume * self.max_bar_width as f64) as f32;
                                    self.draw_horizontal_bar(ui, bar_width, egui::Color32::RED);
                                }
                            });
                        });

                        // Delta column
                        row_ui.col(|ui| {
                            let color = if row.delta > 0.0 {
                                egui::Color32::GREEN
                            } else if row.delta < 0.0 {
                                egui::Color32::RED
                            } else {
                                egui::Color32::GRAY
                            };
                            ui.colored_label(color, format!("{:.4}", row.delta));
                        });
                    });
                }
            });
                },
            );
        });
    }

    /// Draw horizontal bar chart
    fn draw_horizontal_bar(&self, ui: &mut egui::Ui, width: f32, color: egui::Color32) {
        let height = 8.0;
        let (rect, _) = ui.allocate_exact_size(
            egui::Vec2::new(width.max(1.0), height),
            egui::Sense::hover()
        );

        // Draw bar background
        ui.painter().rect_filled(
            rect,
            2.0,
            color
        );

        // Draw border
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, color.linear_multiply(0.5))
        );
    }
}

/// Footprint table row data structure
#[derive(Debug, Clone)]
struct FootprintRow {
    price: f64,
    buy_volume: f64,
    sell_volume: f64,
    delta: f64,
}
