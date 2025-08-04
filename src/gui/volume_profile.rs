use std::collections::BTreeMap;
use ordered_float::OrderedFloat;
use ratatui::{
    style::{Color, Style},
    widgets::Cell,
};

/// Volume Profile 数据结构
#[derive(Debug, Clone)]
pub struct VolumeProfileData {
    /// 价格层级到成交量的映射
    pub price_volumes: BTreeMap<OrderedFloat<f64>, VolumeLevel>,
    /// 最大成交量（用于归一化显示）
    pub max_volume: f64,
    /// 数据更新时间戳
    pub last_update: u64,
}

/// 单个价格层级的成交量数据
#[derive(Debug, Clone)]
pub struct VolumeLevel {
    /// 买单成交量
    pub buy_volume: f64,
    /// 卖单成交量
    pub sell_volume: f64,
    /// 总成交量
    pub total_volume: f64,
    /// 最后更新时间
    pub timestamp: u64,
}

impl VolumeLevel {
    pub fn new() -> Self {
        Self {
            buy_volume: 0.0,
            sell_volume: 0.0,
            total_volume: 0.0,
            timestamp: 0,
        }
    }

    /// 添加交易数据
    pub fn add_trade(&mut self, side: &str, volume: f64, timestamp: u64) {
        match side {
            "buy" => self.buy_volume += volume,
            "sell" => self.sell_volume += volume,
            _ => {}
        }
        self.total_volume = self.buy_volume + self.sell_volume;
        self.timestamp = timestamp;
    }
}

/// Volume Profile 管理器
pub struct VolumeProfileManager {
    data: VolumeProfileData,
    /// 价格精度（1美元聚合）
    price_precision: f64,
}

impl VolumeProfileManager {
    pub fn new() -> Self {
        Self {
            data: VolumeProfileData {
                price_volumes: BTreeMap::new(),
                max_volume: 0.0,
                last_update: 0,
            },
            price_precision: 1.0, // 固定1美元精度
        }
    }

    /// 处理交易数据，累加到对应价格层级
    pub fn handle_trade(&mut self, price: f64, volume: f64, side: &str) {
        let timestamp = self.get_current_timestamp();
        
        // 聚合到1美元精度
        let aggregated_price = (price / self.price_precision).floor() * self.price_precision;
        let price_key = OrderedFloat(aggregated_price);
        
        // 获取或创建价格层级
        let volume_level = self.data.price_volumes
            .entry(price_key)
            .or_insert_with(VolumeLevel::new);
        
        // 添加交易数据
        volume_level.add_trade(side, volume, timestamp);
        
        // 更新最大成交量
        if volume_level.total_volume > self.data.max_volume {
            self.data.max_volume = volume_level.total_volume;
        }
        
        self.data.last_update = timestamp;
    }

    /// 获取Volume Profile数据
    pub fn get_data(&self) -> &VolumeProfileData {
        &self.data
    }

    /// 清理旧数据（可选，用于内存管理）
    pub fn cleanup_old_data(&mut self, max_age_ms: u64) {
        let current_time = self.get_current_timestamp();
        let cutoff_time = current_time.saturating_sub(max_age_ms);
        
        self.data.price_volumes.retain(|_, level| {
            level.timestamp >= cutoff_time
        });
        
        // 重新计算最大成交量
        self.data.max_volume = self.data.price_volumes
            .values()
            .map(|level| level.total_volume)
            .fold(0.0, f64::max);
    }

    /// 获取当前时间戳
    fn get_current_timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// 清空所有数据
    pub fn clear_data(&mut self) {
        self.data.price_volumes.clear();
        self.data.max_volume = 0.0;
        self.data.last_update = self.get_current_timestamp();
    }

