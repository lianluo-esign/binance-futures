use ratatui::{
    layout::Rect,
    // style::Style, // 未使用
    widgets::{Cell, Row},
    Frame,
};

use crate::app::ReactiveApp;
use crate::orderbook::{display_formatter::{simulate_order_data_detailed, aggregate_price_levels_with_conflict_resolution, aggregate_trade_price}};
use crate::gui::OrderBookRenderer;

/// 渲染订单薄的主要函数 - 从main.rs移动过来
pub fn render_orderbook(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    // 使用新的OrderBookRenderer
    static mut ORDERBOOK_RENDERER: Option<OrderBookRenderer> = None;
    
    // 初始化渲染器（只在第一次调用时）
    let renderer = unsafe {
        if ORDERBOOK_RENDERER.is_none() {
            ORDERBOOK_RENDERER = Some(OrderBookRenderer::new());
        }
        ORDERBOOK_RENDERER.as_mut().unwrap()
    };
    
    // 使用新的渲染器进行渲染
    renderer.render(f, app, area);
}

/// 旧版本的订单薄渲染函数 - 保留作为备份
pub fn render_orderbook_old(f: &mut Frame, app: &ReactiveApp, area: Rect) {
    use ratatui::{
        layout::Constraint,
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, Table},
    };

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

    let title = format!("Binance Futures Order Book - {} | 缓冲区: {}/{} | 事件/秒: {:.1} | 状态: {} | 价格跟踪: {} | 自动居中: {}",
        app.get_symbol(), current_buffer_size, max_buffer_capacity, stats.events_processed_per_second, connection_status, price_tracking_status, auto_center_status);

    // 计算订单薄表格区域
    let table_width = area.width.saturating_sub(2);
    let table_height = area.height.saturating_sub(2);

    let centered_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: table_width,
        height: table_height,
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL);

    // 创建表格数据
    let mut rows = Vec::new();
    let snapshot = app.get_market_snapshot();

    // 获取真实的订单簿数据
    let order_flows = app.get_orderbook_manager().get_order_flows();
    let current_price = snapshot.current_price.unwrap_or(50000.0);

    // 获取配置参数
    let max_visible_rows = app.get_max_visible_rows();
    let price_precision = app.get_price_precision();

    // 应用价格精度聚合，处理bid/ask冲突
    let aggregated_order_flows = aggregate_price_levels_with_conflict_resolution(
        &order_flows, 
        snapshot.best_bid_price,
        snapshot.best_ask_price,
        price_precision
    );

    // 获取最近交易高亮信息并应用价格聚合
    let (last_trade_price, last_trade_side, _) = app.get_orderbook_manager().get_last_trade_highlight();
    let aggregated_last_trade_price = last_trade_price.map(|price| aggregate_trade_price(price, price_precision));
    let should_highlight_trade = true; // 总是高亮显示最新交易，支持高速历史数据播放

    // 准备价格值列表用于自动居中计算
    let price_values: Vec<f64>;

    // 如果有真实数据，使用聚合后的数据；否则生成模拟数据用于演示
    if !aggregated_order_flows.is_empty() {
        // 使用聚合后的订单簿数据
        let mut price_levels: Vec<_> = aggregated_order_flows.keys().collect();
        price_levels.sort_by(|a, b| b.cmp(a)); // 从高价到低价排序

        // 使用配置的最大可见行数限制显示的价格层级数量
        let max_levels = max_visible_rows.min(price_levels.len());

        // 提取价格值用于自动居中计算
        price_values = price_levels.iter().take(max_levels).map(|k| k.0).collect();

        for &price_key in price_levels.iter().take(max_levels) {
            let price = price_key.0;
            let order_flow = &aggregated_order_flows[price_key];

            // 获取真实的订单流数据
            let bid_vol = order_flow.bid_ask.bid;
            let ask_vol = order_flow.bid_ask.ask;
            let buy_trade_vol = order_flow.realtime_trade_record.buy_volume;
            let sell_trade_vol = order_flow.realtime_trade_record.sell_volume;
            let bid_cancel_vol = order_flow.realtime_cancel_records.bid_cancel;
            let ask_cancel_vol = order_flow.realtime_cancel_records.ask_cancel;
            let bid_increase_vol = order_flow.realtime_increase_order.bid;
            let ask_increase_vol = order_flow.realtime_increase_order.ask;
            let history_buy_vol = order_flow.history_trade_record.buy_volume;
            let history_sell_vol = order_flow.history_trade_record.sell_volume;

            // 判断是否为当前价格层级
            let is_current_price = (price - current_price).abs() < 0.1;

            // 构建显示字符串（只有数值大于0才显示，否则显示空字符串）
            let bid_str = if bid_vol > 0.0 { format!("{:.3}", bid_vol) } else { String::new() };
            let ask_str = if ask_vol > 0.0 { format!("{:.3}", ask_vol) } else { String::new() };
            let sell_trade_str = if sell_trade_vol > 0.0 { format!("@{:.3}", sell_trade_vol) } else { String::new() };
            let buy_trade_str = if buy_trade_vol > 0.0 { format!("@{:.3}", buy_trade_vol) } else { String::new() };
            let bid_cancel_str = if bid_cancel_vol > 0.0 { format!("-{:.3}", bid_cancel_vol) } else { String::new() };
            let ask_cancel_str = if ask_cancel_vol > 0.0 { format!("-{:.3}", ask_cancel_vol) } else { String::new() };
            let bid_increase_str = if bid_increase_vol > 0.0 { format!("+{:.3}", bid_increase_vol) } else { String::new() };
            let ask_increase_str = if ask_increase_vol > 0.0 { format!("+{:.3}", ask_increase_vol) } else { String::new() };

            // 创建行 - 与备份文件完全一致的列结构
            let row = Row::new(vec![
                Cell::from(bid_cancel_str).style(Style::default().fg(Color::Gray)),
                Cell::from(sell_trade_str).style(Style::default().fg(Color::Red)),
                Cell::from(bid_str).style(Style::default().fg(Color::Green)),
                {
                    let price_str = format!("{:.2}", price);
                    let mut price_cell = Cell::from(price_str).style(Style::default().fg(Color::White));

                    // 检查是否为最近交易价格，优先显示交易高亮
                    if should_highlight_trade &&
                       aggregated_last_trade_price.map_or(false, |trade_price| (price - trade_price).abs() < 0.001) {
                        // 根据交易方向设置背景颜色
                        let trade_bg_color = match last_trade_side.as_deref() {
                            Some("buy") => Color::Green,   // 买单用绿色背景
                            Some("sell") => Color::Red,    // 卖单用红色背景
                            _ => Color::Yellow,            // 未知方向用黄色背景
                        };
                        price_cell = price_cell.style(Style::default()
                            .fg(Color::Black)
                            .bg(trade_bg_color)
                            .add_modifier(Modifier::BOLD));
                    } else if is_current_price {
                        // 如果不是交易高亮，但是当前价格，使用原有的高亮逻辑
                        let highlight_color = if price >= current_price {
                            Color::Green
                        } else {
                            Color::Red
                        };
                        price_cell = price_cell.style(Style::default().fg(Color::Black).bg(highlight_color).add_modifier(Modifier::BOLD));
                    }
                    price_cell
                },
                Cell::from(ask_str).style(Style::default().fg(Color::Red)),
                Cell::from(buy_trade_str).style(Style::default().fg(Color::Green)),
                Cell::from(ask_cancel_str).style(Style::default().fg(Color::Gray)),
                Cell::from(bid_increase_str).style(Style::default().fg(Color::Blue)),
                Cell::from(ask_increase_str).style(Style::default().fg(Color::Blue)),
                {
                    let total_vol = history_buy_vol + history_sell_vol;
                    let active_trade_str = if total_vol > 0.0 {
                        format!("B:{:.3} S:{:.3} T:{:.3}",
                            history_buy_vol,
                            history_sell_vol,
                            total_vol)
                    } else {
                        String::new()
                    };
                    Cell::from(active_trade_str).style(Style::default().fg(Color::White))
                },
            ]);

            rows.push(row);
        }
    } else {
        // 如果没有真实数据，使用模拟数据进行演示
        let price_range = 50; // 显示50个价格层级
        let price_step = 0.1; // 价格步长

        // 为模拟数据创建价格值列表
        price_values = (0..price_range)
            .map(|i| current_price + (price_range as f64 / 2.0 - i as f64) * price_step)
            .collect();

        // 从高价到低价排列
        for i in 0..price_range {
            let price = current_price + (price_range as f64 / 2.0 - i as f64) * price_step;

            // 模拟订单数据
            let (bid_vol, ask_vol, buy_trade_vol, sell_trade_vol, bid_cancel_vol, ask_cancel_vol, bid_increase_vol, ask_increase_vol, history_buy_vol, history_sell_vol) =
                simulate_order_data_detailed(price, current_price);

            // 判断是否为当前价格层级
            let is_current_price = (price - current_price).abs() < price_step / 2.0;

            // 构建显示字符串（只有数值大于0才显示，否则显示空字符串）
            let bid_str = if bid_vol > 0.0 { format!("{:.3}", bid_vol) } else { String::new() };
            let ask_str = if ask_vol > 0.0 { format!("{:.3}", ask_vol) } else { String::new() };
            let sell_trade_str = if sell_trade_vol > 0.0 { format!("@{:.3}", sell_trade_vol) } else { String::new() };
            let buy_trade_str = if buy_trade_vol > 0.0 { format!("@{:.3}", buy_trade_vol) } else { String::new() };
            let bid_cancel_str = if bid_cancel_vol > 0.0 { format!("-{:.3}", bid_cancel_vol) } else { String::new() };
            let ask_cancel_str = if ask_cancel_vol > 0.0 { format!("-{:.3}", ask_cancel_vol) } else { String::new() };
            let bid_increase_str = if bid_increase_vol > 0.0 { format!("+{:.3}", bid_increase_vol) } else { String::new() };
            let ask_increase_str = if ask_increase_vol > 0.0 { format!("+{:.3}", ask_increase_vol) } else { String::new() };

            // 创建行 - 与备份文件完全一致的列结构
            let row = Row::new(vec![
                Cell::from(bid_cancel_str).style(Style::default().fg(Color::Gray)),
                Cell::from(sell_trade_str).style(Style::default().fg(Color::Red)),
                Cell::from(bid_str).style(Style::default().fg(Color::Green)),
                {
                    let price_str = format!("{:.2}", price);
                    let mut price_cell = Cell::from(price_str).style(Style::default().fg(Color::White));

                    // 检查是否为最近交易价格，优先显示交易高亮
                    if should_highlight_trade &&
                       aggregated_last_trade_price.map_or(false, |trade_price| (price - trade_price).abs() < 0.001) {
                        // 根据交易方向设置背景颜色
                        let trade_bg_color = match last_trade_side.as_deref() {
                            Some("buy") => Color::Green,   // 买单用绿色背景
                            Some("sell") => Color::Red,    // 卖单用红色背景
                            _ => Color::Yellow,            // 未知方向用黄色背景
                        };
                        price_cell = price_cell.style(Style::default()
                            .fg(Color::Black)
                            .bg(trade_bg_color)
                            .add_modifier(Modifier::BOLD));
                    } else if is_current_price {
                        // 如果不是交易高亮，但是当前价格，使用原有的高亮逻辑
                        let highlight_color = if price >= current_price {
                            Color::Green
                        } else {
                            Color::Red
                        };
                        price_cell = price_cell.style(Style::default().fg(Color::Black).bg(highlight_color).add_modifier(Modifier::BOLD));
                    }
                    price_cell
                },
                Cell::from(ask_str).style(Style::default().fg(Color::Red)),
                Cell::from(buy_trade_str).style(Style::default().fg(Color::Green)),
                Cell::from(ask_cancel_str).style(Style::default().fg(Color::Gray)),
                Cell::from(bid_increase_str).style(Style::default().fg(Color::Blue)),
                Cell::from(ask_increase_str).style(Style::default().fg(Color::Blue)),
                {
                    let total_vol = history_buy_vol + history_sell_vol;
                    let active_trade_str = if total_vol > 0.0 {
                        format!("B:{:.3} S:{:.3} T:{:.3}",
                            history_buy_vol,
                            history_sell_vol,
                            total_vol)
                    } else {
                        String::new()
                    };
                    Cell::from(active_trade_str).style(Style::default().fg(Color::White))
                },
            ]);

            rows.push(row);
        }
    }

    // 如果没有数据，显示等待状态
    if rows.is_empty() {
        let status_message = if stats.websocket_connected {
            "连接正常，等待订单薄数据..."
        } else {
            "WebSocket连接断开，尝试重连中..."
        };

        let empty_row = Row::new(vec![
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(status_message).style(Style::default().fg(Color::Yellow)),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ]);
        rows.push(empty_row);
    }

    // 应用自动居中逻辑
    let visible_rows_count = centered_area.height.saturating_sub(3) as usize; // 减去边框和表头
    let auto_center_scroll = app.calculate_auto_center_scroll(&price_values, visible_rows_count);

    // 应用滚动偏移
    let effective_scroll = if app.is_auto_scroll() {
        // 自动滚动模式：使用当前滚动偏移（已在事件循环中更新）
        app.get_scroll_offset()
    } else {
        auto_center_scroll
    };

    // 应用滚动偏移到行数据
    let visible_rows: Vec<_> = rows.into_iter().skip(effective_scroll).collect();

    // 创建并渲染表格 - 与备份文件完全一致的表头和列宽
    let table = Table::new(
        visible_rows,
        [
            Constraint::Length(10), // Bid Cancel
            Constraint::Length(10), // Sell Trade
            Constraint::Length(10), // Bid Vol
            Constraint::Length(12), // Price
            Constraint::Length(10), // Ask Vol
            Constraint::Length(10), // Buy Trade
            Constraint::Length(10), // Ask Cancel
            Constraint::Length(10), // Bid Increase
            Constraint::Length(10), // Ask Increase
            Constraint::Length(25), // History Trades
        ]
    )
    .header(
        Row::new(vec![
            Cell::from("Bid Cancel").style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Cell::from("Sell Trade").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Cell::from("Bid Vol").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Cell::from("Price").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Cell::from("Ask Vol").style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Cell::from("Buy Trade").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Cell::from("Ask Cancel").style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Cell::from("Bid Increase").style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
            Cell::from("Ask Increase").style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
            Cell::from("History Trades").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ])
    )
    .block(block);

    f.render_widget(table, centered_area);
}