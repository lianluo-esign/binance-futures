use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use ordered_float::OrderedFloat;

/// Time window duration for order flow accumulation (2 seconds in milliseconds)
const ORDER_FLOW_WINDOW_MS: u64 = 2000;

/// Represents the latest single trade order flow data for a specific price level
#[derive(Debug, Clone)]
pub struct OrderFlowLevel {
    /// Latest single aggressive buy volume (market orders hitting asks)
    pub buy_volume: f64,
    /// Latest single aggressive sell volume (market orders hitting bids)
    pub sell_volume: f64,
    /// Timestamp when this level was last updated
    pub last_update: u64,
}

impl OrderFlowLevel {
    /// Create a new empty order flow level
    pub fn new() -> Self {
        let now = get_current_timestamp();
        Self {
            buy_volume: 0.0,
            sell_volume: 0.0,
            last_update: now,
        }
    }

    /// Set the latest aggressive buy volume for this level
    pub fn set_buy_volume(&mut self, volume: f64, timestamp: u64) {
        self.buy_volume = volume;
        self.sell_volume = 0.0;  // Clear the opposite side
        self.last_update = timestamp;
    }

    /// Set the latest aggressive sell volume for this level
    pub fn set_sell_volume(&mut self, volume: f64, timestamp: u64) {
        self.sell_volume = volume;
        self.buy_volume = 0.0;  // Clear the opposite side
        self.last_update = timestamp;
    }

    /// Check if this level should be cleaned up (no activity for more than 2 seconds)
    pub fn should_cleanup(&self, current_time: u64) -> bool {
        current_time.saturating_sub(self.last_update) > ORDER_FLOW_WINDOW_MS
    }

    /// Get total volume for this level
    pub fn total_volume(&self) -> f64 {
        self.buy_volume + self.sell_volume
    }

    /// Check if this level has any active volume
    pub fn has_volume(&self) -> bool {
        self.buy_volume > 0.0 || self.sell_volume > 0.0
    }

    /// Get the dominant side (buy or sell) for this level
    pub fn dominant_side(&self) -> Option<OrderFlowSide> {
        if self.buy_volume > self.sell_volume {
            Some(OrderFlowSide::Buy)
        } else if self.sell_volume > self.buy_volume {
            Some(OrderFlowSide::Sell)
        } else if self.buy_volume > 0.0 {
            Some(OrderFlowSide::Neutral)
        } else {
            None
        }
    }
}

impl Default for OrderFlowLevel {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents the dominant side of order flow at a price level
#[derive(Debug, Clone, PartialEq)]
pub enum OrderFlowSide {
    Buy,
    Sell,
    Neutral,
}

/// Manages real-time order flow data with time-based cleanup
#[derive(Debug)]
pub struct RealtimeOrderFlow {
    /// Order flow data organized by price level
    flow_levels: BTreeMap<OrderedFloat<f64>, OrderFlowLevel>,
    /// Configuration for cleanup intervals
    last_cleanup: u64,
    cleanup_interval: u64,
    /// Statistics
    total_buy_volume: f64,
    total_sell_volume: f64,
}

impl RealtimeOrderFlow {
    /// Create a new real-time order flow manager
    pub fn new() -> Self {
        Self {
            flow_levels: BTreeMap::new(),
            last_cleanup: get_current_timestamp(),
            cleanup_interval: 1000, // Cleanup every 1 second
            total_buy_volume: 0.0,
            total_sell_volume: 0.0,
        }
    }

    /// Process an aggressive trade (market order) - simplified version
    pub fn process_trade(&mut self, price: f64, volume: f64, is_buyer_maker: bool, timestamp: u64) {
        let price_key = OrderedFloat(price);
        let level = self.flow_levels.entry(price_key).or_insert_with(OrderFlowLevel::new);

        // Set the latest trade volume based on who was the aggressor
        // If buyer is maker, then seller is taker (aggressive sell)
        // If buyer is taker, then it's an aggressive buy
        if is_buyer_maker {
            // Seller is the aggressor (aggressive sell hitting bids)
            level.set_sell_volume(volume, timestamp);
        } else {
            // Buyer is the aggressor (aggressive buy hitting asks)
            level.set_buy_volume(volume, timestamp);
        }

        // Perform cleanup if needed
        if timestamp.saturating_sub(self.last_cleanup) > self.cleanup_interval {
            self.cleanup_expired_levels(timestamp);
        }
    }

