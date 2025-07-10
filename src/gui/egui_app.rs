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

    /// 在菜单栏中渲染高频波动率和价格跳跃因子
    fn render_menu_bar_volatility_jump(&mut self, ui: &mut egui::Ui) {
        // 获取高频波动率和价格跳跃数据
        let (realized_volatility, jump_signal) = self.app.get_volatility_and_jump_data();

        // 高频波动率线型图显示区域 - 固定宽度
        ui.allocate_ui_with_layout(
            egui::Vec2::new(150.0, ui.available_height()),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.colored_label(egui::Color32::WHITE, "📈 RV:");
                
                // 获取RV历史数据
                let rv_history = self.app.get_rv_history();
                
                // 绘制小型线型图
                let (rect, _response) = ui.allocate_exact_size(
                    egui::Vec2::new(80.0, 20.0),
                    egui::Sense::hover()
                );
                
                if rv_history.len() >= 2 {
                    // 计算数据范围
                    let min_rv = rv_history.iter().map(|(_, rv)| *rv).fold(f64::INFINITY, f64::min);
                    let max_rv = rv_history.iter().map(|(_, rv)| *rv).fold(f64::NEG_INFINITY, f64::max);
                    let range = (max_rv - min_rv).max(0.001); // 避免除零
                    
                    // 绘制背景
                    ui.painter().rect_filled(
                        rect,
                        egui::Rounding::same(2.0),
                        egui::Color32::from_rgba_unmultiplied(20, 20, 30, 150)
                    );
                    
                    // 绘制线型图
                    let mut points = Vec::new();
                    for (i, (_, rv)) in rv_history.iter().enumerate() {
                        let x = rect.min.x + (i as f32 / (rv_history.len() - 1) as f32) * rect.width();
                        let normalized_y = ((rv - min_rv) / range) as f32;
                        let y = rect.max.y - normalized_y * rect.height();
                        points.push(egui::Pos2::new(x, y));
                    }
                    
                    // 绘制线条
                    if points.len() >= 2 {
                        for i in 1..points.len() {
                            ui.painter().line_segment(
                                [points[i-1], points[i]],
                                egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 200, 255))
                            );
                        }
                    }
                    
                    // 绘制边框
                    ui.painter().rect_stroke(
                        rect,
                        egui::Rounding::same(2.0),
                        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(100, 100, 120, 100))
                    );
                }
                
                // 显示当前数值
                let volatility_color = if realized_volatility > 2.0 {
                    egui::Color32::from_rgb(255, 100, 100) // 红色 - 高波动
                } else if realized_volatility > 1.0 {
                    egui::Color32::from_rgb(255, 200, 100) // 橙色 - 中等波动
                } else {
                    egui::Color32::from_rgb(120, 255, 120) // 绿色 - 低波动
                };
                
                ui.colored_label(volatility_color, format!("{:4.2}", realized_volatility));
            },
        );

        ui.separator();

        // Jump信号线型图显示区域 - 固定宽度
        ui.allocate_ui_with_layout(
            egui::Vec2::new(150.0, ui.available_height()),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.colored_label(egui::Color32::WHITE, "Jump:");
                // 获取Jump历史数据
                let jump_history = self.app.get_jump_history();
                let (rect, _response) = ui.allocate_exact_size(
                    egui::Vec2::new(80.0, 20.0),
                    egui::Sense::hover()
                );
                if jump_history.len() >= 2 {
                    let min_jump = jump_history.iter().map(|(_, v)| *v).fold(f64::INFINITY, f64::min);
                    let max_jump = jump_history.iter().map(|(_, v)| *v).fold(f64::NEG_INFINITY, f64::max);
                    let range = (max_jump - min_jump).max(0.001);
                    ui.painter().rect_filled(
                        rect,
                        egui::Rounding::same(2.0),
                        egui::Color32::from_rgba_unmultiplied(20, 20, 30, 150)
                    );
                    let mut points = Vec::new();
                    for (i, (_, v)) in jump_history.iter().enumerate() {
                        let x = rect.min.x + (i as f32 / (jump_history.len() - 1) as f32) * rect.width();
                        let normalized_y = ((v - min_jump) / range) as f32;
                        let y = rect.max.y - normalized_y * rect.height();
                        points.push(egui::Pos2::new(x, y));
                    }
                    if points.len() >= 2 {
                        for i in 1..points.len() {
                            ui.painter().line_segment(
                                [points[i-1], points[i]],
                                egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 180, 80))
                            );
                        }
                    }
                    ui.painter().rect_stroke(
                        rect,
                        egui::Rounding::same(2.0),
                        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(180, 120, 80, 100))
                    );
                }
                // 显示当前Jump数值
                let jump_color = if jump_signal > 3.0 {
                    egui::Color32::from_rgb(255, 50, 50)
                } else if jump_signal > 2.0 {
                    egui::Color32::from_rgb(255, 150, 50)
                } else if jump_signal > 1.0 {
                    egui::Color32::from_rgb(255, 255, 100)
                } else {
                    egui::Color32::from_rgb(150, 150, 150)
                };
                ui.colored_label(jump_color, format!("{:4.2}", jump_signal));
            },
        );

        ui.separator();
    }

    /// 在菜单栏中渲染Orderbook Imbalance条形图
    fn render_menu_bar_orderbook_imbalance(&mut self, ui: &mut egui::Ui) {
        // 获取详细的OBI数据
        let (bid_ratio, ask_ratio, total_bid_volume, total_ask_volume, total_volume) = 
            self.app.get_detailed_orderbook_imbalance();

        // OB数值显示区域 - 固定宽度，显示更详细的信息
        ui.allocate_ui_with_layout(
            egui::Vec2::new(200.0, ui.available_height()),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.colored_label(egui::Color32::WHITE, "📊 OBI:");
                ui.colored_label(egui::Color32::from_rgb(120, 180, 255), format!("{:2.0}%", bid_ratio * 100.0));
                ui.colored_label(egui::Color32::GRAY, "/");
                ui.colored_label(egui::Color32::from_rgb(255, 120, 120), format!("{:2.0}%", ask_ratio * 100.0));
                
                // 显示总挂单量信息
                if total_volume > 0.0 {
                    ui.colored_label(egui::Color32::GRAY, " |");
                    if total_volume >= 1000.0 {
                        ui.colored_label(egui::Color32::from_rgb(180, 180, 180), 
                                       format!(" {:4.1}K", total_volume / 1000.0));
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(180, 180, 180), 
                                       format!(" {:4.0}", total_volume));
                    }
                }
            },
        );

        // 绘制横向条形图 - 占满剩余宽度
        let available_width = ui.available_width();
        let bar_height = 16.0; // 条形图高度
        
        // 分配剩余的所有宽度给条形图
        let (rect, _response) = ui.allocate_exact_size(
            egui::Vec2::new(available_width, bar_height),
            egui::Sense::hover()
        );

        // 计算买单和卖单条形图的宽度
        let bid_width = available_width * bid_ratio as f32;
        let ask_width = available_width * ask_ratio as f32;

        // 绘制背景 - 更加半透明
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(2.0),
            egui::Color32::from_rgba_unmultiplied(30, 30, 40, 60) // 更加半透明的背景
        );

        // 绘制买单条形图（蓝色，从左边开始）- 更加半透明
        if bid_width > 1.0 {
            let bid_rect = egui::Rect::from_min_size(
                rect.min,
                egui::Vec2::new(bid_width, bar_height)
            );
            ui.painter().rect_filled(
                bid_rect,
                egui::Rounding::same(2.0),
                egui::Color32::from_rgba_unmultiplied(80, 150, 255, 80) // 更加半透明的蓝色
            );
        }

        // 绘制卖单条形图（红色，从右边开始）- 更加半透明
        if ask_width > 1.0 {
            let ask_rect = egui::Rect::from_min_size(
                egui::Pos2::new(rect.max.x - ask_width, rect.min.y),
                egui::Vec2::new(ask_width, bar_height)
            );
            ui.painter().rect_filled(
                ask_rect,
                egui::Rounding::same(2.0),
                egui::Color32::from_rgba_unmultiplied(255, 80, 80, 80) // 更加半透明的红色
            );
        }

        // 绘制中心分割线 - 更加半透明
        let center_x = rect.min.x + available_width * 0.5;
        ui.painter().line_segment(
            [egui::Pos2::new(center_x, rect.min.y), egui::Pos2::new(center_x, rect.max.y)],
            egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 100)) // 半透明白色
        );

        // 添加边框 - 更加半透明
        ui.painter().rect_stroke(
            rect,
            egui::Rounding::same(2.0),
            egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(80, 80, 100, 80)) // 更加半透明的边框
        );

        // 添加鼠标悬停提示
        if _response.hovered() {
            egui::show_tooltip_at_pointer(ui.ctx(), egui::Id::new("obi_tooltip"), |ui| {
                ui.label(format!("订单簿失衡 (OBI) 详情:"));
                ui.label(format!("买单: {:.2} ({:.1}%)", total_bid_volume, bid_ratio * 100.0));
                ui.label(format!("卖单: {:.2} ({:.1}%)", total_ask_volume, ask_ratio * 100.0));
                ui.label(format!("总挂单量: {:.2}", total_volume));
                ui.label(format!("基于所有价格层级的实时计算"));
            });
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
                // Connection status indicator - 固定宽度区域
                ui.allocate_ui_with_layout(
                    egui::Vec2::new(120.0, ui.available_height()),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        let connection_status = self.app.get_connection_status();
                        let (status_text, status_color) = if connection_status.is_connected {
                            ("Connected".to_string(), egui::Color32::from_rgb(120, 255, 120))
                        } else if connection_status.is_reconnecting {
                            ("Reconnecting".to_string(), egui::Color32::from_rgb(255, 255, 120))
                        } else {
                            ("Disconnected".to_string(), egui::Color32::from_rgb(255, 120, 120))
                        };
                        
                        // 绘制连接状态圆点
                        let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(12.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 6.0, status_color);
                        ui.colored_label(status_color, status_text);
                    },
                );

                ui.separator();

                // Performance metrics - 固定宽度区域
                ui.allocate_ui_with_layout(
                    egui::Vec2::new(100.0, ui.available_height()),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        let stats = self.app.get_stats();
                        // 使用固定宽度格式化，防止抖动
                        ui.label(format!("Events/sec: {:5.1}", stats.events_processed_per_second));
                    },
                );

                ui.separator();

                // RingBuffer capacity usage - 固定宽度区域
                ui.allocate_ui_with_layout(
                    egui::Vec2::new(300.0, ui.available_height()),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
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

                        // 使用固定宽度格式化，防止抖动
                        ui.colored_label(usage_color,
                            format!("Buffer: {:4}/{:4} ({:4.1}%)", current_usage, max_capacity, usage_percentage));
                    },
                );
                
                ui.separator();
                
                // 高频波动率和价格跳跃因子显示
                self.render_menu_bar_volatility_jump(ui);
                
                // 直接在同一行显示Orderbook Imbalance条形图
                self.render_menu_bar_orderbook_imbalance(ui);
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
