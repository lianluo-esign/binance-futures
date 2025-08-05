use std::time::{Duration, Instant};

/// 价格跟踪器 - 管理价格跟踪和窗口居中逻辑
pub struct PriceTracker {
    last_best_bid: Option<f64>,
    center_threshold: f64,
    auto_center_enabled: bool,
    smooth_scroll_enabled: bool,
    price_tolerance: f64,
    last_center_time: Option<Instant>,
    min_center_interval: Duration,
}

impl PriceTracker {
    /// 创建新的价格跟踪器
    pub fn new() -> Self {
        Self {
            last_best_bid: None,
            center_threshold: 1.0, // 价格变化1美元时触发重新居中（提高敏感度）
            auto_center_enabled: true,
            smooth_scroll_enabled: true,
            price_tolerance: 0.5, // 价格匹配容差（放宽以适应聚合）
            last_center_time: None,
            min_center_interval: Duration::from_millis(200), // 最小居中间隔200ms（提高响应速度）
        }
    }

    /// 创建带自定义配置的价格跟踪器
    pub fn with_config(
        center_threshold: f64,
        price_tolerance: f64,
        min_center_interval: Duration,
    ) -> Self {
        Self {
            last_best_bid: None,
            center_threshold,
            auto_center_enabled: true,
            smooth_scroll_enabled: true,
            price_tolerance,
            last_center_time: None,
            min_center_interval,
        }
    }

    /// 判断是否需要重新居中
    pub fn should_recenter(&self, current_best_bid: f64) -> bool {
        if !self.auto_center_enabled {
            return false;
        }

        // 检查最小时间间隔
        if let Some(last_time) = self.last_center_time {
            if last_time.elapsed() < self.min_center_interval {
                return false;
            }
        }

        // 检查价格变化
        match self.last_best_bid {
            Some(last_bid) => {
                let price_change = (current_best_bid - last_bid).abs();
                price_change >= self.center_threshold
            }
            None => true, // 首次设置时需要居中
        }
    }

    /// 计算居中偏移
    pub fn calculate_center_offset(
        &self,
        best_bid: f64,
        price_levels: &[f64],
        visible_rows: usize,
    ) -> usize {
        if price_levels.is_empty() || visible_rows == 0 {
            return 0;
        }

        // 找到best_bid在价格列表中的索引
        let best_bid_index = match self.find_price_index(best_bid, price_levels) {
            Some(index) => index,
            None => {
                // 如果找不到精确匹配，找最接近的价格
                self.find_closest_price_index(best_bid, price_levels)
            }
        };

        // 计算居中偏移，使best_bid显示在窗口中间
        let center_offset = best_bid_index.saturating_sub(visible_rows / 2);

        // 确保偏移不超出有效范围
        let max_offset = price_levels.len().saturating_sub(visible_rows);
        center_offset.min(max_offset)
    }

    /// 更新价格跟踪状态 - 改进版本，支持多种价格参考
    pub fn update_tracking(&mut self, reference_price: Option<f64>) {
        if let Some(price) = reference_price {
            if self.should_recenter(price) {
                self.last_best_bid = Some(price);
                self.last_center_time = Some(Instant::now());
            }
        }
    }

    /// 启用/禁用自动居中
    pub fn set_auto_center_enabled(&mut self, enabled: bool) {
        self.auto_center_enabled = enabled;
        if enabled {
            // 重新启用时重置状态
            self.last_center_time = None;
        }
    }

    /// 启用/禁用平滑滚动
    pub fn enable_smooth_scroll(&mut self, enabled: bool) {
        self.smooth_scroll_enabled = enabled;
    }

    /// 设置居中阈值
    pub fn set_center_threshold(&mut self, threshold: f64) {
        self.center_threshold = threshold.abs(); // 确保为正数
    }

    /// 设置价格容差
    pub fn set_price_tolerance(&mut self, tolerance: f64) {
        self.price_tolerance = tolerance.abs(); // 确保为正数
    }

    /// 强制触发重新居中
    pub fn force_recenter(&mut self) {
        self.last_center_time = None;
        self.last_best_bid = None;
    }