    /// 直接设置某个价格层级的成交量数据（不累加）
    pub fn set_volume_data(&mut self, price: f64, buy_volume: f64, sell_volume: f64) {
        let timestamp = self.get_current_timestamp();
        
        // 聚合到1美元精度
        let aggregated_price = (price / self.price_precision).floor() * self.price_precision;
        let price_key = OrderedFloat(aggregated_price);
        
        // 创建新的成交量层级数据
        let mut volume_level = VolumeLevel::new();
        volume_level.buy_volume = buy_volume;
        volume_level.sell_volume = sell_volume;
        volume_level.total_volume = buy_volume + sell_volume;
        volume_level.timestamp = timestamp;
        
        // 更新最大成交量（在insert之前）
        if volume_level.total_volume > self.data.max_volume {
            self.data.max_volume = volume_level.total_volume;
        }
        
        // 直接设置数据（覆盖而不是累加）
        self.data.price_volumes.insert(price_key, volume_level);
        
        self.data.last_update = timestamp;
    }
}

/// Volume Profile Widget - 独立的Volume Profile显示组件
pub struct VolumeProfileWidget {
    manager: VolumeProfileManager,
    bar_width: u16,
}

impl VolumeProfileWidget {
    pub fn new() -> Self {
        Self {
            manager: VolumeProfileManager::new(),
            bar_width: 30, // 默认柱状图宽度
        }
    }

    /// 设置柱状图宽度
    pub fn set_bar_width(&mut self, width: u16) {
        self.bar_width = width;
    }

    /// 获取Volume Profile管理器的可变引用
    pub fn get_manager_mut(&mut self) -> &mut VolumeProfileManager {
        &mut self.manager
    }

    /// 获取Volume Profile管理器的引用
    pub fn get_manager(&self) -> &VolumeProfileManager {
        &self.manager
    }

    /// 渲染Volume Profile widget
    pub fn render(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, visible_price_range: &[f64]) {
        use ratatui::{
            layout::Constraint,
            style::{Color, Modifier, Style},
            widgets::{Block, Borders, Cell, Row, Table},
        };

        // 创建边框块
        let block = Block::default()
            .title("Volume Profile - 历史成交量分布")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));

        // 获取Volume Profile数据
        let volume_data = self.manager.get_data();

        // 创建表格行
        let mut rows = Vec::new();
        
        for &price in visible_price_range {
            let price_key = ordered_float::OrderedFloat(price);
            
            // 价格列
            let price_cell = Cell::from(format!("{:.0}", price))
                .style(Style::default().fg(Color::White));
            
            // Volume Profile柱状图列
            let volume_cell = if let Some(volume_level) = volume_data.price_volumes.get(&price_key) {
                self.create_volume_bar_cell(volume_level, volume_data.max_volume)
            } else {
                self.create_empty_volume_cell()
            };

            rows.push(Row::new(vec![price_cell, volume_cell]));
        }

        // 如果没有数据，显示等待状态
        if rows.is_empty() {
            let empty_row = Row::new(vec![
                Cell::from("等待成交数据...").style(Style::default().fg(Color::Yellow)),
                Cell::from(""),
            ]);
            rows.push(empty_row);
        }

        // 创建表格
        let table = Table::new(
            rows,
            [
                Constraint::Length(8),  // 价格列
                Constraint::Percentage(100), // Volume Profile柱状图列
            ]
        )
        .header(
            Row::new(vec![
                Cell::from("Price").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Cell::from("Volume").style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
            ])
        )
        .block(block);

        f.render_widget(table, area);
    }

    /// 创建成交量柱状图单元格
    fn create_volume_bar_cell(&self, volume_level: &VolumeLevel, max_volume: f64) -> Cell {
        if volume_level.total_volume <= 0.0 {
            return Cell::from("");
        }

        // 计算色块数量：每个色块代表1 BTC，最大100个色块
        let btc_volume = volume_level.total_volume; // 假设成交量单位就是BTC
        let block_count = (btc_volume.round() as u16).min(100).max(1); // 最小1个，最大100个色块
        
        // 创建蓝色色块bar
        let bar_chars = "█".repeat(block_count as usize);
        
        // 计算剩余空间用于填充
        let remaining_space = if block_count < 100 {
            100 - block_count
        } else {
            0
        };
        let padding = " ".repeat(remaining_space as usize);
        
        // 格式化总成交量显示（在bar右边末尾）
        let volume_text = if btc_volume >= 1000.0 {
            format!("{:.1}K", btc_volume / 1000.0)
        } else if btc_volume >= 100.0 {
            format!("{:.0}", btc_volume)
        } else if btc_volume >= 10.0 {
            format!("{:.1}", btc_volume)
        } else {
            format!("{:.2}", btc_volume)
        };

        // 组合显示：色块bar + 填充空间 + 成交量数值
        let display_text = format!("{}{} {}", bar_chars, padding, volume_text);
        
        Cell::from(display_text)
            .style(Style::default().fg(Color::Blue))
    }

    /// 创建空的成交量单元格
    fn create_empty_volume_cell(&self) -> Cell {
        Cell::from(" ".repeat(self.bar_width as usize + 10))
    }
}

