use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Cell, Row},
};

use super::bar_chart::BarChartRenderer;

/// 布局管理器 - 管理订单薄的布局和列结构
pub struct LayoutManager {
    column_widths: Vec<Constraint>,
    show_merged_column: bool,
    show_order_flow: bool,
    cell_width: u16,
}

impl LayoutManager {
    /// 创建新的布局管理器
    pub fn new() -> Self {
        Self {
            column_widths: vec![
                Constraint::Length(8),  // Price - 价格列
                Constraint::Percentage(70), // Quantity - 数量列（Bid & Ask）- 最大化宽度
                Constraint::Length(12), // Buy - 主动买单列 - 增加宽度以容纳5位小数和右对齐
                Constraint::Length(12), // Sell - 主动卖单列 - 增加宽度以容纳5位小数和右对齐
            ],
            show_merged_column: true,
            show_order_flow: false,
            cell_width: 60, // 合并列的宽度 - 减小以模拟字体缩小效果
        }
    }

    /// 创建带自定义配置的布局管理器
    pub fn with_config(cell_width: u16, show_merged_column: bool, show_order_flow: bool) -> Self {
        let mut manager = Self::new();
        manager.cell_width = cell_width;
        manager.show_merged_column = show_merged_column;
        manager.show_order_flow = show_order_flow;
        
        // 根据配置调整列宽
        if show_order_flow {
            // 4列布局：Price, Quantity, Buy, Sell
            manager.column_widths = vec![
                Constraint::Length(8),      // Price - 价格列
                Constraint::Percentage(70), // Quantity - 数量列（Bid & Ask）- 最大化宽度
                Constraint::Length(12),     // Buy - 主动买单列 - 增加宽度以容纳5位小数和右对齐
                Constraint::Length(12),     // Sell - 主动卖单列 - 增加宽度以容纳5位小数和右对齐
            ];
        } else if show_merged_column {
            // 2列布局：Price, Bid & Ask
            manager.column_widths = vec![
                Constraint::Length(8),        // Price - 价格列
                Constraint::Percentage(100),  // Bid & Ask - 占满剩余全部空间
            ];
        }
        
        manager
    }

    /// 获取列约束
    pub fn get_column_constraints(&self) -> &[Constraint] {
        &self.column_widths
    }

    /// 创建表头行
    pub fn create_header_row(&self) -> Row {
        if self.show_order_flow {
            // 4列布局的表头
            Row::new(vec![
                Cell::from("Price").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Cell::from("Quantity").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Cell::from("Buy").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Cell::from("Sell").style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            ])
        } else {
            // 2列布局的表头
            Row::new(vec![
                Cell::from("Price").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Cell::from("Bid & Ask").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ])
        }
    }