    /// 获取当前配置
    pub fn get_config(&self) -> PriceTrackerConfig {
        PriceTrackerConfig {
            center_threshold: self.center_threshold,
            auto_center_enabled: self.auto_center_enabled,
            smooth_scroll_enabled: self.smooth_scroll_enabled,
            price_tolerance: self.price_tolerance,
            min_center_interval: self.min_center_interval,
        }
    }

    /// 在价格列表中查找指定价格的索引 - 改进版本，支持聚合价格匹配
    fn find_price_index(&self, target_price: f64, price_levels: &[f64]) -> Option<usize> {
        // 首先尝试精确匹配或容差范围内匹配
        if let Some(index) = price_levels.iter()
            .position(|&price| (price - target_price).abs() < self.price_tolerance) {
            return Some(index);
        }
        
        // 如果找不到，尝试匹配聚合后的价格（1美元精度）
        let aggregated_target = (target_price / 1.0).floor() * 1.0;
        price_levels.iter()
            .position(|&price| (price - aggregated_target).abs() < 0.01)
    }

    /// 找到最接近目标价格的索引 - 改进版本，优先匹配聚合价格
    fn find_closest_price_index(&self, target_price: f64, price_levels: &[f64]) -> usize {
        if price_levels.is_empty() {
            return 0;
        }

        // 首先尝试找到聚合价格的精确匹配
        let aggregated_target = (target_price / 1.0).floor() * 1.0;
        if let Some(exact_index) = price_levels.iter()
            .position(|&price| (price - aggregated_target).abs() < 0.01) {
            return exact_index;
        }

        // 如果没有精确匹配，找最接近的价格
        let mut closest_index = 0;
        let mut closest_distance = (price_levels[0] - target_price).abs();

        for (i, &price) in price_levels.iter().enumerate() {
            let distance = (price - target_price).abs();
            if distance < closest_distance {
                closest_distance = distance;
                closest_index = i;
            }
        }

        // 对于价格在两个层级之间的情况，应用智能选择逻辑
        if closest_index > 0 && closest_index < price_levels.len() - 1 {
            let current_price = price_levels[closest_index];
            
            // 检查是否有更好的聚合价格匹配
            let upper_price = price_levels[closest_index - 1];
            let lower_price = price_levels[closest_index + 1];
            
            // 计算到聚合价格的距离
            let distance_to_current_agg = (current_price - aggregated_target).abs();
            let distance_to_upper_agg = (upper_price - aggregated_target).abs();
            let distance_to_lower_agg = (lower_price - aggregated_target).abs();
            
            // 选择最接近聚合价格的层级
            if distance_to_upper_agg < distance_to_current_agg && distance_to_upper_agg <= distance_to_lower_agg {
                closest_index -= 1;
            } else if distance_to_lower_agg < distance_to_current_agg && distance_to_lower_agg < distance_to_upper_agg {
                closest_index += 1;
            }
        }

        closest_index
    }

    /// 计算平滑滚动的中间步骤
    pub fn calculate_smooth_scroll_steps(
        &self,
        current_offset: usize,
        target_offset: usize,
        steps: usize,
    ) -> Vec<usize> {
        if !self.smooth_scroll_enabled || steps <= 1 || current_offset == target_offset {
            return vec![target_offset];
        }

        let mut scroll_steps = Vec::new();
        let diff = target_offset as i32 - current_offset as i32;
        let step_size = diff as f64 / steps as f64;

        for i in 1..=steps {
            let intermediate_offset = current_offset as f64 + (step_size * i as f64);
            scroll_steps.push(intermediate_offset.round() as usize);
        }

        scroll_steps
    }

    /// 检查价格是否在有效范围内 - 改进版本，支持聚合价格匹配
    pub fn is_price_in_range(&self, price: f64, price_levels: &[f64]) -> bool {
        if price_levels.is_empty() {
            return false;
        }

        // 首先检查是否有直接匹配或容差范围内的匹配
        if price_levels.iter().any(|&p| (p - price).abs() < self.price_tolerance) {
            return true;
        }

        // 检查聚合价格是否存在
        let aggregated_price = (price / 1.0).floor() * 1.0;
        if price_levels.iter().any(|&p| (p - aggregated_price).abs() < 0.01) {
            return true;
        }

        // 最后检查价格是否在范围内
        let min_price = price_levels.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_price = price_levels.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        price >= min_price && price <= max_price
    }
}

