use eframe::egui;
use crate::orderbook::{OrderBookManager, OrderFlow};
use crate::app::ReactiveApp;
use std::collections::BTreeMap;
use ordered_float::OrderedFloat;
use std::time::{SystemTime, UNIX_EPOCH};

/// 主动成交订单历史足迹数据组件
pub struct TradeFootprintWidget {
    /// 滚动位置
    scroll_position: f32,
    /// 自动跟踪当前价格
    auto_track_price: bool,
    /// 显示的价格范围
    price_range: f64,
    /// 条形图最大宽度
    max_bar_width: f32,
}

impl Default for TradeFootprintWidget {
    fn default() -> Self {
        Self {
            scroll_position: 0.0,
            auto_track_price: true,
            price_range: 100.0, // 显示当前价格上下100美元范围
            max_bar_width: 200.0,
        }
    }
}

impl TradeFootprintWidget {
    pub fn new() -> Self {
        Self::default()
    }

    /// 渲染交易足迹组件 - 响应式全屏布局
    pub fn show(&mut self, ui: &mut egui::Ui, app: &ReactiveApp) {
        // 获取可用空间
        let available_rect = ui.available_rect_before_wrap();

        ui.allocate_ui_with_layout(
            available_rect.size(),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                // 标题和控制按钮 - 固定高度
                ui.horizontal(|ui| {
                    ui.heading("交易足迹数据");
                    ui.separator();

                    // 自动跟踪价格开关
                    ui.checkbox(&mut self.auto_track_price, "自动跟踪价格");

                    // 价格范围控制
                    ui.label("价格范围:");
                    ui.add(egui::Slider::new(&mut self.price_range, 10.0..=500.0).suffix("$"));

                    // 重置视图按钮
                    if ui.button("重置视图").clicked() {
                        self.scroll_position = 0.0;
                    }
                });

                ui.separator();

                // 获取订单流数据
                let order_flows = app.get_orderbook_manager().get_order_flows();
                let snapshot = app.get_market_snapshot();
                let current_price = snapshot.current_price.unwrap_or(50000.0);

                // 过滤价格范围内的数据
                let filtered_data = self.filter_price_range(&order_flows, current_price);

                // 计算表格可用高度（总高度 - 标题栏高度 - 分隔符高度）
                let header_height = 60.0; // 估算标题栏高度
                let table_height = available_rect.height() - header_height;

                // 创建表格显示足迹数据 - 占满剩余空间
                self.render_footprint_table(ui, &filtered_data, current_price, table_height);
            },
        );
    }

    /// 过滤价格范围内的数据
    fn filter_price_range(
        &self,
        order_flows: &BTreeMap<OrderedFloat<f64>, OrderFlow>,
        current_price: f64,
    ) -> Vec<FootprintRow> {
        let mut rows = Vec::new();
        let price_min = current_price - self.price_range / 2.0;
        let price_max = current_price + self.price_range / 2.0;

        // 按价格从高到低排序，只取价格范围内的数据
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

            // 获取历史交易数据
            let buy_volume = order_flow.history_trade_record.buy_volume;
            let sell_volume = order_flow.history_trade_record.sell_volume;
            let delta = buy_volume - sell_volume;

            // 只显示有交易活动的价格层级
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

    /// 渲染足迹表格 - 响应式高度
    fn render_footprint_table(
        &mut self,
        ui: &mut egui::Ui,
        data: &[FootprintRow],
        current_price: f64,
        table_height: f32,
    ) {
        use egui_extras::{Column, TableBuilder};

        // 计算最大交易量用于条形图缩放
        let max_volume = data
            .iter()
            .map(|row| row.buy_volume.max(row.sell_volume))
            .fold(0.0, f64::max);

        if max_volume == 0.0 {
            ui.label("暂无交易数据");
            return;
        }

        // 使用唯一的UI区域来避免ID冲突
        ui.push_id("footprint_table_container", |ui| {
            // 获取可用宽度
            let available_width = ui.available_width();
            let column_width = available_width / 4.0; // 4列平均分配

            // 设置表格占满可用空间
            ui.allocate_ui_with_layout(
                egui::Vec2::new(available_width, table_height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // 创建表格
                    let table = TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::exact(column_width)) // 价格
                    .column(Column::exact(column_width)) // 买单量 + 条形图
                    .column(Column::exact(column_width)) // 卖单量 + 条形图
                    .column(Column::exact(column_width)) // Delta
                    .max_scroll_height(table_height - 30.0) // 减去表头高度
                    .vscroll(true);

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("价格");
                });
                header.col(|ui| {
                    ui.strong("买单量");
                });
                header.col(|ui| {
                    ui.strong("卖单量");
                });
                header.col(|ui| {
                    ui.strong("Delta");
                });
            })
            .body(|mut body| {
                for row in data {
                    body.row(25.0, |mut row_ui| {
                        // 价格列 - 高亮当前价格附近
                        row_ui.col(|ui| {
                            let price_diff = (row.price - current_price).abs();
                            let is_near_current = price_diff < 1.0; // 1美元范围内

                            if is_near_current {
                                ui.colored_label(egui::Color32::YELLOW, format!("{:.2}", row.price))
                                    .on_hover_text("当前价格附近");
                            } else {
                                ui.label(format!("{:.2}", row.price));
                            }
                        });

                        // 买单量列 + 条形图
                        row_ui.col(|ui| {
                            ui.horizontal(|ui| {
                                // 文本显示
                                ui.colored_label(egui::Color32::GREEN, format!("{:.4}", row.buy_volume));
                                
                                // 条形图
                                if row.buy_volume > 0.0 {
                                    let bar_width = (row.buy_volume / max_volume * self.max_bar_width as f64) as f32;
                                    self.draw_horizontal_bar(ui, bar_width, egui::Color32::GREEN);
                                }
                            });
                        });

                        // 卖单量列 + 条形图
                        row_ui.col(|ui| {
                            ui.horizontal(|ui| {
                                // 文本显示
                                ui.colored_label(egui::Color32::RED, format!("{:.4}", row.sell_volume));
                                
                                // 条形图
                                if row.sell_volume > 0.0 {
                                    let bar_width = (row.sell_volume / max_volume * self.max_bar_width as f64) as f32;
                                    self.draw_horizontal_bar(ui, bar_width, egui::Color32::RED);
                                }
                            });
                        });

                        // Delta列
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

    /// 绘制横向条形图
    fn draw_horizontal_bar(&self, ui: &mut egui::Ui, width: f32, color: egui::Color32) {
        let height = 8.0;
        let (rect, _) = ui.allocate_exact_size(
            egui::Vec2::new(width.max(1.0), height),
            egui::Sense::hover(),
        );
        
        ui.painter().rect_filled(rect, 2.0, color);
    }
}

/// 足迹行数据结构
#[derive(Debug, Clone)]
struct FootprintRow {
    price: f64,
    buy_volume: f64,
    sell_volume: f64,
    delta: f64,
}