    /// 格式化合并的Bid & Ask单元格（增强版，支持1美元精度聚合）
    pub fn format_merged_bid_ask_cell<'a>(
        &self,
        price: f64,
        bid_vol: f64,
        ask_vol: f64,
        best_bid: f64,
        best_ask: f64,
        bar_chart: &'a BarChartRenderer,
        max_bid_volume: f64,
        max_ask_volume: f64,
    ) -> Cell<'a> {
        // 使用1美元精度的容差来判断价格区间
        let price_tolerance = 0.5; // 0.5美元的容差
        
        // 判断当前聚合价格级别应该显示什么数据
        // 首先检查是否有任何挂单量，如果没有则显示空白
        if bid_vol <= 0.0 && ask_vol <= 0.0 {
            // 没有任何挂单量，显示空白
            Cell::from("")
        } else if price <= (best_bid + price_tolerance) && bid_vol > 0.0 {
            // 显示bid数据（绿色背景白色数字的BTC条形图）- 当前价格在或接近最优买价
            bar_chart.create_bar_with_text(bid_vol, max_bid_volume, self.cell_width, true)
        } else if price >= (best_ask - price_tolerance) && ask_vol > 0.0 {
            // 显示ask数据（红色背景白色数字的BTC条形图）- 当前价格在或接近最优卖价
            bar_chart.create_bar_with_text(ask_vol, max_ask_volume, self.cell_width, false)
        } else if price > (best_bid + price_tolerance) && price < (best_ask - price_tolerance) {
            // 在best bid和best ask之间显示分隔符（只有当没有挂单量时）
            if bid_vol <= 0.0 && ask_vol <= 0.0 {
                Cell::from("--- SPREAD ---").style(Style::default().fg(Color::Gray))
            } else {
                Cell::from("")  // 有挂单量但在spread区间内，显示空白
            }
        } else {
            // 显示两个数据都有的情况，或者空白区域
            if bid_vol > 0.0 && ask_vol > 0.0 {
                // 同时显示bid和ask（可能发生在价格聚合后）- 使用紧凑格式
                let combined_text = format!("B:{:.0} A:{:.0}", bid_vol, ask_vol);
                Cell::from(combined_text).style(Style::default().fg(Color::Yellow))
            } else if bid_vol > 0.0 {
                // 只有bid数据
                bar_chart.create_bar_with_text(bid_vol, max_bid_volume, self.cell_width, true)
            } else if ask_vol > 0.0 {
                // 只有ask数据
                bar_chart.create_bar_with_text(ask_vol, max_ask_volume, self.cell_width, false)
            } else {
                // 空白区域（无挂单量）
                Cell::from("")
            }
        }
    }

    /// 创建简化的合并单元格（仅显示数值，不含条形图）- 左对齐显示
    pub fn format_simple_merged_cell(
        &self,
        price: f64,
        bid_vol: f64,
        ask_vol: f64,
        best_bid: f64,
        best_ask: f64,
    ) -> Cell {
        // 首先检查是否有任何挂单量，如果没有则显示空白
        if bid_vol <= 0.0 && ask_vol <= 0.0 {
            // 没有任何挂单量，显示空白
            Cell::from("")
        } else if price <= best_bid && bid_vol > 0.0 {
            // 显示bid数据 - 左对齐，使用紧凑格式
            let volume_text = if bid_vol >= 1000.0 {
                format!("{:.0}K ", bid_vol / 1000.0)
            } else if bid_vol >= 100.0 {
                format!("{:.0} ", bid_vol)
            } else {
                format!("{:.1} ", bid_vol)
            };
            Cell::from(volume_text).style(Style::default().fg(Color::Green))
        } else if price >= best_ask && ask_vol > 0.0 {
            // 显示ask数据 - 左对齐，使用紧凑格式
            let volume_text = if ask_vol >= 1000.0 {
                format!("{:.0}K ", ask_vol / 1000.0)
            } else if ask_vol >= 100.0 {
                format!("{:.0} ", ask_vol)
            } else {
                format!("{:.1} ", ask_vol)
            };
            Cell::from(volume_text).style(Style::default().fg(Color::Red))
        } else if price > best_bid && price < best_ask {
            // 分隔符（只有当没有挂单量时）
            if bid_vol <= 0.0 && ask_vol <= 0.0 {
                Cell::from("--- ").style(Style::default().fg(Color::Gray))
            } else {
                Cell::from("")  // 有挂单量但在spread区间内，显示空白
            }
        } else {
            // 空白区域
            Cell::from("")
        }
    }

    /// 设置列宽
    pub fn set_column_widths(&mut self, widths: Vec<Constraint>) {
        self.column_widths = widths;
    }

    /// 设置合并列宽度
    pub fn set_merged_column_width(&mut self, width: u16) {
        self.cell_width = width;
        if self.show_merged_column && self.column_widths.len() > 2 {
            self.column_widths[2] = Constraint::Length(width);
        }
    }

    /// 启用/禁用合并列显示
    pub fn set_show_merged_column(&mut self, show: bool) {
        self.show_merged_column = show;
    }

    /// 启用/禁用订单流显示
    pub fn set_show_order_flow(&mut self, show: bool) {
        self.show_order_flow = show;
        
        // 更新列宽配置
        if show {
            // 4列布局：Price, Quantity, Buy, Sell
            self.column_widths = vec![
                Constraint::Length(8),      // Price - 价格列
                Constraint::Percentage(70), // Quantity - 数量列（Bid & Ask）- 最大化宽度
                Constraint::Length(12),     // Buy - 主动买单列 - 增加宽度以容纳5位小数和右对齐
                Constraint::Length(12),     // Sell - 主动卖单列 - 增加宽度以容纳5位小数和右对齐
            ];
        } else if self.show_merged_column {
            // 2列布局：Price, Bid & Ask
            self.column_widths = vec![
                Constraint::Length(8),        // Price - 价格列
                Constraint::Percentage(100),  // Bid & Ask - 占满剩余全部空间
            ];
        }
    }

    /// 检查是否显示订单流
    pub fn is_order_flow_enabled(&self) -> bool {
        self.show_order_flow
    }

    /// 创建订单流单元格（Buy列）
    pub fn create_buy_flow_cell(&self, volume: f64, _max_volume: f64) -> Cell {
        if volume == 0.0 {
            Cell::from("")
        } else {
            // 格式化数字，保留5位小数
            let formatted_volume = format!("{:>10.5}", volume); // Right-align with width of 10
            
            // 使用绿色前景色，无背景色
            Cell::from(formatted_volume)
                .style(Style::default().fg(Color::Green))
        }
    }

    /// 创建订单流单元格（Sell列）
    pub fn create_sell_flow_cell(&self, volume: f64, _max_volume: f64) -> Cell {
        if volume == 0.0 {
            Cell::from("")
        } else {
            // 格式化数字，保留5位小数
            let formatted_volume = format!("{:>10.5}", volume); // Right-align with width of 10
            
            // 使用红色前景色，无背景色
            Cell::from(formatted_volume)
                .style(Style::default().fg(Color::Red))
        }
    }

    /// 获取合并列宽度
    pub fn get_merged_column_width(&self) -> u16 {
        self.cell_width
    }

    /// 检查是否显示合并列
    pub fn is_merged_column_enabled(&self) -> bool {
        self.show_merged_column
    }

    /// 创建空行（用于数据加载时的占位）
    pub fn create_empty_row(&self, message: &str) -> Row<'static> {
        if self.show_order_flow {
            Row::new(vec![
                Cell::from(message.to_string()).style(Style::default().fg(Color::Yellow)), // Price列显示消息
                Cell::from(""), // Quantity列
                Cell::from(""), // Buy列
                Cell::from(""), // Sell列
            ])
        } else {
            Row::new(vec![
                Cell::from(message.to_string()).style(Style::default().fg(Color::Yellow)), // Price列显示消息
                Cell::from(""), // Bid & Ask列
            ])
        }
    }

    /// 创建分隔行
    pub fn create_separator_row(&self) -> Row {
        if self.show_order_flow {
            Row::new(vec![
                Cell::from("─".repeat(6)).style(Style::default().fg(Color::Gray)), // Price列分隔符
                Cell::from("─".repeat(15)).style(Style::default().fg(Color::Gray)), // Quantity列分隔符
                Cell::from("─".repeat(10)).style(Style::default().fg(Color::Gray)), // Buy列分隔符
                Cell::from("─".repeat(10)).style(Style::default().fg(Color::Gray)), // Sell列分隔符
            ])
        } else {
            Row::new(vec![
                Cell::from("─".repeat(6)).style(Style::default().fg(Color::Gray)), // Price列分隔符，缩小
                Cell::from("─".repeat(30)).style(Style::default().fg(Color::Gray)), // Bid & Ask列分隔符，缩小
            ])
        }
    }

    /// 验证价格区间逻辑
    pub fn validate_price_range(&self, price: f64, best_bid: f64, best_ask: f64) -> PriceRegion {
        if best_bid > best_ask {
            // 异常情况：bid价格高于ask价格
            return PriceRegion::Invalid;
        }

        if price <= best_bid {
            PriceRegion::BidRegion
        } else if price >= best_ask {
            PriceRegion::AskRegion
        } else {
            PriceRegion::SpreadRegion
        }
    }

    /// 计算最优的列宽分配
    pub fn calculate_optimal_widths(&self, total_width: u16) -> Vec<Constraint> {
        if self.show_order_flow {
            if total_width < 120 {
                // 窗口太小，使用更紧凑的4列布局
                vec![
                    Constraint::Length(6),      // Price - 更紧凑
                    Constraint::Percentage(35), // Quantity - 减小
                    Constraint::Percentage(32), // Buy - 调整
                    Constraint::Percentage(33), // Sell - 调整
                ]
            } else {
                // 标准4列布局
                vec![
                    Constraint::Length(8),      // Price - 价格列
                    Constraint::Percentage(70), // Quantity - 数量列（Bid & Ask）- 最大化宽度
                    Constraint::Length(12),     // Buy - 主动买单列 - 增加宽度以容纳5位小数和右对齐
                    Constraint::Length(12),     // Sell - 主动卖单列 - 增加宽度以容纳5位小数和右对齐
                ]
            }
        } else {
            if total_width < 80 {
                // 窗口太小，使用紧凑宽度
                vec![
                    Constraint::Length(6), // Price - 更紧凑
                    Constraint::Percentage(100), // Bid & Ask - 占满剩余空间
                ]
            } else {
                // 简化的两列布局 - 缩小宽度模拟字体缩小
                vec![
                    Constraint::Length(8), // Price - 固定宽度，从12减到8
                    Constraint::Percentage(100), // Bid & Ask - 占满剩余全部空间
                ]
            }
        }
    }

    /// 获取布局统计信息
    pub fn get_layout_stats(&self) -> LayoutStats {
        let total_fixed_width: u16 = self.column_widths.iter()
            .filter_map(|constraint| {
                if let Constraint::Length(width) = constraint {
                    Some(*width)
                } else {
                    None
                }
            })
            .sum();

        LayoutStats {
            total_columns: if self.show_order_flow { 4 } else { 2 }, // 4列或2列布局
            total_fixed_width,
            merged_column_width: self.cell_width,
            merged_column_enabled: self.show_merged_column,
        }
    }
}

