use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::collections::BTreeMap;

/// Unicode块字符，用于构建柱状图
const BLOCK_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// 时间戳格式化辅助函数
fn format_timestamp_to_time(timestamp: u64) -> String {
    // 简化版本：直接从时间戳计算小时和分钟
    let minutes_since_epoch = (timestamp / (60 * 1000)) % (24 * 60);
    let hours = minutes_since_epoch / 60;
    let minutes = minutes_since_epoch % 60;
    format!("{:02}:{:02}", hours, minutes)
}

/// 分钟级成交量数据点
#[derive(Debug, Clone)]
pub struct VolumeMinuteData {
    /// 分钟边界时间戳（分钟级，秒和毫秒归零）
    pub minute_timestamp: u64,
    /// 该分钟内的总成交量
    pub total_volume: f64,
    /// 买单成交量
    pub buy_volume: f64,
    /// 卖单成交量
    pub sell_volume: f64,
    /// 该分钟内的交易笔数
    pub trade_count: u32,
}

/// 柱状图数据点
#[derive(Debug, Clone)]
struct BarData {
    timestamp: u64,
    volume: f64,
    buy_volume: f64,
    sell_volume: f64,
    height: usize, // 以字符单位计算的柱子高度
}

impl VolumeMinuteData {
    /// 创建新的分钟数据
    pub fn new(minute_timestamp: u64) -> Self {
        Self {
            minute_timestamp,
            total_volume: 0.0,
            buy_volume: 0.0,
            sell_volume: 0.0,
            trade_count: 0,
        }
    }

    /// 添加交易数据到该分钟
    pub fn add_trade(&mut self, volume: f64, is_buyer_maker: bool) {
        self.total_volume += volume;
        self.trade_count += 1;
        
        if is_buyer_maker {
            // 卖单（买方是taker）
            self.sell_volume += volume;
        } else {
            // 买单（卖方是taker）
            self.buy_volume += volume;
        }
    }
}

/// 成交量柱状图渲染器
pub struct VolumeBarChartRenderer {
    /// 分钟级数据存储，使用BTreeMap按时间排序
    minute_data: BTreeMap<u64, VolumeMinuteData>,
    /// 最大保留分钟数（滑动窗口）
    max_minutes: usize,
    /// 最大成交量（用于归一化显示）
    max_volume: f64,
    /// 最小成交量阈值（用于优化显示）
    min_volume_threshold: f64,
}

impl VolumeBarChartRenderer {
    /// 创建新的成交量柱状图渲染器
    pub fn new() -> Self {
        Self {
            minute_data: BTreeMap::new(),
            max_minutes: 30, // 默认保留30分钟
            max_volume: 0.0,
            min_volume_threshold: 0.0001, // 最小成交量阈值
        }
    }

    /// 创建带自定义配置的渲染器
    pub fn with_config(max_minutes: usize, min_volume_threshold: f64) -> Self {
        Self {
            minute_data: BTreeMap::new(),
            max_minutes,
            max_volume: 0.0,
            min_volume_threshold,
        }
    }

    /// 添加交易数据点（从价格图表数据中提取）
    pub fn add_trade_data(&mut self, timestamp: u64, volume: f64, is_buyer_maker: bool) {
        // 将时间戳转换为分钟边界（秒和毫秒归零）
        let minute_timestamp = self.get_minute_boundary(timestamp);
        
        // 获取或创建该分钟的数据条目
        let minute_data = self.minute_data
            .entry(minute_timestamp)
            .or_insert_with(|| VolumeMinuteData::new(minute_timestamp));
        
        // 添加交易数据
        minute_data.add_trade(volume, is_buyer_maker);
        
        // 更新最大成交量
        if minute_data.total_volume > self.max_volume {
            self.max_volume = minute_data.total_volume;
        }
        
        // 维护滑动窗口：删除超过最大分钟数的旧数据
        self.maintain_sliding_window();
    }

