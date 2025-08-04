use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};
use std::collections::BTreeMap;
use ordered_float::OrderedFloat;

use crate::app::ReactiveApp;
use crate::orderbook::{OrderFlow, MarketSnapshot};
use super::bar_chart::BarChartRenderer;
use super::price_tracker::PriceTracker;
use super::layout_manager::LayoutManager;

/// 订单薄渲染器 - 负责订单薄的主要渲染逻辑
pub struct OrderBookRenderer {
    bar_chart: BarChartRenderer,
    price_tracker: PriceTracker,
    layout_manager: LayoutManager,
}

/// 订单薄渲染数据结构
#[derive(Debug, Clone)]
pub struct OrderBookRenderData {
    pub price_levels: Vec<PriceLevel>,
    pub best_bid_price: Option<f64>,
    pub best_ask_price: Option<f64>,
    pub current_trade_price: Option<f64>,
    pub max_bid_volume: f64,
    pub max_ask_volume: f64,
    pub visible_range: (usize, usize),
}

/// 价格层级数据结构
#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: f64,
    pub bid_volume: f64,
    pub ask_volume: f64,
    pub is_best_bid: bool,
    pub is_best_ask: bool,
    pub has_recent_trade: bool,
    pub trade_side: Option<String>,
}

impl OrderBookRenderer {
    /// 创建新的订单薄渲染器
    pub fn new() -> Self {
        Self {
            bar_chart: BarChartRenderer::new(),
            price_tracker: PriceTracker::new(),
            layout_manager: LayoutManager::new(),
        }
    }

    /// 主渲染方法
    pub fn render(&mut self, f: &mut Frame, app: &ReactiveApp, area: Rect) {
        // 获取应用统计信息
        let stats = app.get_stats();
        let connection_status = if stats.websocket_connected {
            "已连接"
        } else {
            "断开连接"
        };

        // 获取缓冲区使用情况
        let (current_buffer_size, max_buffer_capacity) = app.get_buffer_usage();

        // 获取价格跟踪状态
        let price_tracking_status = if app.is_price_tracking_enabled() { "跟踪" } else { "关闭" };
        let auto_center_status = if app.is_auto_center_enabled() { "居中" } else { "关闭" };

        let title = format!(
            "Binance Futures Order Book - {} | 缓冲区: {}/{} | 事件/秒: {:.1} | 状态: {} | 价格跟踪: {} | 自动居中: {}",
            app.get_symbol(), 
            current_buffer_size, 
            max_buffer_capacity, 
            stats.events_processed_per_second, 
            connection_status, 
            price_tracking_status, 
            auto_center_status
        );

        // 创建边框块
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL);

        // 计算内部区域
        let inner_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        // 准备渲染数据
        let render_data = self.prepare_render_data(app);

        // 更新价格跟踪器 - 使用最准确的参考价格
        let reference_price = render_data.current_trade_price
            .or(render_data.best_bid_price)
            .or(render_data.best_ask_price);
        self.price_tracker.update_tracking(reference_price);

        // 计算可见范围
        let visible_rows = inner_area.height.saturating_sub(1) as usize; // 减去表头
        let visible_range = self.calculate_visible_range(&render_data, visible_rows);

