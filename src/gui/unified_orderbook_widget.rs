use eframe::egui;
use crate::orderbook::OrderFlow;
use crate::app::ReactiveApp;

use std::collections::BTreeMap;
use ordered_float::OrderedFloat;
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::Path;
use egui_plot::{Line, Plot, PlotPoints};

/// 智能滚动信息
#[derive(Debug, Clone)]
struct SmartScrollInfo {
    /// 滚动偏移量
    scroll_offset: f32,
    /// 当前价格在数据中的索引
    current_price_index: Option<usize>,
    /// 目标行索引
    target_row: usize,
    /// 可见行数
    visible_rows: usize,
}

/// 统一的订单簿组件 - 合并订单深度和交易足迹数据
pub struct UnifiedOrderBookWidget {
    /// 自动跟踪当前价格
    auto_track_price: bool,
    /// 5秒累计数据的时间窗口
    time_window_seconds: u64,
    /// 显示的价格层级数量（当前价格上下各40层，总共81层）
    visible_price_levels: usize,
    /// 表格滚动位置
    scroll_position: f32,
    /// 条形图最大宽度
    max_bar_width: f32,
    /// 上次更新的价格（用于性能优化）
    last_price: f64,
    /// 上次更新时间（用于限制更新频率）
    last_update_time: std::time::Instant,
    /// 缓存的可见数据行（性能优化）
    cached_visible_data: Vec<UnifiedOrderBookRow>,
    /// 上次数据更新时间戳
    last_data_timestamp: u64,
    /// Logo纹理（可选）
    logo_texture: Option<egui::TextureHandle>,
    /// 交易信号窗口是否打开
    trading_signal_window_open: bool,
    /// 量化回测窗口是否打开
    quantitative_backtest_window_open: bool,
    /// 价格图表模态窗口是否打开
    price_chart_modal_open: bool,
    /// 价格历史数据（用于图表显示）
    price_history: std::collections::VecDeque<(f64, f64, f64, String)>, // (timestamp, price, volume, side)
    /// 最大价格历史数据点数
    max_price_history: usize,
    /// 价格图表固定高度（像素值）
    price_chart_height: f32,
    /// Trade Imbalance 500ms滑动窗口数据 (timestamp, buy_count, sell_count)
    trade_imbalance_window: std::collections::VecDeque<(u64, u32, u32)>,
    /// 当前Trade Imbalance值
    current_trade_imbalance: f64,
}

impl Default for UnifiedOrderBookWidget {
    fn default() -> Self {
        Self {
            auto_track_price: true,
            time_window_seconds: 5,
            visible_price_levels: 40, // 当前价格上下各40层，总共81层
            scroll_position: 0.0,
            max_bar_width: 80.0, // 条形图最大宽度
            last_price: 0.0,
            last_update_time: std::time::Instant::now(),
            cached_visible_data: Vec::new(),
            last_data_timestamp: 0,
            logo_texture: None,
            trading_signal_window_open: false,
            quantitative_backtest_window_open: false,
            price_chart_modal_open: false,
            price_history: std::collections::VecDeque::with_capacity(10000),
            max_price_history: 10000,
            price_chart_height: 200.0, // 默认高度300像素
            trade_imbalance_window: std::collections::VecDeque::new(),
            current_trade_imbalance: 0.0,
        }
    }
}

impl UnifiedOrderBookWidget {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置价格图表固定高度
    ///
    /// # 参数
    /// * `height` - 固定高度（像素值）
    ///   - 0.0: 不显示价格图表
    ///   - 300.0: 默认高度
    ///   - 最小值: 100.0，最大值: 800.0
    pub fn set_price_chart_height(&mut self, height: f32) {
        self.price_chart_height = height.clamp(0.0, 800.0);
    }

    /// 获取当前价格图表固定高度
    pub fn get_price_chart_height(&self) -> f32 {
        self.price_chart_height
    }

    /// 加载Logo纹理
    fn load_logo(&mut self, ctx: &egui::Context) {
        if self.logo_texture.is_none() {
            let logo_path = "src/image/logo.png";

            // 尝试加载Logo文件
            if Path::new(logo_path).exists() {
                match self.load_image_from_path(ctx, logo_path) {
                    Ok(texture) => {
                        self.logo_texture = Some(texture);
                        log::info!("Logo loaded successfully from {}", logo_path);
                    }
                    Err(e) => {
                        log::warn!("Failed to load logo from {}: {}", logo_path, e);
                    }
                }
            } else {
                log::info!("Logo file not found at {}, using text logo", logo_path);
            }
        }
    }

    /// 从文件路径加载图像
    fn load_image_from_path(&self, ctx: &egui::Context, path: &str) -> Result<egui::TextureHandle, Box<dyn std::error::Error>> {
        let image = image::open(path)?;
        let size = [image.width() as _, image.height() as _];
        let image_buffer = image.to_rgba8();
        let pixels = image_buffer.as_flat_samples();

        Ok(ctx.load_texture(
            "logo",
            egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
            Default::default(),
        ))
    }

    /// 渲染Logo
    fn render_logo(&self, ui: &mut egui::Ui, header_height: f32) {
        if let Some(ref logo_texture) = self.logo_texture {
            // 计算Logo显示尺寸，保持纵横比
            let logo_size = header_height * 0.8; // 使用80%的标题高度
            let texture_size = logo_texture.size_vec2();
            let aspect_ratio = texture_size.x / texture_size.y;

            let display_size = if aspect_ratio > 1.0 {
                // 宽图：限制宽度
                egui::Vec2::new(logo_size * aspect_ratio, logo_size)
            } else {
                // 高图或正方形：限制高度
                egui::Vec2::new(logo_size, logo_size / aspect_ratio)
            };

            // 显示Logo图像
            ui.add(egui::Image::new(logo_texture).fit_to_exact_size(display_size));
        } else {
            // 如果没有Logo图像，显示增强的文本Logo
            ui.horizontal(|ui| {
                // 创建一个简单的图标背景
                let logo_size = header_height * 0.7;
                let (rect, _) = ui.allocate_exact_size(
                    egui::Vec2::new(logo_size, logo_size),
                    egui::Sense::hover()
                );

                // 绘制圆形背景
                ui.painter().circle_filled(
                    rect.center(),
                    logo_size / 2.0,
                    egui::Color32::from_rgb(30, 60, 120)
                );

                // 绘制边框
                ui.painter().circle_stroke(
                    rect.center(),
                    logo_size / 2.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255))
                );

                // 在圆形内绘制简单的图表图标
                let center = rect.center();
                let radius = logo_size / 3.0;

                // 绘制上升趋势线
                let points = [
                    center + egui::Vec2::new(-radius * 0.6, radius * 0.3),
                    center + egui::Vec2::new(-radius * 0.2, radius * 0.1),
                    center + egui::Vec2::new(radius * 0.2, -radius * 0.1),
                    center + egui::Vec2::new(radius * 0.6, -radius * 0.3),
                ];

                for i in 0..points.len() - 1 {
                    ui.painter().line_segment(
                        [points[i], points[i + 1]],
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 100))
                    );
                }