    /// 从价格图表数据批量同步数据
    pub fn sync_from_price_data<I>(&mut self, price_points: I)
    where
        I: Iterator<Item = (u64, f64, bool)>, // (timestamp, volume, is_buyer_maker)
    {
        // 为了避免重复累加相同数据，我们需要重新构建数据
        // 但保留时间结构，只重新计算成交量
        
        // 记录同步前的数据量
        let before_count = self.minute_data.len();
        
        // 临时存储新的分钟数据
        let mut new_minute_data: BTreeMap<u64, VolumeMinuteData> = BTreeMap::new();
        
        // 重新聚合所有交易数据
        for (timestamp, volume, is_buyer_maker) in price_points {
            let minute_timestamp = self.get_minute_boundary(timestamp);
            
            // 获取或创建该分钟的数据条目
            let minute_data = new_minute_data
                .entry(minute_timestamp)
                .or_insert_with(|| VolumeMinuteData::new(minute_timestamp));
            
            // 添加交易数据
            minute_data.add_trade(volume, is_buyer_maker);
        }
        
        // 用新数据替换旧数据
        self.minute_data = new_minute_data;
        
        // 维护滑动窗口
        self.maintain_sliding_window();
        
        // 重新计算最大成交量
        self.recalculate_max_volume();
        
        // 打印调试信息（可以通过日志查看）
        let after_count = self.minute_data.len();
        if after_count != before_count {
            log::debug!("Volume bar chart: {} -> {} minutes", before_count, after_count);
        }
    }

    /// 将时间戳转换为分钟边界
    fn get_minute_boundary(&self, timestamp: u64) -> u64 {
        // 将毫秒时间戳转换为分钟边界（归零秒和毫秒）
        let seconds = timestamp / 1000; // 转换为秒
        let minutes = seconds / 60; // 转换为分钟
        let minute_boundary_seconds = minutes * 60; // 归零到分钟边界
        minute_boundary_seconds * 1000 // 转换回毫秒
    }