/// Volume Profile 渲染器（保留原有接口兼容性）
pub struct VolumeProfileRenderer {
    bar_width: u16,
}

impl VolumeProfileRenderer {
    pub fn new() -> Self {
        Self {
            bar_width: 30, // 默认柱状图宽度
        }
    }

    /// 设置柱状图宽度
    pub fn set_bar_width(&mut self, width: u16) {
        self.bar_width = width;
    }

    /// 创建价格单元格
    pub fn create_price_cell(&self, price: f64) -> Cell {
        Cell::from(format!("{:.0}", price))
            .style(Style::default().fg(Color::White))
    }

    /// 创建成交量柱状图单元格
    pub fn create_volume_bar_cell(&self, volume_level: &VolumeLevel, max_volume: f64) -> Cell {
        if volume_level.total_volume <= 0.0 {
            return Cell::from("");
        }

        // 计算色块数量：每个色块代表1 BTC，最大100个色块
        let btc_volume = volume_level.total_volume; // 假设成交量单位就是BTC
        let block_count = (btc_volume.round() as u16).min(100).max(1); // 最小1个，最大100个色块
        
        // 创建蓝色色块bar
        let bar_chars = "█".repeat(block_count as usize);
        
        // 计算剩余空间用于填充
        let remaining_space = if block_count < 100 {
            100 - block_count
        } else {
            0
        };
        let padding = " ".repeat(remaining_space as usize);
        
        // 格式化总成交量显示（在bar右边末尾）
        let volume_text = if btc_volume >= 1000.0 {
            format!("{:.1}K", btc_volume / 1000.0)
        } else if btc_volume >= 100.0 {
            format!("{:.0}", btc_volume)
        } else if btc_volume >= 10.0 {
            format!("{:.1}", btc_volume)
        } else {
            format!("{:.2}", btc_volume)
        };

        // 组合显示：色块bar + 填充空间 + 成交量数值
        let display_text = format!("{}{} {}", bar_chars, padding, volume_text);
        
        Cell::from(display_text)
            .style(Style::default().fg(Color::Blue))
    }

    /// 创建空的成交量单元格
    pub fn create_empty_volume_cell(&self) -> Cell {
        Cell::from(" ".repeat(self.bar_width as usize + 10))
    }
}

impl Default for VolumeProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for VolumeProfileRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_level_add_trade() {
        let mut level = VolumeLevel::new();
        
        level.add_trade("buy", 10.0, 1000);
        assert_eq!(level.buy_volume, 10.0);
        assert_eq!(level.total_volume, 10.0);
        
        level.add_trade("sell", 5.0, 2000);
        assert_eq!(level.sell_volume, 5.0);
        assert_eq!(level.total_volume, 15.0);
    }

    #[test]
    fn test_volume_profile_manager() {
        let mut manager = VolumeProfileManager::new();
        
        // 添加交易数据
        manager.handle_trade(110001.5, 10.0, "buy");
        manager.handle_trade(110001.8, 5.0, "sell");
        
        // 验证数据聚合到110001层级
        let data = manager.get_data();
        let level = data.price_volumes.get(&OrderedFloat(110001.0)).unwrap();
        
        assert_eq!(level.buy_volume, 10.0);
        assert_eq!(level.sell_volume, 5.0);
        assert_eq!(level.total_volume, 15.0);
        assert_eq!(data.max_volume, 15.0);
    }

    #[test]
    fn test_volume_profile_renderer() {
        let renderer = VolumeProfileRenderer::new();
        
        let mut level = VolumeLevel::new();
        level.add_trade("buy", 100.0, 1000);
        
        let cell = renderer.create_volume_bar_cell(&level, 100.0);
        // 验证单元格创建成功（具体内容验证在集成测试中进行）
    }
}