/// 价格区间枚举
#[derive(Debug, Clone, PartialEq)]
pub enum PriceRegion {
    BidRegion,    // 价格在bid区域
    AskRegion,    // 价格在ask区域
    SpreadRegion, // 价格在bid-ask价差区域
    Invalid,      // 无效的价格区间
}

/// 布局统计信息
#[derive(Debug, Clone)]
pub struct LayoutStats {
    pub total_columns: usize,
    pub total_fixed_width: u16,
    pub merged_column_width: u16,
    pub merged_column_enabled: bool,
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_price_range() {
        let manager = LayoutManager::new();
        
        // 正常情况
        assert_eq!(manager.validate_price_range(99.0, 100.0, 101.0), PriceRegion::BidRegion);
        assert_eq!(manager.validate_price_range(100.0, 100.0, 101.0), PriceRegion::BidRegion);
        assert_eq!(manager.validate_price_range(100.5, 100.0, 101.0), PriceRegion::SpreadRegion);
        assert_eq!(manager.validate_price_range(101.0, 100.0, 101.0), PriceRegion::AskRegion);
        assert_eq!(manager.validate_price_range(102.0, 100.0, 101.0), PriceRegion::AskRegion);
        
        // 异常情况
        assert_eq!(manager.validate_price_range(100.0, 101.0, 100.0), PriceRegion::Invalid);
    }

