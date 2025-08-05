use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use ordered_float::OrderedFloat;
use serde_json::Value;

use super::realtime_flow::{RealtimeOrderFlow, OrderFlowLevel, OrderFlowStatistics, OrderFlowConfig};
use super::data_structures::TradeRecord;

/// Thread-safe order flow manager that processes trades and maintains real-time order flow data
pub struct OrderFlowManager {
    /// The core order flow processor
    flow_processor: Arc<Mutex<RealtimeOrderFlow>>,
    /// Configuration settings
    config: OrderFlowConfig,
    /// Performance statistics
    stats: OrderFlowManagerStats,
    /// Trade processing cache for performance optimization
    trade_cache: Vec<CachedTrade>,
    /// Last cleanup timestamp
    last_cleanup: u64,
}

/// Cached trade data for batch processing
#[derive(Debug, Clone)]
struct CachedTrade {
    price: f64,
    volume: f64,
    is_buyer_maker: bool,
    timestamp: u64,
}

/// Statistics for the order flow manager
#[derive(Debug, Clone, Default)]
pub struct OrderFlowManagerStats {
    pub trades_processed: u64,
    pub total_buy_volume: f64,
    pub total_sell_volume: f64,
    pub cache_hits: u64,
    pub cleanup_operations: u64,
    pub processing_time_ms: f64,
    pub last_update_timestamp: u64,
}

impl OrderFlowManager {
    /// Create a new order flow manager
    pub fn new() -> Self {
        Self::with_config(OrderFlowConfig::default())
    }

    /// Create a new order flow manager with custom configuration
    pub fn with_config(config: OrderFlowConfig) -> Self {
        Self {
            flow_processor: Arc::new(Mutex::new(RealtimeOrderFlow::new())),
            config,
            stats: OrderFlowManagerStats::default(),
            trade_cache: Vec::with_capacity(1000),
            last_cleanup: get_current_timestamp(),
        }
    }

    /// Process a trade from WebSocket data
    pub fn process_trade_data(&mut self, trade_data: &Value) -> Result<(), OrderFlowError> {
        let start_time = get_current_timestamp();

        // Extract trade information from JSON
        let trade = self.parse_trade_data(trade_data)?;
        
        // Process the trade
        self.process_trade_internal(trade)?;

        // Update performance statistics
        let processing_time = get_current_timestamp() - start_time;
        self.update_stats(processing_time);

        // Perform cleanup if needed
        self.maybe_cleanup();

        Ok(())
    }

    /// Process a trade directly with parsed data
    pub fn process_trade(&mut self, price: f64, volume: f64, is_buyer_maker: bool) -> Result<(), OrderFlowError> {
        let timestamp = get_current_timestamp();
        let trade = CachedTrade {
            price,
            volume,
            is_buyer_maker,
            timestamp,
        };

        self.process_trade_internal(trade)
    }

    /// Get order flow data for a specific price level
    pub fn get_flow_at_price(&self, price: f64) -> Option<(f64, f64)> {
        if let Ok(processor) = self.flow_processor.lock() {
            if let Some(level) = processor.get_level(price) {
                Some((level.buy_volume, level.sell_volume))
            } else {
                Some((0.0, 0.0))
            }
        } else {
            None
        }
    }

    /// Get order flow data for multiple price levels (optimized for rendering)
    pub fn get_flow_for_prices(&self, prices: &[f64]) -> Vec<OrderFlowDisplayData> {
        if let Ok(processor) = self.flow_processor.lock() {
            let render_data = processor.get_render_data(prices);
            let result: Vec<OrderFlowDisplayData> = render_data.into_iter()
                .map(|(price, buy_vol, sell_vol)| OrderFlowDisplayData {
                    price,
                    buy_volume: buy_vol,
                    sell_volume: sell_vol,
                    total_volume: buy_vol + sell_vol,
                    buy_ratio: if buy_vol + sell_vol > 0.0 { 
                        buy_vol / (buy_vol + sell_vol) 
                    } else { 
                        0.0 
                    },
                })
                .collect();
            
            
            result
        } else {
            // Return empty data if lock fails
            prices.iter().map(|&price| OrderFlowDisplayData {
                price,
                buy_volume: 0.0,
                sell_volume: 0.0,
                total_volume: 0.0,
                buy_ratio: 0.0,
            }).collect()
        }
    }

    /// Get comprehensive order flow statistics
    pub fn get_statistics(&self) -> OrderFlowStatistics {
        if let Ok(processor) = self.flow_processor.lock() {
            processor.get_statistics()
        } else {
            // Return default statistics if lock fails
            OrderFlowStatistics {
                total_buy_volume: 0.0,
                total_sell_volume: 0.0,
                active_levels: 0,
                levels_with_data: 0,
                max_buy_volume: 0.0,
                max_sell_volume: 0.0,
                buy_sell_ratio: 1.0,
            }
        }
    }

    /// Get manager-specific statistics
    pub fn get_manager_stats(&self) -> OrderFlowManagerStats {
        self.stats.clone()
    }

