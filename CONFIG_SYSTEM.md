# Configuration System Documentation

## Overview

The Binance Futures Trading System uses a hierarchical configuration system with a global configuration file and provider-specific configurations. This design follows OOP principles while maintaining backward compatibility.

## Architecture

```
config.toml                     # Global configuration
├── configs/
│   └── providers/
│       ├── binance_websocket.toml
│       ├── gzip_historical.toml
│       ├── binance_rest.toml
│       └── mock.toml
```

## Features

- **Hierarchical Configuration**: Global settings with provider-specific overrides
- **Type-Safe**: Strongly typed configuration structures with validation
- **Backward Compatible**: Works with existing code without modifications
- **Runtime Updates**: Support for configuration updates without restart
- **Multiple Formats**: TOML configuration with JSON export capability
- **Factory Pattern**: Automated provider creation from configuration

## Quick Start

### 1. Basic Usage

```rust
use binance_futures::config::{ConfigManager, init_config};

// Initialize global configuration
init_config()?;

// Access configuration
let config = get_config();
println!("Active providers: {:?}", config.global().providers.active);
```

### 2. Create Provider from Configuration

```rust
use binance_futures::core::provider::ConfiguredProviderFactory;

// Create factory
let config_manager = Arc::new(ConfigManager::new());
config_manager.load_or_default();
let factory = ConfiguredProviderFactory::new(config_manager);

// Create provider
let provider = factory.create_provider("binance_websocket")?;
```

### 3. Custom Configuration Path

```rust
let mut config = ConfigManager::with_path("custom_config.toml");
config.load()?;
```

## Configuration Files

### Global Configuration (config.toml)

The main configuration file controls system-wide settings and provider activation:

```toml
[system]
name = "Binance Futures Trading System"
version = "1.0.0"
log_level = "info"
performance_mode = "high"

[providers]
active = ["binance_websocket", "gzip_historical"]

[[providers.config]]
name = "binance_websocket"
type = "BinanceWebSocket"
enabled = true
priority = 1
config_file = "configs/providers/binance_websocket.toml"
```

### Provider Configurations

Each provider has its own configuration file with specific settings:

#### Binance WebSocket (configs/providers/binance_websocket.toml)

```toml
[connection]
base_url = "wss://fstream.binance.com"
reconnect_enabled = true

[subscription]
symbols = ["BTCFDUSD", "ETHFDUSD"]
streams = ["bookTicker", "trade", "depth@100ms"]
```

#### Gzip Historical (configs/providers/gzip_historical.toml)

```toml
[data_source]
data_directory = "./data"
file_pattern = "*.gz"
symbols = ["BTCFDUSD"]

[playback]
initial_speed = 1.0
auto_start = true
```

## Configuration Structure

### Core Components

1. **ConfigManager**: Main configuration management class
   - Loads and manages all configurations
   - Provides access to global and provider configs
   - Supports runtime updates

2. **GlobalConfig**: System-wide settings
   - System configuration
   - Runtime settings
   - Provider activation
   - GUI preferences
   - Monitoring setup

3. **ProviderConfig**: Provider-specific settings
   - Connection parameters
   - Authentication credentials
   - Subscription settings
   - Performance tuning
   - Error handling

## OOP Design Principles

### 1. Encapsulation
- Configuration details are encapsulated within dedicated structures
- Private helper methods handle internal logic
- Public interfaces provide controlled access

### 2. Polymorphism
- `ProviderConfig` enum enables polymorphic provider configurations
- `ProviderConfigTrait` defines common interface
- Factory pattern creates providers based on configuration type

### 3. Composition
- Configuration structures compose smaller, focused components
- Providers compose configuration with functionality
- Modular design enables easy extension

### 4. Backward Compatibility
- Default implementations for all configurations
- Optional fields with sensible defaults
- Graceful fallback when configuration files missing
- Compatible with existing provider implementations

## Advanced Usage

### Runtime Configuration Updates

```rust
// Load initial configuration
let mut provider = BinanceProvider::from_config(&config)?;

// Update configuration
let mut new_config = config.clone();
new_config.subscription.symbols.push("BNBFDUSD".to_string());

// Apply changes
provider.update_from_config(&new_config)?;
```

### Configuration Validation

```rust
use binance_futures::config::ProviderConfigTrait;

let config = BinanceWebSocketConfig::default();
match config.validate() {
    Ok(_) => println!("Configuration valid"),
    Err(e) => println!("Invalid configuration: {}", e),
}
```

### Programmatic Configuration

```rust
use binance_futures::config::GlobalConfig;

let config = GlobalConfig::builder()
    .system(SystemConfig {
        name: "Custom System".to_string(),
        log_level: "debug".to_string(),
        ..Default::default()
    })
    .runtime(RuntimeConfig {
        cpu_affinity: true,
        cpu_cores: vec![0, 1],
        ..Default::default()
    })
    .build();
```

## Environment Variables

The system supports environment variable overrides:

- `CONFIG_PATH`: Override default config.toml location
- `LOG_LEVEL`: Override logging level
- `PROVIDER_CONFIG_DIR`: Override provider config directory

## Migration Guide

### From Hard-Coded Configuration

Before:
```rust
let provider = BinanceProvider::new();
provider.connect("wss://fstream.binance.com");
provider.subscribe("BTCFDUSD");
```

After:
```rust
let config = BinanceWebSocketConfig::default();
let provider = BinanceProvider::from_config(&config)?;
// Configuration includes URL and symbols
```

### From Environment Variables

Before:
```rust
let url = std::env::var("BINANCE_WS_URL")?;
let symbol = std::env::var("TRADING_SYMBOL")?;
```

After:
```rust
// All settings in configuration files
let config = get_config();
let provider_config = config.provider("binance_websocket");
```

## Best Practices

1. **Keep Secrets Secure**: Never commit API keys to version control
2. **Use Defaults**: Leverage default configurations for development
3. **Validate Early**: Validate configurations at startup
4. **Log Changes**: Log configuration updates for debugging
5. **Test Configurations**: Include configuration tests in CI/CD

## Troubleshooting

### Configuration Not Loading

```rust
// Use load_or_default for resilience
let mut config = ConfigManager::new();
config.load_or_default();
```

### Provider Creation Fails

```rust
// Check if configuration exists and is valid
if let Some(config) = config_manager.provider("provider_name") {
    // Validate before use
    match create_provider(config) {
        Ok(p) => use_provider(p),
        Err(e) => log::error!("Provider creation failed: {}", e),
    }
}
```

### Performance Issues

- Reduce buffer sizes in configuration
- Disable unnecessary features
- Use appropriate performance mode

## Examples

See `examples/config_usage.rs` for complete working examples.

## Configuration Reference

For detailed configuration options, see the provider configuration files in `configs/providers/`.