    #[test]
    fn test_calculate_optimal_widths() {
        let manager = LayoutManager::new();
        
        let widths = manager.calculate_optimal_widths(100);
        assert_eq!(widths.len(), 4); // Now default has 4 columns (order flow enabled)
        
        // 测试窗口太小的情况
        let widths = manager.calculate_optimal_widths(30);
        assert_eq!(widths.len(), 4); // Still 4 columns even when small
        // 第一列应该是固定长度，第二列应该是百分比
        if let Constraint::Length(w) = widths[0] {
            assert_eq!(w, 6);
        }
        if let Constraint::Percentage(p) = widths[1] {
            assert_eq!(p, 35); // Updated for 4-column layout
        }
    }

    #[test]
    fn test_merged_column_configuration() {
        let mut manager = LayoutManager::new();
        
        assert!(manager.is_merged_column_enabled());
        assert_eq!(manager.get_merged_column_width(), 60);
        
        manager.set_merged_column_width(100);
        assert_eq!(manager.get_merged_column_width(), 100);
        
        manager.set_show_merged_column(false);
        assert!(!manager.is_merged_column_enabled());
    }

    #[test]
    fn test_layout_stats() {
        let manager = LayoutManager::new();
        let stats = manager.get_layout_stats();
        
        assert_eq!(stats.total_columns, 4); // Now default has 4 columns
        assert!(stats.total_fixed_width > 0);
        assert_eq!(stats.merged_column_width, 60);
        assert!(stats.merged_column_enabled);
    }

    #[test]
    fn test_create_header_row() {
        let manager = LayoutManager::new();
        let header = manager.create_header_row();
        
        // 验证表头行创建成功（具体内容验证在集成测试中进行）
        // 这里主要确保不会panic
    }

    #[test]
    fn test_create_empty_row() {
        let manager = LayoutManager::new();
        let empty_row = manager.create_empty_row("Loading...");
        
        // 验证空行创建成功
        // 主要确保不会panic
    }
}