                // 添加一些点表示数据点
                for point in &points {
                    ui.painter().circle_filled(*point, 2.0, egui::Color32::WHITE);
                }
            });
        }
        ui.add_space(10.0); // 在Logo后添加间距
    }

    /// 渲染统一订单簿组件 - 全屏布局（100%宽度）
    pub fn show(&mut self, ui: &mut egui::Ui, app: &ReactiveApp) {
        // 加载Logo（如果还未加载）
        self.load_logo(ui.ctx());

        // 获取总可用空间
        let total_rect = ui.available_rect_before_wrap();
        let total_height = total_rect.height();
        let total_width = total_rect.width();

        // 计算全屏尺寸
        let header_height = total_height * 0.05; // 5% 用于标题
        let content_height = total_height; // 95% 用于内容

        ui.vertical(|ui| {
            // 1. 顶部标题区域：5% 高度
            ui.allocate_ui_with_layout(
                egui::Vec2::new(total_width, header_height),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.heading("订单流分析");

                    // 显示当前价格并更新价格历史
                    let snapshot = app.get_market_snapshot();
                    if let Some(current_price) = snapshot.current_price {
                        ui.separator();
                        ui.label("当前价格:");
                        ui.colored_label(egui::Color32::YELLOW, format!("{:.2}", current_price));

                        // 获取最新交易信息并更新价格历史数据
                        let (_, last_side, _, last_volume) = app.get_orderbook_manager().get_last_trade_highlight();
                        let volume = last_volume.unwrap_or(0.0);
                        let side = last_side.unwrap_or_else(|| "unknown".to_string());
                        self.update_price_history(current_price, volume, side);
                    }

                    // 在右侧添加按钮
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // 量化回测按钮
                        if ui.button("量化回测").clicked() {
                            self.quantitative_backtest_window_open = true;
                        }

                        ui.add_space(10.0); // 按钮间距

                        // 交易信号按钮
                        if ui.button("交易信号").clicked() {
                            self.trading_signal_window_open = true;
                        }

                        ui.add_space(10.0); // 按钮间距

                    });
                },
            );

            // 2. 主要内容区域：95% 高度，水平布局 - orderbook占一半宽度
            ui.allocate_ui_with_layout(
                egui::Vec2::new(total_width, content_height),
                egui::Layout::left_to_right(egui::Align::TOP),
                |ui| {
                    // 左侧：订单簿表格 - 占窗体宽度的一半
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(total_width * 0.5, content_height),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            // 获取当前价格和数据
                            let snapshot = app.get_market_snapshot();
                            let current_price = snapshot.current_price.unwrap_or(50000.0);

                            // 获取订单流数据
                            let order_flows = app.get_orderbook_manager().get_order_flows();
                            let current_time = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64;

                            // 使用数据驱动的UI渲染：提取可见范围数据
                            let visible_data = self.extract_visible_data(&order_flows, current_time, current_price);

                            if visible_data.is_empty() {
                                ui.centered_and_justified(|ui| {
                                    ui.label("暂无订单簿数据");
                                });
                            } else {
                                // 渲染订单簿表格，占据左侧一半空间
                                self.render_bounded_table(ui, &visible_data, current_price, content_height);
                            }
                        },
                    );

                    // 右侧：预留空间 - 占窗体宽度的另一半
                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(total_width * 0.5, content_height),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            // 上半部分：实时价格图表 - 使用固定高度
                            let chart_height = self.price_chart_height.min(content_height - 200.0); // 确保至少留200像素给两个指标区域
                            if chart_height > 0.0 {
                                ui.allocate_ui_with_layout(
                                    egui::Vec2::new(total_width * 0.5, chart_height),
                                    egui::Layout::top_down(egui::Align::LEFT),
                                    |ui| {
                                        self.render_embedded_price_chart(ui, app);
                                    },
                                );
                            }

                            // 中间部分：Orderbook Imbalance指标 - 固定高度100像素
                            let imbalance_height = 100.0;
                            ui.allocate_ui_with_layout(
                                egui::Vec2::new(total_width * 0.5, imbalance_height),
                                egui::Layout::top_down(egui::Align::LEFT),
                                |ui| {
                                    self.render_orderbook_imbalance(ui, app);
                                },
                            );

                            // 下半部分：Trade Imbalance指标 - 固定高度100像素
                            let trade_imbalance_height = 100.0;
                            ui.allocate_ui_with_layout(
                                egui::Vec2::new(total_width * 0.5, trade_imbalance_height),
                                egui::Layout::top_down(egui::Align::LEFT),
                                |ui| {
                                    self.render_trade_imbalance(ui, app);
                                },
                            );
                        },
                    );
                },
            );
        });

        // 渲染弹出窗口
        self.render_popup_windows(ui.ctx());
    }

    /// 数据驱动UI：提取当前价格±40层的可见数据（总共最多81行）
    fn extract_visible_data(
        &mut self,
        order_flows: &BTreeMap<OrderedFloat<f64>, OrderFlow>,
        current_time: u64,
        current_price: f64,
    ) -> Vec<UnifiedOrderBookRow> {
        // 性能优化：检查是否需要重新计算
        let should_update = current_time != self.last_data_timestamp ||
                           (current_price - self.last_price).abs() > 0.1;

        if !should_update && !self.cached_visible_data.is_empty() {
            return self.cached_visible_data.clone();
        }

        // 更新缓存时间戳
        self.last_data_timestamp = current_time;
        self.last_price = current_price;

        // 首先获取所有有效的价格层级并转换为1美元聚合级别
        let mut existing_price_levels: Vec<i64> = order_flows
            .keys()
            .map(|k| k.0.floor() as i64) // 使用向下取整聚合到1美元级别，转换为整数
            .collect::<std::collections::HashSet<_>>() // 去重
            .into_iter()
            .collect();

        // 当前价格对应的聚合级别
        let current_price_level = current_price.floor() as i64;

        // 生成完整的价格级别范围：当前价格上下各40个美元级别
        let mut all_price_levels: Vec<i64> = Vec::new();

        // 添加当前价格上方的价格级别（从高到低）
        for i in 0..=self.visible_price_levels {
            all_price_levels.push(current_price_level + i as i64);
        }

        // 添加当前价格下方的价格级别（从高到低）
        for i in 1..=self.visible_price_levels {
            all_price_levels.push(current_price_level - i as i64);
        }

        // 添加现有数据中的其他价格级别（确保不遗漏任何现有数据）
        for &existing_level in &existing_price_levels {
            if !all_price_levels.contains(&existing_level) {
                all_price_levels.push(existing_level);
            }
        }

        // 转换回f64并排序（从高到低）
        let mut all_price_levels: Vec<f64> = all_price_levels
            .into_iter()
            .map(|level| level as f64)
            .collect();
        all_price_levels.sort_by(|a, b| b.partial_cmp(a).unwrap());

        // 找到当前价格在排序列表中的位置
        let current_price_level_f64 = current_price.floor();
        let current_price_index = all_price_levels
            .iter()
            .position(|&price_level| price_level <= current_price_level_f64)
            .unwrap_or(all_price_levels.len() / 2);

        // 计算可见范围：确保当前价格上下各有40个级别
        let start_index = current_price_index.saturating_sub(self.visible_price_levels);
        let end_index = std::cmp::min(
            current_price_index + self.visible_price_levels + 1,
            all_price_levels.len()
        );

        // 提取可见范围内的聚合价格级别
        let visible_price_levels = &all_price_levels[start_index..end_index];

        // 为每个聚合价格级别收集所有相关的原始价格
        // 如果某个价格级别没有实际数据，我们仍然需要包含它以显示空数据
        let mut visible_prices = Vec::new();
        for &price_level in visible_price_levels {
            let mut found_data = false;
            // 找到属于这个聚合级别的所有原始价格
            for price_key in order_flows.keys() {
                let original_price = price_key.0;
                if original_price.floor() == price_level {
                    visible_prices.push(original_price);
                    found_data = true;
                }
            }

            // 如果这个价格级别没有实际数据，添加一个虚拟价格以确保显示空行
            if !found_data {
                visible_prices.push(price_level);
            }
        }

        // 构建可见数据行
        let visible_data = self.build_visible_rows(order_flows, &visible_prices, current_time);

        // 缓存结果
        self.cached_visible_data = visible_data.clone();

        visible_data
    }

    /// 构建可见数据行（带价格聚合功能）
    fn build_visible_rows(
        &self,
        order_flows: &BTreeMap<OrderedFloat<f64>, OrderFlow>,
        visible_prices: &[f64],
        current_time: u64,
    ) -> Vec<UnifiedOrderBookRow> {
        let time_threshold = current_time.saturating_sub(self.time_window_seconds * 1000);

        // 第一步：将价格聚合到1美元级别
        let aggregated_data = self.aggregate_prices_to_usd_levels(order_flows, visible_prices, time_threshold);

        // 第二步：转换为显示行
        let mut rows: Vec<UnifiedOrderBookRow> = aggregated_data
            .into_iter()
            .map(|(price_level, aggregated_flow)| UnifiedOrderBookRow {
                price: price_level.0, // 提取OrderedFloat中的f64值
                bid_volume: aggregated_flow.bid_volume,
                ask_volume: aggregated_flow.ask_volume,
                active_buy_volume_5s: aggregated_flow.active_buy_volume_5s,
                active_sell_volume_5s: aggregated_flow.active_sell_volume_5s,
                history_buy_volume: aggregated_flow.history_buy_volume,
                history_sell_volume: aggregated_flow.history_sell_volume,
                delta: aggregated_flow.history_buy_volume - aggregated_flow.history_sell_volume,
            })
            .collect();

        // 按价格从高到低排序
        rows.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());

        rows
    }

    /// 将价格聚合到1美元级别（使用向下取整策略）
    fn aggregate_prices_to_usd_levels(
        &self,
        order_flows: &BTreeMap<OrderedFloat<f64>, OrderFlow>,
        visible_prices: &[f64],
        time_threshold: u64,
    ) -> BTreeMap<OrderedFloat<f64>, AggregatedOrderFlow> {
        use std::collections::HashMap;

        let mut aggregated_map: HashMap<i64, AggregatedOrderFlow> = HashMap::new();

        // 遍历所有可见价格，进行聚合
        for &price_val in visible_prices {
            // 使用向下取整策略：floor(price) 聚合到1美元级别
            let price_level_int = price_val.floor() as i64;
            let price_key = OrderedFloat(price_val);

            // 确保每个价格级别都有一个条目（即使没有数据也显示空行）
            let entry = aggregated_map.entry(price_level_int).or_insert_with(|| AggregatedOrderFlow::new());

            // 获取该价格的订单流数据（如果存在）
            if let Some(order_flow) = order_flows.get(&price_key) {
                // 聚合订单簿深度数据
                entry.bid_volume += order_flow.bid_ask.bid;
                entry.ask_volume += order_flow.bid_ask.ask;

                // 聚合5秒内的主动交易数据
                if order_flow.realtime_trade_record.timestamp >= time_threshold {
                    entry.active_buy_volume_5s += order_flow.realtime_trade_record.buy_volume;
                    entry.active_sell_volume_5s += order_flow.realtime_trade_record.sell_volume;
                }

                // 聚合历史交易足迹数据
                entry.history_buy_volume += order_flow.history_trade_record.buy_volume;
                entry.history_sell_volume += order_flow.history_trade_record.sell_volume;
            }
            // 如果没有订单流数据，entry 保持为默认的零值，这样会显示空行
        }

        // 转换为BTreeMap以保持排序，并将整数价格转换回浮点数
        aggregated_map
            .into_iter()
            .map(|(price_int, flow)| (OrderedFloat(price_int as f64), flow))
            .collect()
    }

    /// 渲染边界受限的表格 - 严格控制在95%区域内
    fn render_bounded_table(
        &mut self,
        ui: &mut egui::Ui,
        data: &[UnifiedOrderBookRow],
        current_price: f64,
        table_height: f32,
    ) {
        use egui_extras::{Column, TableBuilder};

        // 计算各列的最大值用于条形图缩放
        let max_history_buy = data.iter().map(|row| row.history_buy_volume).fold(0.0, f64::max);
        let max_history_sell = data.iter().map(|row| row.history_sell_volume).fold(0.0, f64::max);
        let max_delta = data.iter().map(|row| row.delta.abs()).fold(0.0, f64::max);

        // 设置自定义列宽 - 前5列使用固定较小宽度
        let available_width = ui.available_width();
        let fixed_buyselltrade_width = 50.0;  // 主动买单和卖单的宽度
        let price_width = 47.0;
        let fixed_column_width = 80.0; // 前5列的固定宽度（比之前更小）
        let remaining_width = available_width - (fixed_column_width * 5.0);
        let flexible_column_width = remaining_width / 3.0; // 后3列平均分配剩余宽度

        // 使用严格边界控制的表格容器
        ui.allocate_ui_with_layout(
            egui::Vec2::new(available_width, table_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                // 设置剪切区域，确保内容不会溢出95%边界
                ui.set_clip_rect(ui.available_rect_before_wrap());

                let table = TableBuilder::new(ui)
                    .striped(false)
                    .resizable(false) // 禁用调整大小以保持固定宽度
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::exact(fixed_buyselltrade_width)) // 主动卖单累计(5s) - 固定80px
                    .column(Column::exact(fixed_column_width)) // 买单深度 - 固定80px
                    .column(Column::exact(price_width)) // 价格 - 固定50px
                    .column(Column::exact(fixed_column_width)) // 卖单深度 - 固定80px
                    .column(Column::exact(fixed_buyselltrade_width)) // 主动买单累计(5s) - 固定80px
                    .column(Column::exact(flexible_column_width)) // 历史累计主动买单量 - 灵活宽度
                    .column(Column::exact(flexible_column_width)) // 历史累计主动卖单量 - 灵活宽度
                    .column(Column::remainder()) // 主动订单delta - 使用剩余空间
                    .max_scroll_height(table_height - 30.0) // 为表头预留空间
                    .scroll_to_row(self.calculate_center_row_index(data, current_price), None);

                table
                    .header(25.0, |mut header| {
                        header.col(|ui| { ui.strong("主动卖单"); });
                        header.col(|ui| { ui.strong("买单深度"); });
                        header.col(|ui| { ui.strong("价格"); });
                        header.col(|ui| { ui.strong("卖单深度"); });
                        header.col(|ui| { ui.strong("主动买单"); });
                        header.col(|ui| { ui.strong("历史累计买单"); });
                        header.col(|ui| { ui.strong("历史累计卖单"); });
                        header.col(|ui| { ui.strong("Delta"); });
                    })
                    .body(|mut body| {
                        // 渲染所有可见数据行（最多81行）
                        for row in data {
                            body.row(25.0, |mut row_ui| {
                                self.render_table_row(&mut row_ui, row, current_price, max_history_buy, max_history_sell, max_delta);
                            });
                        }
                    });
            },
        );
    }

    /// 计算当前价格在数据中的中心行索引
    fn calculate_center_row_index(&self, data: &[UnifiedOrderBookRow], current_price: f64) -> usize {
        if data.is_empty() {
            return 0;
        }

        // 找到最接近当前价格的行
        data.iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let diff_a = (a.price - current_price).abs();
                let diff_b = (b.price - current_price).abs();
                diff_a.partial_cmp(&diff_b).unwrap()
            })
            .map(|(index, _)| index)
            .unwrap_or(data.len() / 2)
    }

    /// 判断是否为当前价格行（确保只有一个价格层级被高亮）
    fn is_current_price_row(&self, row_price: f64, current_price: f64) -> bool {
        // 使用缓存的可见数据来确定最接近的价格
        if self.cached_visible_data.is_empty() {
            return false;
        }

        // 找到最接近当前价格的行
        let closest_price = self.cached_visible_data
            .iter()
            .min_by(|a, b| {
                let diff_a = (a.price - current_price).abs();
                let diff_b = (b.price - current_price).abs();
                diff_a.partial_cmp(&diff_b).unwrap()
            })
            .map(|row| row.price)
            .unwrap_or(current_price);

        // 只有最接近的价格才被标记为当前价格行
        (row_price - closest_price).abs() < 0.001 // 使用小的容差来处理浮点数精度问题
    }

    /// 计算智能滚动位置（优化版本）
    fn calculate_smart_scroll_position(
        &mut self,
        data: &[UnifiedOrderBookRow],
        current_price: f64,
        table_height: f32,
    ) -> SmartScrollInfo {
        let row_height = 25.0;
        let header_height = 25.0;
        let effective_table_height = table_height - header_height;
        let visible_rows = (effective_table_height / row_height) as usize;

        // 性能优化：限制更新频率
        let now = std::time::Instant::now();
        let should_update = now.duration_since(self.last_update_time).as_millis() > 50 || // 更频繁的更新（50ms）
                           (current_price - self.last_price).abs() > 0.1; // 更敏感的价格变化检测

        if !should_update && self.scroll_position > 0.0 {
            // 返回缓存的滚动信息
            return SmartScrollInfo {
                scroll_offset: self.scroll_position,
                current_price_index: None,
                target_row: 0,
                visible_rows,
            };
        }

        // 更新缓存
        self.last_price = current_price;
        self.last_update_time = now;

        // 找到当前价格在数据中的位置
        let current_price_index = if self.auto_track_price && !data.is_empty() {
            data.iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let diff_a = (a.price - current_price).abs();
                    let diff_b = (b.price - current_price).abs();
                    diff_a.partial_cmp(&diff_b).unwrap()
                })
                .map(|(index, _)| index)
        } else {
            None
        };

        // 计算目标滚动位置
        let scroll_offset = if let Some(index) = current_price_index {
            // 让当前价格显示在表格中心
            let center_offset = visible_rows / 2;
            let target_row = if index >= center_offset {
                index - center_offset
            } else {
                0 // 如果数据不够，从顶部开始显示
            };

            let new_scroll_position = (target_row as f32) * row_height;

            // 初始渲染时直接跳转到目标位置
            if self.scroll_position == 0.0 {
                self.scroll_position = new_scroll_position;
                new_scroll_position
            } else {
                // 后续更新使用平滑滚动
                let scroll_diff = (new_scroll_position - self.scroll_position).abs();
                if scroll_diff > 10.0 {
                    // 使用更快的插值因子实现更响应的滚动
                    let lerp_factor = 0.6;
                    let interpolated_position = self.scroll_position + (new_scroll_position - self.scroll_position) * lerp_factor;
                    self.scroll_position = interpolated_position;
                    interpolated_position
                } else {
                    // 小幅度变化直接更新
                    self.scroll_position = new_scroll_position;
                    new_scroll_position
                }
            }
        } else {
            // 没有找到当前价格，保持当前滚动位置
            self.scroll_position
        };

        SmartScrollInfo {
            scroll_offset,
            current_price_index,
            target_row: 0,
            visible_rows,
        }
    }

    /// 渲染带自动滚动的统一表格
    fn render_unified_table_with_auto_scroll(
        &mut self,
        ui: &mut egui::Ui,
        data: &[UnifiedOrderBookRow],
        current_price: f64,
        table_height: f32,
    ) {
        // 计算智能滚动位置
        let scroll_info = self.calculate_smart_scroll_position(data, current_price, table_height);

        // 添加简洁的状态指示器
        if self.auto_track_price && scroll_info.current_price_index.is_some() {
            ui.horizontal(|ui| {
                ui.small("🎯 自动追踪价格");
                ui.separator();
                ui.small(format!("数据行数: {}", data.len()));
                if let Some(index) = scroll_info.current_price_index {
                    ui.separator();
                    ui.small(format!("当前位置: {}/{}", index + 1, data.len()));
                }
            });
            ui.separator();
        }

        // 使用ScrollArea实现精确的滚动控制
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .scroll_offset(egui::Vec2::new(0.0, scroll_info.scroll_offset))
            .max_height(table_height)
            .show(ui, |ui| {
                self.render_unified_table_content(ui, data, current_price, scroll_info);
            });
    }

    /// 渲染表格内容（在ScrollArea内部）
    fn render_unified_table_content(
        &self,
        ui: &mut egui::Ui,
        data: &[UnifiedOrderBookRow],
        current_price: f64,
        scroll_info: SmartScrollInfo,
    ) {
        use egui_extras::{Column, TableBuilder};

        // 计算各列的最大值用于条形图缩放
        let max_history_buy = data.iter().map(|row| row.history_buy_volume).fold(0.0, f64::max);
        let max_history_sell = data.iter().map(|row| row.history_sell_volume).fold(0.0, f64::max);
        let max_delta = data.iter().map(|row| row.delta.abs()).fold(0.0, f64::max);
        let max_bid_volume = data.iter().map(|row| row.bid_volume).fold(0.0, f64::max);
        let max_ask_volume = data.iter().map(|row| row.ask_volume).fold(0.0, f64::max);

        ui.push_id("unified_orderbook_table", |ui| {
            // 获取可用宽度并平均分配给8列
            let available_width = ui.available_width();
            let column_width = available_width / 8.0;

            let table = TableBuilder::new(ui)
                .striped(false)
                .resizable(false) // 禁用调整大小以保持均匀分布
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(column_width)) // 主动卖单累计(5s)
                .column(Column::exact(column_width)) // 买单深度
                .column(Column::exact(column_width)) // 价格
                .column(Column::exact(column_width)) // 卖单深度
                .column(Column::exact(column_width)) // 主动买单累计(5s)
                .column(Column::exact(column_width)) // 历史累计主动买单量
                .column(Column::exact(column_width)) // 历史累计主动卖单量
                .column(Column::remainder()) // 主动订单delta - 使用剩余空间
                .sense(egui::Sense::click()); // 不使用内置滚动，由外部ScrollArea控制

            table
                .header(25.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("主动卖单累计(5s)");
                    });
                    header.col(|ui| {
                        ui.strong("买单深度");
                    });
                    header.col(|ui| {
                        ui.strong("价格");
                    });
                    header.col(|ui| {
                        ui.strong("卖单深度");
                    });
                    header.col(|ui| {
                        ui.strong("主动买单累计(5s)");
                    });
                    header.col(|ui| {
                        ui.strong("历史累计买单");
                    });
                    header.col(|ui| {
                        ui.strong("历史累计卖单");
                    });
                    header.col(|ui| {
                        ui.strong("Delta");
                    });
                })
                .body(|mut body| {
                    // 渲染所有数据行
                    for row in data {
                        body.row(25.0, |mut row_ui| {
                            self.render_table_row(&mut row_ui, row, current_price, max_history_buy, max_history_sell, max_delta);
                        });
                    }
                });
        });
    }

    /// 直接渲染表格，占满整个可用空间（保留用于兼容性）
    fn render_unified_table_direct(
        &mut self,
        ui: &mut egui::Ui,
        data: &[UnifiedOrderBookRow],
        current_price: f64,
        table_height: f32,
    ) {
        use egui_extras::{Column, TableBuilder};

        // 计算各列的最大值用于条形图缩放
        let max_history_buy = data.iter().map(|row| row.history_buy_volume).fold(0.0, f64::max);
        let max_history_sell = data.iter().map(|row| row.history_sell_volume).fold(0.0, f64::max);
        let max_delta = data.iter().map(|row| row.delta.abs()).fold(0.0, f64::max);
        let max_bid_volume = data.iter().map(|row| row.bid_volume).fold(0.0, f64::max);
        let max_ask_volume = data.iter().map(|row| row.ask_volume).fold(0.0, f64::max);

        // 找到当前价格在数据中的位置，用于自动滚动
        let current_price_index = if self.auto_track_price && !data.is_empty() {
            data.iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let diff_a = (a.price - current_price).abs();
                    let diff_b = (b.price - current_price).abs();
                    diff_a.partial_cmp(&diff_b).unwrap()
                })
                .map(|(index, _)| index)
        } else {
            None
        };

        ui.push_id("unified_orderbook_table", |ui| {
            // 获取可用宽度并平均分配给8列
            let available_width = ui.available_width();
            let column_width = available_width / 8.0;

            let table = TableBuilder::new(ui)
                .striped(false)
                .resizable(false) // 禁用调整大小以保持均匀分布
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(column_width)) // 主动卖单累计(5s)
                .column(Column::exact(column_width)) // 买单深度
                .column(Column::exact(column_width)) // 价格
                .column(Column::exact(column_width)) // 卖单深度
                .column(Column::exact(column_width)) // 主动买单累计(5s)
                .column(Column::exact(column_width)) // 历史累计主动买单量
                .column(Column::exact(column_width)) // 历史累计主动卖单量
                .column(Column::remainder()) // 主动订单delta - 使用剩余空间
                .vscroll(true) // 启用内置滚动
                .max_scroll_height(table_height); // 设置最大滚动高度

            table
                .header(25.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("主动卖单累计(5s)");
                    });
                    header.col(|ui| {
                        ui.strong("买单深度");
                    });
                    header.col(|ui| {
                        ui.strong("价格");
                    });
                    header.col(|ui| {
                        ui.strong("卖单深度");
                    });
                    header.col(|ui| {
                        ui.strong("主动买单累计(5s)");
                    });
                    header.col(|ui| {
                        ui.strong("历史累计买单");
                    });
                    header.col(|ui| {
                        ui.strong("历史累计卖单");
                    });
                    header.col(|ui| {
                        ui.strong("Delta");
                    });
                })
                .body(|mut body| {
                    // 渲染所有数据行，表格内置滚动会自动处理
                    for row in data {
                        body.row(25.0, |mut row_ui| {
                            self.render_table_row(&mut row_ui, row, current_price, max_history_buy, max_history_sell, max_delta);
                        });
                    }
                });
        });
    }

    /// 渲染统一表格 - 9列布局（保留原方法以防需要）
    fn render_unified_table(
        &mut self,
        ui: &mut egui::Ui,
        data: &[UnifiedOrderBookRow],
        current_price: f64,
        _table_height: f32,
    ) {
        use egui_extras::{Column, TableBuilder};

        // 计算各列的最大值用于条形图缩放
        let max_history_buy = data.iter().map(|row| row.history_buy_volume).fold(0.0, f64::max);
        let max_history_sell = data.iter().map(|row| row.history_sell_volume).fold(0.0, f64::max);
        let max_delta = data.iter().map(|row| row.delta.abs()).fold(0.0, f64::max);
        let max_bid_volume = data.iter().map(|row| row.bid_volume).fold(0.0, f64::max);
        let max_ask_volume = data.iter().map(|row| row.ask_volume).fold(0.0, f64::max);

        ui.push_id("unified_orderbook_table", |ui| {
            // 获取可用宽度并平均分配给8列
            let available_width = ui.available_width();
            let column_width = available_width / 8.0;

            let table = TableBuilder::new(ui)
                .striped(false)
                .resizable(false) // 禁用调整大小以保持均匀分布
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(column_width)) // 主动卖单累计(5s)
                .column(Column::exact(column_width)) // 买单深度
                .column(Column::exact(column_width)) // 价格
                .column(Column::exact(column_width)) // 卖单深度
                .column(Column::exact(column_width)) // 主动买单累计(5s)
                .column(Column::exact(column_width)) // 历史累计主动买单量
                .column(Column::exact(column_width)) // 历史累计主动卖单量
                .column(Column::remainder()) // 主动订单delta - 使用剩余空间
                .sense(egui::Sense::click()); // 移除内置滚动，使用外部ScrollArea

            table
                .header(25.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("主动卖单累计(5s)");
                    });
                    header.col(|ui| {
                        ui.strong("买单深度");
                    });
                    header.col(|ui| {
                        ui.strong("价格");
                    });
                    header.col(|ui| {
                        ui.strong("卖单深度");
                    });
                    header.col(|ui| {
                        ui.strong("主动买单累计(5s)");
                    });
                    header.col(|ui| {
                        ui.strong("历史累计买单");
                    });
                    header.col(|ui| {
                        ui.strong("历史累计卖单");
                    });
                    header.col(|ui| {
                        ui.strong("Delta");
                    });
                })
                .body(|mut body| {
                    // 直接渲染所有行，滚动由外部ScrollArea控制

                    for row in data {
                        body.row(25.0, |mut row_ui| {
                            // 第1列：主动卖单累计(5s) - 加粗显示
                            row_ui.col(|ui| {
                                if row.active_sell_volume_5s > 0.0 {
                                    ui.add(egui::Label::new(egui::RichText::new(format!("{:.4}", row.active_sell_volume_5s))
                                        .color(egui::Color32::from_rgb(255, 120, 120))
                                        .strong()));
                                } else {
                                    ui.colored_label(egui::Color32::GRAY, "--");
                                }
                            });

                            // 第2列：买单深度 + 背景条形图
                            row_ui.col(|ui| {
                                if row.bid_volume > 0.0 {
                                    // 计算条形图宽度
                                    let bar_width = self.calculate_bar_width(row.bid_volume, max_bid_volume);

                                    // 使用层叠布局：先绘制背景条形图，再显示文本
                                    ui.allocate_ui_with_layout(
                                        ui.available_size(),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            if bar_width > 1.0 {
                                                // 绘制背景条形图（使用与文本相同的颜色但更透明）
                                                self.draw_background_bar(ui, bar_width, egui::Color32::from_rgb(120, 180, 255));
                                            }

                                            // 重置UI位置到开始处，在条形图上方显示文本
                                            ui.allocate_ui_at_rect(ui.max_rect(), |ui| {
                                                ui.colored_label(egui::Color32::from_rgb(120, 180, 255), format!("{:.4}", row.bid_volume));
                                            });
                                        }
                                    );
                                } else {
                                    ui.colored_label(egui::Color32::GRAY, "--");
                                }
                            });

                            // 第3列：价格 - 精确的当前价格高亮（只有一个价格层级被高亮）
                            row_ui.col(|ui| {
                                let is_current_price_row = self.is_current_price_row(row.price, current_price);
                                // 格式化价格为整数美元显示（1美元聚合级别）
                                let price_display = format!("{:.0}", row.price);

                                if is_current_price_row {
                                    // 当前价格行 - 使用强烈高亮和背景
                                    ui.scope(|ui| {
                                        ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
                                        let response = ui.colored_label(egui::Color32::from_rgb(255, 255, 0), price_display);

                                        // 添加背景高亮
                                        let rect = response.rect;
                                        ui.painter().rect_filled(
                                            rect.expand(2.0),
                                            egui::Rounding::same(3.0),
                                            egui::Color32::from_rgb(255, 255, 0).gamma_multiply(0.3)
                                        );

                                        response.on_hover_text("🎯 当前价格 (1美元聚合级别)");
                                    });
                                } else {
                                    // 普通价格行 - 白色文本
                                    ui.colored_label(egui::Color32::WHITE, price_display);
                                }
                            });

                            // 第4列：卖单深度 + 背景条形图
                            row_ui.col(|ui| {
                                if row.ask_volume > 0.0 {
                                    // 计算条形图宽度
                                    let bar_width = self.calculate_bar_width(row.ask_volume, max_ask_volume);

                                    // 使用层叠布局：先绘制背景条形图，再显示文本
                                    ui.allocate_ui_with_layout(
                                        ui.available_size(),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            if bar_width > 1.0 {
                                                // 绘制背景条形图（使用与文本相同的颜色但更透明）
                                                self.draw_background_bar(ui, bar_width, egui::Color32::from_rgb(255, 120, 120));
                                            }

                                            // 重置UI位置到开始处，在条形图上方显示文本
                                            ui.allocate_ui_at_rect(ui.max_rect(), |ui| {
                                                ui.colored_label(egui::Color32::from_rgb(255, 120, 120), format!("{:.4}", row.ask_volume));
                                            });
                                        }
                                    );
                                } else {
                                    ui.colored_label(egui::Color32::GRAY, "--");
                                }
                            });

                            // 第5列：主动买单累计(5s) - 加粗显示
                            row_ui.col(|ui| {
                                if row.active_buy_volume_5s > 0.0 {
                                    ui.add(egui::Label::new(egui::RichText::new(format!("{:.4}", row.active_buy_volume_5s))
                                        .color(egui::Color32::from_rgb(120, 255, 120))
                                        .strong()));
                                } else {
                                    ui.colored_label(egui::Color32::GRAY, "--");
                                }
                            });

                            // 第6列：历史累计主动买单量 + 条形图
                            row_ui.col(|ui| {
                                ui.horizontal(|ui| {
                                    if row.history_buy_volume > 0.0 {
                                        ui.colored_label(egui::Color32::from_rgb(120, 255, 120), format!("{:.4}", row.history_buy_volume));

                                        // 绘制条形图
                                        let bar_width = self.calculate_bar_width(row.history_buy_volume, max_history_buy);
                                        if bar_width > 1.0 {
                                            self.draw_horizontal_bar(ui, bar_width, egui::Color32::from_rgb(120, 255, 120));
                                        }
                                    } else {
                                        ui.colored_label(egui::Color32::GRAY, "--");
                                    }
                                });
                            });

                            // 第7列：历史累计主动卖单量 + 条形图
                            row_ui.col(|ui| {
                                ui.horizontal(|ui| {
                                    if row.history_sell_volume > 0.0 {
                                        ui.colored_label(egui::Color32::from_rgb(255, 120, 120), format!("{:.4}", row.history_sell_volume));

                                        // 绘制条形图
                                        let bar_width = self.calculate_bar_width(row.history_sell_volume, max_history_sell);
                                        if bar_width > 1.0 {
                                            self.draw_horizontal_bar(ui, bar_width, egui::Color32::from_rgb(255, 120, 120));
                                        }
                                    } else {
                                        ui.colored_label(egui::Color32::GRAY, "--");
                                    }
                                });
                            });

                            // 第8列：主动订单delta + 条形图
                            row_ui.col(|ui| {
                                ui.horizontal(|ui| {
                                    if row.delta.abs() > 0.0001 {
                                        let color = if row.delta > 0.0 {
                                            egui::Color32::from_rgb(120, 255, 120)
                                        } else {
                                            egui::Color32::from_rgb(255, 120, 120)
                                        };
                                        ui.colored_label(color, format!("{:+.4}", row.delta));

                                        // 绘制条形图
                                        let bar_width = self.calculate_bar_width(row.delta.abs(), max_delta);
                                        if bar_width > 1.0 {
                                            self.draw_horizontal_bar(ui, bar_width, color);
                                        }
                                    } else {
                                        ui.colored_label(egui::Color32::GRAY, "--");
                                    }
                                });
                            });


                        });
                    }
                });
        });
    }

    /// 绘制增强的横向条形图
    fn draw_horizontal_bar(&self, ui: &mut egui::Ui, width: f32, color: egui::Color32) {
        let (rect, response) = ui.allocate_exact_size(
            egui::Vec2::new(width, 12.0),
            egui::Sense::hover()
        );

        // 基础条形图
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(3.0),
            color.gamma_multiply(0.6)
        );

        // 添加渐变效果
        let gradient_rect = egui::Rect::from_min_size(
            rect.min,
            egui::Vec2::new(rect.width(), rect.height() / 2.0)
        );
        ui.painter().rect_filled(
            gradient_rect,
            egui::Rounding::same(3.0),
            color.gamma_multiply(0.8)
        );

        // 悬停效果
        if response.hovered() {
            ui.painter().rect_stroke(
                rect.expand(1.0),
                egui::Rounding::same(3.0),
                egui::Stroke::new(1.0, color)
            );
        }
    }

    /// 计算条形图宽度比例
    fn calculate_bar_width(&self, value: f64, max_value: f64) -> f32 {
        if max_value > 0.0 {
            (value / max_value * self.max_bar_width as f64) as f32
        } else {
            0.0
        }
    }

    /// 绘制背景条形图（用于订单深度列）
    fn draw_background_bar(&self, ui: &mut egui::Ui, width: f32, color: egui::Color32) {
        let available_rect = ui.available_rect_before_wrap();
        let bar_height = available_rect.height() * 0.8; // 使用80%的行高度

        // 创建背景条形图的矩形
        let bar_rect = egui::Rect::from_min_size(
            available_rect.min,
            egui::Vec2::new(width, bar_height)
        );

        // 绘制半透明背景条形图
        ui.painter().rect_filled(
            bar_rect,
            egui::Rounding::same(2.0),
            color.gamma_multiply(0.3) // 使用30%透明度作为背景
        );
    }

    /// 渲染表格行
    fn render_table_row(
        &self,
        row_ui: &mut egui_extras::TableRow,
        row: &UnifiedOrderBookRow,
        current_price: f64,
        max_history_buy: f64,
        max_history_sell: f64,
        max_delta: f64,
    ) {
        // 计算买单和卖单深度的最大值用于条形图缩放
        let max_bid_volume = self.cached_visible_data.iter().map(|r| r.bid_volume).fold(0.0, f64::max);
        let max_ask_volume = self.cached_visible_data.iter().map(|r| r.ask_volume).fold(0.0, f64::max);

        // 计算是否为当前价格行（只有最接近的一行会被标记为当前价格）
        let is_current_price_row = self.is_current_price_row(row.price, current_price);
        // 第1列：主动卖单累计(5s) - 加粗显示
        row_ui.col(|ui| {
            if row.active_sell_volume_5s > 0.0 {
                ui.add(egui::Label::new(egui::RichText::new(format!("{:.4}", row.active_sell_volume_5s))
                    .color(egui::Color32::from_rgb(255, 120, 120))
                    .strong()));
            } else {
                ui.colored_label(egui::Color32::GRAY, "--");
            }
        });

        // 第2列：买单深度 + 背景条形图
        row_ui.col(|ui| {
            if row.bid_volume > 0.0 {
                // 计算条形图宽度
                let bar_width = self.calculate_bar_width(row.bid_volume, max_bid_volume);

                // 使用层叠布局：先绘制背景条形图，再显示文本
                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        if bar_width > 1.0 {
                            // 绘制背景条形图（使用与文本相同的颜色但更透明）
                            self.draw_background_bar(ui, bar_width, egui::Color32::from_rgb(120, 180, 255));
                        }

                        // 重置UI位置到开始处，在条形图上方显示文本
                        ui.allocate_ui_at_rect(ui.max_rect(), |ui| {
                            ui.colored_label(egui::Color32::from_rgb(120, 180, 255), format!("{:.4}", row.bid_volume));
                        });
                    }
                );
            } else {
                ui.colored_label(egui::Color32::GRAY, "--");
            }
        });

        // 第3列：价格 - 精确的当前价格高亮（只有一个价格层级被高亮）
        row_ui.col(|ui| {
            // 格式化价格为整数美元显示（1美元聚合级别）
            let price_display = format!("{:.0}", row.price);

            if is_current_price_row {
                // 当前价格行 - 使用强烈高亮和背景
                ui.scope(|ui| {
                    ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
                    let response = ui.colored_label(egui::Color32::from_rgb(255, 255, 0), price_display);

                    // 添加背景高亮
                    let rect = response.rect;
                    ui.painter().rect_filled(
                        rect.expand(2.0),
                        egui::Rounding::same(3.0),
                        egui::Color32::from_rgb(255, 255, 0).gamma_multiply(0.3)
                    );

                    response.on_hover_text("🎯 当前价格 (1美元聚合级别)");
                });
            } else {
                // 普通价格行 - 白色文本
                ui.colored_label(egui::Color32::WHITE, price_display);
            }
        });

        // 第4列：卖单深度 + 背景条形图
        row_ui.col(|ui| {
            if row.ask_volume > 0.0 {
                // 计算条形图宽度
                let bar_width = self.calculate_bar_width(row.ask_volume, max_ask_volume);

                // 使用层叠布局：先绘制背景条形图，再显示文本
                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        if bar_width > 1.0 {
                            // 绘制背景条形图（使用与文本相同的颜色但更透明）
                            self.draw_background_bar(ui, bar_width, egui::Color32::from_rgb(255, 120, 120));
                        }

                        // 重置UI位置到开始处，在条形图上方显示文本
                        ui.allocate_ui_at_rect(ui.max_rect(), |ui| {
                            ui.colored_label(egui::Color32::from_rgb(255, 120, 120), format!("{:.4}", row.ask_volume));
                        });
                    }
                );
            } else {
                ui.colored_label(egui::Color32::GRAY, "--");
            }
        });

        // 第5列：主动买单累计(5s) - 加粗显示
        row_ui.col(|ui| {
            if row.active_buy_volume_5s > 0.0 {
                ui.add(egui::Label::new(egui::RichText::new(format!("{:.4}", row.active_buy_volume_5s))
                    .color(egui::Color32::from_rgb(120, 255, 120))
                    .strong()));
            } else {
                ui.colored_label(egui::Color32::GRAY, "--");
            }
        });

        // 第6列：历史累计主动买单量 + 条形图
        row_ui.col(|ui| {
            ui.horizontal(|ui| {
                if row.history_buy_volume > 0.0 {
                    ui.colored_label(egui::Color32::from_rgb(120, 255, 120), format!("{:.4}", row.history_buy_volume));

                    // 绘制条形图
                    let bar_width = self.calculate_bar_width(row.history_buy_volume, max_history_buy);
                    if bar_width > 1.0 {
                        self.draw_horizontal_bar(ui, bar_width, egui::Color32::from_rgb(120, 255, 120));
                    }
                } else {
                    ui.colored_label(egui::Color32::GRAY, "--");
                }
            });
        });

        // 第7列：历史累计主动卖单量 + 条形图
        row_ui.col(|ui| {
            ui.horizontal(|ui| {
                if row.history_sell_volume > 0.0 {
                    ui.colored_label(egui::Color32::from_rgb(255, 120, 120), format!("{:.4}", row.history_sell_volume));

                    // 绘制条形图
                    let bar_width = self.calculate_bar_width(row.history_sell_volume, max_history_sell);
                    if bar_width > 1.0 {
                        self.draw_horizontal_bar(ui, bar_width, egui::Color32::from_rgb(255, 120, 120));
                    }
                } else {
                    ui.colored_label(egui::Color32::GRAY, "--");
                }
            });
        });

        // 第8列：主动订单delta + 条形图
        row_ui.col(|ui| {
            ui.horizontal(|ui| {
                if row.delta.abs() > 0.0001 {
                    let color = if row.delta > 0.0 {
                        egui::Color32::from_rgb(120, 255, 120)
                    } else {
                        egui::Color32::from_rgb(255, 120, 120)
                    };
                    ui.colored_label(color, format!("{:+.4}", row.delta));

                    // 绘制条形图
                    let bar_width = self.calculate_bar_width(row.delta.abs(), max_delta);
                    if bar_width > 1.0 {
                        self.draw_horizontal_bar(ui, bar_width, color);
                    }
                } else {
                    ui.colored_label(egui::Color32::GRAY, "--");
                }
            });
        });


    }
}