    /// Get order flow data for a specific price level
    pub fn get_level(&self, price: f64) -> Option<&OrderFlowLevel> {
        self.flow_levels.get(&OrderedFloat(price))
    }

    /// Get all order flow levels
    pub fn get_all_levels(&self) -> &BTreeMap<OrderedFloat<f64>, OrderFlowLevel> {
        &self.flow_levels
    }

    /// Get order flow data for rendering (filtered and sorted)
    pub fn get_render_data(&self, visible_prices: &[f64]) -> Vec<(f64, f64, f64)> {
        let mut render_data = Vec::new();
        
        for &price in visible_prices {
            let (buy_vol, sell_vol) = if let Some(level) = self.get_level(price) {
                (level.buy_volume, level.sell_volume)
            } else {
                (0.0, 0.0)
            };
            
            render_data.push((price, buy_vol, sell_vol));
        }
        
        render_data
    }

    /// Clean up expired order flow levels (simplified)
    fn cleanup_expired_levels(&mut self, current_time: u64) {
        // Remove levels that haven't been updated for more than 2 seconds
        self.flow_levels.retain(|_, level| !level.should_cleanup(current_time));
        
        // Reset total volumes since we no longer accumulate
        self.total_buy_volume = 0.0;
        self.total_sell_volume = 0.0;
        
        self.last_cleanup = current_time;
    }


    /// Force cleanup of all expired data
    pub fn force_cleanup(&mut self) {
        let current_time = get_current_timestamp();
        self.cleanup_expired_levels(current_time);
    }

    /// Get statistics about the order flow (simplified)
    pub fn get_statistics(&self) -> OrderFlowStatistics {
        let current_time = get_current_timestamp();
        let active_levels = self.flow_levels.len();
        let mut levels_with_data = 0;
        let mut max_buy_volume: f64 = 0.0;
        let mut max_sell_volume: f64 = 0.0;
        let mut current_buy_volume = 0.0;
        let mut current_sell_volume = 0.0;

        for level in self.flow_levels.values() {
            // Only count levels that haven't expired (within 2 seconds)
            if current_time.saturating_sub(level.last_update) <= ORDER_FLOW_WINDOW_MS {
                if level.has_volume() {
                    levels_with_data += 1;
                    max_buy_volume = max_buy_volume.max(level.buy_volume);
                    max_sell_volume = max_sell_volume.max(level.sell_volume);
                    current_buy_volume += level.buy_volume;
                    current_sell_volume += level.sell_volume;
                }
            }
        }

        OrderFlowStatistics {
            total_buy_volume: current_buy_volume,
            total_sell_volume: current_sell_volume,
            active_levels,
            levels_with_data,
            max_buy_volume,
            max_sell_volume,
            buy_sell_ratio: if current_sell_volume > 0.0 {
                current_buy_volume / current_sell_volume
            } else {
                f64::INFINITY
            },
        }
    }

    /// Clear all order flow data
    pub fn clear(&mut self) {
        self.flow_levels.clear();
        self.total_buy_volume = 0.0;
        self.total_sell_volume = 0.0;
        self.last_cleanup = get_current_timestamp();
    }

    /// Get the number of active price levels
    pub fn active_levels_count(&self) -> usize {
        self.flow_levels.len()
    }

    /// Check if there's any active order flow data (within 2 seconds)
    pub fn has_active_data(&self) -> bool {
        let current_time = get_current_timestamp();
        self.flow_levels.values().any(|level| 
            current_time.saturating_sub(level.last_update) <= ORDER_FLOW_WINDOW_MS && level.has_volume()
        )
    }
}

impl Default for RealtimeOrderFlow {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the current order flow state
#[derive(Debug, Clone)]
pub struct OrderFlowStatistics {
    pub total_buy_volume: f64,
    pub total_sell_volume: f64,
    pub active_levels: usize,
    pub levels_with_data: usize,
    pub max_buy_volume: f64,
    pub max_sell_volume: f64,
    pub buy_sell_ratio: f64,
}

/// Configuration for real-time order flow processing
#[derive(Debug, Clone)]
pub struct OrderFlowConfig {
    /// Time window for accumulating order flow (milliseconds)
    pub window_duration_ms: u64,
    /// Interval for cleanup operations (milliseconds)
    pub cleanup_interval_ms: u64,
    /// Maximum number of price levels to track
    pub max_levels: usize,
    /// Minimum volume threshold for display
    pub min_volume_threshold: f64,
}

impl Default for OrderFlowConfig {
    fn default() -> Self {
        Self {
            window_duration_ms: ORDER_FLOW_WINDOW_MS,
            cleanup_interval_ms: 1000,
            max_levels: 1000,
            min_volume_threshold: 0.01,
        }
    }
}

/// Utility function to get current timestamp in milliseconds
fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_flow_level_creation() {
        let level = OrderFlowLevel::new();
        assert_eq!(level.buy_volume, 0.0);
        assert_eq!(level.sell_volume, 0.0);
        assert!(!level.has_volume());
    }

