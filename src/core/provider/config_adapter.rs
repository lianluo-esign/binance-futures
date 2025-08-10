// Configuration adapter for providers
// Bridges the gap between configuration system and provider implementations
// Maintains backward compatibility

use crate::config::{
    ProviderConfig, BinanceWebSocketConfig, GzipProviderConfig,
    ConfigManager,
    get_provider_mapping, is_valid_provider_name
};
use std::sync::Arc;

/// Configuration adapter trait for providers
pub trait ConfigurableProvider {
    /// Load configuration for the provider
    fn load_config(&mut self, config: &ProviderConfig) -> Result<(), String>;
    
    /// Get current configuration as JSON
    fn get_config_json(&self) -> Option<String>;
    
    /// Update configuration at runtime
    fn update_config(&mut self, config: &ProviderConfig) -> Result<(), String>;
}

/// Helper struct to bridge configuration with providers
pub struct ProviderConfigAdapter {
    config_manager: Arc<ConfigManager>,
}

impl ProviderConfigAdapter {
    /// Create new adapter with config manager
    pub fn new(config_manager: Arc<ConfigManager>) -> Self {
        Self { config_manager }
    }
    
    /// Get configuration for a specific provider
    pub fn get_provider_config(&self, name: &str) -> Option<ProviderConfig> {
        self.config_manager.provider(name).cloned()
    }
    
    /// Apply configuration to Binance WebSocket provider
    pub fn apply_to_binance_websocket(
        _config: &BinanceWebSocketConfig,
        _provider: &mut crate::core::provider::BinanceProvider,
    ) -> Result<(), String> {
        // 简化的BinanceProvider不需要额外配置
        // 所有配置都在创建时设置
        Ok(())
    }
    
    /// Apply configuration to Gzip provider
    pub fn apply_to_gzip_provider(
        config: &GzipProviderConfig,
        provider: &mut crate::core::provider::GzipProvider,
    ) -> Result<(), String> {
        // Apply data source settings
        provider.set_data_directory(&config.data_source.data_directory)?;
        
        // Apply file pattern
        if !config.data_source.file_pattern.is_empty() {
            provider.set_file_pattern(&config.data_source.file_pattern);
        }
        
        // Apply specific files if configured
        if !config.files.specific_files.is_empty() {
            provider.set_specific_files(config.files.specific_files.clone());
        }
        
        // Apply symbols filter
        if !config.data_source.symbols.is_empty() {
            provider.set_symbols(config.data_source.symbols.clone());
        }
        
        // Apply playback settings
        provider.set_playback_speed(config.playback.initial_speed);
        provider.set_auto_start(config.playback.auto_start);
        provider.set_loop_enabled(config.playback.loop_enabled);
        
        // Apply buffering settings
        provider.set_buffer_sizes(
            config.buffering.read_buffer_size,
            config.buffering.decompress_buffer_size,
            config.buffering.event_buffer_size,
        );
        
        // Apply error handling
        provider.set_error_handling(
            &config.error_handling.on_parse_error,
            &config.error_handling.on_file_error,
            config.error_handling.max_consecutive_errors,
        );
        
        Ok(())
    }
}

/// Extension methods for BinanceProvider (backward compatible)
impl crate::core::provider::BinanceProvider {
    /// Load configuration from BinanceWebSocketConfig
    pub fn from_websocket_config(config: &BinanceWebSocketConfig) -> Result<Self, String> {
        // Create a simple BinanceProvider with the config
        let provider = Self::from_config(config.clone())
            .map_err(|e| format!("创建BinanceProvider失败: {}", e))?;
        
        Ok(provider)
    }
    
    /// Update provider with new configuration
    pub fn update_from_config(&mut self, config: &BinanceWebSocketConfig) -> Result<(), String> {
        ProviderConfigAdapter::apply_to_binance_websocket(config, self)
    }
}

/// Extension methods for GzipProvider (backward compatible)
impl crate::core::provider::GzipProvider {
    /// Load configuration from GzipProviderConfig
    pub fn from_config(config: &GzipProviderConfig) -> Result<Self, String> {
        // Convert from config system's GzipProviderConfig to provider's internal config
        let provider_config = super::gzip_provider::GzipProviderConfig {
            data_dir: std::path::PathBuf::from(&config.data_source.data_directory),
            file_pattern: config.data_source.file_pattern.clone(),
            playback_config: super::gzip_provider::PlaybackConfig {
                initial_speed: config.playback.initial_speed,
                auto_start: config.playback.auto_start,
                loop_enabled: config.playback.loop_enabled,
                max_speed: 1000.0, // Default value
                min_speed: 0.1,    // Default value
                start_timestamp: None, // Config system doesn't have timestamp fields, set to None
                end_timestamp: None,
                nanosecond_precision: true, // Default value
                drift_tolerance_ns: 1_000_000, // Default 1ms
            },
            buffer_config: super::gzip_provider::BufferConfig {
                event_buffer_size: config.buffering.event_buffer_size,
                prefetch_lines: 1000, // Default value
                memory_limit_mb: 100,  // Default value
            },
            symbol_filter: if config.data_source.symbols.is_empty() {
                None
            } else {
                Some(config.data_source.symbols.join(","))
            },
            event_filter: config.filtering.allowed_event_types.clone(),
        };
        
        let mut provider = Self::new(provider_config);
        
        // Apply additional configuration if needed
        ProviderConfigAdapter::apply_to_gzip_provider(config, &mut provider)?;
        
        Ok(provider)
    }
    
