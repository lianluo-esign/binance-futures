use eframe::egui;
use crate::app::ReactiveApp;
use std::time::{SystemTime, UNIX_EPOCH};

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
        egui::Window::new("🔧 调试信息")
            .open(&mut show)
            .default_width(800.0)
            .default_height(600.0)
            .show(ctx, |ui| {
                self.render_debug_content(ui, app);
            });
        self.show = show;
    }

    fn render_debug_content(&mut self, ui: &mut egui::Ui, app: &ReactiveApp) {
        ui.heading("系统状态");
        
        // 基本统计信息
        let stats = app.get_stats();
        ui.horizontal(|ui| {
            ui.label("运行状态:");
            let color = if stats.running { egui::Color32::GREEN } else { egui::Color32::RED };
            ui.colored_label(color, if stats.running { "✅ 运行中" } else { "❌ 已停止" });
        });

        ui.horizontal(|ui| {
            ui.label("WebSocket连接:");
            let color = if stats.websocket_connected { egui::Color32::GREEN } else { egui::Color32::RED };
            ui.colored_label(color, if stats.websocket_connected { "✅ 已连接" } else { "❌ 未连接" });
        });

        ui.label(format!("事件处理速度: {:.2} events/sec", stats.events_processed_per_second));
        ui.label(format!("待处理事件: {}", stats.pending_events));
        ui.label(format!("WebSocket消息: {}", stats.websocket_messages_received));
        ui.label(format!("订单簿更新: {}", stats.orderbook_updates));
        ui.label(format!("交易处理: {}", stats.trades_processed));

        ui.separator();

        // 市场数据
        ui.heading("市场数据");
        let snapshot = app.get_market_snapshot();
        
        ui.horizontal(|ui| {
            ui.label("交易对:");
            ui.strong(app.get_symbol());
        });

        if let Some(current_price) = snapshot.current_price {
            ui.label(format!("当前价格: {:.2}", current_price));
        } else {
            ui.colored_label(egui::Color32::YELLOW, "当前价格: 暂无数据");
        }

        if let Some(bid_price) = snapshot.best_bid_price {
            ui.label(format!("最优买价: {:.2}", bid_price));
        } else {
            ui.colored_label(egui::Color32::YELLOW, "最优买价: 暂无数据");
        }

        if let Some(ask_price) = snapshot.best_ask_price {
            ui.label(format!("最优卖价: {:.2}", ask_price));
        } else {
            ui.colored_label(egui::Color32::YELLOW, "最优卖价: 暂无数据");
        }

        ui.separator();

        // 订单流数据
        ui.heading("订单流数据");
        let order_flows = app.get_orderbook_manager().get_order_flows();
        ui.label(format!("总条目数: {}", order_flows.len()));

        if !order_flows.is_empty() {
            ui.label("前10个价格级别:");
            
            egui::ScrollArea::vertical()
                .id_source("debug_orderflow_scroll")
                .max_height(200.0)
                .show(ui, |ui| {
                    for (i, (price, order_flow)) in order_flows.iter().take(10).enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}.", i + 1));
                            ui.label(format!("价格: {:.2}", price.0));
                            ui.label(format!("Bid: {:.4}", order_flow.bid_ask.bid));
                            ui.label(format!("Ask: {:.4}", order_flow.bid_ask.ask));
                            ui.label(format!("买量: {:.4}", order_flow.realtime_trade_record.buy_volume));
                            ui.label(format!("卖量: {:.4}", order_flow.realtime_trade_record.sell_volume));
                        });
                    }
                });
        }

        ui.separator();

        // 错误信息
        ui.heading("错误信息");
        if let Some(error) = &self.last_error {
            ui.colored_label(egui::Color32::RED, format!("最后错误: {}", error));
            if ui.button("清除错误").clicked() {
                self.last_error = None;
            }
        } else {
            ui.colored_label(egui::Color32::GREEN, "✅ 无错误");
        }

        ui.separator();

        // 控制按钮
        ui.heading("控制");
        ui.horizontal(|ui| {
            if ui.button("🔄 刷新数据").clicked() {
                // 触发数据刷新
            }
            
            if ui.button("📋 复制调试信息").clicked() {
                let debug_info = format!(
                    "系统状态: {}\nWebSocket: {}\n事件速度: {:.2}/s\n订单流条目: {}\n当前价格: {:?}",
                    if stats.running { "运行中" } else { "已停止" },
                    if stats.websocket_connected { "已连接" } else { "未连接" },
                    stats.events_processed_per_second,
                    order_flows.len(),
                    snapshot.current_price
                );
                ui.output_mut(|o| o.copied_text = debug_info);
            }
        });

        ui.separator();

        // 测试区域
        ui.heading("测试");
        if ui.button("🧪 测试egui表格").clicked() {
            self.test_egui_table(ui);
        }
    }

    fn test_egui_table(&mut self, ui: &mut egui::Ui) {
        use egui_extras::{Column, TableBuilder};
        
        ui.label("测试简单表格:");
        
        // 尝试创建一个简单的表格
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let table = TableBuilder::new(ui)
                .striped(true)
                .column(Column::auto().at_least(100.0))
                .column(Column::auto().at_least(100.0))
                .min_scrolled_height(100.0);

            table
                .header(20.0, |mut header| {
                    header.col(|ui| { ui.strong("列1"); });
                    header.col(|ui| { ui.strong("列2"); });
                })
                .body(|mut body| {
                    for i in 0..5 {
                        body.row(18.0, |mut row| {
                            row.col(|ui| { ui.label(format!("行{}", i + 1)); });
                            row.col(|ui| { ui.label(format!("数据{}", i + 1)); });
                        });
                    }
                });
        })) {
            Ok(_) => {
                ui.colored_label(egui::Color32::GREEN, "✅ 表格测试成功");
            }
            Err(e) => {
                let error_msg = format!("❌ 表格测试失败: {:?}", e);
                ui.colored_label(egui::Color32::RED, &error_msg);
                self.last_error = Some(error_msg);
            }
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.last_error = Some(error);
    }
}
