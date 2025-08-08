// Configuration module for managing system and provider configurations
// Follows OOP principles with backward compatibility

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub mod provider_config;
pub mod system_config;
pub mod provider_mapping;
pub mod consistency_checker;
pub mod provider_identity;
pub mod startup_validator;

pub use provider_config::*;
pub use system_config::*;
pub use provider_mapping::*;
pub use consistency_checker::*;
pub use provider_identity::*;
pub use startup_validator::*;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),
    
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),
    
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

/// Main configuration manager implementing OOP principles
pub struct ConfigManager {
    config_path: PathBuf,
    global_config: GlobalConfig,
    provider_configs: HashMap<String, ProviderConfig>,
    loaded: bool,
}

impl ConfigManager {
    /// Create new configuration manager with default path
    pub fn new() -> Self {
        Self::with_path("config.toml")
    }
    
    /// Create configuration manager with custom path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            config_path: path.as_ref().to_path_buf(),
            global_config: GlobalConfig::default(),
            provider_configs: HashMap::new(),
            loaded: false,
        }
    }
    
    /// Load all configurations (backward compatible - returns Result)
    pub fn load(&mut self) -> Result<(), ConfigError> {
        // Load global configuration
        self.global_config = self.load_global_config()?;
        
        // Validate against provider implementations (CRITICAL - will exit on failure)
        validate_configuration_or_exit(&self.global_config);
        
        // Validate provider configurations against mappings
        self.validate_provider_configurations()?;
        
        // Load provider configurations
        self.provider_configs = self.load_provider_configs()?;
        
        self.loaded = true;
        Ok(())
    }
    
    /// Load with fallback to defaults if files don't exist (backward compatible)
    pub fn load_or_default(&mut self) -> &Self {
        if let Err(e) = self.load() {
            log::warn!("Failed to load config, using defaults: {}", e);
            self.global_config = GlobalConfig::default();
            self.provider_configs = self.create_default_provider_configs();
            self.loaded = true;
        }
        self
    }
    
    /// Get global configuration
    pub fn global(&self) -> &GlobalConfig {
        &self.global_config
    }
    
    /// Get mutable global configuration
    pub fn global_mut(&mut self) -> &mut GlobalConfig {
        &mut self.global_config
    }
    
    /// Get provider configuration by name
    pub fn provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.provider_configs.get(name)
    }
    
    /// Get all provider configurations
    pub fn providers(&self) -> &HashMap<String, ProviderConfig> {
        &self.provider_configs
    }
    
    /// Get active provider configurations
    pub fn active_providers(&self) -> Vec<&ProviderConfig> {
        self.global_config
            .providers
            .configs
            .iter()
            .filter(|c| c.enabled)
            .filter_map(|c| self.provider_configs.get(&c.name))
            .collect()
    }
    
    /// Save current configuration
    pub fn save(&self) -> Result<(), ConfigError> {
        let toml_str = toml::to_string_pretty(&self.global_config)
            .map_err(|e| ConfigError::Invalid(e.to_string()))?;
        fs::write(&self.config_path, toml_str)?;
        
        // Save provider configs
        for (name, config) in &self.provider_configs {
            if let Some(provider_meta) = self.global_config.providers.configs
                .iter()
                .find(|c| c.name == *name) {
                let provider_path = Path::new(&provider_meta.config_file);
                let provider_toml = toml::to_string_pretty(config)
                    .map_err(|e| ConfigError::Invalid(e.to_string()))?;
                
                // Create directory if it doesn't exist
                if let Some(parent) = provider_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                fs::write(provider_path, provider_toml)?;
            }
        }
        
        Ok(())
    }
    
    /// Check if configuration is loaded
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
    
    /// Reload configuration from disk
    pub fn reload(&mut self) -> Result<(), ConfigError> {
        self.loaded = false;
        self.load()
    }
    
    // Private helper methods
    
    /// Validate provider configurations against explicit mappings
    fn validate_provider_configurations(&self) -> Result<(), ConfigError> {
        for provider_meta in &self.global_config.providers.configs {
            // Check if provider name is registered in mappings
            if !is_valid_provider_name(&provider_meta.name) {
                return Err(ConfigError::Invalid(format!(
                    "Unknown provider name '{}'. Valid names are: {:?}",
                    provider_meta.name,
                    get_all_provider_names()
                )));
            }
            
            // Validate type matches mapping
            if let Err(e) = validate_provider_config(&provider_meta.name, &provider_meta.provider_type) {
                return Err(ConfigError::Invalid(e));
            }
            
            log::info!("✓ Provider '{}' validated against mapping", provider_meta.name);
        }
        
        Ok(())
    }
    
    fn load_global_config(&self) -> Result<GlobalConfig, ConfigError> {
        let content = fs::read_to_string(&self.config_path)
            .map_err(|_| ConfigError::FileNotFound(self.config_path.display().to_string()))?;
        toml::from_str(&content).map_err(ConfigError::from)
    }
    
    fn load_provider_configs(&self) -> Result<HashMap<String, ProviderConfig>, ConfigError> {
        let mut configs = HashMap::new();
        
        for provider_meta in &self.global_config.providers.configs {
            let config_path = Path::new(&provider_meta.config_file);
            
            // Skip if file doesn't exist and provider is disabled
            if !config_path.exists() && !provider_meta.enabled {
                continue;
            }
            
            // Try to load provider config
            match self.load_provider_config(config_path, &provider_meta.provider_type) {
                Ok(config) => {
                    configs.insert(provider_meta.name.clone(), config);
                }
                Err(e) => {
                    if provider_meta.enabled {
                        log::error!("Failed to load config for {}: {}", provider_meta.name, e);
                    }
                }
            }
        }
        
        Ok(configs)
    }
    
    fn load_provider_config(&self, path: &Path, provider_type: &str) -> Result<ProviderConfig, ConfigError> {
        let content = fs::read_to_string(path)
            .map_err(|_| ConfigError::FileNotFound(path.display().to_string()))?;
        
        // Parse based on provider type for backward compatibility
        match provider_type {
            "BinanceWebSocket" => {
                let config: BinanceWebSocketConfig = toml::from_str(&content)?;
                Ok(ProviderConfig::BinanceWebSocket(config))
            }
            "GzipProvider" => {
                let config: GzipProviderConfig = toml::from_str(&content)?;
                Ok(ProviderConfig::Gzip(config))
            }
            "BinanceRest" => {
                let config: BinanceRestConfig = toml::from_str(&content)?;
                Ok(ProviderConfig::BinanceRest(config))
            }
            "Mock" => {
                let config: MockProviderConfig = toml::from_str(&content)?;
                Ok(ProviderConfig::Mock(config))
            }
            _ => Err(ConfigError::Invalid(format!("Unknown provider type: {}", provider_type)))
        }
    }
    
    fn create_default_provider_configs(&self) -> HashMap<String, ProviderConfig> {
        let mut configs = HashMap::new();
        
        // Create default Binance WebSocket config
        configs.insert(
            "binance_websocket".to_string(),
            ProviderConfig::BinanceWebSocket(BinanceWebSocketConfig::default())
        );
        
        // Create default Gzip provider config
        configs.insert(
            "gzip_historical".to_string(),
            ProviderConfig::Gzip(GzipProviderConfig::default())
        );
        
        configs
    }
}

