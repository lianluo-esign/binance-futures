use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{BarChart, Block, Borders},
    Frame,
};
use std::collections::BTreeMap;

/// 使用 ratatui 内置 BarChart 的成交量柱状图渲染器
pub struct RatatuiVolumeBarChartRenderer {
    /// 分钟级数据存储
    minute_data: BTreeMap<u64, VolumeMinuteData>,
    /// 最大保留分钟数
    max_minutes: usize,
    /// 最大成交量
    max_volume: f64,
}

/// 分钟级成交量数据
#[derive(Debug, Clone)]
pub struct VolumeMinuteData {
    pub minute_timestamp: u64,
    pub total_volume: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub trade_count: u32,
}

impl VolumeMinuteData {
    pub fn new(minute_timestamp: u64) -> Self {
        Self {
            minute_timestamp,
            total_volume: 0.0,
            buy_volume: 0.0,
            sell_volume: 0.0,
            trade_count: 0,
        }
    }

    pub fn add_trade(&mut self, volume: f64, is_buyer_maker: bool) {
        self.total_volume += volume;
        self.trade_count += 1;
        
        if is_buyer_maker {
            self.sell_volume += volume;
        } else {
            self.buy_volume += volume;
        }
    }
}

impl RatatuiVolumeBarChartRenderer {
    /// 创建新的渲染器
    pub fn new() -> Self {
        Self {
            minute_data: BTreeMap::new(),
            max_minutes: 20, // 显示最近20分钟
            max_volume: 0.0,
        }
    }

    /// 添加交易数据
    pub fn add_trade_data(&mut self, timestamp: u64, volume: f64, is_buyer_maker: bool) {
        let minute_timestamp = self.get_minute_boundary(timestamp);
        
        let minute_data = self.minute_data
            .entry(minute_timestamp)
            .or_insert_with(|| VolumeMinuteData::new(minute_timestamp));
        
        minute_data.add_trade(volume, is_buyer_maker);
        
        if minute_data.total_volume > self.max_volume {
            self.max_volume = minute_data.total_volume;
        }
        
        self.maintain_sliding_window();
    }

    /// 批量同步数据
    pub fn sync_from_price_data<I>(&mut self, price_points: I)
    where
        I: Iterator<Item = (u64, f64, bool)>,
    {
        self.minute_data.clear();
        self.max_volume = 0.0;
        
        for (timestamp, volume, is_buyer_maker) in price_points {
            self.add_trade_data(timestamp, volume, is_buyer_maker);
        }
    }

