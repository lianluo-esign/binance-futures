# Symbol Configuration for Binance Provider

## 📋 Overview

The Binance Market Provider now supports configuring multiple symbols directly in the configuration file. Each symbol will be automatically subscribed to all configured streams.

## ⚙️ Configuration

### Binance Provider Config (`configs/providers/binance_market_provider.toml`)

```toml
# Binance WebSocket Provider Configuration
# Real-time market data streaming via WebSocket

[provider]
# Provider identification - MUST match BinanceProvider::CANONICAL_NAME
name = "binance_market_provider"  # MUST match BinanceProvider::CANONICAL_NAME
type = "BinanceWebSocket"         # MUST match BinanceProvider::CANONICAL_TYPE  
version = "1.0.0"

[connection]
# WebSocket connection settings
base_url = "wss://fstream.binance.com"
reconnect_enabled = true
reconnect_interval = 5000  # milliseconds
max_reconnect_attempts = 10
ping_interval = 30000  # milliseconds
timeout = 60000  # milliseconds

[authentication]
# API credentials (optional for public streams)
api_key = ""  # Leave empty for public data
api_secret = ""  # Leave empty for public data

[subscription]
# Market data subscriptions - CONFIGURE YOUR SYMBOLS HERE
symbols = ["BTCFDUSD", "ETHFDUSD", "BNBFDUSD"]  # Add your desired symbols
streams = ["bookTicker", "trade", "depth@100ms"]  # Stream types for ALL symbols

# Stream-specific configurations
[subscription.book_ticker]
enabled = true
throttle = 0  # milliseconds, 0 = no throttle

[subscription.trade]
enabled = true
buffer_size = 1000

[subscription.depth]
enabled = true
levels = 20  # Order book depth levels
update_speed = "100ms"  # 100ms, 250ms, 500ms, 1000ms
```

## 🚀 How It Works

### Symbol Configuration

1. **Multiple Symbols**: Add any number of symbols to the `symbols` array
2. **Stream Multiplication**: Each symbol will be subscribed to ALL configured streams
3. **Automatic Generation**: WebSocket stream names are automatically generated

### Example Stream Generation

With configuration:
```toml
symbols = ["BTCFDUSD", "ETHFDUSD"]
streams = ["bookTicker", "trade"]
```

Generated WebSocket streams:
- `btcfdusd@bookTicker`
- `btcfdusd@trade`  
- `ethfdusd@bookTicker`
- `ethfdusd@trade`

### Stream Types Supported

| Config String | Binance Stream | Description |
|---------------|----------------|-------------|
| `"bookTicker"` | `symbol@bookTicker` | Best bid/ask prices |
| `"trade"` | `symbol@trade` | Individual trades |
| `"depth@100ms"` | `symbol@depth20@100ms` | Order book updates |
| `"depth@250ms"` | `symbol@depth20@250ms` | Order book updates |
| `"depth@500ms"` | `symbol@depth20@500ms` | Order book updates |

## 💻 Usage Examples

### Minimal Configuration (Single Symbol)

```toml
[subscription]
symbols = ["BTCFDUSD"]
streams = ["bookTicker"]
```

### Multi-Symbol Trading Setup

```toml
[subscription]
symbols = [
    "BTCFDUSD",   # Bitcoin
    "ETHFDUSD",   # Ethereum  
    "BNBFDUSD",   # BNB
    "ADAFDUSD",   # Cardano
    "SOLFDUSD"    # Solana
]
streams = ["bookTicker", "trade", "depth@100ms"]
```

### High-Frequency Trading Setup

```toml
[subscription]
symbols = ["BTCFDUSD", "ETHFDUSD"]
streams = ["trade", "depth@100ms"]  # Fast updates only

[subscription.depth]
levels = 5     # Less depth for faster processing
update_speed = "100ms"  # Fastest updates
```

## 🔧 Implementation Details

### Provider Creation

```rust
// Configuration is automatically loaded
let config_manager = ConfigManager::new();
config_manager.load()?;

// Provider created with all configured symbols
let ws_config = config_manager.provider("binance_market_provider")?;
let provider = BinanceProvider::from_config(ws_config)?;

// All symbols and streams are automatically configured
provider.initialize()?;
```

### Multi-Symbol WebSocket

The provider automatically:
1. **Reads symbols** from configuration
2. **Generates stream names** for each symbol-stream combination  
3. **Creates WebSocket subscriptions** for all combinations
4. **Handles reconnection** for all streams
5. **Routes events** with proper symbol identification

### Performance Considerations

- **More symbols = more data**: Each additional symbol multiplies stream count
- **Network bandwidth**: Consider connection capacity for many symbols
- **Processing load**: More events require more CPU for processing
- **Memory usage**: Larger event buffers for multiple streams

## 📊 Monitoring

The provider logs all configured symbols at startup:

```
INFO WebSocket管理器初始化完成，主要symbol: BTCFDUSD，所有symbols: ["BTCFDUSD", "ETHFDUSD", "BNBFDUSD"]
INFO 构建WebSocket流订阅: ["btcfdusd@bookTicker", "btcfdusd@trade", "ethfdusd@bookTicker", "ethfdusd@trade", "bnbfdusd@bookTicker", "bnbfdusd@trade"]
```

## 🚨 Important Notes

1. **Symbol Format**: Use uppercase format (e.g., "BTCFDUSD", not "btcfdusd")
2. **Stream Limits**: Binance has limits on concurrent streams per connection
3. **Rate Limits**: More symbols may trigger rate limiting faster
4. **Configuration Validation**: Invalid symbols will cause initialization to fail
5. **Primary Symbol**: First symbol in list is used as "primary" for WebSocket config

## 🧪 Testing

Test your symbol configuration:

```bash
cargo run --example test_binance_symbols_config
```

This will:
- Load your configuration
- Show all configured symbols
- Test provider creation
- Validate WebSocket stream generation
- Display any configuration issues