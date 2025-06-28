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

        // 初始化应用程序
        if let Err(e) = app.initialize() {
            // 初始化错误写入日志文件，不输出到控制台
            log::error!("应用程序初始化失败: {}", e);
        }

        // 创建并配置统一订单簿组件
        let mut unified_orderbook_widget = UnifiedOrderBookWidget::new();
        unified_orderbook_widget.set_price_chart_height(300.0); // 设置价格图表高度为300像素

        Self {
            app,
            last_update: Instant::now(),
            update_interval: Duration::from_millis(1), // 1ms 刷新间隔
            show_settings: false,
            show_stats: false,
            unified_orderbook_widget,
            debug_window: DebugWindow::new(),
        }
    }
}

impl eframe::App for TradingGUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 更新应用程序状态
        let now = Instant::now();
        if now.duration_since(self.last_update) >= self.update_interval {
            self.app.event_loop(); // 使用正确的方法名
            self.last_update = now;
        }
        
        // 顶部菜单栏
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // ui.menu_button("视图", |ui| {
                //     if ui.button("设置").clicked() {
                //         self.show_settings = !self.show_settings;
                //     }
                //     if ui.button("统计").clicked() {
                //         self.show_stats = !self.show_stats;
                //     }
                //     if ui.button("🔧 调试").clicked() {
                //         self.debug_window.show = !self.debug_window.show;
                //     }
                // });
                
                // ui.separator();
                
                // 连接状态指示器 - 显示重连信息
                let connection_status = self.app.get_connection_status();
                let (status_text, status_color) = if connection_status.is_connected {
                    ("🟢 已连接".to_string(), egui::Color32::from_rgb(120, 255, 120))
                } else if connection_status.is_reconnecting {
                    (
                        format!("🟡 重连中... ({}/{}) - 3秒间隔",
                            connection_status.reconnect_attempts,
                            connection_status.max_attempts
                        ),
                        egui::Color32::from_rgb(255, 255, 120)
                    )
                } else {
                    ("🔴 未连接".to_string(), egui::Color32::from_rgb(255, 120, 120))
                };
                ui.colored_label(status_color, status_text);

                // 显示总重连次数
                if connection_status.total_reconnects > 0 {
                    ui.separator();
                    ui.colored_label(egui::Color32::GRAY,
                        format!("总重连: {}", connection_status.total_reconnects));
                }

                // 显示最后的错误信息（简短版本）
                if let Some(error) = &connection_status.last_error {
                    ui.separator();
                    let short_error = if error.len() > 30 {
                        format!("{}...", &error[..30])
                    } else {
                        error.clone()
                    };
                    ui.colored_label(egui::Color32::from_rgb(255, 180, 120),
                        format!("错误: {}", short_error));
                }

                // 性能指标
                let stats = self.app.get_stats();
                ui.label(format!("事件/秒: {:.1}", stats.events_processed_per_second));
            });
        });
        

        // 主要内容区域 - 统一的订单流分析表格
        egui::CentralPanel::default().show(ctx, |ui| {
            // 使用统一的订单簿组件，占满整个中央面板
            self.unified_orderbook_widget.show(ui, &self.app);
        });
        
        // 设置窗口
        if self.show_settings {
            egui::Window::new("设置")
                .open(&mut self.show_settings)
                .show(ctx, |ui| {
                    ui.label("更新间隔 (ms):");
                    let mut interval_ms = self.update_interval.as_millis() as u64;
                    if ui.add(egui::Slider::new(&mut interval_ms, 50..=1000)).changed() {
                        self.update_interval = Duration::from_millis(interval_ms);
                    }
                    
                    ui.separator();
                    
                    if ui.button("重新连接").clicked() {
                        // 触发重连逻辑
                    }
                });
        }
        
        // 统计窗口
        if self.show_stats {
            egui::Window::new("统计信息")
                .open(&mut self.show_stats)
                .show(ctx, |ui| {
                    let stats = self.app.get_stats();

                    ui.label(format!("运行状态: {}", if stats.running { "运行中" } else { "已停止" }));
                    ui.label(format!("事件处理速度: {:.2} events/sec", stats.events_processed_per_second));
                    ui.label(format!("待处理事件: {}", stats.pending_events));
                    ui.label(format!("WebSocket连接: {}", if stats.websocket_connected { "已连接" } else { "未连接" }));

                    ui.separator();

                    ui.label("事件统计:");
                    ui.indent("event_stats", |ui| {
                        ui.label(format!("已发布事件: {}", stats.total_events_published));
                        ui.label(format!("已处理事件: {}", stats.total_events_processed));
                        ui.label(format!("WebSocket消息: {}", stats.websocket_messages_received));
                        ui.label(format!("订单簿更新: {}", stats.orderbook_updates));
                        ui.label(format!("交易处理: {}", stats.trades_processed));
                    });
                });
        }

        // 显示调试窗口
        self.debug_window.show_window(ctx, &self.app);

        // 请求重绘以实现实时更新
        ctx.request_repaint_after(self.update_interval);
    }
    
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // 清理资源
        self.app.stop();
    }
}

impl Drop for TradingGUI {
    fn drop(&mut self) {
        self.app.stop();
    }
}