    /// 渲染 BarChart
    pub fn render(&self, f: &mut Frame, area: Rect) {
        // 总是准备完整的bar数据，即使没有实际数据也要显示空的bars
        // 这样可以保证chart始终填满整个窗口

        // 准备 BarChart 数据（总是生成完整的20个bar，即使某些分钟没有数据）
        let bar_data: Vec<(&str, u64)> = self.prepare_bar_data();
        
        // 调试信息：记录实际生成的bar数量
        log::debug!("Volume bar chart: Generated {} bars, minute_data has {} entries", 
                   bar_data.len(), self.minute_data.len());
        
        // 计算统计信息
        let total_volume: f64 = self.minute_data.values().map(|d| d.total_volume).sum();
        let avg_volume = if !self.minute_data.is_empty() { 
            total_volume / self.minute_data.len() as f64 
        } else { 
            0.0 
        };

        // 创建标题
        let title = format!(
            "Volume Chart (1min) | {} mins | Avg: {:.3} BTC | Max: {:.3} BTC",
            bar_data.len(), // 使用bar_data的长度而不是minute_data的长度
            avg_volume,
            self.max_volume
        );

        // 创建 ratatui BarChart
        let barchart = BarChart::default()
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White))
            )
            .data(&bar_data)
            .bar_width(3)
            .bar_style(Style::default().fg(Color::Cyan))
            .value_style(Style::default().fg(Color::White))
            .label_style(Style::default().fg(Color::Gray))
            .bar_gap(1);

        f.render_widget(barchart, area);
    }

    /// 准备 BarChart 数据 - 修复版本：确保填满整个窗口
    pub fn prepare_bar_data(&self) -> Vec<(&'static str, u64)> {
        let labels = ["M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8", "M9", "M10",
                     "M11", "M12", "M13", "M14", "M15", "M16", "M17", "M18", "M19", "M20"];
        
        // 获取当前时间并计算最近的分钟边界
        let current_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let current_minute = self.get_minute_boundary(current_timestamp);
        
        let mut data = Vec::new();
        
        // 生成最近max_minutes个分钟的数据，从当前分钟向前推
        for i in 0..self.max_minutes {
            if i >= labels.len() {
                break;
            }
            
            // 计算这个slot对应的分钟时间戳（从当前分钟向前推）
            let minutes_back = (self.max_minutes - 1 - i) as u64;
            let target_minute = current_minute - (minutes_back * 60 * 1000);
            
            // 查找这个分钟是否有数据
            let volume_scaled = if let Some(minute_data) = self.minute_data.get(&target_minute) {
                // 将 BTC 成交量转换为整数（乘以1000以保持精度）
                (minute_data.total_volume * 1000.0) as u64
            } else {
                // 没有数据的分钟显示为0
                0
            };
            
            data.push((labels[i], volume_scaled));
        }
        
        data
    }

    /// 渲染空图表
    fn render_empty_chart(&self, f: &mut Frame, area: Rect) {
        let empty_barchart = BarChart::default()
            .block(
                Block::default()
                    .title("Volume Chart (1min) - No Data")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
            )
            .data(&[])
            .bar_width(3)
            .bar_style(Style::default().fg(Color::Gray))
            .value_style(Style::default().fg(Color::White))
            .label_style(Style::default().fg(Color::Gray));

        f.render_widget(empty_barchart, area);
    }

    /// 获取分钟边界
    fn get_minute_boundary(&self, timestamp: u64) -> u64 {
        let seconds = timestamp / 1000;
        let minutes = seconds / 60;
        let minute_boundary_seconds = minutes * 60;
        minute_boundary_seconds * 1000
    }

    /// 维护滑动窗口 - 修复版本：保留最新的数据，移除最旧的数据
    fn maintain_sliding_window(&mut self) {
        if self.minute_data.len() <= self.max_minutes {
            return;
        }

        // 计算当前时间的分钟边界
        let current_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let current_minute = self.get_minute_boundary(current_timestamp);
        
        // 计算时间窗口的开始边界（保留最近max_minutes分钟的数据）
        let window_start = current_minute - ((self.max_minutes - 1) as u64 * 60 * 1000);
        
        // 移除窗口外的旧数据
        let keys_to_remove: Vec<u64> = self.minute_data
            .keys()
            .filter(|&&key| key < window_start)
            .copied()
            .collect();
        
        for key in keys_to_remove {
            self.minute_data.remove(&key);
        }
        
        self.recalculate_max_volume();
    }

    /// 重新计算最大成交量
    fn recalculate_max_volume(&mut self) {
        self.max_volume = self.minute_data
            .values()
            .map(|data| data.total_volume)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);
    }

    /// 清空数据
    pub fn clear_data(&mut self) {
        self.minute_data.clear();
        self.max_volume = 0.0;
    }

    /// 获取数据点数量
    pub fn get_data_count(&self) -> usize {
        self.minute_data.len()
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> RatatuiVolumeBarChartStats {
        let total_minutes = self.minute_data.len();
        let total_volume: f64 = self.minute_data.values().map(|d| d.total_volume).sum();
        let total_trades: u32 = self.minute_data.values().map(|d| d.trade_count).sum();
        let total_buy_volume: f64 = self.minute_data.values().map(|d| d.buy_volume).sum();
        let total_sell_volume: f64 = self.minute_data.values().map(|d| d.sell_volume).sum();
        
        let avg_volume = if total_minutes > 0 { total_volume / total_minutes as f64 } else { 0.0 };
        let avg_trades = if total_minutes > 0 { total_trades as f64 / total_minutes as f64 } else { 0.0 };

        RatatuiVolumeBarChartStats {
            total_minutes,
            total_volume,
            total_trades,
            avg_volume,
            avg_trades,
            max_volume: self.max_volume,
            buy_volume: total_buy_volume,
            sell_volume: total_sell_volume,
            buy_sell_ratio: if total_sell_volume > 0.0 { total_buy_volume / total_sell_volume } else { 0.0 },
        }
    }
}

/// 统计信息
#[derive(Debug, Clone, Default)]
pub struct RatatuiVolumeBarChartStats {
    pub total_minutes: usize,
    pub total_volume: f64,
    pub total_trades: u32,
    pub avg_volume: f64,
    pub avg_trades: f64,
    pub max_volume: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub buy_sell_ratio: f64,
}

impl Default for RatatuiVolumeBarChartRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_ratatui_volume_bar_chart_creation() {
        let chart = RatatuiVolumeBarChartRenderer::new();
        assert_eq!(chart.get_data_count(), 0);
        assert_eq!(chart.max_minutes, 20);
    }

    #[test]
    fn test_add_trade_data() {
        let mut chart = RatatuiVolumeBarChartRenderer::new();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        chart.add_trade_data(timestamp, 1.0, false);
        assert_eq!(chart.get_data_count(), 1);
        
        let stats = chart.get_stats();
        assert_eq!(stats.total_volume, 1.0);
        assert_eq!(stats.buy_volume, 1.0);
        assert_eq!(stats.sell_volume, 0.0);
    }

    #[test]
    fn test_prepare_bar_data() {
        let mut chart = RatatuiVolumeBarChartRenderer::new();
        let base_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 添加3分钟的数据
        for i in 0..3 {
            let timestamp = base_timestamp - ((2 - i) * 60 * 1000); // 从当前时间向前推
            chart.add_trade_data(timestamp, (i + 1) as f64, false);
        }
        
        let bar_data = chart.prepare_bar_data();
        // 现在应该总是生成20个bar，即使只有3个分钟有数据
        assert_eq!(bar_data.len(), 20, "Should always generate 20 bars");
        
        // 检查最后几个bar应该有数据（因为是最近的数据）
        assert!(bar_data[17].1 > 0, "Recent bars should have data");
        assert!(bar_data[18].1 > 0, "Recent bars should have data");
        assert!(bar_data[19].1 > 0, "Recent bars should have data");
        
        // 检查早期的bar应该是0（没有数据）
        assert_eq!(bar_data[0].1, 0, "Old bars should be zero");
    }
}