        // 渲染订单薄表格
        self.render_orderbook_table(f, inner_area, &render_data, visible_range, block);
    }

    /// 准备渲染数据
    fn prepare_render_data(&self, app: &ReactiveApp) -> OrderBookRenderData {
        let snapshot = app.get_market_snapshot();
        let order_flows = app.get_orderbook_manager().get_order_flows();
        let price_precision = app.get_price_precision();

        // 应用价格精度聚合，处理bid/ask冲突
        let aggregated_order_flows = self.aggregate_price_levels(
            &order_flows, 
            price_precision,
            snapshot.best_bid_price,
            snapshot.best_ask_price
        );

        // 构建价格层级列表
        let mut price_levels: Vec<_> = aggregated_order_flows.keys().collect();
        price_levels.sort_by(|a, b| b.cmp(a)); // 从高价到低价排序

        let max_levels = app.get_max_visible_rows().min(price_levels.len());
        let mut levels = Vec::new();
        let mut max_bid_volume = 0.0;
        let mut max_ask_volume = 0.0;

        for &price_key in price_levels.iter().take(max_levels) {
            let price = price_key.0;
            let order_flow = &aggregated_order_flows[price_key];

            let bid_volume = order_flow.bid_ask.bid;
            let ask_volume = order_flow.bid_ask.ask;

            // 更新最大音量
            if bid_volume > max_bid_volume {
                max_bid_volume = bid_volume;
            }
            if ask_volume > max_ask_volume {
                max_ask_volume = ask_volume;
            }

            // 判断是否为最优价格
            let is_best_bid = snapshot.best_bid_price.map_or(false, |best| (price - best).abs() < 0.001);
            let is_best_ask = snapshot.best_ask_price.map_or(false, |best| (price - best).abs() < 0.001);

            // 移除交易高亮显示逻辑
            levels.push(PriceLevel {
                price,
                bid_volume,
                ask_volume,
                is_best_bid,
                is_best_ask,
                has_recent_trade: false,  // 禁用交易高亮
                trade_side: None,         // 不显示交易方向
            });
        }

        OrderBookRenderData {
            price_levels: levels,
            best_bid_price: snapshot.best_bid_price,
            best_ask_price: snapshot.best_ask_price,
            current_trade_price: snapshot.current_price,
            max_bid_volume,
            max_ask_volume,
            visible_range: (0, 0), // 将在后续计算
        }
    }

    /// 计算可见范围 - 修复价格下跌时的居中问题
    fn calculate_visible_range(&mut self, render_data: &OrderBookRenderData, visible_rows: usize) -> (usize, usize) {
        // 优先使用当前交易价格，如果没有则使用best_bid，最后使用best_ask
        let reference_price = render_data.current_trade_price
            .or(render_data.best_bid_price)
            .or(render_data.best_ask_price);
            
        if let Some(price) = reference_price {
            let price_levels: Vec<f64> = render_data.price_levels.iter().map(|level| level.price).collect();
            let center_offset = self.price_tracker.calculate_center_offset(price, &price_levels, visible_rows);
            let end_offset = (center_offset + visible_rows).min(render_data.price_levels.len());
            (center_offset, end_offset)
        } else {
            (0, visible_rows.min(render_data.price_levels.len()))
        }
    }

    /// 渲染订单薄表格
    fn render_orderbook_table(
        &self,
        f: &mut Frame,
        area: Rect,
        render_data: &OrderBookRenderData,
        visible_range: (usize, usize),
        block: Block,
    ) {
        // 获取可见的价格层级
        let visible_levels = &render_data.price_levels[visible_range.0..visible_range.1];

        // 创建表格行
        let rows = self.create_table_rows(visible_levels, render_data);

        // 创建表格
        let table = Table::new(
            rows,
            self.layout_manager.get_column_constraints()
        )
        .header(self.layout_manager.create_header_row())
        .block(block);

        f.render_widget(table, area);
    }

    /// 创建表格行
    fn create_table_rows(&self, levels: &[PriceLevel], render_data: &OrderBookRenderData) -> Vec<Row> {
        let mut rows = Vec::new();

        for level in levels {
            let row = self.create_price_level_row(level, render_data);
            rows.push(row);
        }

        // 如果没有数据，显示等待状态
        if rows.is_empty() {
            let empty_row = Row::new(vec![
                Cell::from("连接正常，等待订单薄数据...").style(Style::default().fg(Color::Yellow)),
                Cell::from(""),
            ]);
            rows.push(empty_row);
        }

        rows
    }

    /// 创建单个价格层级的行
    fn create_price_level_row(&self, level: &PriceLevel, render_data: &OrderBookRenderData) -> Row {
        // 价格单元格 - 移除所有高亮显示，统一使用普通样式
        let price_cell = Cell::from(format!("{:.0}", level.price))
            .style(Style::default().fg(Color::White));

        // 合并的 Bid & Ask 单元格 - 修复best_bid/best_ask为None时的问题
        let merged_cell = if let (Some(best_bid), Some(best_ask)) = (render_data.best_bid_price, render_data.best_ask_price) {
            // 只有当best_bid和best_ask都有效时才使用正常逻辑
            self.layout_manager.format_merged_bid_ask_cell(
                level.price,
                level.bid_volume,
                level.ask_volume,
                best_bid,
                best_ask,
                &self.bar_chart,
                render_data.max_bid_volume,
                render_data.max_ask_volume,
            )
        } else {
            // 如果best_bid或best_ask无效，使用简化逻辑：bid显示绿色，ask显示红色
            if level.bid_volume > 0.0 && level.ask_volume > 0.0 {
                // 同时有bid和ask，显示组合信息
                let combined_text = format!("B:{:.0} A:{:.0}", level.bid_volume, level.ask_volume);
                Cell::from(combined_text).style(Style::default().fg(Color::Yellow))
            } else if level.bid_volume > 0.0 {
                // 只有bid，显示绿色
                self.bar_chart.create_bar_with_text(level.bid_volume, render_data.max_bid_volume, 60, true)
            } else if level.ask_volume > 0.0 {
                // 只有ask，显示红色
                self.bar_chart.create_bar_with_text(level.ask_volume, render_data.max_ask_volume, 60, false)
            } else {
                // 没有数据
                Cell::from("")
            }
        };

        // 简化为两列布局：Price (左侧) 和 Bid & Ask (右侧)
        Row::new(vec![
            price_cell,     // Price - 左侧
            merged_cell,    // Bid & Ask - 右侧，占满剩余空间
        ])
    }

    /// 聚合价格层级数据，强制使用1美元精度向下取整，处理bid/ask冲突
    fn aggregate_price_levels(
        &self,
        order_flows: &BTreeMap<OrderedFloat<f64>, OrderFlow>,
        precision: f64,
        best_bid_price: Option<f64>,
        best_ask_price: Option<f64>,
    ) -> BTreeMap<OrderedFloat<f64>, OrderFlow> {
        // 使用增强的聚合函数处理bid/ask冲突
        crate::orderbook::display_formatter::aggregate_price_levels_with_conflict_resolution(
            order_flows,
            best_bid_price,
            best_ask_price,
            precision
        )
    }
}

impl Default for OrderBookRenderer {
    fn default() -> Self {
        Self::new()
    }
}