    /// Force cleanup of expired order flow data
    pub fn force_cleanup(&mut self) {
        if let Ok(mut processor) = self.flow_processor.lock() {
            processor.force_cleanup();
            self.stats.cleanup_operations += 1;
            self.last_cleanup = get_current_timestamp();
        }
    }

    /// Clear all order flow data
    pub fn clear(&mut self) {
        if let Ok(mut processor) = self.flow_processor.lock() {
            processor.clear();
        }
        self.trade_cache.clear();
        self.stats = OrderFlowManagerStats::default();
        self.last_cleanup = get_current_timestamp();
    }

    /// Check if there's any active order flow data
    pub fn has_active_data(&self) -> bool {
        if let Ok(processor) = self.flow_processor.lock() {
            processor.has_active_data()
        } else {
            false
        }
    }

    /// Get the maximum volumes for scaling display
    pub fn get_max_volumes(&self) -> (f64, f64) {
        let stats = self.get_statistics();
        (stats.max_buy_volume, stats.max_sell_volume)
    }

    // Internal methods

    /// Parse trade data from WebSocket JSON
    fn parse_trade_data(&self, data: &Value) -> Result<CachedTrade, OrderFlowError> {
        let price_str = data["p"].as_str()
            .ok_or_else(|| OrderFlowError::ParseError("Missing price field".to_string()))?;
        
        let volume_str = data["q"].as_str()
            .ok_or_else(|| OrderFlowError::ParseError("Missing quantity field".to_string()))?;
        
        let is_buyer_maker = data["m"].as_bool()
            .ok_or_else(|| OrderFlowError::ParseError("Missing maker field".to_string()))?;

        let price = price_str.parse::<f64>()
            .map_err(|e| OrderFlowError::ParseError(format!("Invalid price: {}", e)))?;
        
        let volume = volume_str.parse::<f64>()
            .map_err(|e| OrderFlowError::ParseError(format!("Invalid volume: {}", e)))?;

        let timestamp = if let Some(time_str) = data["T"].as_u64() {
            time_str
        } else {
            get_current_timestamp()
        };

        Ok(CachedTrade {
            price,
            volume,
            is_buyer_maker,
            timestamp,
        })
    }

    /// Process a trade internally
    fn process_trade_internal(&mut self, trade: CachedTrade) -> Result<(), OrderFlowError> {
        // Validate trade data
        if trade.price <= 0.0 || trade.volume <= 0.0 {
            return Err(OrderFlowError::InvalidTrade("Invalid price or volume".to_string()));
        }

        // Add to cache for batch processing if needed
        self.trade_cache.push(trade.clone());

        // Process the trade through the flow processor
        if let Ok(mut processor) = self.flow_processor.lock() {
            processor.process_trade(trade.price, trade.volume, trade.is_buyer_maker, trade.timestamp);
            self.stats.trades_processed += 1;
        } else {
            return Err(OrderFlowError::LockError("Failed to acquire processor lock".to_string()));
        }

        // Note: Volume statistics are now calculated dynamically in get_statistics method
        // No longer accumulating totals here since we only show latest single trades

        // Trim cache if it gets too large
        if self.trade_cache.len() > 1000 {
            self.trade_cache.drain(0..500); // Remove first 500 entries
        }

        Ok(())
    }

    /// Update performance statistics
    fn update_stats(&mut self, processing_time: u64) {
        self.stats.processing_time_ms = processing_time as f64;
        self.stats.last_update_timestamp = get_current_timestamp();
    }

    /// Perform cleanup if needed
    fn maybe_cleanup(&mut self) {
        let current_time = get_current_timestamp();
        if current_time.saturating_sub(self.last_cleanup) > self.config.cleanup_interval_ms {
            self.force_cleanup();
        }
    }

    /// Create a thread-safe clone of the flow processor for read access
    pub fn get_flow_processor(&self) -> Arc<Mutex<RealtimeOrderFlow>> {
        Arc::clone(&self.flow_processor)
    }
}

impl Default for OrderFlowManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Display data structure for order flow rendering
#[derive(Debug, Clone)]
pub struct OrderFlowDisplayData {
    pub price: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub total_volume: f64,
    pub buy_ratio: f64,
}

impl OrderFlowDisplayData {
    /// Check if this level has any volume to display
    pub fn has_volume(&self) -> bool {
        self.total_volume > 0.0
    }

    /// Get the dominant side
    pub fn dominant_side(&self) -> Option<&str> {
        if self.buy_volume > self.sell_volume {
            Some("buy")
        } else if self.sell_volume > self.buy_volume {
            Some("sell")
        } else if self.total_volume > 0.0 {
            Some("neutral")
        } else {
            None
        }
    }

    /// Format volume for display
    pub fn format_buy_volume(&self) -> String {
        format_volume(self.buy_volume)
    }

    /// Format sell volume for display
    pub fn format_sell_volume(&self) -> String {
        format_volume(self.sell_volume)
    }
}

