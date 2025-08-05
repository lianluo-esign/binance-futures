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
    pub sequence: u64, // 数据点序号，用于X轴显示
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

    /// 添加新的价格数据点
    pub fn add_price_point(&mut self, price: f64) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let point = PricePoint {
            timestamp,
            price,
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

    /// 更新价格范围
    fn update_price_range(&mut self) {
        if self.data_points.is_empty() {
            self.min_price = f64::MAX;
            self.max_price = f64::MIN;
            return;
        }

        self.min_price = self.data_points
            .iter()
            .map(|p| p.price)
            .fold(f64::INFINITY, f64::min);

        self.max_price = self.data_points
            .iter()
            .map(|p| p.price)
            .fold(f64::NEG_INFINITY, f64::max);
    }

    /// 渲染价格图表
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if self.data_points.is_empty() {
            // 如果没有数据，显示空图表
            self.render_empty_chart(f, area);
            return;
        }

        // 准备数据集
        let dataset_data: Vec<(f64, f64)> = self.data_points
            .iter()
            .map(|point| (point.sequence as f64, point.price))
            .collect();

        // 计算X轴范围
        let x_min = self.data_points.front().unwrap().sequence as f64;
        let x_max = self.data_points.back().unwrap().sequence as f64;
        let x_range = x_max - x_min;

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

        // 创建数据集
        let dataset = Dataset::default()
            .name("Price")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&dataset_data);

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

        // 创建图表
        let chart = Chart::new(vec![dataset])
            .block(
                Block::default()
                    .title("Price Chart")
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

    /// 获取统计信息
    pub fn get_stats(&self) -> PriceChartStats {
        if self.data_points.is_empty() {
            return PriceChartStats::default();
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
            min_price: self.min_price,
            max_price: self.max_price,
            avg_price: avg,
            std_deviation: std_dev,
            latest_price: self.get_latest_price(),
            price_range: self.max_price - self.min_price,
        }
    }
}

/// 价格图表统计信息
#[derive(Debug, Clone, Default)]
pub struct PriceChartStats {
    pub data_points: usize,
    pub min_price: f64,
    pub max_price: f64,
    pub avg_price: f64,
    pub std_deviation: f64,
    pub latest_price: Option<f64>,
    pub price_range: f64,
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
        
        chart.add_price_point(100.0);
        assert_eq!(chart.get_data_count(), 1);
        assert_eq!(chart.get_latest_price(), Some(100.0));
        
        chart.add_price_point(101.0);
        assert_eq!(chart.get_data_count(), 2);
        assert_eq!(chart.get_latest_price(), Some(101.0));
    }

    #[test]
    fn test_sliding_window() {
        let mut chart = PriceChartRenderer::new(3);
        
        chart.add_price_point(100.0);
        chart.add_price_point(101.0);
        chart.add_price_point(102.0);
        assert_eq!(chart.get_data_count(), 3);
        
        chart.add_price_point(103.0);
        assert_eq!(chart.get_data_count(), 3); // 应该保持在最大值
        assert_eq!(chart.get_latest_price(), Some(103.0));
    }

    #[test]
    fn test_price_range_calculation() {
        let mut chart = PriceChartRenderer::new(1000);
        
        chart.add_price_point(100.0);
        chart.add_price_point(105.0);
        chart.add_price_point(95.0);
        
        let (min, max) = chart.get_price_range();
        assert_eq!(min, 95.0);
        assert_eq!(max, 105.0);
    }

    #[test]
    fn test_stats_calculation() {
        let mut chart = PriceChartRenderer::new(1000);
        
        chart.add_price_point(100.0);
        chart.add_price_point(102.0);
        chart.add_price_point(98.0);
        
        let stats = chart.get_stats();
        assert_eq!(stats.data_points, 3);
        assert_eq!(stats.min_price, 98.0);
        assert_eq!(stats.max_price, 102.0);
        assert_eq!(stats.avg_price, 100.0);
        assert_eq!(stats.latest_price, Some(98.0));
    }

    #[test]
    fn test_clear_data() {
        let mut chart = PriceChartRenderer::new(1000);
        
        chart.add_price_point(100.0);
        chart.add_price_point(101.0);
        assert_eq!(chart.get_data_count(), 2);
        
        chart.clear_data();
        assert_eq!(chart.get_data_count(), 0);
        assert_eq!(chart.get_latest_price(), None);
    }
}