/// 价格跟踪器配置
#[derive(Debug, Clone)]
pub struct PriceTrackerConfig {
    pub center_threshold: f64,
    pub auto_center_enabled: bool,
    pub smooth_scroll_enabled: bool,
    pub price_tolerance: f64,
    pub min_center_interval: Duration,
}

impl Default for PriceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_recenter() {
        let mut tracker = PriceTracker::new();
        
        // 首次设置应该触发居中
        assert!(tracker.should_recenter(100.0));
        
        // 手动设置last_best_bid而不触发时间检查
        tracker.last_best_bid = Some(100.0);
        
        // 小幅变化不应该触发居中
        assert!(!tracker.should_recenter(100.05));
        
        // 大幅变化应该触发居中
        assert!(tracker.should_recenter(100.2));
    }

    #[test]
    fn test_calculate_center_offset() {
        let tracker = PriceTracker::new();
        let price_levels = vec![105.0, 104.0, 103.0, 102.0, 101.0, 100.0, 99.0, 98.0, 97.0, 96.0];
        
        // 测试居中计算
        let offset = tracker.calculate_center_offset(100.0, &price_levels, 6);
        assert_eq!(offset, 2); // 100.0在索引5，居中应该是5-3=2
        
        // 测试边界情况
        let offset = tracker.calculate_center_offset(105.0, &price_levels, 6);
        assert_eq!(offset, 0); // 不能为负数
        
        let offset = tracker.calculate_center_offset(96.0, &price_levels, 6);
        assert_eq!(offset, 4); // 不能超出范围
    }

    #[test]
    fn test_find_price_index() {
        let tracker = PriceTracker::new();
        let price_levels = vec![105.0, 104.0, 103.0, 102.0, 101.0];
        
        assert_eq!(tracker.find_price_index(103.0, &price_levels), Some(2));
        assert_eq!(tracker.find_price_index(106.0, &price_levels), None);
        
        // 测试容差范围内的匹配
        assert_eq!(tracker.find_price_index(103.0005, &price_levels), Some(2));
    }

    #[test]
    fn test_find_closest_price_index() {
        let tracker = PriceTracker::new();
        let price_levels = vec![105.0, 104.0, 103.0, 102.0, 101.0];
        
        assert_eq!(tracker.find_closest_price_index(103.4, &price_levels), 2); // 更接近103.0
        assert_eq!(tracker.find_closest_price_index(103.6, &price_levels), 1); // 更接近104.0
        assert_eq!(tracker.find_closest_price_index(106.0, &price_levels), 0); // 更接近105.0
        assert_eq!(tracker.find_closest_price_index(100.0, &price_levels), 4); // 更接近101.0
    }

    #[test]
    fn test_smooth_scroll_steps() {
        let tracker = PriceTracker::new();
        
        let steps = tracker.calculate_smooth_scroll_steps(0, 10, 5);
        assert_eq!(steps, vec![2, 4, 6, 8, 10]);
        
        let steps = tracker.calculate_smooth_scroll_steps(10, 0, 5);
        assert_eq!(steps, vec![8, 6, 4, 2, 0]);
        
        // 测试相同位置
        let steps = tracker.calculate_smooth_scroll_steps(5, 5, 3);
        assert_eq!(steps, vec![5]);
    }

    #[test]
    fn test_auto_center_toggle() {
        let mut tracker = PriceTracker::new();
        
        assert!(tracker.should_recenter(100.0));
        
        tracker.set_auto_center_enabled(false);
        assert!(!tracker.should_recenter(100.0));
        
        tracker.set_auto_center_enabled(true);
        assert!(tracker.should_recenter(100.0));
    }

    #[test]
    fn test_is_price_in_range() {
        let tracker = PriceTracker::new();
        let price_levels = vec![105.0, 104.0, 103.0, 102.0, 101.0];
        
        assert!(tracker.is_price_in_range(103.0, &price_levels));
        assert!(tracker.is_price_in_range(101.0, &price_levels));
        assert!(tracker.is_price_in_range(105.0, &price_levels));
        assert!(!tracker.is_price_in_range(100.0, &price_levels));
        assert!(!tracker.is_price_in_range(106.0, &price_levels));
        
        // 测试空列表
        assert!(!tracker.is_price_in_range(100.0, &[]));
    }
}