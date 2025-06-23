use eframe::egui;
use crate::orderbook::{OrderBookManager, OrderFlow};
use crate::app::ReactiveApp;
use std::collections::BTreeMap;
use ordered_float::OrderedFloat;
use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// OrderBook深度数据展示组件
pub struct OrderBookWidget {
    /// 滚动位置
    scroll_position: f32,
    /// 自动跟踪当前价格
    auto_track_price: bool,
    /// 5秒累计数据的时间窗口
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

    /// 渲染OrderBook组件 - 响应式全屏布局
    pub fn show(&mut self, ui: &mut egui::Ui, app: &ReactiveApp) {
        // 获取可用空间
        let available_rect = ui.available_rect_before_wrap();

        ui.allocate_ui_with_layout(
            available_rect.size(),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                // 标题和控制按钮 - 固定高度
                ui.horizontal(|ui| {
                    ui.heading("订单流深度数据");
                    ui.separator();

                    // 自动跟踪价格开关
                    ui.checkbox(&mut self.auto_track_price, "自动跟踪价格");

                    // 重置滚动按钮
                    if ui.button("重置视图").clicked() {
                        self.scroll_position = 0.0;
                    }
                });

                ui.separator();

                // 获取订单流数据
                let order_flows = app.get_orderbook_manager().get_order_flows();
                let current_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                // 过滤和聚合5秒内的数据
                let aggregated_data = self.aggregate_5s_data(&order_flows, current_time);

                // 获取当前价格用于居中
                let snapshot = app.get_market_snapshot();
                let current_price = snapshot.current_price.unwrap_or(50000.0);

                // 计算表格可用高度（总高度 - 标题栏高度 - 分隔符高度）
                let header_height = 60.0; // 估算标题栏高度
                let table_height = available_rect.height() - header_height;

                // 渲染表格 - 占满剩余空间
                if aggregated_data.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label("暂无数据");
                    });
                } else {
                    self.render_orderbook_table(ui, &aggregated_data, current_price, table_height);
                }
            },
        );
    }

    /// 聚合5秒内的主动买单和卖单数据
    fn aggregate_5s_data(
        &self,
        order_flows: &BTreeMap<OrderedFloat<f64>, OrderFlow>,
        current_time: u64,
    ) -> Vec<OrderBookRow> {
        let mut rows = Vec::new();
        let time_threshold = current_time - self.time_window_seconds;

        // 按价格从高到低排序
        let mut sorted_flows: Vec<_> = order_flows.iter().collect();
        sorted_flows.sort_by(|a, b| b.0.cmp(a.0));

        for (price, order_flow) in sorted_flows {
            let price_val = price.0;

            // 获取订单簿数据
            let bid_volume = order_flow.bid_ask.bid;
            let ask_volume = order_flow.bid_ask.ask;

            // 计算5秒内的主动买单和卖单累计
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

    /// 渲染订单簿表格 - 响应式高度
    fn render_orderbook_table(
        &mut self,
        ui: &mut egui::Ui,
        data: &[OrderBookRow],
        current_price: f64,
        table_height: f32,
    ) {
        use egui_extras::{Column, TableBuilder};

        // 使用唯一的UI区域来避免ID冲突
        ui.push_id("orderbook_table_container", |ui| {
            // 获取可用宽度
            let available_width = ui.available_width();
            let column_width = available_width / 5.0; // 5列平均分配

            // 设置表格占满可用空间
            ui.allocate_ui_with_layout(
                egui::Vec2::new(available_width, table_height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // 创建表格 - 新布局：主动买单(5s) | Bid挂单 | 价格 | Ask挂单 | 主动卖单(5s)
                    let table = TableBuilder::new(ui)
                        .striped(false)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::exact(column_width)) // 主动买单累计(5s)
                        .column(Column::exact(column_width)) // Bid挂单
                        .column(Column::exact(column_width)) // 价格
                        .column(Column::exact(column_width)) // Ask挂单
                        .column(Column::exact(column_width)) // 主动卖单累计(5s)
                        .max_scroll_height(table_height - 30.0) // 减去表头高度
                        .vscroll(true);

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("主动买单(5s)");
                });
                header.col(|ui| {
                    ui.strong("Bid挂单");
                });
                header.col(|ui| {
                    ui.strong("价格");
                });
                header.col(|ui| {
                    ui.strong("Ask挂单");
                });
                header.col(|ui| {
                    ui.strong("主动卖单(5s)");
                });
            })
            .body(|mut body| {
                for row in data {
                    body.row(18.0, |mut row_ui| {
                        // 第1列：主动买单累计(5s)
                        row_ui.col(|ui| {
                            if row.buy_volume_5s > 0.0 {
                                ui.colored_label(egui::Color32::LIGHT_GREEN, format!("{:.4}", row.buy_volume_5s));
                            } else {
                                ui.label("--");
                            }
                        });

                        // 第2列：Bid挂单
                        row_ui.col(|ui| {
                            if row.bid_volume > 0.0 {
                                ui.colored_label(egui::Color32::GREEN, format!("{:.4}", row.bid_volume));
                            } else {
                                ui.label("--");
                            }
                        });

                        // 第3列：价格 - 高亮当前价格附近
                        row_ui.col(|ui| {
                            let price_diff = (row.price - current_price).abs();
                            let is_near_current = price_diff < 1.0; // 1美元范围内

                            if is_near_current {
                                // 当前价格附近用黄色背景高亮
                                let _color = if row.price >= current_price {
                                    egui::Color32::from_rgb(0, 100, 0) // 深绿色背景
                                } else {
                                    egui::Color32::from_rgb(100, 0, 0) // 深红色背景
                                };
                                ui.colored_label(egui::Color32::WHITE, format!("{:.2}", row.price))
                                    .on_hover_text("当前价格附近");
                            } else {
                                ui.label(format!("{:.2}", row.price));
                            }
                        });

                        // 第4列：Ask挂单
                        row_ui.col(|ui| {
                            if row.ask_volume > 0.0 {
                                ui.colored_label(egui::Color32::RED, format!("{:.4}", row.ask_volume));
                            } else {
                                ui.label("--");
                            }
                        });

                        // 第5列：主动卖单累计(5s)
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

/// 订单簿行数据结构
#[derive(Debug, Clone)]
struct OrderBookRow {
    price: f64,
    bid_volume: f64,
    ask_volume: f64,
    buy_volume_5s: f64,
    sell_volume_5s: f64,
}
