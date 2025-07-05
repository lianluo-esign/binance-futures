use eframe::egui;
use crate::app::ReactiveApp;

pub struct DebugWindow {
    pub show: bool,
    last_error: Option<String>,
}

impl Default for DebugWindow {
    fn default() -> Self {
        Self {
            show: false,
            last_error: None,
        }
    }
}

impl DebugWindow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show_window(&mut self, ctx: &egui::Context, app: &ReactiveApp) {
        if !self.show {
            return;
        }

        let mut show = self.show;
        egui::Window::new("🔧 Debug Information")
            .open(&mut show)
            .default_width(800.0)
            .default_height(600.0)
            .show(ctx, |ui| {
                self.render_debug_content(ui, app);
            });
        self.show = show;
    }

    fn render_debug_content(&mut self, ui: &mut egui::Ui, app: &ReactiveApp) {
        ui.heading("System Status");
        
        // Basic statistics
        let stats = app.get_stats();
        ui.horizontal(|ui| {
            ui.label("Running Status:");
            let color = if stats.running { egui::Color32::GREEN } else { egui::Color32::RED };
            ui.colored_label(color, if stats.running { "✅ Running" } else { "❌ Stopped" });
        });

        ui.horizontal(|ui| {
            ui.label("WebSocket Connection:");
            let color = if stats.websocket_connected { egui::Color32::GREEN } else { egui::Color32::RED };
            let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(12.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 6.0, color);
            ui.label(if stats.websocket_connected { "Connected" } else { "Disconnected" });
        });

        ui.label(format!("Event Processing Speed: {:.2} events/sec", stats.events_processed_per_second));
        ui.label(format!("Pending Events: {}", stats.pending_events));
        ui.label(format!("WebSocket Messages: {}", stats.websocket_messages_received));
        ui.label(format!("Orderbook Updates: {}", stats.orderbook_updates));
        ui.label(format!("Trades Processed: {}", stats.trades_processed));

        ui.separator();

        // Market data
        ui.heading("Market Data");
        let snapshot = app.get_market_snapshot();
        
        ui.horizontal(|ui| {
            ui.label("Symbol:");
            ui.strong(app.get_symbol());
        });

        if let Some(current_price) = snapshot.current_price {
            ui.label(format!("Current Price: {:.2}", current_price));
        } else {
            ui.colored_label(egui::Color32::YELLOW, "Current Price: No Data");
        }

        if let Some(bid_price) = snapshot.best_bid_price {
            ui.label(format!("Best Bid: {:.2}", bid_price));
        } else {
            ui.colored_label(egui::Color32::YELLOW, "Best Bid: No Data");
        }

        if let Some(ask_price) = snapshot.best_ask_price {
            ui.label(format!("Best Ask: {:.2}", ask_price));
        } else {
            ui.colored_label(egui::Color32::YELLOW, "Best Ask: No Data");
        }

        ui.separator();

        // Order flow data
        ui.heading("Order Flow Data");
        let order_flows = app.get_orderbook_manager().get_order_flows();
        ui.label(format!("Total Entries: {}", order_flows.len()));

        if !order_flows.is_empty() {
            ui.label("Top 10 Price Levels:");
            
            egui::ScrollArea::vertical()
                .id_source("debug_orderflow_scroll")
                .max_height(200.0)
                .show(ui, |ui| {
                    for (i, (price, order_flow)) in order_flows.iter().take(10).enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}.", i + 1));
                            ui.label(format!("Price: {:.2}", price.0));
                            ui.label(format!("Bid: {:.4}", order_flow.bid_ask.bid));
                            ui.label(format!("Ask: {:.4}", order_flow.bid_ask.ask));
                            ui.label(format!("Buy Vol: {:.4}", order_flow.realtime_trade_record.buy_volume));
                            ui.label(format!("Sell Vol: {:.4}", order_flow.realtime_trade_record.sell_volume));
                        });
                    }
                });
        }

        ui.separator();

        // Error information
        ui.heading("Error Information");
        if let Some(error) = &self.last_error {
            ui.colored_label(egui::Color32::RED, format!("Last Error: {}", error));
            if ui.button("Clear Error").clicked() {
                self.last_error = None;
            }
        } else {
            ui.colored_label(egui::Color32::GREEN, "✅ No Errors");
        }

        ui.separator();

        // Control buttons
        ui.heading("Controls");
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh Data").clicked() {
                // Trigger data refresh
            }
            
            if ui.button("📋 Copy Debug Info").clicked() {
                let debug_info = format!(
                    "System Status: {}\nWebSocket: {}\nEvent Speed: {:.2}/s\nOrder Flow Entries: {}\nCurrent Price: {:?}",
                    if stats.running { "Running" } else { "Stopped" },
                    if stats.websocket_connected { "Connected" } else { "Disconnected" },
                    stats.events_processed_per_second,
                    order_flows.len(),
                    snapshot.current_price
                );
                ui.output_mut(|o| o.copied_text = debug_info);
            }
        });

        ui.separator();

        // Test area
        ui.heading("Testing");
        if ui.button("🧪 Test egui Table").clicked() {
            self.test_egui_table(ui);
        }
    }

    fn test_egui_table(&mut self, ui: &mut egui::Ui) {
        use egui_extras::{Column, TableBuilder};
        
        ui.label("Testing Simple Table:");
        
        // Try to create a simple table
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let table = TableBuilder::new(ui)
                .striped(true)
                .column(Column::auto().at_least(100.0))
                .column(Column::auto().at_least(100.0))
                .min_scrolled_height(100.0);

            table
                .header(20.0, |mut header| {
                    header.col(|ui| { ui.strong("Column 1"); });
                    header.col(|ui| { ui.strong("Column 2"); });
                })
                .body(|mut body| {
                    for i in 0..5 {
                        body.row(18.0, |mut row| {
                            row.col(|ui| { ui.label(format!("Row {}", i + 1)); });
                            row.col(|ui| { ui.label(format!("Data {}", i + 1)); });
                        });
                    }
                });
        })) {
            Ok(_) => {
                ui.colored_label(egui::Color32::GREEN, "✅ Table Test Successful");
            }
            Err(e) => {
                let error_msg = format!("❌ Table Test Failed: {:?}", e);
                ui.colored_label(egui::Color32::RED, &error_msg);
                self.last_error = Some(error_msg);
            }
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.last_error = Some(error);
    }
}
