use crate::gui::{VolumeProfileWidget, PriceChartRenderer, StatusBar};
use crate::orderbook::render_orderbook;
use crate::ReactiveApp;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

/// UI管理器 - 负责管理和协调所有GUI相关功能
pub struct UIManager {
    /// Volume Profile widget
    volume_profile_widget: VolumeProfileWidget,
    /// 价格图表渲染器，使用20000个数据点的滑动窗口
    price_chart_renderer: PriceChartRenderer,
}

impl UIManager {
    /// 创建新的UI管理器实例
    pub fn new() -> Self {
        Self {
            volume_profile_widget: VolumeProfileWidget::new(),
            price_chart_renderer: PriceChartRenderer::new(20000),
        }
    }

    /// 更新所有UI组件的数据
    pub fn update_data(&mut self, app: &ReactiveApp) {
        // 更新Volume Profile数据
        self.update_volume_profile(app);
        
        // 更新价格图表数据
        self.update_price_chart(app);
    }

    /// 渲染主UI - 带状态栏的三列布局版本
    pub fn render_ui(&self, f: &mut Frame, app: &ReactiveApp) {
        let size = f.area();

        // 垂直布局：顶部状态栏，下方主要内容区域
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),      // 顶部状态栏高度改为3
                Constraint::Min(0),         // 主要内容区域
            ])
            .split(size);

        let status_bar_area = main_chunks[0];
        let content_area = main_chunks[1];

        // 渲染顶部状态栏
        StatusBar::render(f, app, status_bar_area);

        // 创建三列布局：订单薄(20%)、Volume Profile(40%)、图表区域(40%)
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20), // 订单薄占20%
                Constraint::Percentage(40), // Volume Profile占40%
                Constraint::Percentage(40), // 图表区域占40%
            ])
            .split(content_area);

        let orderbook_area = horizontal_chunks[0];
        let volume_profile_area = horizontal_chunks[1];
        let chart_area = horizontal_chunks[2];

        // 渲染各个组件
        render_orderbook(f, app, orderbook_area);
        
        // 渲染Volume Profile widget
        self.render_volume_profile(f, app, volume_profile_area);
        
        // 渲染价格图表
        self.render_price_chart(f, chart_area);
    }

    /// 处理键盘输入事件
    pub fn handle_key_event(&self, app: &mut ReactiveApp, key_code: KeyCode) -> bool {
        match key_code {
            KeyCode::Char('q') => return true, // 退出应用
            KeyCode::Up => {
                if app.get_scroll_offset() > 0 {
                    app.set_scroll_offset(app.get_scroll_offset() - 1);
                    app.set_auto_scroll(false);
                    app.set_auto_center_enabled(false); // 禁用自动居中
                }
            }
            KeyCode::Down => {
                app.set_scroll_offset(app.get_scroll_offset() + 1);
                app.set_auto_scroll(false);
                app.set_auto_center_enabled(false); // 禁用自动居中
            }
            KeyCode::Home => {
                app.set_scroll_offset(0);
                app.set_auto_scroll(false);
                app.set_auto_center_enabled(false); // 禁用自动居中
            }
            KeyCode::End => {
                app.set_auto_scroll(true);
                app.set_auto_center_enabled(true); // 重新启用自动居中
            }
            KeyCode::Char(' ') => {
                app.set_auto_scroll(!app.is_auto_scroll());
                if app.is_auto_scroll() {
                    app.set_auto_center_enabled(true); // 启用自动滚动时重新启用自动居中
                }
            }
            KeyCode::Char('c') => {
                // 'c' 键切换自动居中功能
                app.set_auto_center_enabled(!app.is_auto_center_enabled());
            }
            KeyCode::Char('t') => {
                // 't' 键切换价格跟踪功能
                app.set_price_tracking_enabled(!app.is_price_tracking_enabled());
            }
            KeyCode::Char('r') => {
                // 'r' 键手动重新居中到当前交易价格
                let current_price = app.get_market_snapshot().current_price;
                if let Some(_price) = current_price {
                    // 临时启用价格跟踪来触发居中
                    let was_tracking = app.is_price_tracking_enabled();
                    app.set_price_tracking_enabled(true);
                    // 通过设置阈值为0来强制触发重新居中
                    app.force_recenter_on_current_price();
                    app.set_price_tracking_enabled(was_tracking);
                }
            }
            _ => {}
        }
        false // 不退出应用
    }

    /// 更新Volume Profile数据
    fn update_volume_profile(&mut self, app: &ReactiveApp) {
        // 直接从应用的Volume Profile管理器获取数据
        // 这个管理器只在实际交易事件发生时才会更新
        let app_volume_manager = app.get_volume_profile_manager();
        let app_data = app_volume_manager.get_data();
        
        // 获取 orderbook 的 order flow 数据
        let _order_flows = app.get_orderbook_manager().get_order_flows();
        
        // 获取widget的管理器
        let widget_manager = self.volume_profile_widget.get_manager_mut();
        
        // 直接同步应用层的volume profile累积数据到widget
        // 保持原有的累积数据和最新更新信息
        for (price_key, app_volume_level) in &app_data.price_volumes {
            let price = price_key.0;
            
            // 检查widget中是否已有这个价格层级的数据
            let widget_data = widget_manager.get_data();
            let existing_level = widget_data.price_volumes.get(price_key);
            
            // 如果是新数据或者数据有更新，则同步并保持更新状态
            match existing_level {
                Some(existing) => {
                    // 检查是否有新的交易数据
                    if existing.total_volume != app_volume_level.total_volume {
                        // 数据有更新，直接设置完整的level数据，包括last_update_side
                        widget_manager.sync_volume_level_with_update_info(
                            price, 
                            app_volume_level.buy_volume, 
                            app_volume_level.sell_volume,
                            app_volume_level.timestamp,
                            app_volume_level.last_update_side.clone()
                        );
                    }
                },
                None => {
                    // 新的价格层级，同步数据
                    widget_manager.sync_volume_level_with_update_info(
                        price, 
                        app_volume_level.buy_volume, 
                        app_volume_level.sell_volume,
                        app_volume_level.timestamp,
                        app_volume_level.last_update_side.clone()
                    );
                }
            }
        }
    }

    /// 渲染Volume Profile widget
    fn render_volume_profile(&self, f: &mut Frame, app: &ReactiveApp, area: Rect) {
        // 根据实际widget区域高度计算可见行数
        let actual_visible_rows = calculate_visible_rows_from_area(area);
        
        // 获取当前可见的价格范围（基于实际widget高度）
        let visible_price_range = get_visible_price_range_for_area(app, actual_visible_rows);
        
        // 获取最新交易价格用于高亮显示
        let latest_trade_price = app.get_market_snapshot().current_price;
        
        // 获取orderbook数据用于显示buy/sell列
        let orderbook_data = app.get_orderbook_manager().get_order_flows();
        
        // 渲染Volume Profile widget
        self.volume_profile_widget.render(f, area, &visible_price_range, latest_trade_price, Some(orderbook_data));
    }

    /// 更新价格图表数据
    fn update_price_chart(&mut self, app: &ReactiveApp) {
        // 优先使用应用直接存储的交易数据（确保捕获所有交易事件）
        let direct_price = app.get_last_trade_price();
        let direct_side = app.get_last_trade_side();
        let direct_volume = app.get_last_trade_volume();
        
        if let (Some(price), Some(side)) = (direct_price, direct_side) {
            // 使用应用直接存储的交易数据（无时间过滤，确保高速播放时不丢失数据）
            let is_buyer_maker = side == "sell";
            let volume = direct_volume.unwrap_or(0.001);
            
            // 添加价格点到图表（移除了所有时间过滤）
            self.price_chart_renderer.add_price_point(price, volume, is_buyer_maker);
        } else {
            // 备用方案：获取最新交易数据（来自订单簿管理器）
            let orderbook_manager = app.get_orderbook_manager();
            let (last_trade_price, last_trade_side, last_trade_timestamp) = orderbook_manager.get_last_trade_highlight();
            
            // 如果有最新交易数据，直接添加价格点（移除时间过滤以支持高速历史数据播放）
            if let (Some(price), Some(side), Some(_timestamp)) = (last_trade_price, last_trade_side, last_trade_timestamp) {
                // 确定交易方向：buy是买单（绿色），sell是卖单（红色）
                let is_buyer_maker = side == "sell";
                
                // 获取真实的成交量数据
                let volume = app.get_last_trade_volume().unwrap_or(0.001); // 使用真实成交量，默认0.001
                
                // 统一使用add_price_point，现在包含交易信息
                // 移除时间过滤以确保高速历史数据播放时所有交易点都能显示
                self.price_chart_renderer.add_price_point(price, volume, is_buyer_maker);
            } else {
                // 如果没有最新交易数据，使用市场快照中的价格（作为默认的小量买单）
                let market_snapshot = app.get_market_snapshot();
                if let Some(current_price) = market_snapshot.current_price {
                    self.price_chart_renderer.add_price_point(current_price, 0.001, false); // 默认小量买单
                }
            }
        }
    }

    /// 渲染价格图表
    fn render_price_chart(&self, f: &mut Frame, area: Rect) {
        self.price_chart_renderer.render(f, area);
    }
}