// Implement Default for backward compatibility
impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global singleton for configuration (backward compatible)
use lazy_static::lazy_static;
use std::sync::RwLock;

lazy_static! {
    static ref CONFIG: RwLock<ConfigManager> = RwLock::new(ConfigManager::new());
}

/// Get global configuration manager (backward compatible)
pub fn get_config() -> std::sync::RwLockReadGuard<'static, ConfigManager> {
    CONFIG.read().unwrap()
}

/// Get mutable global configuration manager
pub fn get_config_mut() -> std::sync::RwLockWriteGuard<'static, ConfigManager> {
    CONFIG.write().unwrap()
}

/// Initialize configuration (backward compatible)
pub fn init_config() -> Result<(), ConfigError> {
    let mut config = CONFIG.write().unwrap();
    config.load()
}

/// Initialize with custom path
pub fn init_config_with_path<P: AsRef<Path>>(path: P) -> Result<(), ConfigError> {
    let mut config = CONFIG.write().unwrap();
    *config = ConfigManager::with_path(path);
    config.load()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_manager_creation() {
        let manager = ConfigManager::new();
        assert!(!manager.is_loaded());
    }
    
    #[test]
    fn test_default_config() {
        let mut manager = ConfigManager::new();
        manager.load_or_default();
        assert!(manager.is_loaded());
    }
}