    #[test]
    fn test_order_flow_level_volume_setting() {
        let mut level = OrderFlowLevel::new();
        let timestamp = get_current_timestamp();
        
        level.set_buy_volume(10.0, timestamp);
        assert_eq!(level.buy_volume, 10.0);
        assert_eq!(level.sell_volume, 0.0);  // Opposite side cleared
        assert_eq!(level.total_volume(), 10.0);
        assert!(level.has_volume());
        
        level.set_sell_volume(5.0, timestamp);
        assert_eq!(level.sell_volume, 5.0);
        assert_eq!(level.buy_volume, 0.0);  // Opposite side cleared
        assert_eq!(level.total_volume(), 5.0);
    }

    #[test]
    fn test_dominant_side_detection() {
        let mut level = OrderFlowLevel::new();
        let timestamp = get_current_timestamp();
        
        // Test no volume
        assert_eq!(level.dominant_side(), None);
        
        // Test buy dominant
        level.set_buy_volume(10.0, timestamp);
        assert_eq!(level.dominant_side(), Some(OrderFlowSide::Buy));
        
        // Test sell dominant (will clear buy volume)
        level.set_sell_volume(15.0, timestamp);
        assert_eq!(level.dominant_side(), Some(OrderFlowSide::Sell));
        
        // Since we clear opposite side, neutral is only when both are 0 but we had volume
        level.buy_volume = 0.0;
        level.sell_volume = 0.0;
        assert_eq!(level.dominant_side(), None);
    }

    #[test]
    fn test_realtime_order_flow_processing() {
        let mut flow = RealtimeOrderFlow::new();
        let timestamp = get_current_timestamp();
        
        // Process some trades - only latest trade at each price level should be kept
        flow.process_trade(100.0, 5.0, false, timestamp); // Aggressive buy
        flow.process_trade(100.0, 3.0, true, timestamp);  // Aggressive sell (overwrites buy)
        flow.process_trade(101.0, 2.0, false, timestamp); // Aggressive buy
        
        // Check statistics - should show only the latest trades
        let stats = flow.get_statistics();
        assert_eq!(stats.total_buy_volume, 2.0);  // Only from price 101.0
        assert_eq!(stats.total_sell_volume, 3.0); // Only from price 100.0
        assert_eq!(stats.active_levels, 2);
        
        // Check specific level - should only have sell volume (latest trade)
        let level = flow.get_level(100.0).unwrap();
        assert_eq!(level.buy_volume, 0.0);  // Cleared by the sell trade
        assert_eq!(level.sell_volume, 3.0);
    }

    #[test]
    fn test_cleanup_behavior() {
        let mut level = OrderFlowLevel::new();
        let timestamp = get_current_timestamp();
        
        // Set some volume
        level.set_buy_volume(10.0, timestamp);
        assert_eq!(level.buy_volume, 10.0);
        
        // Check if level should be cleaned up immediately (shouldn't)
        assert!(!level.should_cleanup(timestamp));
        
        // Simulate time passing beyond window
        let future_timestamp = timestamp + ORDER_FLOW_WINDOW_MS + 1000;
        
        // Now it should be eligible for cleanup
        assert!(level.should_cleanup(future_timestamp));
    }

    #[test]
    fn test_flow_cleanup_behavior() {
        let mut flow = RealtimeOrderFlow::new();
        let timestamp = get_current_timestamp();
        
        // Add some data
        flow.process_trade(100.0, 5.0, false, timestamp);
        assert_eq!(flow.active_levels_count(), 1);
        
        // Force cleanup shouldn't remove recent data
        flow.force_cleanup();
        assert_eq!(flow.active_levels_count(), 1);
        
        // Simulate old data by manually setting timestamp
        if let Some(level) = flow.flow_levels.get_mut(&OrderedFloat(100.0)) {
            level.last_update = timestamp.saturating_sub(ORDER_FLOW_WINDOW_MS + 1000);
        }
        
        flow.force_cleanup();
        assert_eq!(flow.active_levels_count(), 0);
    }
}