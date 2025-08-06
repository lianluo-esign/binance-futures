use ratatui::{
    layout::Rect,
    style::{Color, Style},
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Frame,
};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

/// 价格图表数据点
#[derive(Debug, Clone)]
pub struct PricePoint {
    pub timestamp: u64,
    pub price: f64,
    pub volume: f64,
    pub is_buyer_maker: bool, // true表示卖单(红色)，false表示买单(绿色)
    pub sequence: u64, // 数据点序号，用于X轴显示
}

/// 交易数据点
#[derive(Debug, Clone)]
pub struct TradePoint {
    pub timestamp: u64,
    pub price: f64,
    pub volume: f64,
    pub is_buyer_maker: bool, // true表示卖单(红色)，false表示买单(绿色)
    pub sequence: u64, // 数据点序号，用于X轴显示
}

/// 交易圆点大小枚举
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TradeCircleSize {
    Small,  // 0.0001 BTC 及以下
    Medium, // 0.001 BTC (10倍)
    Large,  // 0.01 BTC (100倍)
}

impl TradeCircleSize {
    /// 根据成交量确定圆点大小
    pub fn from_volume(volume: f64) -> Self {
        if volume >= 0.01 {
            Self::Large
        } else if volume >= 0.001 {
            Self::Medium
        } else {
            Self::Small
        }
    }
    
    /// 获取对应的 Unicode 圆点字符
    pub fn get_marker(&self, is_buyer_maker: bool) -> &'static str {
        match (self, is_buyer_maker) {
            // 买单 (绿色)
            (Self::Small, false) => "●",   // 小圆点
            (Self::Medium, false) => "⬢",  // 中型六边形
            (Self::Large, false) => "⬣",   // 大型六边形
            // 卖单 (红色)  
            (Self::Small, true) => "●",    // 小圆点
            (Self::Medium, true) => "⬢",   // 中型六边形 
            (Self::Large, true) => "⬣",    // 大型六边形
        }
    }
    
    /// 获取对应的颜色
    pub fn get_color(&self, is_buyer_maker: bool) -> Color {
        if is_buyer_maker {
            Color::Red   // 卖单用红色
        } else {
            Color::Green // 买单用绿色
        }
    }
}

/// 价格图表渲染器
pub struct PriceChartRenderer {
    data_points: VecDeque<PricePoint>,
    max_data_points: usize,
    sequence_counter: u64,
    min_price: f64,
    max_price: f64,
    price_scale_interval: f64, // Y轴刻度间隔（1美元）
}

impl PriceChartRenderer {
    /// 创建新的价格图表渲染器
    pub fn new(max_data_points: usize) -> Self {
        Self {
            data_points: VecDeque::with_capacity(max_data_points),
            max_data_points,
            sequence_counter: 0,
            min_price: f64::MAX,
            max_price: f64::MIN,
            price_scale_interval: 1.0, // 1美元间隔
        }
    }

    /// 添加新的价格数据点（带交易信息）
    pub fn add_price_point(&mut self, price: f64, volume: f64, is_buyer_maker: bool) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let point = PricePoint {
            timestamp,
            price,
            volume,
            is_buyer_maker,
            sequence: self.sequence_counter,
        };

        // 添加数据点
        self.data_points.push_back(point);
        self.sequence_counter += 1;

        // 保持滑动窗口大小
        if self.data_points.len() > self.max_data_points {
            self.data_points.pop_front();
        }

