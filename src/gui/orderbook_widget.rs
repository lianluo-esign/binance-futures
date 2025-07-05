use eframe::egui;
use crate::orderbook::OrderFlow;
use crate::app::ReactiveApp;
use std::collections::BTreeMap;
use ordered_float::OrderedFloat;
use std::time::{SystemTime, UNIX_EPOCH};

/// OrderBook depth data display component
pub struct OrderBookWidget {
    /// Scroll position
    scroll_position: f32,
    /// Auto track current price
    auto_track_price: bool,
    /// 5-second cumulative data time window
    time_window_seconds: u64,
}

impl Default for OrderBookWidget {
    fn default() -> Self {
        Self {
            scroll_position: 0.0,
            auto_track_price: true,
            time_window_seconds: 5,
        }
    }
}

impl OrderBookWidget {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render OrderBook component - responsive full-screen layout
    pub fn show(&mut self, ui: &mut egui::Ui, app: &ReactiveApp) {
        // Get available space
        let available_rect = ui.available_rect_before_wrap();

        ui.allocate_ui_with_layout(
            available_rect.size(),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                // 只保留表格和数据内容，不显示顶部标题和控件
            },
        );
    }

    /// Aggregate 5-second active buy and sell order data
    fn aggregate_5s_data(
        &self,
        order_flows: &BTreeMap<OrderedFloat<f64>, OrderFlow>,
        current_time: u64,
    ) -> Vec<OrderBookRow> {
        let mut rows = Vec::new();
        let time_threshold = current_time - self.time_window_seconds;

        // Sort by price from high to low
        let mut sorted_flows: Vec<_> = order_flows.iter().collect();
        sorted_flows.sort_by(|a, b| b.0.cmp(a.0));

        for (price, order_flow) in sorted_flows {
            let price_val = price.0;

            // Get orderbook data
            let bid_volume = order_flow.bid_ask.bid;
            let ask_volume = order_flow.bid_ask.ask;

            // Calculate 5-second active buy and sell order accumulation
            let active_buy_volume = if order_flow.realtime_trade_record.timestamp >= time_threshold {
                order_flow.realtime_trade_record.buy_volume
            } else {
                0.0
            };

            let active_sell_volume = if order_flow.realtime_trade_record.timestamp >= time_threshold {
                order_flow.realtime_trade_record.sell_volume
            } else {
                0.0
            };

            rows.push(OrderBookRow {
                price: price_val,
                bid_volume,
                ask_volume,
                buy_volume_5s: active_buy_volume,
                sell_volume_5s: active_sell_volume,
            });
        }

        rows
    }

    /// Render orderbook table - responsive height
    fn render_orderbook_table(
        &mut self,
        ui: &mut egui::Ui,
        data: &[OrderBookRow],
        current_price: f64,
        table_height: f32,
    ) {
        use egui_extras::{Column, TableBuilder};

        // Use unique UI area to avoid ID conflicts
        ui.push_id("orderbook_table_container", |ui| {
            // Get available width
            let available_width = ui.available_width();
            let column_width = available_width / 5.0; // 5 columns evenly distributed

            // Set table to occupy available space
            ui.allocate_ui_with_layout(
                egui::Vec2::new(available_width, table_height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // Create table - new layout: Active Buy (5s) | Bid Orders | Price | Ask Orders | Active Sell (5s)
                    let table = TableBuilder::new(ui)
                        .striped(false)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::exact(column_width)) // Active buy accumulation (5s)
                        .column(Column::exact(column_width)) // Bid orders
                        .column(Column::exact(column_width)) // Price
                        .column(Column::exact(column_width)) // Ask orders
                        .column(Column::exact(column_width)) // Active sell accumulation (5s)
                        .max_scroll_height(table_height - 30.0) // Subtract header height
                        .vscroll(true);

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Active Buy (5s)");
                });
                header.col(|ui| {
                    ui.strong("Bid Orders");
                });
                header.col(|ui| {
                    ui.strong("Price");
                });
                header.col(|ui| {
                    ui.strong("Ask Orders");
                });
                header.col(|ui| {
                    ui.strong("Active Sell (5s)");
                });
            })
            .body(|mut body| {
                for row in data {
                    body.row(18.0, |mut row_ui| {
                        // Column 1: Active buy accumulation (5s)
                        row_ui.col(|ui| {
                            if row.buy_volume_5s > 0.0 {
                                ui.colored_label(egui::Color32::LIGHT_GREEN, format!("{:.4}", row.buy_volume_5s));
                            } else {
                                ui.label("--");
                            }
                        });

                        // Column 2: Bid orders
                        row_ui.col(|ui| {
                            if row.bid_volume > 0.0 {
                                ui.colored_label(egui::Color32::GREEN, format!("{:.4}", row.bid_volume));
                            } else {
                                ui.label("--");
                            }
                        });

                        // Column 3: Price - highlight near current price
                        row_ui.col(|ui| {
                            let price_diff = (row.price - current_price).abs();
                            let is_near_current = price_diff < 1.0; // Within $1 range

                            if is_near_current {
                                // Highlight near current price with yellow background
                                let _color = if row.price >= current_price {
                                    egui::Color32::from_rgb(0, 100, 0) // Dark green background
                                } else {
                                    egui::Color32::from_rgb(100, 0, 0) // Dark red background
                                };
                                ui.colored_label(egui::Color32::WHITE, format!("{:.2}", row.price))
                                    .on_hover_text("Near current price");
                            } else {
                                ui.label(format!("{:.2}", row.price));
                            }
                        });

                        // Column 4: Ask orders
                        row_ui.col(|ui| {
                            if row.ask_volume > 0.0 {
                                ui.colored_label(egui::Color32::RED, format!("{:.4}", row.ask_volume));
                            } else {
                                ui.label("--");
                            }
                        });

                        // Column 5: Active sell accumulation (5s)
                        row_ui.col(|ui| {
                            if row.sell_volume_5s > 0.0 {
                                ui.colored_label(egui::Color32::LIGHT_RED, format!("{:.4}", row.sell_volume_5s));
                            } else {
                                ui.label("--");
                            }
                        });
                    });
                }
            });
                },
            );
        });
    }
}

/// OrderBook table row data structure
#[derive(Debug, Clone)]
struct OrderBookRow {
    price: f64,
    bid_volume: f64,
    ask_volume: f64,
    buy_volume_5s: f64,
    sell_volume_5s: f64,
}
