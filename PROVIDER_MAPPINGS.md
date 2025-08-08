# Provider Implementation Mappings

This document shows the explicit mapping between configuration provider names and their implementations.

| Config Name | Provider Type | Implementation File | Struct Name | Config Struct |
|-------------|---------------|---------------------|-------------|---------------|
| binance_websocket | BinanceWebSocket | `src/core/provider/binance_provider.rs` | `BinanceProvider` | `BinanceWebSocketConfig` |
| gzip_historical | GzipProvider | `src/core/provider/gzip_provider.rs` | `GzipProvider` | `GzipProviderConfig` |
| binance_rest | BinanceRest | `src/core/provider/binance_provider.rs` | `BinanceRestProvider` | `BinanceRestConfig` |
| historical_file | HistoricalFile | `src/core/provider/historical_provider.rs` | `HistoricalDataProvider` | `HistoricalDataConfig` |
| mock_provider | Mock | `src/core/provider/mock_provider.rs` | `MockProvider` | `MockProviderConfig` |

## Usage in config.toml

```toml
[[providers.config]]
name = "binance_websocket"     # Must match Config Name in table above
type = "BinanceWebSocket"       # Must match Provider Type in table above
enabled = true
config_file = "configs/providers/binance_websocket.toml"
```

## Explicit Mapping Declaration

The mapping between configuration names and implementation files is explicitly declared in `src/config/provider_mapping.rs`. This eliminates any guesswork and ensures:

1. **Clear Correspondence**: Every config name has a defined implementation file
2. **Type Safety**: Provider type must match the declared mapping
3. **Validation**: Configuration loading validates against these mappings
4. **Documentation**: Auto-generated documentation from the mappings

## Configuration Validation

The system validates configurations against these mappings:

- ✅ **Valid**: `binance_websocket` + `BinanceWebSocket` 
- ✅ **Valid**: `gzip_historical` + `GzipProvider`
- ❌ **Invalid**: `binance_websocket` + `WrongType` (type mismatch)
- ❌ **Invalid**: `unknown_provider` + `AnyType` (unknown provider)

## Implementation Status

| Provider | Implementation Status | Notes |
|----------|----------------------|--------|
| `binance_websocket` | ✅ Implemented | `BinanceProvider` in `binance_provider.rs` |
| `gzip_historical` | ✅ Implemented | `GzipProvider` in `gzip_provider.rs` |
| `binance_rest` | ⚠️ Partial | Needs `BinanceRestProvider` struct |
| `historical_file` | ✅ Implemented | `HistoricalDataProvider` in `historical_provider.rs` |
| `mock_provider` | ❌ TODO | Need to create `mock_provider.rs` |

## Adding New Providers

To add a new provider:

1. **Add mapping** in `src/config/provider_mapping.rs`
2. **Create implementation** file 
3. **Add configuration** struct in `provider_config.rs`
4. **Update factory** in `config_adapter.rs`
5. **Add to config.toml** as needed

## Source of Truth

**`src/config/provider_mapping.rs`** is the single source of truth for all provider mappings. All other code references this mapping registry.