        // 更新价格范围
        self.update_price_range();
    }

    /// 添加新的价格数据点（兼容旧接口，默认买单）
    pub fn add_price_point_simple(&mut self, price: f64) {
        self.add_price_point(price, 0.001, false); // 默认小量买单
    }

    /// 添加新的交易数据点（统一使用add_price_point）
    #[deprecated(since = "1.0.0", note = "Use add_price_point instead")]
    pub fn add_trade_point(&mut self, price: f64, volume: f64, is_buyer_maker: bool) {
        self.add_price_point(price, volume, is_buyer_maker);
    }


    /// 更新价格范围
    fn update_price_range(&mut self) {
        let mut min_price = f64::INFINITY;
        let mut max_price = f64::NEG_INFINITY;

        // 检查价格数据点
        for point in &self.data_points {
            min_price = min_price.min(point.price);
            max_price = max_price.max(point.price);
        }

        // 如果有有效数据，更新范围
        if min_price != f64::INFINITY && max_price != f64::NEG_INFINITY {
            self.min_price = min_price;
            self.max_price = max_price;
        } else {
            self.min_price = f64::MAX;
            self.max_price = f64::MIN;
        }
    }

    /// 渲染价格图表
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if self.data_points.is_empty() {
            // 如果没有数据，显示空图表
            self.render_empty_chart(f, area);
            return;
        }

        // 计算X轴范围
        let x_min = self.data_points.front().unwrap().sequence as f64;
        let x_max = self.data_points.back().unwrap().sequence as f64;

        // 计算Y轴刻度
        let mut y_min = (self.min_price / self.price_scale_interval).floor() * self.price_scale_interval;
        let mut y_max = (self.max_price / self.price_scale_interval).ceil() * self.price_scale_interval;
        
        // 确保价格范围至少有一定的跨度（仅用于渲染）
        let price_range = y_max - y_min;
        if price_range < 10.0 {
            let center = (y_min + y_max) / 2.0;
            y_min = center - 5.0;
            y_max = center + 5.0;
        }

        // 按买卖方向分组数据点
        let buy_points: Vec<(f64, f64)> = self.data_points
            .iter()
            .filter(|point| !point.is_buyer_maker) // 买单
            .map(|point| (point.sequence as f64, point.price))
            .collect();

        let sell_points: Vec<(f64, f64)> = self.data_points
            .iter()
            .filter(|point| point.is_buyer_maker) // 卖单
            .map(|point| (point.sequence as f64, point.price))
            .collect();

        // 准备数据集
        let mut datasets = Vec::new();

        // 添加买单数据集（绿色圆点）
        if !buy_points.is_empty() {
            let buy_dataset = Dataset::default()
                .name("Buy Orders")
                .marker(symbols::Marker::Dot)
                .graph_type(GraphType::Scatter)
                .style(Style::default().fg(Color::Green))
                .data(&buy_points);
            datasets.push(buy_dataset);
        }

        // 添加卖单数据集（红色圆点）
        if !sell_points.is_empty() {
            let sell_dataset = Dataset::default()
                .name("Sell Orders")
                .marker(symbols::Marker::Dot)
                .graph_type(GraphType::Scatter)
                .style(Style::default().fg(Color::Red))
                .data(&sell_points);
            datasets.push(sell_dataset);
        }

        // 创建X轴
        let x_axis = Axis::default()
            .title("Data Points")
            .style(Style::default().fg(Color::Gray))
            .bounds([x_min, x_max])
            .labels(vec![
                format!("{:.0}", x_min),
                format!("{:.0}", (x_min + x_max) / 2.0),
                format!("{:.0}", x_max),
            ]);

        // 创建Y轴（价格轴，1美元间隔）
        let y_labels = self.generate_y_axis_labels(y_min, y_max);
        let y_axis = Axis::default()
            .title("Price (USD)")
            .style(Style::default().fg(Color::Gray))
            .bounds([y_min, y_max])
            .labels(y_labels);

        // 创建图表标题
        let buy_count = buy_points.len();
        let sell_count = sell_points.len();
        let title = format!("Price Chart (Buy: {} | Sell: {})", buy_count, sell_count);

        // 创建图表
        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White))
            )
            .x_axis(x_axis)
            .y_axis(y_axis);

        f.render_widget(chart, area);
    }

    /// 渲染空图表
    fn render_empty_chart(&self, f: &mut Frame, area: Rect) {
        let empty_dataset = Dataset::default()
            .name("Price")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Gray))
            .data(&[]);

        let x_axis = Axis::default()
            .title("Data Points")
            .style(Style::default().fg(Color::Gray))
            .bounds([0.0, 100.0])
            .labels(vec!["0".to_string(), "50".to_string(), "100".to_string()]);

        let y_axis = Axis::default()
            .title("Price (USD)")
            .style(Style::default().fg(Color::Gray))
            .bounds([100000.0, 120000.0])
            .labels(vec!["100000".to_string(), "110000".to_string(), "120000".to_string()]);

        let chart = Chart::new(vec![empty_dataset])
            .block(
                Block::default()
                    .title("Price Chart (No Data)")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
            )
            .x_axis(x_axis)
            .y_axis(y_axis);

        f.render_widget(chart, area);
    }

    /// 生成Y轴标签（1美元间隔）
    fn generate_y_axis_labels(&self, y_min: f64, y_max: f64) -> Vec<String> {
        let mut labels = Vec::new();
        let range = y_max - y_min;
        
        // 根据价格范围决定标签数量
        let label_count = if range <= 20.0 {
            // 小范围：每1美元一个标签
            ((y_max - y_min) / self.price_scale_interval) as usize + 1
        } else if range <= 100.0 {
            // 中等范围：每5美元一个标签
            5
        } else {
            // 大范围：只显示3个标签
            3
        };

        if label_count <= 1 {
            labels.push(format!("{:.0}", (y_min + y_max) / 2.0));
        } else {
            for i in 0..label_count {
                let value = y_min + (range * i as f64 / (label_count - 1) as f64);
                labels.push(format!("{:.0}", value));
            }
        }

        labels
    }

    /// 获取当前数据点数量
    pub fn get_data_count(&self) -> usize {
        self.data_points.len()
    }

    /// 获取最新价格
    pub fn get_latest_price(&self) -> Option<f64> {
        self.data_points.back().map(|p| p.price)
    }

    /// 获取价格范围
    pub fn get_price_range(&self) -> (f64, f64) {
        (self.min_price, self.max_price)
    }

    /// 清空数据
    pub fn clear_data(&mut self) {
        self.data_points.clear();
        self.sequence_counter = 0;
        self.min_price = f64::MAX;
        self.max_price = f64::MIN;
    }

    /// 获取买单数量
    pub fn get_buy_count(&self) -> usize {
        self.data_points.iter().filter(|p| !p.is_buyer_maker).count()
    }

    /// 获取卖单数量
    pub fn get_sell_count(&self) -> usize {
        self.data_points.iter().filter(|p| p.is_buyer_maker).count()
    }

    /// 设置最大数据点数量
    pub fn set_max_data_points(&mut self, max_points: usize) {
        self.max_data_points = max_points;
        
        // 如果当前数据点超过新的限制，移除多余的点
        while self.data_points.len() > max_points {
            self.data_points.pop_front();
        }
        
        // 更新价格范围
        self.update_price_range();
    }

    /// 设置价格刻度间隔
    pub fn set_price_scale_interval(&mut self, interval: f64) {
        self.price_scale_interval = interval.abs().max(0.01); // 确保间隔为正数且不小于0.01
    }

    /// 获取所有数据点的只读访问（用于数据同步）
    pub fn get_data_points(&self) -> impl Iterator<Item = &PricePoint> {
        self.data_points.iter()
    }

    /// 获取最近N个数据点的只读访问
    pub fn get_recent_data_points(&self, count: usize) -> impl Iterator<Item = &PricePoint> {
        let start_index = self.data_points.len().saturating_sub(count);
        self.data_points.iter().skip(start_index)
    }

    /// 获取指定时间范围内的数据点
    pub fn get_data_points_in_range(&self, start_timestamp: u64, end_timestamp: u64) -> impl Iterator<Item = &PricePoint> {
        self.data_points.iter()
            .filter(move |point| point.timestamp >= start_timestamp && point.timestamp <= end_timestamp)
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> PriceChartStats {
        // 计算交易统计
        let buy_trades = self.get_buy_count();
        let sell_trades = self.get_sell_count();
        
        // 如果没有价格数据点，返回简化的统计信息
        if self.data_points.is_empty() {
            return PriceChartStats {
                data_points: 0,
                trade_points: 0,
                min_price: self.min_price,
                max_price: self.max_price,
                avg_price: 0.0,
                std_deviation: 0.0,
                latest_price: None,
                price_range: self.max_price - self.min_price,
                buy_trades,
                sell_trades,
                trade_points_enabled: true, // 现在所有点都是“交易点”
            };
        }

        let prices: Vec<f64> = self.data_points.iter().map(|p| p.price).collect();
        let sum: f64 = prices.iter().sum();
        let avg = sum / prices.len() as f64;

        // 计算标准差
        let variance: f64 = prices.iter()
            .map(|price| (price - avg).powi(2))
            .sum::<f64>() / prices.len() as f64;
        let std_dev = variance.sqrt();

        PriceChartStats {
            data_points: self.data_points.len(),
            trade_points: self.data_points.len(), // 所有点都是交易点
            min_price: self.min_price,
            max_price: self.max_price,
            avg_price: avg,
            std_deviation: std_dev,
            latest_price: self.get_latest_price(),
            price_range: self.max_price - self.min_price,
            buy_trades,
            sell_trades,
            trade_points_enabled: true, // 所有点都是交易点
        }
    }
}