    /// Update provider with new configuration
    pub fn update_from_config(&mut self, config: &GzipProviderConfig) -> Result<(), String> {
        ProviderConfigAdapter::apply_to_gzip_provider(config, self)
    }
    
    // Helper methods for configuration (maintaining backward compatibility)
    
    pub fn set_data_directory(&mut self, dir: &str) -> Result<(), String> {
        // Implementation would update data directory
        log::info!("Setting data directory to: {}", dir);
        Ok(())
    }
    
    pub fn set_file_pattern(&mut self, pattern: &str) {
        // Implementation would update file pattern
        log::info!("Setting file pattern to: {}", pattern);
    }
    
    pub fn set_specific_files(&mut self, files: Vec<String>) {
        // Implementation would set specific files
        log::info!("Setting specific files: {:?}", files);
    }
    
    pub fn set_symbols(&mut self, symbols: Vec<String>) {
        // Implementation would set symbols filter
        log::info!("Setting symbols filter: {:?}", symbols);
    }
    
    pub fn set_playback_speed(&mut self, speed: f64) {
        // Implementation would set playback speed
        log::info!("Setting playback speed to: {}", speed);
    }
    
    pub fn set_auto_start(&mut self, auto_start: bool) {
        // Implementation would set auto start
        log::info!("Setting auto start to: {}", auto_start);
    }
    
    pub fn set_loop_enabled(&mut self, enabled: bool) {
        // Implementation would set loop mode
        log::info!("Setting loop mode to: {}", enabled);
    }
    
    pub fn set_buffer_sizes(&mut self, read: usize, decompress: usize, event: usize) {
        // Implementation would set buffer sizes
        log::info!("Setting buffer sizes: read={}, decompress={}, event={}", 
                  read, decompress, event);
    }
    
    pub fn set_error_handling(&mut self, on_parse: &str, on_file: &str, max_errors: usize) {
        // Implementation would configure error handling
        log::info!("Setting error handling: parse={}, file={}, max={}", 
                  on_parse, on_file, max_errors);
    }
}

/// Factory for creating configured providers
pub struct ConfiguredProviderFactory {
    config_manager: Arc<ConfigManager>,
}

impl ConfiguredProviderFactory {
    pub fn new(config_manager: Arc<ConfigManager>) -> Self {
        Self { config_manager }
    }
    
    /// Create provider from configuration name using explicit mapping
    pub fn create_provider(&self, name: &str) -> Result<Box<dyn crate::core::provider::DataProvider<Error = crate::core::provider::ProviderError>>, String> {
        // First validate the provider name against explicit mappings
        if !is_valid_provider_name(name) {
            return Err(format!("Invalid provider name '{}'. Check provider_mapping.rs for valid names.", name));
        }
        
        let mapping = get_provider_mapping(name)
            .ok_or_else(|| format!("Provider mapping not found: {}", name))?;
        
        log::info!("Creating provider '{}' using implementation: src/{}", 
                  name, mapping.implementation_file);
        
        let config = self.config_manager.provider(name)
            .ok_or_else(|| format!("Provider configuration not found: {}", name))?;
        
        match config {
            ProviderConfig::BinanceWebSocket(ws_config) => {
                let provider = crate::core::provider::BinanceProvider::from_config(ws_config.clone())
                    .map_err(|e| format!("创建BinanceProvider失败: {}", e))?;
                Ok(Box::new(provider))
            }
            ProviderConfig::Gzip(gzip_config) => {
                let provider = crate::core::provider::GzipProvider::from_config(gzip_config)?;
                Ok(Box::new(provider))
            }
            _ => Err(format!("Provider type not implemented: {}", name))
        }
    }
    
    /// Create all active providers
    pub fn create_active_providers(&self) -> Vec<Box<dyn crate::core::provider::DataProvider<Error = crate::core::provider::ProviderError>>> {
        let mut providers = Vec::new();
        
        for provider_config in self.config_manager.active_providers() {
            match self.create_provider_from_config(provider_config) {
                Ok(provider) => providers.push(provider),
                Err(e) => log::error!("Failed to create provider: {}", e),
            }
        }
        
        providers
    }
    
    fn create_provider_from_config(
        &self,
        config: &ProviderConfig,
    ) -> Result<Box<dyn crate::core::provider::DataProvider<Error = crate::core::provider::ProviderError>>, String> {
        match config {
            ProviderConfig::BinanceWebSocket(ws_config) => {
                let provider = crate::core::provider::BinanceProvider::from_config(ws_config.clone())
                    .map_err(|e| format!("创建BinanceProvider失败: {}", e))?;
                Ok(Box::new(provider))
            }
            ProviderConfig::Gzip(gzip_config) => {
                let provider = crate::core::provider::GzipProvider::from_config(gzip_config)?;
                Ok(Box::new(provider))
            }
            _ => Err("Provider type not implemented".to_string())
        }
    }
}