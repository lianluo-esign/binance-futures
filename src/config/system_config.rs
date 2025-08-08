// System-wide configuration structures
// Following OOP principles with serialization support

use serde::{Deserialize, Serialize};

/// Main global configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub system: SystemConfig,
    pub runtime: RuntimeConfig,
    pub providers: ProvidersConfig,
    pub gui: GuiConfig,
    pub monitoring: MonitoringConfig,
}

/// System configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub name: String,
    pub version: String,
    pub log_level: String,
    pub log_file: String,
    pub performance_mode: String,
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub cpu_affinity: bool,
    pub cpu_cores: Vec<usize>,
    pub thread_pool_size: usize,
    pub event_buffer_size: usize,
}

/// Providers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    pub active: Vec<String>,
    #[serde(rename = "config")]
    pub configs: Vec<ProviderMetadata>,
}

/// Provider metadata for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetadata {
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub enabled: bool,
    pub priority: u32,
    pub config_file: String,
}

/// GUI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiConfig {
    pub theme: String,
    pub fps: u32,
    pub show_debug_info: bool,
    pub layout: String,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub metrics_port: u16,
    pub health_check_interval: u32,
    pub export_metrics: bool,
    pub metrics_file: String,
}

// Default implementations for backward compatibility

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            system: SystemConfig::default(),
            runtime: RuntimeConfig::default(),
            providers: ProvidersConfig::default(),
            gui: GuiConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            name: "Binance Futures Trading System".to_string(),
            version: "1.0.0".to_string(),
            log_level: "info".to_string(),
            log_file: "binance_futures.log".to_string(),
            performance_mode: "high".to_string(),
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            cpu_affinity: false,
            cpu_cores: vec![0, 1, 2, 3],
            thread_pool_size: 4,
            event_buffer_size: 10000,
        }
    }
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            active: vec!["binance_websocket".to_string()],
            configs: vec![
                ProviderMetadata {
                    name: "binance_websocket".to_string(),
                    provider_type: "BinanceWebSocket".to_string(),
                    enabled: true,
                    priority: 1,
                    config_file: "configs/providers/binance_websocket.toml".to_string(),
                },
            ],
        }
    }
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            fps: 0,  // Maximum refresh rate (no limiting)
            show_debug_info: false,
            layout: "default".to_string(),
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_port: 9090,
            health_check_interval: 30,
            export_metrics: true,
            metrics_file: "metrics.json".to_string(),
        }
    }
}

// Builder pattern for fluent configuration (OOP style)

impl GlobalConfig {
    pub fn builder() -> GlobalConfigBuilder {
        GlobalConfigBuilder::new()
    }
}

pub struct GlobalConfigBuilder {
    config: GlobalConfig,
}

impl GlobalConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: GlobalConfig::default(),
        }
    }
    
    pub fn system(mut self, system: SystemConfig) -> Self {
        self.config.system = system;
        self
    }
    
    pub fn runtime(mut self, runtime: RuntimeConfig) -> Self {
        self.config.runtime = runtime;
        self
    }
    
    pub fn providers(mut self, providers: ProvidersConfig) -> Self {
        self.config.providers = providers;
        self
    }
    
    pub fn gui(mut self, gui: GuiConfig) -> Self {
        self.config.gui = gui;
        self
    }
    
    pub fn monitoring(mut self, monitoring: MonitoringConfig) -> Self {
        self.config.monitoring = monitoring;
        self
    }
    
    pub fn build(self) -> GlobalConfig {
        self.config
    }
}