    /// 维护滑动窗口，删除过期数据
    fn maintain_sliding_window(&mut self) {
        if self.minute_data.len() <= self.max_minutes {
            return;
        }

        // 计算需要删除的条目数
        let excess_count = self.minute_data.len() - self.max_minutes;
        
        // 获取最旧的键并删除
        let keys_to_remove: Vec<u64> = self.minute_data
            .keys()
            .take(excess_count)
            .copied()
            .collect();
        
        for key in keys_to_remove {
            self.minute_data.remove(&key);
        }
        
        // 重新计算最大成交量
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

    /// 渲染成交量柱状图
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if self.minute_data.is_empty() {
            self.render_empty_chart(f, area);
            return;
        }

        // 常量：最小单元代表的BTC数量 - 调整为更大的值以适应终端显示
        const BTC_PER_UNIT: f64 = 1.0; // 每个单位代表1 BTC，使柱子高度更合理
        
        // 计算图表内容区域（减去边框）
        let chart_height = area.height.saturating_sub(3) as usize; // 减去标题和边框
        let chart_width = area.width.saturating_sub(2) as usize; // 减去左右边框
        
        if chart_height == 0 || chart_width == 0 {
            self.render_empty_chart(f, area);
            return;
        }

        // 准备柱状图数据
        let chart_data = self.prepare_bar_chart_data(chart_width, chart_height, BTC_PER_UNIT);
        
        // 构建统计信息
        let total_minutes = self.minute_data.len();
        let total_volume: f64 = self.minute_data.values().map(|d| d.total_volume).sum();
        let avg_volume = if total_minutes > 0 { total_volume / total_minutes as f64 } else { 0.0 };
        
        let title = format!(
            "Volume Chart (1min) | {} mins | Avg: {:.3} BTC | Max: {:.3} BTC | Unit: {:.1} BTC",
            total_minutes, avg_volume, self.max_volume, BTC_PER_UNIT
        );

        // 创建柱状图内容
        let content = self.build_chart_content(chart_data, chart_height);

        // 创建段落widget来显示柱状图
        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White))
            )
            .alignment(Alignment::Left);

        f.render_widget(paragraph, area);
    }

    /// 准备柱状图数据 - 修改为占满窗体宽度并支持滑动
    fn prepare_bar_chart_data(&self, chart_width: usize, chart_height: usize, btc_per_unit: f64) -> Vec<BarData> {
        let data_count = self.minute_data.len();
        if data_count == 0 {
            return Vec::new();
        }

        // 使用整个图表宽度，每个字符位置显示一个bar
        let available_width = chart_width.saturating_sub(2); // 减去左右边框，每个位置显示一个bar
        
        let mut chart_data = Vec::new();
        
        // 如果数据点数量少于可用宽度，均匀分布
        if data_count <= available_width {
            // 数据点少，可以全部显示
            let sorted_data: Vec<_> = self.minute_data.iter().collect();
            
            for (i, (timestamp, data)) in sorted_data.iter().enumerate() {
                let height = self.calculate_bar_height(data.total_volume, chart_height, btc_per_unit);
                chart_data.push(BarData {
                    timestamp: **timestamp,
                    volume: data.total_volume,
                    buy_volume: data.buy_volume,
                    sell_volume: data.sell_volume,
                    height,
                });
            }
            
            // 填充剩余空间为空bar，确保占满整个宽度
            while chart_data.len() < available_width {
                chart_data.push(BarData {
                    timestamp: 0,
                    volume: 0.0,
                    buy_volume: 0.0,
                    sell_volume: 0.0,
                    height: 0,
                });
            }
        } else {
            // 数据点多，需要滑动窗口显示最新的数据
            let sorted_data: Vec<_> = self.minute_data.iter().collect();
            
            // 取最新的 available_width 个数据点
            let start_index = data_count.saturating_sub(available_width);
            
            for i in start_index..data_count {
                if let Some((timestamp, data)) = sorted_data.get(i) {
                    let height = self.calculate_bar_height(data.total_volume, chart_height, btc_per_unit);
                    chart_data.push(BarData {
                        timestamp: **timestamp,
                        volume: data.total_volume,
                        buy_volume: data.buy_volume,
                        sell_volume: data.sell_volume,
                        height,
                    });
                }
            }
        }

        chart_data
    }

    /// 计算柱子高度（以字符行数计算）- 基于实际BTC成交量动态增长，不设置最大限制
    fn calculate_bar_height(&self, volume: f64, max_height: usize, btc_per_unit: f64) -> usize {
        if volume <= 0.0 {
            return 0;
        }

        // 基于实际BTC成交量计算高度，每个字符行代表固定的BTC量
        // 每个字符行代表的BTC量 = btc_per_unit / 8 (因为有8个不同的Unicode块字符)
        let btc_per_char_line = btc_per_unit / 8.0;
        let total_char_lines_needed = (volume / btc_per_char_line).ceil() as usize;
        
        // 不设置最大限制，让柱子根据实际成交量自由增长
        // 如果超出可用高度，渲染时会自动处理（显示部分或调整显示区域）
        total_char_lines_needed.max(1) // 确保至少有1个字符高度
    }

    /// 构建图表内容
    fn build_chart_content(&self, chart_data: Vec<BarData>, chart_height: usize) -> Vec<Line> {
        let mut lines = Vec::new();
        
        if chart_data.is_empty() {
            lines.push(Line::from(Span::styled("No data available", Style::default().fg(Color::Gray))));
            return lines;
        }

        // 找到最高的柱子高度
        let max_bar_height = chart_data.iter().map(|bar| bar.height).max().unwrap_or(0);
        
        // 使用可用高度和最高柱子高度中的较大值，但要为时间轴预留1行
        let content_height = chart_height.saturating_sub(1); // 为时间轴预留1行
        let actual_display_height = max_bar_height.max(content_height);

        // 构建柱状图（从上往下渲染，但柱子从下往上增长）
        for row in 0..actual_display_height {
            let mut line_spans = Vec::new();
            
            // 计算当前行（从上数第几行）
            let current_row_from_bottom = actual_display_height - row - 1;
            
            // 为每个数据点生成字符
            for (i, bar) in chart_data.iter().enumerate() {
                let char_to_display = self.get_bar_char_at_height(bar, current_row_from_bottom);
                let color = self.get_bar_color(bar);
                
                line_spans.push(Span::styled(char_to_display.to_string(), Style::default().fg(color)));
                
                // 添加间隔（如果不是最后一个）
                if i < chart_data.len() - 1 {
                    line_spans.push(Span::raw(" "));
                }
            }
            
            lines.push(Line::from(line_spans));
        }

        // 添加时间轴标签 - 智能显示时间，避免过于密集
        let mut time_labels_str = String::new();
        let chart_len = chart_data.len();
        
        // 根据图表宽度决定显示间隔
        let time_display_interval = if chart_len <= 20 {
            1 // 20个以下每个都显示
        } else if chart_len <= 60 {
            3 // 21-60个每3个显示一次
        } else {
            5 // 60个以上每5个显示一次
        };
        
        for (i, bar) in chart_data.iter().enumerate() {
            if bar.timestamp == 0 {
                // 空数据点，显示空格
                time_labels_str.push(' ');
            } else if i % time_display_interval == 0 || i == chart_len - 1 {
                // 显示时间标签
                let time_str = format_timestamp_to_time(bar.timestamp);
                let short_time = if time_str.len() >= 5 { &time_str[0..5] } else { &time_str };
                time_labels_str.push_str(short_time);
            } else {
                // 占位符
                time_labels_str.push(' ');
            }
        }
        
        lines.push(Line::from(Span::styled(
            time_labels_str, 
            Style::default().fg(Color::Gray)
        )));

        lines
    }

    /// 获取指定高度处的柱子字符 - 使用 Unicode 精细显示（学习自 volume profile）
    fn get_bar_char_at_height(&self, bar: &BarData, row_from_bottom: usize) -> char {
        if bar.height == 0 {
            return ' ';
        }

        // 使用与 volume profile 相同的 Unicode 字符精细显示方法
        // 每个完整字符 █ 代表 8 个单位，部分字符代表 1-7 个单位
        let full_char_lines = bar.height / 8; // 完整的字符行数
        let partial_char_height = bar.height % 8; // 部分字符的高度

        if row_from_bottom < full_char_lines {
            // 完整字符行 - 使用满字符
            '█'
        } else if row_from_bottom == full_char_lines && partial_char_height > 0 {
            // 部分字符行 - 使用对应的 Unicode 部分字符
            match partial_char_height {
                1 => '▁', // 1/8 高度
                2 => '▂', // 2/8 高度
                3 => '▃', // 3/8 高度
                4 => '▄', // 4/8 高度
                5 => '▅', // 5/8 高度
                6 => '▆', // 6/8 高度
                7 => '▇', // 7/8 高度
                _ => ' ', // 不应该到达这里
            }
        } else {
            // 空白区域
            ' '
        }
    }

    /// 获取柱子颜色（基于买卖比例）
    fn get_bar_color(&self, bar: &BarData) -> Color {
        if bar.volume <= 0.0 {
            return Color::Gray;
        }

        let buy_ratio = bar.buy_volume / bar.volume;
        
        if buy_ratio > 0.6 {
            Color::Green // 买单占主导
        } else if buy_ratio < 0.4 {
            Color::Red   // 卖单占主导
        } else {
            Color::Yellow // 买卖均衡
        }
    }

    /// 渲染空图表
    fn render_empty_chart(&self, f: &mut Frame, area: Rect) {
        let paragraph = Paragraph::new("No volume data available")
            .block(
                Block::default()
                    .title("Volume Chart (1min) - No Data")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
            )
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }


    /// 获取统计信息
    pub fn get_stats(&self) -> VolumeBarChartStats {
        let total_minutes = self.minute_data.len();
        let total_volume: f64 = self.minute_data.values().map(|d| d.total_volume).sum();
        let total_trades: u32 = self.minute_data.values().map(|d| d.trade_count).sum();
        let total_buy_volume: f64 = self.minute_data.values().map(|d| d.buy_volume).sum();
        let total_sell_volume: f64 = self.minute_data.values().map(|d| d.sell_volume).sum();
        
        let avg_volume = if total_minutes > 0 { total_volume / total_minutes as f64 } else { 0.0 };
        let avg_trades = if total_minutes > 0 { total_trades as f64 / total_minutes as f64 } else { 0.0 };

        VolumeBarChartStats {
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

    /// 清空所有数据
    pub fn clear_data(&mut self) {
        self.minute_data.clear();
        self.max_volume = 0.0;
    }

    /// 设置最大分钟数
    pub fn set_max_minutes(&mut self, max_minutes: usize) {
        self.max_minutes = max_minutes;
        self.maintain_sliding_window();
    }

    /// 设置最小成交量阈值
    pub fn set_min_volume_threshold(&mut self, threshold: f64) {
        self.min_volume_threshold = threshold.max(0.0);
    }

    /// 获取当前数据点数量
    pub fn get_data_count(&self) -> usize {
        self.minute_data.len()
    }

    /// 获取最新分钟的成交量
    pub fn get_latest_volume(&self) -> Option<f64> {
        self.minute_data.values().last().map(|data| data.total_volume)
    }

    /// 获取时间范围
    pub fn get_time_range(&self) -> Option<(u64, u64)> {
        if self.minute_data.is_empty() {
            None
        } else {
            let min_time = *self.minute_data.keys().min().unwrap();
            let max_time = *self.minute_data.keys().max().unwrap();
            Some((min_time, max_time))
        }
    }
}

/// 成交量柱状图统计信息
#[derive(Debug, Clone, Default)]
pub struct VolumeBarChartStats {
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

impl Default for VolumeBarChartRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_bar_chart_creation() {
        let chart = VolumeBarChartRenderer::new();
        assert_eq!(chart.get_data_count(), 0);
        assert_eq!(chart.max_minutes, 30);
        assert_eq!(chart.max_volume, 0.0);
    }

    #[test]
    fn test_add_trade_data() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let mut chart = VolumeBarChartRenderer::new();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 添加买单
        chart.add_trade_data(timestamp, 0.001, false);
        assert_eq!(chart.get_data_count(), 1);
        
        // 同一分钟内添加卖单
        chart.add_trade_data(timestamp + 30000, 0.002, true); // +30秒
        assert_eq!(chart.get_data_count(), 1); // 仍然是同一分钟
        
        // 下一分钟添加交易
        chart.add_trade_data(timestamp + 70000, 0.003, false); // +70秒，下一分钟
        assert_eq!(chart.get_data_count(), 2);
    }

    #[test]
    fn test_minute_boundary_calculation() {
        let chart = VolumeBarChartRenderer::new();
        
        // 测试时间戳边界计算
        let timestamp1 = 1609459200000; // 2021-01-01 00:00:00.000
        let timestamp2 = 1609459230000; // 2021-01-01 00:00:30.000
        let timestamp3 = 1609459260000; // 2021-01-01 00:01:00.000
        
        let boundary1 = chart.get_minute_boundary(timestamp1);
        let boundary2 = chart.get_minute_boundary(timestamp2);
        let boundary3 = chart.get_minute_boundary(timestamp3);
        
        assert_eq!(boundary1, boundary2); // 同一分钟
        assert_ne!(boundary2, boundary3); // 不同分钟
        assert_eq!(boundary3 - boundary1, 60000); // 相差60秒
    }

    #[test]
    fn test_sliding_window() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let mut chart = VolumeBarChartRenderer::with_config(3, 0.0001);
        let base_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 添加4分钟的数据
        for i in 0..4 {
            let timestamp = base_timestamp + (i * 60 * 1000); // 每分钟
            chart.add_trade_data(timestamp, 0.001, false);
        }
        
        // 应该只保留最后3分钟的数据
        assert_eq!(chart.get_data_count(), 3);
    }

    #[test]
    fn test_stats_calculation() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let mut chart = VolumeBarChartRenderer::new();
        let base_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 添加测试数据
        chart.add_trade_data(base_timestamp, 0.001, false); // 买单
        chart.add_trade_data(base_timestamp, 0.002, true);  // 卖单
        chart.add_trade_data(base_timestamp + 60000, 0.003, false); // 下一分钟买单
        
        let stats = chart.get_stats();
        assert_eq!(stats.total_minutes, 2);
        assert_eq!(stats.total_volume, 0.006);
        assert_eq!(stats.total_trades, 3);
        assert_eq!(stats.buy_volume, 0.004);
        assert_eq!(stats.sell_volume, 0.002);
        assert_eq!(stats.avg_volume, 0.003);
    }

    #[test]
    fn test_clear_data() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let mut chart = VolumeBarChartRenderer::new();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        chart.add_trade_data(timestamp, 0.001, false);
        assert_eq!(chart.get_data_count(), 1);
        
        chart.clear_data();
        assert_eq!(chart.get_data_count(), 0);
        assert_eq!(chart.max_volume, 0.0);
    }

    #[test]
    fn test_volume_aggregation() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let mut chart = VolumeBarChartRenderer::new();
        let base_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // 同一分钟内多笔交易
        chart.add_trade_data(base_timestamp, 0.001, false);
        chart.add_trade_data(base_timestamp + 10000, 0.002, true); // +10秒
        chart.add_trade_data(base_timestamp + 30000, 0.003, false); // +30秒
        
        let stats = chart.get_stats();
        assert_eq!(stats.total_minutes, 1);
        assert_eq!(stats.total_volume, 0.006); // 聚合成交量
        assert_eq!(stats.total_trades, 3);
        assert_eq!(stats.max_volume, 0.006);
    }
    
    #[test]
    fn test_unicode_bar_chart_generation() {
        let mut chart = VolumeBarChartRenderer::new();
        
        // 添加一些测试数据，模拟不同成交量
        let base_timestamp = 1000000000000u64; // 固定基准时间戳
        
        // 添加递增的成交量数据
        for i in 0..5 {
            let timestamp = base_timestamp + (i * 60 * 1000); // 每分钟
            let volume = 0.1 * (i + 1) as f64; // 0.1, 0.2, 0.3, 0.4, 0.5 BTC
            chart.add_trade_data(timestamp, volume, i % 2 == 0); // 交替买卖
        }
        
        assert_eq!(chart.get_data_count(), 5);
        assert_eq!(chart.max_volume, 0.5);
        
        // 测试柱状图数据生成
        let chart_data = chart.prepare_bar_chart_data(80, 10, 0.1);
        assert_eq!(chart_data.len(), 5);
        
        // 验证高度计算
        // 0.1 BTC 应该对应 8 个字符高度 (0.1 BTC / (0.1/8) = 8)
        assert_eq!(chart_data[0].height, 8);
        // 0.5 BTC 应该对应 40 个字符高度，但限制在10以内
        assert_eq!(chart_data[4].height, 10);
    }
}