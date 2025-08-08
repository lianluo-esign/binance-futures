# Configuration Naming Standards

## 📋 Overview

This document defines the **mandatory naming consistency standards** for the configuration system. All configurations must follow these rules to ensure proper system operation.

## 🔧 Naming Consistency Rules

### Rule 1: Provider Name Consistency

**MUST BE IDENTICAL** across all configuration files:

```toml
# config.toml
[[providers.config]]
name = "binance_websocket"           # ← Base name
type = "BinanceWebSocket"

# configs/providers/binance_websocket.toml  
[provider]
name = "binance_websocket"           # ← MUST match exactly
type = "BinanceWebSocket"            # ← MUST match exactly
```

### Rule 2: Provider Type Consistency

**Provider type identifiers MUST match** between:
- Global config `type` field
- Provider config `type` field  
- Mapping registry entry

### Rule 3: Mapping Registry Consistency

All provider names **MUST be registered** in `src/config/provider_mapping.rs`:

```rust
map.insert("binance_websocket", ProviderMapping {
    config_name: "binance_websocket",    # ← Must match config files
    provider_type: "BinanceWebSocket",   # ← Must match config files
    implementation_file: "core/provider/binance_provider.rs",
    struct_name: "BinanceProvider",
    config_struct: "BinanceWebSocketConfig",
});
```

## ✅ Current Valid Configurations

### Binance WebSocket Provider

**Global Config (`config.toml`):**
```toml
[[providers.config]]
name = "binance_websocket"
type = "BinanceWebSocket"
enabled = true
config_file = "configs/providers/binance_websocket.toml"
```

**Provider Config (`configs/providers/binance_websocket.toml`):**
```toml
[provider]
name = "binance_websocket"  # ✅ Matches global config
type = "BinanceWebSocket"   # ✅ Matches global config
version = "1.0.0"
```

**Implementation:** `src/core/provider/binance_provider.rs` → `BinanceProvider`

### Gzip Historical Provider

**Global Config (`config.toml`):**
```toml
[[providers.config]]
name = "gzip_historical"
type = "GzipProvider"
enabled = true
config_file = "configs/providers/gzip_historical.toml"
```

**Provider Config (`configs/providers/gzip_historical.toml`):**
```toml
[provider]
name = "gzip_historical"    # ✅ Matches global config
type = "GzipProvider"       # ✅ Matches global config
version = "1.0.0"
```

**Implementation:** `src/core/provider/gzip_provider.rs` → `GzipProvider`

## ❌ Common Mistakes

### Mistake 1: Inconsistent Names
```toml
# config.toml
name = "binance_websocket"

# binance_websocket.toml
name = "Binance WebSocket Provider"  # ❌ WRONG - doesn't match
```

### Mistake 2: Inconsistent Types
```toml
# config.toml
type = "BinanceWebSocket"

# binance_websocket.toml  
type = "BinanceWS"                   # ❌ WRONG - doesn't match
```

### Mistake 3: Unregistered Providers
```toml
# config.toml
name = "new_provider"                # ❌ WRONG - not in mapping registry
```

## 🔍 Validation Process

The system performs automatic validation:

1. **Load-time Validation**: Checks consistency when loading configurations
2. **Mapping Validation**: Ensures all names are registered in mapping registry
3. **Type Validation**: Verifies type consistency across files
4. **File Existence**: Confirms referenced config files exist

### Running Validation

```bash
# Check consistency
cargo run --example check_config_consistency

# Show all mappings  
cargo run --example show_provider_mappings
```

## 🛠️ Adding New Providers

To add a new provider, follow these steps **in order**:

### Step 1: Register in Mapping Registry

Add to `src/config/provider_mapping.rs`:
```rust
map.insert("my_new_provider", ProviderMapping {
    config_name: "my_new_provider",
    provider_type: "MyNewProvider", 
    implementation_file: "core/provider/my_new_provider.rs",
    struct_name: "MyNewProvider",
    config_struct: "MyNewProviderConfig",
});
```

### Step 2: Add to Global Config

Add to `config.toml`:
```toml
[[providers.config]]
name = "my_new_provider"        # Must match Step 1
type = "MyNewProvider"          # Must match Step 1  
enabled = true
config_file = "configs/providers/my_new_provider.toml"
```

### Step 3: Create Provider Config File

Create `configs/providers/my_new_provider.toml`:
```toml
[provider]
name = "my_new_provider"        # Must match Steps 1 & 2
type = "MyNewProvider"          # Must match Steps 1 & 2
version = "1.0.0"
```

### Step 4: Implement Provider

Create `src/core/provider/my_new_provider.rs` with `MyNewProvider` struct.

## 🚨 Enforcement

- **Build-time**: Configuration loading will fail with mismatched names
- **Runtime**: Provider creation will fail with validation errors
- **Tests**: Consistency tests verify all configurations match

## 📊 Consistency Report

Run the consistency checker to generate a detailed report:

```bash
cargo run --example check_config_consistency
```

This generates `CONFIG_CONSISTENCY_REPORT.md` with validation results.

## 🎯 Summary

**CRITICAL REQUIREMENTS:**
1. Provider `name` must be **identical** across all config files
2. Provider `type` must be **identical** across all config files  
3. All provider names must be **registered** in mapping registry
4. Implementation files must **exist** at declared paths
5. Use **snake_case** for provider names (e.g., `binance_websocket`)
6. Use **PascalCase** for provider types (e.g., `BinanceWebSocket`)

**Violations will cause system startup failures!**