impl Default for UIManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 根据widget区域计算实际可见行数
fn calculate_visible_rows_from_area(area: Rect) -> usize {
    // 只减去表头（1行），因为ratatui的Table with .block() 会自动处理边框空间
    // 这样与Order Book的计算方式保持一致，确保边框高度对齐
    let available_height = area.height.saturating_sub(1); // 只减去表头1行
    available_height as usize
}

/// 获取基于实际widget区域的价格范围
fn get_visible_price_range_for_area(app: &ReactiveApp, visible_rows: usize) -> Vec<f64> {
    let snapshot = app.get_market_snapshot();
    
    // 获取volume profile数据范围
    let volume_manager = app.get_volume_profile_manager();
    let volume_data = volume_manager.get_data();
    
    // 优先使用当前交易价格，如果没有则使用best_bid，最后使用best_ask
    let reference_price = snapshot.current_price
        .or(snapshot.best_bid_price)
        .or(snapshot.best_ask_price);
    
    // 如果有volume profile数据，扩展价格范围以包含这些数据
    let (min_volume_price, max_volume_price) = if !volume_data.price_volumes.is_empty() {
        let min_price = volume_data.price_volumes.keys().next().unwrap().0;
        let max_price = volume_data.price_volumes.keys().next_back().unwrap().0;
        (Some(min_price), Some(max_price))
    } else {
        (None, None)
    };
        
    if let Some(center_price) = reference_price {
        // 动态生成价格层级：以当前价格为中心，上下各扩展足够的层级
        // 使用1美元精度（与VolumeProfileManager的price_precision保持一致）
        let price_precision = 1.0;
        
        // 计算中心价格的聚合值（向下取整到最近的美元）
        let center_aggregated = (center_price / price_precision).floor() * price_precision;
        
        // 根据可见行数和volume数据范围动态计算需要的层级数
        let half_visible = visible_rows / 2;
        let mut levels_above = half_visible + 20; // 减少缓冲，因为我们要包含volume数据
        let mut levels_below = half_visible + 20;
        
        // 如果有volume数据，扩展范围以包含这些价格
        if let Some(max_vol_price) = max_volume_price {
            let levels_needed_above = ((max_vol_price - center_aggregated) / price_precision).ceil() as usize;
            levels_above = levels_above.max(levels_needed_above + 10);
        }
        if let Some(min_vol_price) = min_volume_price {
            let levels_needed_below = ((center_aggregated - min_vol_price) / price_precision).ceil() as usize;
            levels_below = levels_below.max(levels_needed_below + 10);
        }
        
        let total_levels = levels_above + levels_below + 1;
        
        let mut price_levels = Vec::with_capacity(total_levels);
        
        // 从高价到低价生成价格层级
        // 上方层级
        for i in (1..=levels_above).rev() {
            let price = center_aggregated + (i as f64) * price_precision;
            price_levels.push(price);
        }
        
        // 中心价格
        price_levels.push(center_aggregated);
        
        // 下方层级
        for i in 1..=levels_below {
            let price = center_aggregated - (i as f64) * price_precision;
            price_levels.push(price);
        }
        
        // 截取可见范围：以中心价格为基准
        let center_index = levels_above; // 中心价格在数组中的索引
        let start_index = center_index.saturating_sub(half_visible);
        let end_index = (start_index + visible_rows).min(price_levels.len());
        
        price_levels[start_index..end_index].to_vec()
    } else {
        // 如果没有参考价格，生成一个默认的价格范围（以110000为中心）
        let default_center = 110000.0;
        let price_precision = 1.0;
        let half_visible = visible_rows / 2;
        let levels_above = half_visible + 50;
        let levels_below = half_visible + 50;
        
        let mut price_levels = Vec::with_capacity(levels_above + levels_below + 1);
        
        // 从高价到低价生成价格层级
        for i in (1..=levels_above).rev() {
            let price = default_center + (i as f64) * price_precision;
            price_levels.push(price);
        }
        
        price_levels.push(default_center);
        
        for i in 1..=levels_below {
            let price = default_center - (i as f64) * price_precision;
            price_levels.push(price);
        }
        
        let center_index = levels_above;
        let start_index = center_index.saturating_sub(half_visible);
        let end_index = (start_index + visible_rows).min(price_levels.len());
        
        price_levels[start_index..end_index].to_vec()
    }
}