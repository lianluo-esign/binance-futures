use eframe::egui;
use crate::orderbook::OrderFlow;
use crate::app::ReactiveApp;
use std::collections::BTreeMap;
use ordered_float::OrderedFloat;
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::Path;

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
        }
    }
}

impl UnifiedOrderBookWidget {
    pub fn new() -> Self {
        Self::default()
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

    /// 渲染统一订单簿组件 - 固定比例布局（5% 标题 + 95% 表格）
    pub fn show(&mut self, ui: &mut egui::Ui, app: &ReactiveApp) {
        // 加载Logo（如果还未加载）
        self.load_logo(ui.ctx());

        // 获取总可用空间
        let total_rect = ui.available_rect_before_wrap();
        let total_height = total_rect.height();
        let total_width = total_rect.width();

        // 计算固定比例尺寸
        let header_height = total_height * 0.05; // 5% 用于标题
        let table_height = total_height * 0.95;  // 95% 用于表格

        ui.vertical(|ui| {
            // 1. 顶部固定区域：5% 高度用于标题和当前价格信息
            ui.allocate_ui_with_layout(
                egui::Vec2::new(total_width, header_height),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.horizontal(|ui| {
                        // Logo显示
                        // self.render_logo(ui, header_height);

                        // ui.heading("订单流分析");
                        // ui.separator();

                        // 显示当前价格
                        // let snapshot = app.get_market_snapshot();
                        // if let Some(current_price) = snapshot.current_price {
                        //     ui.label("当前价格:");
                        //     ui.colored_label(egui::Color32::YELLOW, format!("{:.2}", current_price));
                        // }
                    });
                },
            );

            // 2. 底部表格区域：95% 高度，严格边界控制
            ui.allocate_ui_with_layout(
                egui::Vec2::new(total_width, table_height),
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
                            ui.label("暂无数据");
                        });
                    } else {
                        // 渲染表格，严格限制在95%区域内
                        self.render_bounded_table(ui, &visible_data, current_price, table_height);
                    }
                },
            );
        });
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
        let mut all_price_levels: Vec<i64> = order_flows
            .keys()
            .map(|k| k.0.floor() as i64) // 使用向下取整聚合到1美元级别，转换为整数
            .collect::<std::collections::HashSet<_>>() // 去重
            .into_iter()
            .collect();

        // 转换回f64用于后续处理
        let mut all_price_levels: Vec<f64> = all_price_levels
            .into_iter()
            .map(|level| level as f64)
            .collect();
        all_price_levels.sort_by(|a, b| b.partial_cmp(a).unwrap()); // 从高到低排序

        // 找到当前价格对应的聚合级别在排序列表中的位置
        let current_price_level = current_price.floor();
        let current_price_index = all_price_levels
            .iter()
            .position(|&price_level| price_level <= current_price_level)
            .unwrap_or(all_price_levels.len() / 2);

        // 计算可见范围：当前价格上下各40个美元级别
        let start_index = current_price_index.saturating_sub(self.visible_price_levels);
        let end_index = std::cmp::min(
            current_price_index + self.visible_price_levels + 1,
            all_price_levels.len()
        );

        // 提取可见范围内的聚合价格级别
        let visible_price_levels = &all_price_levels[start_index..end_index];

        // 为每个聚合价格级别收集所有相关的原始价格
        let mut visible_prices = Vec::new();
        for &price_level in visible_price_levels {
            // 找到属于这个聚合级别的所有原始价格
            for price_key in order_flows.keys() {
                let original_price = price_key.0;
                if original_price.floor() == price_level {
                    visible_prices.push(original_price);
                }
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
                total_volume: aggregated_flow.history_buy_volume + aggregated_flow.history_sell_volume,
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

            // 获取该价格的订单流数据
            if let Some(order_flow) = order_flows.get(&price_key) {
                let entry = aggregated_map.entry(price_level_int).or_insert_with(|| AggregatedOrderFlow::new());

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
        let max_total = data.iter().map(|row| row.total_volume).fold(0.0, f64::max);

        // 获取可用宽度并平均分配给9列
        let available_width = ui.available_width();
        let column_width = available_width / 9.0;

        // 使用严格边界控制的表格容器
        ui.allocate_ui_with_layout(
            egui::Vec2::new(available_width, table_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                // 设置剪切区域，确保内容不会溢出95%边界
                ui.set_clip_rect(ui.available_rect_before_wrap());

                let table = TableBuilder::new(ui)
                    .striped(true)
                    .resizable(false) // 禁用调整大小以保持均匀分布
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::exact(column_width)) // 主动卖单累计(5s)
                    .column(Column::exact(column_width)) // 买单深度
                    .column(Column::exact(column_width)) // 价格
                    .column(Column::exact(column_width)) // 卖单深度
                    .column(Column::exact(column_width)) // 主动买单累计(5s)
                    .column(Column::exact(column_width)) // 历史累计主动买单量
                    .column(Column::exact(column_width)) // 历史累计主动卖单量
                    .column(Column::exact(column_width)) // 主动订单delta
                    .column(Column::remainder()) // 主动订单总量 - 使用剩余空间
                    .max_scroll_height(table_height - 30.0) // 为表头预留空间
                    .scroll_to_row(self.calculate_center_row_index(data, current_price), None);

                table
                    .header(25.0, |mut header| {
                        header.col(|ui| { ui.strong("主动卖单累计(5s)"); });
                        header.col(|ui| { ui.strong("买单深度"); });
                        header.col(|ui| { ui.strong("价格"); });
                        header.col(|ui| { ui.strong("卖单深度"); });
                        header.col(|ui| { ui.strong("主动买单累计(5s)"); });
                        header.col(|ui| { ui.strong("历史累计买单"); });
                        header.col(|ui| { ui.strong("历史累计卖单"); });
                        header.col(|ui| { ui.strong("Delta"); });
                        header.col(|ui| { ui.strong("总量"); });
                    })
                    .body(|mut body| {
                        // 渲染所有可见数据行（最多81行）
                        for row in data {
                            body.row(25.0, |mut row_ui| {
                                self.render_table_row(&mut row_ui, row, current_price, max_history_buy, max_history_sell, max_delta, max_total);
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
        let max_total = data.iter().map(|row| row.total_volume).fold(0.0, f64::max);
        let max_bid_volume = data.iter().map(|row| row.bid_volume).fold(0.0, f64::max);
        let max_ask_volume = data.iter().map(|row| row.ask_volume).fold(0.0, f64::max);

        ui.push_id("unified_orderbook_table", |ui| {
            // 获取可用宽度并平均分配给9列
            let available_width = ui.available_width();
            let column_width = available_width / 9.0;

            let table = TableBuilder::new(ui)
                .striped(true)
                .resizable(false) // 禁用调整大小以保持均匀分布
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(column_width)) // 主动卖单累计(5s)
                .column(Column::exact(column_width)) // 买单深度
                .column(Column::exact(column_width)) // 价格
                .column(Column::exact(column_width)) // 卖单深度
                .column(Column::exact(column_width)) // 主动买单累计(5s)
                .column(Column::exact(column_width)) // 历史累计主动买单量
                .column(Column::exact(column_width)) // 历史累计主动卖单量
                .column(Column::exact(column_width)) // 主动订单delta
                .column(Column::remainder()) // 主动订单总量 - 使用剩余空间
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
                    header.col(|ui| {
                        ui.strong("总量");
                    });
                })
                .body(|mut body| {
                    // 渲染所有数据行
                    for row in data {
                        body.row(25.0, |mut row_ui| {
                            self.render_table_row(&mut row_ui, row, current_price, max_history_buy, max_history_sell, max_delta, max_total);
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
        let max_total = data.iter().map(|row| row.total_volume).fold(0.0, f64::max);
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
            // 获取可用宽度并平均分配给9列
            let available_width = ui.available_width();
            let column_width = available_width / 9.0;

            let table = TableBuilder::new(ui)
                .striped(true)
                .resizable(false) // 禁用调整大小以保持均匀分布
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(column_width)) // 主动卖单累计(5s)
                .column(Column::exact(column_width)) // 买单深度
                .column(Column::exact(column_width)) // 价格
                .column(Column::exact(column_width)) // 卖单深度
                .column(Column::exact(column_width)) // 主动买单累计(5s)
                .column(Column::exact(column_width)) // 历史累计主动买单量
                .column(Column::exact(column_width)) // 历史累计主动卖单量
                .column(Column::exact(column_width)) // 主动订单delta
                .column(Column::remainder()) // 主动订单总量 - 使用剩余空间
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
                    header.col(|ui| {
                        ui.strong("总量");
                    });
                })
                .body(|mut body| {
                    // 渲染所有数据行，表格内置滚动会自动处理
                    for row in data {
                        body.row(25.0, |mut row_ui| {
                            self.render_table_row(&mut row_ui, row, current_price, max_history_buy, max_history_sell, max_delta, max_total);
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
        let max_total = data.iter().map(|row| row.total_volume).fold(0.0, f64::max);
        let max_bid_volume = data.iter().map(|row| row.bid_volume).fold(0.0, f64::max);
        let max_ask_volume = data.iter().map(|row| row.ask_volume).fold(0.0, f64::max);

        ui.push_id("unified_orderbook_table", |ui| {
            // 获取可用宽度并平均分配给9列
            let available_width = ui.available_width();
            let column_width = available_width / 9.0;

            let table = TableBuilder::new(ui)
                .striped(true)
                .resizable(false) // 禁用调整大小以保持均匀分布
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(column_width)) // 主动卖单累计(5s)
                .column(Column::exact(column_width)) // 买单深度
                .column(Column::exact(column_width)) // 价格
                .column(Column::exact(column_width)) // 卖单深度
                .column(Column::exact(column_width)) // 主动买单累计(5s)
                .column(Column::exact(column_width)) // 历史累计主动买单量
                .column(Column::exact(column_width)) // 历史累计主动卖单量
                .column(Column::exact(column_width)) // 主动订单delta
                .column(Column::remainder()) // 主动订单总量 - 使用剩余空间
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
                    header.col(|ui| {
                        ui.strong("总量");
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

                            // 第9列：主动订单总量 + 条形图
                            row_ui.col(|ui| {
                                ui.horizontal(|ui| {
                                    if row.total_volume > 0.0 {
                                        ui.colored_label(egui::Color32::from_rgb(200, 200, 200), format!("{:.4}", row.total_volume));

                                        // 绘制条形图
                                        let bar_width = self.calculate_bar_width(row.total_volume, max_total);
                                        if bar_width > 1.0 {
                                            self.draw_horizontal_bar(ui, bar_width, egui::Color32::from_rgb(150, 150, 150));
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
        max_total: f64,
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

        // 第9列：主动订单总量 + 条形图
        row_ui.col(|ui| {
            ui.horizontal(|ui| {
                if row.total_volume > 0.0 {
                    ui.colored_label(egui::Color32::from_rgb(200, 200, 200), format!("{:.4}", row.total_volume));

                    // 绘制条形图
                    let bar_width = self.calculate_bar_width(row.total_volume, max_total);
                    if bar_width > 1.0 {
                        self.draw_horizontal_bar(ui, bar_width, egui::Color32::from_rgb(150, 150, 150));
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
    total_volume: f64,         // 主动订单总量 (买单量 + 卖单量)
}