/// 价格图表统计信息
#[derive(Debug, Clone, Default)]
pub struct PriceChartStats {
    pub data_points: usize,
    pub trade_points: usize, // 交易点数量
    pub min_price: f64,
    pub max_price: f64,
    pub avg_price: f64,
    pub std_deviation: f64,
    pub latest_price: Option<f64>,
    pub price_range: f64,
    pub buy_trades: usize,  // 买单数量
    pub sell_trades: usize, // 卖单数量
    pub trade_points_enabled: bool, // 是否启用交易点显示
}

impl Default for PriceChartRenderer {
    fn default() -> Self {
        Self::new(10000) // 默认10000个数据点
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_chart_creation() {
        let chart = PriceChartRenderer::new(1000);
        assert_eq!(chart.get_data_count(), 0);
        assert_eq!(chart.get_latest_price(), None);
    }

    #[test]
    fn test_add_price_point() {
        let mut chart = PriceChartRenderer::new(1000);
        
        chart.add_price_point(100.0, 0.001, false); // 买单
        assert_eq!(chart.get_data_count(), 1);
        assert_eq!(chart.get_latest_price(), Some(100.0));
        assert_eq!(chart.get_buy_count(), 1);
        assert_eq!(chart.get_sell_count(), 0);
        
        chart.add_price_point(101.0, 0.002, true); // 卖单
        assert_eq!(chart.get_data_count(), 2);
        assert_eq!(chart.get_latest_price(), Some(101.0));
        assert_eq!(chart.get_buy_count(), 1);
        assert_eq!(chart.get_sell_count(), 1);
    }

    #[test]
    fn test_sliding_window() {
        let mut chart = PriceChartRenderer::new(3);
        
        chart.add_price_point(100.0, 0.001, false);
        chart.add_price_point(101.0, 0.001, false);
        chart.add_price_point(102.0, 0.001, true);
        assert_eq!(chart.get_data_count(), 3);
        
        chart.add_price_point(103.0, 0.001, true);
        assert_eq!(chart.get_data_count(), 3); // 应该保持在最大值
        assert_eq!(chart.get_latest_price(), Some(103.0));
    }

    #[test]
    fn test_price_range_calculation() {
        let mut chart = PriceChartRenderer::new(1000);
        
        chart.add_price_point(100.0, 0.001, false);
        chart.add_price_point(105.0, 0.001, true);
        chart.add_price_point(95.0, 0.001, false);
        
        let (min, max) = chart.get_price_range();
        assert_eq!(min, 95.0);
        assert_eq!(max, 105.0);
    }

    #[test]
    fn test_stats_calculation() {
        let mut chart = PriceChartRenderer::new(1000);
        
        chart.add_price_point(100.0, 0.001, false); // 买单
        chart.add_price_point(102.0, 0.001, true);  // 卖单
        chart.add_price_point(98.0, 0.001, false);  // 买单
        
        let stats = chart.get_stats();
        assert_eq!(stats.data_points, 3);
        assert_eq!(stats.min_price, 98.0);
        assert_eq!(stats.max_price, 102.0);
        assert_eq!(stats.avg_price, 100.0);
        assert_eq!(stats.latest_price, Some(98.0));
        assert_eq!(stats.buy_trades, 2);
        assert_eq!(stats.sell_trades, 1);
    }

    #[test]
    fn test_clear_data() {
        let mut chart = PriceChartRenderer::new(1000);
        
        chart.add_price_point(100.0, 0.001, false);
        chart.add_price_point(101.0, 0.001, true);
        assert_eq!(chart.get_data_count(), 2);
        
        chart.clear_data();
        assert_eq!(chart.get_data_count(), 0);
        assert_eq!(chart.get_latest_price(), None);
    }

    #[test]
    fn test_buy_sell_counts() {
        let mut chart = PriceChartRenderer::new(1000);
        
        // 添加买单交易点
        chart.add_price_point(100.0, 0.005, false);
        assert_eq!(chart.get_buy_count(), 1);
        assert_eq!(chart.get_sell_count(), 0);
        
        // 添加卖单交易点
        chart.add_price_point(101.0, 0.002, true);
        assert_eq!(chart.get_buy_count(), 1);
        assert_eq!(chart.get_sell_count(), 1);
        
        let stats = chart.get_stats();
        assert_eq!(stats.buy_trades, 1);
        assert_eq!(stats.sell_trades, 1);
    }

    #[test]
    fn test_trade_circle_size() {
        assert_eq!(TradeCircleSize::from_volume(0.00005), TradeCircleSize::Small);
        assert_eq!(TradeCircleSize::from_volume(0.005), TradeCircleSize::Medium);
        assert_eq!(TradeCircleSize::from_volume(0.05), TradeCircleSize::Large);
    }

    #[test]
    fn test_simple_api_compatibility() {
        let mut chart = PriceChartRenderer::new(1000);
        
        // 测试兼容性接口
        chart.add_price_point_simple(100.0);
        assert_eq!(chart.get_data_count(), 1);
        assert_eq!(chart.get_buy_count(), 1); // 默认为买单
        assert_eq!(chart.get_sell_count(), 0);
    }

    #[test]
    fn test_price_chart_edge_cases() {
        let mut chart = PriceChartRenderer::new(100);
        
        // 测试相同价格
        chart.add_price_point(100.0, 0.001, false);
        chart.add_price_point(100.0, 0.001, true);
        chart.add_price_point(100.0, 0.001, false);
        
        let (min_price, max_price) = chart.get_price_range();
        // 当所有价格相同时，min和max应该相等
        assert_eq!(min_price, max_price);
        assert_eq!(min_price, 100.0);
        
        // 测试极端价格值
        chart.clear_data();
        chart.add_price_point(0.01, 0.001, false);
        chart.add_price_point(1000000.0, 0.001, true);
        
        let (min_price, max_price) = chart.get_price_range();
        assert_eq!(min_price, 0.01);
        assert_eq!(max_price, 1000000.0);
    }
}