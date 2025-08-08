# Canonical Provider Names System

## 🎯 Overview

The new canonical provider names system eliminates configuration guesswork by having each provider implementation define its own **canonical name** and **canonical type**. This ensures perfect consistency between code and configuration.

## ✅ Key Benefits

1. **No External Mappings**: Provider implementations define their own canonical names
2. **Startup Validation**: Program exits immediately if configuration names don't match
3. **Zero Ambiguity**: Each provider has exactly one valid name
4. **Type Safety**: Configuration loading validates against implementation constants
5. **Clear Error Messages**: Detailed error output when validation fails

## 🏗️ Implementation

### Provider Identity Trait

Each provider implements `ProviderIdentity`:

```rust
pub trait ProviderIdentity {
    const CANONICAL_NAME: &'static str;    // The ONLY valid config name
    const CANONICAL_TYPE: &'static str;    // The ONLY valid config type
    const DISPLAY_NAME: &'static str;      // Human-readable name
    const VERSION: &'static str;           // Provider version
    
    fn validate_config_name(&self, name: &str) -> Result<(), String>;
    fn validate_config_type(&self, type: &str) -> Result<(), String>;
}
```

### Provider Implementations

**BinanceProvider** (`src/core/provider/binance_provider.rs`):
```rust
impl ProviderIdentity for BinanceProvider {
    const CANONICAL_NAME: &'static str = "binance_market_provider";
    const CANONICAL_TYPE: &'static str = "BinanceWebSocket";
    const DISPLAY_NAME: &'static str = "Binance Market Data Provider";
    const VERSION: &'static str = "1.0.0";
}
```

**GzipProvider** (`src/core/provider/gzip_provider.rs`):
```rust
impl ProviderIdentity for GzipProvider {
    const CANONICAL_NAME: &'static str = "gzip_historical_provider";
    const CANONICAL_TYPE: &'static str = "GzipProvider";
    const DISPLAY_NAME: &'static str = "Gzip Historical Data Provider";
    const VERSION: &'static str = "1.0.0";
}
```

## 📋 Required Configuration

### Global Config (`config.toml`)

```toml
[providers]
active = ["binance_market_provider", "gzip_historical_provider"]

[[providers.config]]
name = "binance_market_provider"    # MUST match BinanceProvider::CANONICAL_NAME
type = "BinanceWebSocket"           # MUST match BinanceProvider::CANONICAL_TYPE
enabled = true
config_file = "configs/providers/binance_market_provider.toml"

[[providers.config]]
name = "gzip_historical_provider"   # MUST match GzipProvider::CANONICAL_NAME
type = "GzipProvider"               # MUST match GzipProvider::CANONICAL_TYPE
enabled = true
config_file = "configs/providers/gzip_historical_provider.toml"
```

### Provider Config Files

**`configs/providers/binance_market_provider.toml`**:
```toml
[provider]
name = "binance_market_provider"  # MUST match BinanceProvider::CANONICAL_NAME
type = "BinanceWebSocket"         # MUST match BinanceProvider::CANONICAL_TYPE
version = "1.0.0"
```

**`configs/providers/gzip_historical_provider.toml`**:
```toml
[provider]
name = "gzip_historical_provider"  # MUST match GzipProvider::CANONICAL_NAME
type = "GzipProvider"              # MUST match GzipProvider::CANONICAL_TYPE
version = "1.0.0"
```

## 🚨 Startup Validation

The system performs **mandatory validation** at startup:

```rust
// This happens automatically when loading configuration
validate_configuration_or_exit(&global_config);
```

### If Validation Fails

The program **immediately exits** with a detailed error message:

```
🚨 CONFIGURATION ERROR - STARTUP ABORTED
Provider: wrong_provider_name
Error: Provider name mismatch
Expected: 'binance_market_provider'
Found: 'wrong_provider_name'
Canonical Name: 'binance_market_provider'

📋 Fix Required:
Update your configuration files to use the correct provider name.
The provider implementation defines the canonical name that MUST be used.

🛑 Program terminated due to configuration errors.
Fix the configuration and try again.
```

## 📚 Valid Provider Names

| Implementation | Canonical Name | Canonical Type | Config File |
|---------------|----------------|----------------|-------------|
| `BinanceProvider` | `binance_market_provider` | `BinanceWebSocket` | `binance_market_provider.toml` |
| `GzipProvider` | `gzip_historical_provider` | `GzipProvider` | `gzip_historical_provider.toml` |

## 🛠️ Adding New Providers

To add a new provider:

1. **Implement ProviderIdentity**:
```rust
impl ProviderIdentity for MyNewProvider {
    const CANONICAL_NAME: &'static str = "my_new_provider";
    const CANONICAL_TYPE: &'static str = "MyNewProvider";
    const DISPLAY_NAME: &'static str = "My New Provider";
    const VERSION: &'static str = "1.0.0";
}
```

2. **Update startup validator** to recognize the new provider in `startup_validator.rs`

3. **Use canonical name in configs**:
```toml
[[providers.config]]
name = "my_new_provider"     # Must match CANONICAL_NAME
type = "MyNewProvider"       # Must match CANONICAL_TYPE
```

## 🧪 Testing

Run the canonical names test:
```bash
cargo run --example test_canonical_names
```

This will:
- Show all canonical names
- Test configuration validation
- Demonstrate error handling
- Verify provider identity validation

## 🎯 Migration from Old System

### Old System (❌ Removed)
- External `provider_mapping.rs` file
- Manual mapping maintenance
- Guesswork about correct names

### New System (✅ Active)
- Provider implementations define their own names
- Automatic validation at startup
- Zero configuration ambiguity
- Program exits immediately on mismatch

## 🔒 Guarantees

1. **Configuration Always Matches Code**: Impossible to have mismatched names
2. **Fast Failure**: Errors detected immediately at startup, not runtime
3. **Clear Error Messages**: Exact fix instructions provided
4. **No External Dependencies**: Everything defined in provider code
5. **Version Control Friendly**: Changes to names require code changes

## 📖 Summary

The canonical provider names system ensures **perfect consistency** between provider implementations and configuration files. Provider implementations are the **single source of truth** for their names and types, eliminating all guesswork and configuration errors.

**Key Rule**: If the configuration doesn't match the provider's canonical name exactly, the program will not start.