/// 聚合订单流数据结构（用于1美元级别聚合）
#[derive(Debug, Clone)]
struct AggregatedOrderFlow {
    bid_volume: f64,           // 聚合买单深度
    ask_volume: f64,           // 聚合卖单深度
    active_buy_volume_5s: f64, // 聚合5秒内主动买单累计
    active_sell_volume_5s: f64,// 聚合5秒内主动卖单累计
    history_buy_volume: f64,   // 聚合历史累计主动买单量
    history_sell_volume: f64,  // 聚合历史累计主动卖单量
}

impl AggregatedOrderFlow {
    fn new() -> Self {
        Self {
            bid_volume: 0.0,
            ask_volume: 0.0,
            active_buy_volume_5s: 0.0,
            active_sell_volume_5s: 0.0,
            history_buy_volume: 0.0,
            history_sell_volume: 0.0,
        }
    }
}

/// 统一订单簿行数据结构
#[derive(Debug, Clone)]
struct UnifiedOrderBookRow {
    price: f64,
    bid_volume: f64,           // 买单深度
    ask_volume: f64,           // 卖单深度
    active_buy_volume_5s: f64, // 5秒内主动买单累计
    active_sell_volume_5s: f64,// 5秒内主动卖单累计
    history_buy_volume: f64,   // 历史累计主动买单量
    history_sell_volume: f64,  // 历史累计主动卖单量
    delta: f64,                // 主动订单delta (买单量 - 卖单量)
}

