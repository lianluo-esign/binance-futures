use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use std::collections::BTreeMap;
use ordered_float::OrderedFloat;

use crate::app::ReactiveApp;
use crate::orderbook::{OrderFlow, OrderFlowDisplayData};
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
    pub order_flow_data: Vec<OrderFlowDisplayData>,
    pub max_buy_flow_volume: f64,
    pub max_sell_flow_volume: f64,
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
        // 创建简洁的边框块（移除复杂的标题信息）
        // 确保与Volume Profile的边框高度对齐
        let block = Block::default()
            .title("Order Book")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        // 计算内部区域 - 确保与Volume Profile的内部区域计算一致
        let inner_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        // 准备渲染数据
        let render_data = self.prepare_render_data(app);

        // 更新价格跟踪器 - 使用bookticker的best bid price作为主要基准进行追踪
        let reference_price = render_data.best_bid_price
            .or(render_data.current_trade_price)
            .or(render_data.best_ask_price);
        
        // 确保参考价格在聚合后的价格列表中存在，使用更精确的价格验证
        if let Some(price) = reference_price {
            let price_levels: Vec<f64> = render_data.price_levels.iter().map(|level| level.price).collect();
            let validated_price = if self.price_tracker.is_price_in_range(price, &price_levels) {
                Some(price)
            } else {
                // 使用聚合后的价格，确保精度匹配
                let aggregated_price = (price / 1.0).floor() * 1.0;
                if price_levels.iter().any(|&p| (p - aggregated_price).abs() < 0.01) {
                    Some(aggregated_price)
                } else {
                    // 如果聚合价格也不存在，使用最接近的价格层级
                    price_levels.iter()
                        .min_by(|&&a, &&b| {
                            let dist_a = (a - price).abs();
                            let dist_b = (b - price).abs();
                            dist_a.partial_cmp(&dist_b).unwrap()
                        })
                        .copied()
                }
            };
            self.price_tracker.update_tracking(validated_price);
        } else {
            self.price_tracker.update_tracking(reference_price);
        }

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

        // 确保关键价格层级始终被包含
        let essential_prices = self.get_essential_price_levels(
            &price_levels,
            snapshot.best_bid_price,
            snapshot.best_ask_price,
            snapshot.current_price
        );

        let max_levels = app.get_max_visible_rows().min(price_levels.len());
        let mut levels = Vec::new();
        let mut max_bid_volume = 0.0;
        let mut max_ask_volume = 0.0;

        // 使用智能选择的价格层级，确保关键价格不被截断
        let selected_price_keys = self.select_price_levels_with_essentials(
            &price_levels,
            &essential_prices,
            max_levels
        );

        for &price_key in selected_price_keys.iter() {
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

        // 获取订单流数据
        let visible_prices: Vec<f64> = levels.iter().map(|level| level.price).collect();
        let order_flow_data = app.get_orderbook_manager().get_order_flow_data(&visible_prices);
        let (max_buy_flow_volume, max_sell_flow_volume) = app.get_orderbook_manager().get_order_flow_max_volumes();
        

        OrderBookRenderData {
            price_levels: levels,
            best_bid_price: snapshot.best_bid_price,
            best_ask_price: snapshot.best_ask_price,
            current_trade_price: snapshot.current_price,
            max_bid_volume,
            max_ask_volume,
            visible_range: (0, 0), // 将在后续计算
            order_flow_data,
            max_buy_flow_volume,
            max_sell_flow_volume,
        }
    }

    /// 计算可见范围 - 使用bookticker的best bid price作为基准进行居中追踪
    fn calculate_visible_range(&mut self, render_data: &OrderBookRenderData, visible_rows: usize) -> (usize, usize) {
        // 使用bookticker的best bid price作为主要基准，备用current_trade_price和best_ask
        let reference_price = render_data.best_bid_price
            .or(render_data.current_trade_price)
            .or(render_data.best_ask_price);
            
        if let Some(price) = reference_price {
            let price_levels: Vec<f64> = render_data.price_levels.iter().map(|level| level.price).collect();

            // 确保参考价格在价格列表的有效范围内
            let center_offset = if self.price_tracker.is_price_in_range(price, &price_levels) {
                self.price_tracker.calculate_center_offset(price, &price_levels, visible_rows)
            } else {
                // 如果参考价格不在范围内，使用best_bid或best_ask的聚合价格
                let fallback_price = self.find_closest_aggregated_price(
                    price,
                    render_data.best_bid_price,
                    render_data.best_ask_price,
                    &price_levels
                );
                self.price_tracker.calculate_center_offset(fallback_price, &price_levels, visible_rows)
            };
            
            let end_offset = (center_offset + visible_rows).min(render_data.price_levels.len());
            
            // 边界检查：确保关键价格在可见范围内
            let validated_range = self.validate_visible_range(
                (center_offset, end_offset),
                render_data,
                visible_rows
            );
            
            validated_range
        } else {
            (0, visible_rows.min(render_data.price_levels.len()))
        }
    }

    /// 查找最接近的聚合价格
    fn find_closest_aggregated_price(
        &self,
        target_price: f64,
        best_bid_price: Option<f64>,
        best_ask_price: Option<f64>,
        price_levels: &[f64],
    ) -> f64 {
        // 首先尝试使用best_bid或best_ask的聚合价格
        if let Some(bid) = best_bid_price {
            let aggregated_bid = (bid / 1.0).floor() * 1.0;
            if price_levels.iter().any(|&p| (p - aggregated_bid).abs() < 0.01) {
                return aggregated_bid;
            }
        }
        
        if let Some(ask) = best_ask_price {
            let aggregated_ask = (ask / 1.0).floor() * 1.0;
            if price_levels.iter().any(|&p| (p - aggregated_ask).abs() < 0.01) {
                return aggregated_ask;
            }
        }
        
        // 如果都找不到，返回最接近的价格层级
        price_levels.iter()
            .min_by(|&&a, &&b| {
                let dist_a = (a - target_price).abs();
                let dist_b = (b - target_price).abs();
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .copied()
            .unwrap_or(target_price)
    }

    /// 验证可见范围，确保关键价格在范围内
    fn validate_visible_range(
        &self,
        proposed_range: (usize, usize),
        render_data: &OrderBookRenderData,
        visible_rows: usize,
    ) -> (usize, usize) {
        let (start, end) = proposed_range;
        let price_levels = &render_data.price_levels;
        
        // 检查best_bid和best_ask是否在可见范围内
        let mut adjustment_needed = false;
        let mut required_indices = Vec::new();
        
        // 查找best_bid在价格列表中的索引
        if let Some(best_bid) = render_data.best_bid_price {
            if let Some(index) = price_levels.iter().position(|level| {
                (level.price - (best_bid / 1.0).floor() * 1.0).abs() < 0.01
            }) {
                if index < start || index >= end {
                    required_indices.push(index);
                    adjustment_needed = true;
                }
            }
        }
        
        // 查找best_ask在价格列表中的索引
        if let Some(best_ask) = render_data.best_ask_price {
            if let Some(index) = price_levels.iter().position(|level| {
                (level.price - (best_ask / 1.0).floor() * 1.0).abs() < 0.01
            }) {
                if index < start || index >= end {
                    required_indices.push(index);
                    adjustment_needed = true;
                }
            }
        }
        
        if !adjustment_needed {
            return proposed_range;
        }
        
        // 计算调整后的范围以包含所有必需的索引
        required_indices.sort();
        let min_required = required_indices.first().copied().unwrap_or(start);
        let max_required = required_indices.last().copied().unwrap_or(end.saturating_sub(1));
        
        let adjusted_start = min_required.min(start);
        let adjusted_end = (max_required + 1).max(end).min(price_levels.len());
        
        // 确保范围不超过visible_rows
        if adjusted_end - adjusted_start > visible_rows {
            // 如果范围太大，优先保证包含关键价格，从中间截取
            let center = (min_required + max_required) / 2;
            let half_rows = visible_rows / 2;
            let new_start = center.saturating_sub(half_rows);
            let new_end = (new_start + visible_rows).min(price_levels.len());
            (new_start, new_end)
        } else {
            (adjusted_start, adjusted_end)
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

        for (index, level) in levels.iter().enumerate() {
            let row = self.create_price_level_row(level, render_data, index);
            rows.push(row);
        }

        // 如果没有数据，显示等待状态
        if rows.is_empty() {
            let empty_row = if self.layout_manager.is_order_flow_enabled() {
                Row::new(vec![
                    Cell::from("连接正常，等待订单薄数据...").style(Style::default().fg(Color::Yellow)),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                ])
            } else {
                Row::new(vec![
                    Cell::from("连接正常，等待订单薄数据...").style(Style::default().fg(Color::Yellow)),
                    Cell::from(""),
                ])
            };
            rows.push(empty_row);
        }

        rows
    }

    /// 创建单个价格层级的行
    fn create_price_level_row(&self, level: &PriceLevel, render_data: &OrderBookRenderData, _index: usize) -> Row {
        // 价格单元格 - 移除所有高亮显示，统一使用普通样式
        let price_cell = Cell::from(format!("{:.0}", level.price))
            .style(Style::default().fg(Color::White));

        if self.layout_manager.is_order_flow_enabled() {
            // 2列布局：Price, Quantity
            
            // 合并的 Bid & Ask 单元格 (Quantity列)
            let quantity_cell = if let (Some(best_bid), Some(best_ask)) = (render_data.best_bid_price, render_data.best_ask_price) {
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

            Row::new(vec![
                price_cell,    // Price - 价格列
                quantity_cell, // Quantity - 数量列（Bid & Ask）
            ])
        } else {
            // 2列布局：Price, Bid & Ask
            
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

            Row::new(vec![
                price_cell,     // Price - 左侧
                merged_cell,    // Bid & Ask - 右侧，占满剩余空间
            ])
        }
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

    /// 获取关键价格层级 - 确保best_bid、best_ask和当前交易价格始终被包含
    fn get_essential_price_levels(
        &self,
        all_price_levels: &[&OrderedFloat<f64>],
        best_bid_price: Option<f64>,
        best_ask_price: Option<f64>,
        current_trade_price: Option<f64>,
    ) -> Vec<OrderedFloat<f64>> {
        let mut essential_prices = Vec::new();
        
        // 添加best_bid对应的聚合价格层级
        if let Some(best_bid) = best_bid_price {
            let aggregated_bid = OrderedFloat((best_bid / 1.0).floor() * 1.0);
            if all_price_levels.iter().any(|&&p| p == aggregated_bid) {
                essential_prices.push(aggregated_bid);
            }
        }
        
        // 添加best_ask对应的聚合价格层级
        if let Some(best_ask) = best_ask_price {
            let aggregated_ask = OrderedFloat((best_ask / 1.0).floor() * 1.0);
            if all_price_levels.iter().any(|&&p| p == aggregated_ask) {
                essential_prices.push(aggregated_ask);
            }
        }
        
        // 添加当前交易价格对应的聚合价格层级
        if let Some(current_price) = current_trade_price {
            let aggregated_current = OrderedFloat((current_price / 1.0).floor() * 1.0);
            if all_price_levels.iter().any(|&&p| p == aggregated_current) {
                essential_prices.push(aggregated_current);
            }
        }
        
        // 去重并排序
        essential_prices.sort_by(|a, b| b.cmp(a));
        essential_prices.dedup();
        
        essential_prices
    }

    /// 智能选择价格层级，确保关键价格始终被包含
    fn select_price_levels_with_essentials<'a>(
        &self,
        all_price_levels: &'a [&'a OrderedFloat<f64>],
        essential_prices: &[OrderedFloat<f64>],
        max_levels: usize,
    ) -> Vec<&'a OrderedFloat<f64>> {
        if max_levels >= all_price_levels.len() {
            return all_price_levels.to_vec();
        }
        
        let mut selected = Vec::new();
        let mut remaining_slots = max_levels;
        
        // 首先确保所有关键价格都被包含
        for essential_price in essential_prices {
            if let Some(price_ref) = all_price_levels.iter().find(|&p| **p == *essential_price) {
                if !selected.contains(price_ref) {
                    selected.push(*price_ref);
                    remaining_slots = remaining_slots.saturating_sub(1);
                }
            }
        }
        
        // 如果还有剩余槽位，按原始顺序填充其他价格层级
        for &price_level in all_price_levels.iter() {
            if remaining_slots == 0 {
                break;
            }
            
            if !selected.contains(&price_level) {
                selected.push(price_level);
                remaining_slots -= 1;
            }
        }
        
        // 按价格从高到低重新排序
        selected.sort_by(|a, b| b.cmp(a));
        selected
    }
}

impl Default for OrderBookRenderer {
    fn default() -> Self {
        Self::new()
    }
}