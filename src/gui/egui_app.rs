use eframe::egui;
use crate::app::ReactiveApp;
use crate::Config;
use crate::orderbook::MarketSnapshot;
use std::time::{Duration, Instant};

pub struct TradingGUI {
    app: ReactiveApp,
    last_update: Instant,
    update_interval: Duration,
    show_settings: bool,
    show_stats: bool,
}

impl TradingGUI {
    pub fn new(config: Config) -> Self {
        let mut app = ReactiveApp::new(config);
        
        // 初始化应用程序
        if let Err(e) = app.initialize() {
            eprintln!("Failed to initialize app: {}", e);
        }
        
        Self {
            app,
            last_update: Instant::now(),
            update_interval: Duration::from_millis(100), // 10 FPS
            show_settings: false,
            show_stats: false,
        }
    }
}

impl eframe::App for TradingGUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 更新应用程序状态
        let now = Instant::now();
        if now.duration_since(self.last_update) >= self.update_interval {
            self.app.process_events();
            self.last_update = now;
        }
        
        // 顶部菜单栏
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("视图", |ui| {
                    if ui.button("设置").clicked() {
                        self.show_settings = !self.show_settings;
                    }
                    if ui.button("统计").clicked() {
                        self.show_stats = !self.show_stats;
                    }
                });
                
                ui.separator();
                
                // 连接状态指示器
                let connected = self.app.is_connected();
                let color = if connected { 
                    egui::Color32::GREEN 
                } else { 
                    egui::Color32::RED 
                };
                ui.colored_label(color, if connected { "已连接" } else { "未连接" });
                
                // 性能指标
                let stats = self.app.get_stats();
                ui.label(format!("事件/秒: {:.1}", stats.events_per_second));
            });
        });
        
        // 主要内容区域
        egui::CentralPanel::default().show(ctx, |ui| {
            let snapshot = self.app.get_market_snapshot();
            
            // 价格信息面板
            ui.group(|ui| {
                ui.heading("市场信息");
                ui.horizontal(|ui| {
                    ui.label("交易对:");
                    ui.strong(&self.app.get_symbol());
                });
                
                ui.horizontal(|ui| {
                    ui.label("当前价格:");
                    let price_color = if snapshot.price_change_24h >= 0.0 {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::RED
                    };
                    ui.colored_label(price_color, format!("{:.2}", snapshot.current_price));
                });
                
                ui.horizontal(|ui| {
                    ui.label("24h变化:");
                    let change_color = if snapshot.price_change_24h >= 0.0 {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::RED
                    };
                    ui.colored_label(change_color, format!("{:.2}%", snapshot.price_change_24h));
                });
                
                ui.horizontal(|ui| {
                    ui.label("24h成交量:");
                    ui.label(format!("{:.2}", snapshot.volume_24h));
                });
            });
            
            ui.separator();
            
            // 订单簿显示
            ui.horizontal(|ui| {
                // 买单 (Bids)
                ui.group(|ui| {
                    ui.heading("买单");
                    ui.separator();
                    
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for (price, quantity) in snapshot.bids.iter().take(20) {
                                ui.horizontal(|ui| {
                                    ui.colored_label(egui::Color32::GREEN, format!("{:.2}", price));
                                    ui.label(format!("{:.4}", quantity));
                                });
                            }
                        });
                });
                
                ui.separator();
                
                // 卖单 (Asks)
                ui.group(|ui| {
                    ui.heading("卖单");
                    ui.separator();
                    
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for (price, quantity) in snapshot.asks.iter().take(20) {
                                ui.horizontal(|ui| {
                                    ui.colored_label(egui::Color32::RED, format!("{:.2}", price));
                                    ui.label(format!("{:.4}", quantity));
                                });
                            }
                        });
                });
            });
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
                    
                    ui.label(format!("总事件数: {}", stats.total_events));
                    ui.label(format!("事件处理速度: {:.2} events/sec", stats.events_per_second));
                    ui.label(format!("WebSocket延迟: {:.2} ms", stats.websocket_latency_ms));
                    ui.label(format!("内存使用: {:.2} MB", stats.memory_usage_mb));
                    
                    ui.separator();
                    
                    ui.label("事件类型统计:");
                    ui.indent("event_stats", |ui| {
                        ui.label(format!("价格更新: {}", stats.price_events));
                        ui.label(format!("深度更新: {}", stats.depth_events));
                        ui.label(format!("交易事件: {}", stats.trade_events));
                        ui.label(format!("信号事件: {}", stats.signal_events));
                    });
                });
        }
        
        // 请求重绘以实现实时更新
        ctx.request_repaint_after(self.update_interval);
    }
    
    fn on_close_event(&mut self) -> bool {
        // 清理资源
        self.app.stop();
        true
    }
}

impl Drop for TradingGUI {
    fn drop(&mut self) {
        self.app.stop();
    }
}