impl UnifiedOrderBookWidget {
    /// 渲染弹出窗口
    fn render_popup_windows(&mut self, ctx: &egui::Context) {
        // 交易信号窗口
        if self.trading_signal_window_open {
            egui::Window::new("交易信号")
                .open(&mut self.trading_signal_window_open)
                .default_size(egui::Vec2::new(600.0, 400.0))
                .resizable(true)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading("交易信号分析");
                        ui.add_space(20.0);
                        ui.label("此功能正在开发中...");
                        ui.add_space(20.0);
                        ui.separator();
                        ui.add_space(10.0);
                        ui.label("未来将包含:");
                        ui.label("• 技术指标信号");
                        ui.label("• 订单流信号");
                        ui.label("• 价格行为信号");
                        ui.label("• 自定义信号策略");
                    });
                });
        }

        // 量化回测窗口
        if self.quantitative_backtest_window_open {
            egui::Window::new("量化回测")
                .open(&mut self.quantitative_backtest_window_open)
                .default_size(egui::Vec2::new(800.0, 600.0))
                .resizable(true)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.heading("量化回测系统");
                        ui.add_space(20.0);
                        ui.label("此功能正在开发中...");
                        ui.add_space(20.0);
                        ui.separator();
                        ui.add_space(10.0);
                        ui.label("未来将包含:");
                        ui.label("• 策略回测引擎");
                        ui.label("• 历史数据分析");
                        ui.label("• 风险评估");
                        ui.label("• 收益率分析");
                        ui.label("• 参数优化");
                        ui.label("• 回测报告生成");
                    });
                });
        }

        // 价格图表模态窗口
        if self.price_chart_modal_open {
            // 克隆价格历史数据以避免借用冲突
            let price_history = self.price_history.clone();
            let max_price_history = self.max_price_history;

            egui::Window::new("📈 BTCUSDT 实时价格图表")
                .open(&mut self.price_chart_modal_open)
                .default_size(egui::Vec2::new(1000.0, 600.0))
                .resizable(true)
                .show(ctx, |ui| {
                    Self::render_price_chart_static(ui, &price_history, max_price_history);
                });
        }
    }

    /// 更新价格历史数据
    fn update_price_history(&mut self, current_price: f64, volume: f64, side: String) {
        // 过滤异常价格值
        if !Self::is_valid_price(current_price) {
            log::warn!("过滤异常价格值: {}", current_price);
            return;
        }

        // 过滤异常成交量值
        if !Self::is_valid_volume(volume) {
            log::warn!("过滤异常成交量值: {}", volume);
            return;
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        // 添加新的价格数据点（包含成交量和交易方向）
        self.price_history.push_back((current_time, current_price, volume, side));

        // 保持最大数据点数量
        if self.price_history.len() > self.max_price_history {
            self.price_history.pop_front();
        }
    }

    /// 渲染价格图表（静态方法）
    fn render_price_chart_static(
        ui: &mut egui::Ui,
        price_history: &std::collections::VecDeque<(f64, f64, f64, String)>,
        max_price_history: usize
    ) {
        ui.vertical(|ui| {
            // 顶部状态栏
            ui.horizontal(|ui| {
                ui.heading("BTCUSDT 实时价格图表");
                ui.separator();

                // 显示数据点数量
                ui.label(format!("数据点: {}/{}", price_history.len(), max_price_history));

                if let Some((_, latest_price, latest_volume, latest_side)) = price_history.back() {
                    ui.separator();
                    ui.label("当前价格:");
                    ui.colored_label(egui::Color32::YELLOW, format!("{:.2}", latest_price));
                    ui.separator();
                    ui.label("最新成交量:");
                    ui.colored_label(egui::Color32::LIGHT_BLUE, format!("{:.4}", latest_volume));
                    ui.separator();
                    ui.label("交易方向:");
                    let side_color = if latest_side == "buy" {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::RED
                    };
                    ui.colored_label(side_color, latest_side);
                }
            });

            ui.separator();

            // 主图表区域
            if price_history.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("等待价格数据...");
                });
            } else {
                // 过滤有效的价格历史数据
                let valid_data: Vec<(usize, (f64, f64, f64, String))> = price_history
                    .iter()
                    .enumerate()
                    .filter(|(_, (_, price, volume, _))| {
                        Self::is_valid_price(*price) && Self::is_valid_volume(*volume)
                    })
                    .map(|(i, data)| (i, data.clone()))
                    .collect();

                if valid_data.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label("暂无有效价格数据...");
                    });
                    return;
                }

                // 准备图表数据 - 使用过滤后的有效数据
                let points: PlotPoints = valid_data
                    .iter()
                    .map(|(i, (_, price, _, _))| [*i as f64, *price])
                    .collect();

                // 计算Y轴范围 - 使用过滤后的有效价格
                let prices: Vec<f64> = valid_data.iter().map(|(_, (_, price, _, _))| *price).collect();

                // 计算成交量范围用于圆点大小缩放 - 使用过滤后的有效成交量
                let volumes: Vec<f64> = valid_data.iter().map(|(_, (_, _, volume, _))| *volume).collect();
                let max_volume = volumes.iter().fold(0.0f64, |a, &b| a.max(b));
                let min_volume = volumes.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let volume_range = max_volume - min_volume;
                let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let price_range = max_price - min_price;
                let y_margin = price_range * 0.05; // 5% 边距
                let y_min = min_price - y_margin;
                let y_max = max_price + y_margin;

                // 创建图表 - 添加固定1美元Y轴刻度
                let plot = Plot::new("price_chart_modal")
                    .view_aspect(2.0)
                    .show_axes([true, true])
                    .show_grid([true, true]) // 启用网格显示
                    .allow_zoom(true)
                    .allow_drag(true)
                    .allow_scroll(true)
                    .include_x(0.0)
                    .include_x(price_history.len() as f64)
                    .include_y(y_min)
                    .include_y(y_max)
                    .y_grid_spacer(Self::price_grid_spacer_1_dollar) // 设置1美元固定间距
                    .y_axis_formatter(|y, _range, _ctx| {
                        format!("{:.0}", y.value) // 格式化Y轴为整数
                    });

                plot.show(ui, |plot_ui| {
                    // 绘制价格线
                    let line = Line::new(points)
                        .color(egui::Color32::from_rgb(0, 150, 255))
                        .width(2.0)
                        .name("BTCUSDT价格");

                    plot_ui.line(line);

                    // 绘制基于成交量的圆点 - 使用过滤后的有效数据，只有成交量>=1时才绘制
                    for (i, (_, price, volume, side)) in valid_data.iter() {
                        // 只有成交量大于等于1时才绘制圆点
                        if *volume >= 0.01 {
                            // 计算圆点半径（基于成交量）
                            let radius = if volume_range > 0.0 {
                                let normalized_volume = (volume - min_volume) / volume_range;
                                (2.0 + normalized_volume * 8.0) as f32 // 半径范围：2.0 到 10.0，转换为f32
                            } else {
                                3.0f32 // 默认半径
                            };

                            // 根据买单/卖单选择颜色
                            let color = if side == "buy" {
                                egui::Color32::GREEN // 买单：绿色
                            } else if side == "sell" {
                                egui::Color32::RED // 卖单：红色
                            } else {
                                egui::Color32::GRAY // 未知：灰色
                            };

                            plot_ui.points(
                                egui_plot::Points::new(vec![[*i as f64, *price]])
                                    .color(color)
                                    .radius(radius)
                                    .name(&format!("{}: {:.4}", if side == "buy" { "买单" } else { "卖单" }, volume))
                            );
                        }
                    }

                    // 添加当前价格的高亮标记 - 使用过滤后的有效数据
                    if let Some((i, (_, current_price, _, _))) = valid_data.last() {
                        // 绘制当前价格点
                        plot_ui.points(
                            egui_plot::Points::new(vec![[*i as f64, *current_price]])
                                .color(egui::Color32::YELLOW)
                                .radius(8.0)
                                .name("当前价格")
                        );
                    }
                });
            }
        });
    }

    /// Y轴价格网格间距器 - 固定1美元间距，强制显示刻度
    fn price_grid_spacer_1_dollar(input: egui_plot::GridInput) -> Vec<egui_plot::GridMark> {
        let mut marks = Vec::new();

        // 强制固定1美元间距，不管数据点多少
        let step_size = 1.0;

        // 计算起始和结束的价格标记，向下和向上取整到1美元的倍数
        let start_price = input.bounds.0.floor() as i64;
        let end_price = input.bounds.1.ceil() as i64;

        // 调试信息：打印Y轴边界和刻度范围
        log::info!("Y轴刻度生成: bounds=({:.2}, {:.2}), start_price={}, end_price={}",
            input.bounds.0, input.bounds.1, start_price, end_price);

        // 限制刻度数量以避免过多刻度导致显示问题
        let max_marks = 50usize; // 最多50个刻度
        let price_range = end_price - start_price;
        let step = if price_range > max_marks as i64 {
            (price_range / max_marks as i64).max(1) // 如果范围太大，增加步长
        } else {
            1 // 否则保持1美元间距
        };

        // 生成网格标记
        let mut price = start_price;
        while price <= end_price && marks.len() < max_marks {
            let value = price as f64;
            if value >= input.bounds.0 && value <= input.bounds.1 {
                marks.push(egui_plot::GridMark {
                    value,
                    step_size: step as f64,
                });
            }
            price += step; // 按计算的步长增加
        }

        log::info!("Y轴刻度生成完成: 生成了{}个刻度标记，步长={}", marks.len(), step);
        marks
    }

    /// 验证价格是否有效
    fn is_valid_price(price: f64) -> bool {
        // 过滤异常价格值
        price > 0.0 &&                    // 价格必须大于0
        price.is_finite() &&              // 价格必须是有限数
        !price.is_nan() &&                // 价格不能是NaN
        price < 1_000_000.0 &&            // 价格不能过大（100万美元以下）
        price > 0.01                      // 价格不能过小（1分以上）
    }

    /// 验证成交量是否有效
    fn is_valid_volume(volume: f64) -> bool {
        // 过滤异常成交量值
        volume >= 0.0 &&                  // 成交量必须非负
        volume.is_finite() &&             // 成交量必须是有限数
        !volume.is_nan() &&               // 成交量不能是NaN
        volume < 1_000_000.0              // 成交量不能过大（100万以下）
    }

    /// 渲染嵌入式实时价格图表（在预留区域上半部分）
    fn render_embedded_price_chart(&mut self, ui: &mut egui::Ui, app: &crate::app::reactive_app::ReactiveApp) {
        // 添加标题
        // ui.horizontal(|ui| {
        //     ui.label(egui::RichText::new("📈 实时价格图表").size(14.0).strong());
        // });
        // ui.separator();

        // 更新价格历史数据
        if let Some(current_price) = app.get_market_snapshot().current_price {
            // 从最新的交易数据中获取成交量和交易方向
            let order_flows = app.get_orderbook_manager().get_order_flows();
            if let Some((_, order_flow)) = order_flows.iter().find(|(price, _)| {
                (price.into_inner() - current_price).abs() < 0.5 // 找到最接近当前价格的订单流
            }) {
                let recent_trades = &order_flow.realtime_trade_record;
                if recent_trades.buy_volume > 0.0 || recent_trades.sell_volume > 0.0 {
                    let (volume, side) = if recent_trades.buy_volume >= recent_trades.sell_volume {
                        (recent_trades.buy_volume, "buy".to_string())
                    } else {
                        (recent_trades.sell_volume, "sell".to_string())
                    };
                    self.update_price_history(current_price, volume, side);
                }
            }
        }

        let price_history = &self.price_history;

        if price_history.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("等待价格数据...");
            });
        } else {
            // 过滤有效的价格历史数据
            let valid_data: Vec<(usize, (f64, f64, f64, String))> = price_history
                .iter()
                .enumerate()
                .filter(|(_, (_, price, volume, _))| {
                    Self::is_valid_price(*price) && Self::is_valid_volume(*volume)
                })
                .map(|(i, data)| (i, data.clone()))
                .collect();

            if valid_data.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("暂无有效价格数据...");
                });
                return;
            }

            // 准备图表数据 - 使用过滤后的有效数据
            let points: PlotPoints = valid_data
                .iter()
                .map(|(i, (_, price, _, _))| [*i as f64, *price])
                .collect();

            // 计算Y轴范围 - 使用过滤后的有效价格
            let prices: Vec<f64> = valid_data.iter().map(|(_, (_, price, _, _))| *price).collect();

            // 计算成交量范围用于圆点大小缩放 - 使用过滤后的有效成交量
            let volumes: Vec<f64> = valid_data.iter().map(|(_, (_, _, volume, _))| *volume).collect();
            let max_volume = volumes.iter().fold(0.0f64, |a, &b| a.max(b));
            let min_volume = volumes.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let volume_range = max_volume - min_volume;

            let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let y_min = min_price - 5.0;
            let y_max = max_price + 5.0;

            // 调试信息：打印Y轴范围
            log::info!("嵌入式图表 Y轴范围: min_price={:.2}, max_price={:.2}, y_min={:.2}, y_max={:.2}, 数据点数={}",
                min_price, max_price, y_min, y_max, prices.len());

            // 获取可用的UI区域高度，确保图表严格遵守高度限制
            let available_height = ui.available_height();
            let chart_height = self.price_chart_height.min(available_height);

            // 使用固定高度的容器来限制图表大小
            ui.allocate_ui_with_layout(
                egui::Vec2::new(ui.available_width(), chart_height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // 创建嵌入式图表 - 移除view_aspect以避免高度冲突，添加固定1美元Y轴刻度，移除margin
                    // 设置固定的X轴显示窗口，只显示最近的1000个数据点，防止数据增多时图表缩小
                    let display_window_size = 1000.0; // 固定显示窗口大小
                    let data_len = valid_data.len() as f64;
                    let x_min = if data_len > display_window_size {
                        data_len - display_window_size // 显示最近的1000个点
                    } else {
                        0.0 // 如果数据不足1000个，从0开始显示
                    };
                    let x_max = data_len.max(display_window_size); // 确保X轴范围至少为1000

                    let plot = Plot::new("embedded_price_chart")
                        .width(ui.available_width()) // 明确设置图表宽度占满可用宽度
                        .height(chart_height) // 明确设置图表高度
                        .show_axes([true, true])
                        .show_grid([true, true]) // 启用网格显示
                        .allow_zoom(true) // 重新启用缩放
                        .allow_drag(true) // 重新启用拖拽
                        .allow_scroll(true) // 重新启用滚动
                        .include_x(x_min) // 使用固定窗口的起始位置
                        .include_x(x_max) // 使用固定窗口的结束位置
                        .include_y(y_min)
                        .include_y(y_max)
                        .y_axis_formatter(|y, _range, _ctx| {
                            format!("{:.0}", y.value) // 格式化Y轴为整数
                        });

                    plot.show(ui, |plot_ui| {
                        // 绘制价格线
                        plot_ui.line(
                            egui_plot::Line::new(points)
                                .color(egui::Color32::WHITE)
                                .width(1.5)
                                .name("价格")
                        );

                        // 绘制基于成交量的圆点 - 使用过滤后的有效数据，只有成交量>=1时才绘制
                        for (i, (_, price, volume, side)) in valid_data.iter() {
                            // 只有成交量大于等于1时才绘制圆点
                            if *volume >= 0.01 {
                                // 计算圆点半径（基于成交量）
                                let radius = if volume_range > 0.0 {
                                    let normalized_volume = (volume - min_volume) / volume_range;
                                    (2.0 + normalized_volume * 8.0) as f32 // 半径范围：2.0 到 10.0，转换为f32
                                } else {
                                    3.0f32 // 默认半径
                                };

                                // 根据买单/卖单选择颜色
                                let color = if side == "buy" {
                                    egui::Color32::GREEN // 买单：绿色
                                } else if side == "sell" {
                                    egui::Color32::RED // 卖单：红色
                                } else {
                                    egui::Color32::GRAY // 未知：灰色
                                };

                                plot_ui.points(
                                    egui_plot::Points::new(vec![[*i as f64, *price]])
                                        .color(color)
                                        .radius(radius)
                                        .name(&format!("{}: {:.4}", if side == "buy" { "买单" } else { "卖单" }, volume))
                                );
                            }
                        }

                        // 添加当前价格的高亮标记 - 使用过滤后的有效数据
                        if let Some((i, (_, current_price, _, _))) = valid_data.last() {
                            // 绘制当前价格点
                            plot_ui.points(
                                egui_plot::Points::new(vec![[*i as f64, *current_price]])
                                    .color(egui::Color32::YELLOW)
                                    .radius(8.0)
                                    .name("当前价格")
                            );
                        }
                    });
                },
            );
        }
    }

    /// 渲染Orderbook Imbalance指标
    fn render_orderbook_imbalance(&mut self, ui: &mut egui::Ui, app: &crate::app::reactive_app::ReactiveApp) {
        // 获取市场快照数据
        let snapshot = app.get_market_snapshot();
        let bid_ratio = snapshot.bid_volume_ratio;
        let ask_ratio = snapshot.ask_volume_ratio;

        // 创建带边框的面板 - 移除左边距以与价格图表左对齐
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(25, 25, 35))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 80)))
            .inner_margin(egui::Margin {
                left: 0.0,    // 移除左边距
                right: 8.0,   // 保持右边距
                top: 8.0,     // 保持上边距
                bottom: 8.0,  // 保持下边距
            })
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // 标题
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::WHITE, "📊 Orderbook Imbalance");
                    });

                    ui.add_space(5.0);

                    // 显示比率数值
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(120, 180, 255),
                            format!("买单: {:.1}%", bid_ratio * 100.0));
                        ui.separator();
                        ui.colored_label(egui::Color32::from_rgb(255, 120, 120),
                            format!("卖单: {:.1}%", ask_ratio * 100.0));
                    });

                    ui.add_space(8.0);

                    // 绘制横向条形图
                    let available_width = ui.available_width() - 20.0; // 留出边距
                    let bar_height = 20.0;

                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(available_width, bar_height),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let rect = ui.available_rect_before_wrap();

                            // 计算买单和卖单条形图的宽度
                            let bid_width = available_width * bid_ratio as f32;
                            let ask_width = available_width * ask_ratio as f32;

                            // 绘制买单条形图（蓝色，从左边开始）
                            if bid_width > 1.0 {
                                let bid_rect = egui::Rect::from_min_size(
                                    rect.min,
                                    egui::Vec2::new(bid_width, bar_height)
                                );
                                ui.painter().rect_filled(bid_rect, 2.0, egui::Color32::from_rgb(120, 180, 255));
                            }

                            // 绘制卖单条形图（红色，从右边开始）
                            if ask_width > 1.0 {
                                let ask_rect = egui::Rect::from_min_size(
                                    egui::Pos2::new(rect.max.x - ask_width, rect.min.y),
                                    egui::Vec2::new(ask_width, bar_height)
                                );
                                ui.painter().rect_filled(ask_rect, 2.0, egui::Color32::from_rgb(255, 120, 120));
                            }

                            // 绘制中心分割线
                            let center_x = rect.min.x + available_width * 0.5;
                            ui.painter().line_segment(
                                [egui::Pos2::new(center_x, rect.min.y), egui::Pos2::new(center_x, rect.max.y)],
                                egui::Stroke::new(1.0, egui::Color32::WHITE)
                            );

                            // 占用整个区域以防止其他元素覆盖
                            ui.allocate_rect(rect, egui::Sense::hover());
                        }
                    );

                    ui.add_space(5.0);

                    // 显示多空压力指示
                    let imbalance = bid_ratio - ask_ratio;
                    let pressure_text = if imbalance > 0.1 {
                        ("🟢 多头压力", egui::Color32::from_rgb(120, 255, 120))
                    } else if imbalance < -0.1 {
                        ("🔴 空头压力", egui::Color32::from_rgb(255, 120, 120))
                    } else {
                        ("⚪ 均衡状态", egui::Color32::GRAY)
                    };

                    ui.horizontal(|ui| {
                        ui.colored_label(pressure_text.1, pressure_text.0);
                        ui.colored_label(egui::Color32::GRAY,
                            format!("(差值: {:.1}%)", imbalance * 100.0));
                    });
                });
            });
    }

    /// 渲染Trade Imbalance指标 - 基于500ms滑动窗口的tick trade数据
    fn render_trade_imbalance(&mut self, ui: &mut egui::Ui, app: &crate::app::reactive_app::ReactiveApp) {
        // 获取Trade Imbalance数据 - 从orderbook manager获取实时计算的TI值
        let trade_imbalance = app.get_orderbook_manager().get_trade_imbalance();

        // 计算买单和卖单的比例（用于可视化显示）
        let buy_ratio = if trade_imbalance >= 0.0 {
            (trade_imbalance + 1.0) / 2.0 // 将[-1,1]映射到[0,1]，正值时买单比例更高
        } else {
            0.5 + trade_imbalance / 2.0 // 负值时买单比例较低
        };
        let sell_ratio = 1.0 - buy_ratio;

        // 创建带边框的面板 - 移除左边距以与上方组件左对齐
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(25, 25, 35))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 80)))
            .inner_margin(egui::Margin {
                left: 0.0,    // 移除左边距
                right: 8.0,   // 保持右边距
                top: 8.0,     // 保持上边距
                bottom: 8.0,  // 保持下边距
            })
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // 标题
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::WHITE, "⚖️ Trade Imbalance (500ms滑动窗口)");
                    });

                    ui.add_space(5.0);

                    // 显示TI数值和买卖比例
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::LIGHT_BLUE,
                            format!("TI: {:.3}", trade_imbalance));
                        ui.separator();
                        ui.colored_label(egui::Color32::from_rgb(120, 255, 120),
                            format!("买单: {:.1}%", buy_ratio * 100.0));
                        ui.separator();
                        ui.colored_label(egui::Color32::from_rgb(255, 120, 120),
                            format!("卖单: {:.1}%", sell_ratio * 100.0));
                    });

                    ui.add_space(8.0);

                    // 绘制横向条形图
                    let available_width = ui.available_width() - 20.0; // 留出边距
                    let bar_height = 20.0;

                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(available_width, bar_height),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let rect = ui.available_rect_before_wrap();

                            // 计算买单和卖单条形图的宽度 - 与Orderbook Imbalance显示效果一致
                            let buy_width = available_width * buy_ratio as f32;
                            let sell_width = available_width * sell_ratio as f32;

                            // 绘制买单条形图（绿色，从左边开始）
                            if buy_width > 1.0 {
                                let buy_rect = egui::Rect::from_min_size(
                                    rect.min,
                                    egui::Vec2::new(buy_width, bar_height)
                                );
                                ui.painter().rect_filled(buy_rect, 2.0, egui::Color32::from_rgb(120, 255, 120));
                            }

                            // 绘制卖单条形图（红色，从右边开始）
                            if sell_width > 1.0 {
                                let sell_rect = egui::Rect::from_min_size(
                                    egui::Pos2::new(rect.max.x - sell_width, rect.min.y),
                                    egui::Vec2::new(sell_width, bar_height)
                                );
                                ui.painter().rect_filled(sell_rect, 2.0, egui::Color32::from_rgb(255, 120, 120));
                            }

                            // 绘制中心分割线
                            let center_x = rect.min.x + available_width * 0.5;
                            ui.painter().line_segment(
                                [egui::Pos2::new(center_x, rect.min.y), egui::Pos2::new(center_x, rect.max.y)],
                                egui::Stroke::new(1.0, egui::Color32::WHITE)
                            );

                            // 占用整个区域以防止其他元素覆盖
                            ui.allocate_rect(rect, egui::Sense::hover());
                        }
                    );

                    ui.add_space(5.0);

                    // 显示交易压力指示 - 基于买卖比例差值
                    let imbalance = buy_ratio - sell_ratio; // 计算不平衡程度
                    let (pressure_text, pressure_color) = if imbalance > 0.3 {
                        ("🟢 强买压", egui::Color32::from_rgb(120, 255, 120))
                    } else if imbalance > 0.1 {
                        ("🟡 轻买压", egui::Color32::from_rgb(255, 255, 120))
                    } else if imbalance < -0.3 {
                        ("🔴 强卖压", egui::Color32::from_rgb(255, 120, 120))
                    } else if imbalance < -0.1 {
                        ("🟠 轻卖压", egui::Color32::from_rgb(255, 180, 120))
                    } else {
                        ("⚪ 均衡", egui::Color32::GRAY)
                    };

                    ui.horizontal(|ui| {
                        ui.colored_label(pressure_color, pressure_text);
                        ui.colored_label(egui::Color32::GRAY,
                            format!("(差值: {:.1}%)", imbalance * 100.0));
                    });
                });
            });
    }
}