/// Error types for order flow processing
#[derive(Debug, Clone)]
pub enum OrderFlowError {
    ParseError(String),
    InvalidTrade(String),
    LockError(String),
    ConfigError(String),
}

impl std::fmt::Display for OrderFlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderFlowError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            OrderFlowError::InvalidTrade(msg) => write!(f, "Invalid trade: {}", msg),
            OrderFlowError::LockError(msg) => write!(f, "Lock error: {}", msg),
            OrderFlowError::ConfigError(msg) => write!(f, "Config error: {}", msg),
        }
    }
}

impl std::error::Error for OrderFlowError {}

/// Utility function to format volume for display
fn format_volume(volume: f64) -> String {
    if volume == 0.0 {
        String::new()
    } else if volume >= 1000.0 {
        format!("{:.1}K", volume / 1000.0)
    } else if volume >= 100.0 {
        format!("{:.0}", volume)
    } else if volume >= 10.0 {
        format!("{:.1}", volume)
    } else {
        format!("{:.2}", volume)
    }
}

/// Utility function to get current timestamp
fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_order_flow_manager_creation() {
        let manager = OrderFlowManager::new();
        assert!(!manager.has_active_data());
        
        let stats = manager.get_statistics();
        assert_eq!(stats.total_buy_volume, 0.0);
        assert_eq!(stats.total_sell_volume, 0.0);
    }

    #[test]
    fn test_trade_processing() {
        let mut manager = OrderFlowManager::new();
        
        // Process some trades - with simplified logic, only latest trade at each price is kept
        assert!(manager.process_trade(100.0, 5.0, false).is_ok()); // Aggressive buy
        assert!(manager.process_trade(101.0, 3.0, true).is_ok());  // Aggressive sell at different price
        
        let stats = manager.get_manager_stats();
        assert_eq!(stats.trades_processed, 2);
        
        // Verify we have active order flow data
        assert!(manager.has_active_data());
        
        // Check that both trades are visible in the flow data
        let flow_stats = manager.get_statistics();
        assert!(flow_stats.levels_with_data >= 1); // At least one level should have data
    }

    #[test]
    fn test_websocket_data_parsing() {
        let mut manager = OrderFlowManager::new();
        
        let trade_data = json!({
            "p": "50000.0",
            "q": "0.1",
            "m": false,
            "T": 1234567890
        });
        
        assert!(manager.process_trade_data(&trade_data).is_ok());
        
        let flow_data = manager.get_flow_at_price(50000.0);
        assert!(flow_data.is_some());
        let (buy_vol, sell_vol) = flow_data.unwrap();
        assert!(buy_vol > 0.0 || sell_vol > 0.0);
    }

    #[test]
    fn test_flow_display_data() {
        let mut manager = OrderFlowManager::new();
        
        // Add some test data
        assert!(manager.process_trade(100.0, 5.0, false).is_ok());
        assert!(manager.process_trade(101.0, 3.0, true).is_ok());
        
        let prices = vec![100.0, 101.0, 102.0];
        let display_data = manager.get_flow_for_prices(&prices);
        
        assert_eq!(display_data.len(), 3);
        
        // Check that we have data for the traded prices
        let level_100 = &display_data[0];
        assert!(level_100.has_volume());
        assert_eq!(level_100.dominant_side(), Some("buy"));
        
        let level_101 = &display_data[1];
        assert!(level_101.has_volume());
        assert_eq!(level_101.dominant_side(), Some("sell"));
        
        // Level 102 should have no volume
        let level_102 = &display_data[2];
        assert!(!level_102.has_volume());
        assert_eq!(level_102.dominant_side(), None);
    }

    #[test]
    fn test_error_handling() {
        let mut manager = OrderFlowManager::new();
        
        // Test invalid trade data
        assert!(manager.process_trade(-100.0, 5.0, false).is_err());
        assert!(manager.process_trade(100.0, -5.0, false).is_err());
        assert!(manager.process_trade(0.0, 5.0, false).is_err());
        
        // Test malformed JSON
        let bad_data = json!({
            "invalid": "data"
        });
        assert!(manager.process_trade_data(&bad_data).is_err());
    }

    #[test]
    fn test_volume_formatting() {
        assert_eq!(format_volume(0.0), "");
        assert_eq!(format_volume(5.123), "5.12");
        assert_eq!(format_volume(15.7), "15.7");
        assert_eq!(format_volume(150.0), "150");
        assert_eq!(format_volume(1500.0), "1.5K");
    }

    #[test]
    fn test_cleanup_functionality() {
        let mut manager = OrderFlowManager::new();
        
        // Add some data
        assert!(manager.process_trade(100.0, 5.0, false).is_ok());
        assert!(manager.has_active_data());
        
        // Force cleanup shouldn't remove recent data
        manager.force_cleanup();
        assert!(manager.has_active_data());
        
        // Clear should remove all data
        manager.clear();
        assert!(!manager.has_active_data());
    }
}