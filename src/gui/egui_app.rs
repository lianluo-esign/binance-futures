use eframe::egui;
use crate::app::ReactiveApp;
use crate::Config;
use crate::gui::{UnifiedOrderBookWidget, DebugWindow};
use std::time::{Duration, Instant};

pub struct TradingGUI {
    app: ReactiveApp,
    last_update: Instant,
    update_interval: Duration,
    show_settings: bool,
    show_stats: bool,
    unified_orderbook_widget: UnifiedOrderBookWidget,
    debug_window: DebugWindow,
}

impl TradingGUI {
    pub fn new(config: Config) -> Self {
        let mut app = ReactiveApp::new(config);

        // Initialize application
        if let Err(e) = app.initialize() {
            // Write initialization error to log file, not output to console
            log::error!("Application initialization failed: {}", e);
        }

        // Create and configure unified orderbook widget
        let mut unified_orderbook_widget = UnifiedOrderBookWidget::new();
        unified_orderbook_widget.set_price_chart_height(300.0); // Set price chart height to 300 pixels

        Self {
            app,
            last_update: Instant::now(),
            update_interval: Duration::from_millis(1), // 1ms refresh interval
            show_settings: false,
            show_stats: false,
            unified_orderbook_widget,
            debug_window: DebugWindow::new(),
        }
    }
}

impl eframe::App for TradingGUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update application status
        let now = Instant::now();
        if now.duration_since(self.last_update) >= self.update_interval {
            self.app.event_loop(); // Use correct method name
            self.last_update = now;
        }
        
        // Top menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // ui.menu_button("View", |ui| {
                //     if ui.button("Settings").clicked() {
                //         self.show_settings = !self.show_settings;
                //     }
                //     if ui.button("Statistics").clicked() {
                //         self.show_stats = !self.show_stats;
                //     }
                //     if ui.button("🔧 Debug").clicked() {
                //         self.debug_window.show = !self.debug_window.show;
                //     }
                // });
                
                // ui.separator();
                
                // Connection status indicator - display reconnection information
                let connection_status = self.app.get_connection_status();
                let (status_text, status_color) = if connection_status.is_connected {
                    ("🟢 Connected".to_string(), egui::Color32::from_rgb(120, 255, 120))
                } else if connection_status.is_reconnecting {
                    (
                        format!("🟡 Reconnecting... ({}/{}) - 3s interval",
                            connection_status.reconnect_attempts,
                            connection_status.max_attempts
                        ),
                        egui::Color32::from_rgb(255, 255, 120)
                    )
                } else {
                    ("🔴 Disconnected".to_string(), egui::Color32::from_rgb(255, 120, 120))
                };
                ui.colored_label(status_color, status_text);

                // Display total reconnection count
                if connection_status.total_reconnects > 0 {
                    ui.separator();
                    ui.colored_label(egui::Color32::GRAY,
                        format!("Total Reconnects: {}", connection_status.total_reconnects));
                }

                // Display last error message (short version)
                if let Some(error) = &connection_status.last_error {
                    ui.separator();
                    let short_error = if error.len() > 30 {
                        format!("{}...", &error[..30])
                    } else {
                        error.clone()
                    };
                    ui.colored_label(egui::Color32::from_rgb(255, 180, 120),
                        format!("Error: {}", short_error));
                }

                // Performance metrics
                let stats = self.app.get_stats();
                ui.label(format!("Events/sec: {:.1}", stats.events_processed_per_second));

                // RingBuffer capacity usage
                ui.separator();
                let (current_usage, max_capacity) = self.app.get_buffer_usage();
                let usage_percentage = if max_capacity > 0 {
                    (current_usage as f64 / max_capacity as f64 * 100.0)
                } else {
                    0.0
                };

                // Choose color based on usage rate
                let usage_color = if usage_percentage >= 90.0 {
                    egui::Color32::from_rgb(255, 100, 100) // Red - high usage
                } else if usage_percentage >= 70.0 {
                    egui::Color32::from_rgb(255, 200, 100) // Orange - medium usage
                } else {
                    egui::Color32::from_rgb(120, 255, 120) // Green - low usage
                };

                ui.colored_label(usage_color,
                    format!("Buffer: {}/{} ({:.1}%)", current_usage, max_capacity, usage_percentage));
            });
        });
        

        // Main content area - unified order flow analysis table
        egui::CentralPanel::default().show(ctx, |ui| {
            // Use unified orderbook widget, occupying the entire central panel
            self.unified_orderbook_widget.show(ui, &self.app);
        });
        
        // Settings window
        if self.show_settings {
            egui::Window::new("Settings")
                .open(&mut self.show_settings)
                .show(ctx, |ui| {
                    ui.label("Update Interval (ms):");
                    let mut interval_ms = self.update_interval.as_millis() as u64;
                    if ui.add(egui::Slider::new(&mut interval_ms, 50..=1000)).changed() {
                        self.update_interval = Duration::from_millis(interval_ms);
                    }
                    
                    ui.separator();
                    
                    if ui.button("Reconnect").clicked() {
                        // Trigger reconnection logic
                    }
                });
        }
        
        // Statistics window
        if self.show_stats {
            egui::Window::new("Statistics")
                .open(&mut self.show_stats)
                .show(ctx, |ui| {
                    let stats = self.app.get_stats();

                    ui.label(format!("Running Status: {}", if stats.running { "Running" } else { "Stopped" }));
                    ui.label(format!("Event Processing Speed: {:.2} events/sec", stats.events_processed_per_second));
                    ui.label(format!("Pending Events: {}", stats.pending_events));
                    ui.label(format!("WebSocket Connection: {}", if stats.websocket_connected { "Connected" } else { "Disconnected" }));

                    ui.separator();

                    ui.label("Event Statistics:");
                    ui.indent("event_stats", |ui| {
                        ui.label(format!("Total Events Published: {}", stats.total_events_published));
                        ui.label(format!("Total Events Processed: {}", stats.total_events_processed));
                        ui.label(format!("WebSocket Messages: {}", stats.websocket_messages_received));
                        ui.label(format!("Orderbook Updates: {}", stats.orderbook_updates));
                        ui.label(format!("Trades Processed: {}", stats.trades_processed));
                    });
                });
        }

        // Show debug window
        self.debug_window.show_window(ctx, &self.app);

        // Request repaint for real-time updates
        ctx.request_repaint_after(self.update_interval);
    }
    
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Clean up resources
        self.app.stop();
    }
}

impl Drop for TradingGUI {
    fn drop(&mut self) {
        self.app.stop();
    }
}
