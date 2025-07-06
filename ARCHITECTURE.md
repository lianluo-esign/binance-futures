# Technical Architecture Document
## Binance Futures Trading Application - FlowSight

### 1. System Overview

FlowSight is a high-performance, real-time cryptocurrency trading analysis application built in Rust, designed for professional traders requiring ultra-low latency market data processing and visualization. The system employs an event-driven architecture with lock-free data structures to achieve sub-millisecond processing times.

#### Key Architectural Principles
- **Event-Driven Architecture**: Decoupled components communicating through a high-performance EventBus
- **Lock-Free Design**: Utilizing atomic operations and lock-free data structures for maximum performance
- **Single-Threaded Core**: Optimized for single-core performance with CPU affinity support
- **Memory Efficiency**: Cache-friendly data layouts and minimal memory allocations
- **Real-Time Processing**: Sub-millisecond event processing with 1ms UI refresh rates

### 2. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        FlowSight Application                    │
├─────────────────────────────────────────────────────────────────┤
│  GUI Layer (egui)                                              │
│  ├── TradingGUI                                                │
│  ├── UnifiedOrderBookWidget                                    │
│  └── DebugWindow                                               │
├─────────────────────────────────────────────────────────────────┤
│  Application Layer                                             │
│  ├── ReactiveApp (Main Coordinator)                           │
│  ├── Config Management                                         │
│  └── Performance Monitoring                                    │
├─────────────────────────────────────────────────────────────────┤
│  Business Logic Layer                                          │
│  ├── OrderBookManager                                          │
│  ├── Event Handlers                                            │
│  └── Market Data Processing                                    │
├─────────────────────────────────────────────────────────────────┤
│  Event System Layer                                            │
│  ├── LockFreeEventDispatcher                                   │
│  ├── EventBus                                                  │
│  └── Event Types & Routing                                     │
├─────────────────────────────────────────────────────────────────┤
│  Core Infrastructure                                           │
│  ├── LockFreeRingBuffer                                        │
│  ├── RingBuffer                                                │
│  └── Performance Monitoring                                    │
├─────────────────────────────────────────────────────────────────┤
│  Network Layer                                                 │
│  ├── WebSocketManager                                          │
│  ├── WebSocketConnection                                       │
│  └── Binance API Integration                                   │
└─────────────────────────────────────────────────────────────────┘
```

### 3. Core Components

#### 3.1 Event System Architecture

**LockFreeEventDispatcher**
- Central event coordination using atomic operations
- Multi-producer, single-consumer pattern
- Zero-allocation event publishing
- Batch processing capabilities for high throughput

**EventBus Implementation**
- Type-safe event routing with compile-time guarantees
- Event filtering and prioritization
- Comprehensive statistics and monitoring
- Graceful error handling and recovery

**Event Types**
```rust
pub enum EventType {
    TickPrice(Value),      // Price tick updates
    DepthUpdate(Value),    // Order book depth changes
    Trade(Value),          // Individual trade executions
    BookTicker(Value),     // Best bid/ask updates
    Signal(Value),         // Generated trading signals
    WebSocketError(String) // Connection errors
}
```

#### 3.2 High-Performance Data Structures

**LockFreeRingBuffer**
- Atomic pointer-based implementation
- Cache-line aligned memory layout (64-byte alignment)
- CPU cache prefetching for optimal performance
- Power-of-2 sizing with bit-mask optimization
- SPMC (Single Producer, Multiple Consumer) support

**RingBuffer (Fallback)**
- Traditional mutex-based implementation
- Batch operation support
- Memory-efficient with MaybeUninit optimization
- Overflow handling with configurable policies

#### 3.3 WebSocket Integration

**Connection Management**
- Automatic reconnection with exponential backoff
- 24-hour connection lifecycle management (Binance requirement)
- Non-blocking I/O with proper error handling
- Multiple stream subscription support

**Data Streams**
- `{symbol}@depth20@100ms`: Order book depth (20 levels, 100ms updates)
- `{symbol}@trade`: Individual trade executions
- `{symbol}@bookTicker`: Best bid/ask price updates

### 4. Data Flow Architecture

```
Binance WebSocket API
        │
        ▼
WebSocketManager ──► Message Parsing ──► Event Creation
        │                                      │
        ▼                                      ▼
Connection Health                    LockFreeEventDispatcher
Monitoring                                     │
        │                                      ▼
        ▼                              Event Processing
Error Recovery                         (Batch Mode)
& Reconnection                               │
                                            ▼
                                   OrderBookManager
                                   (State Updates)
                                            │
                                            ▼
                                   Market Data Analysis
                                   (Price/Volume/Signals)
                                            │
                                            ▼
                                      GUI Rendering
                                   (1ms Refresh Rate)
```

### 5. Module Structure

```
src/
├── core/                          # Core data structures
│   ├── mod.rs                     # Module exports
│   ├── ring_buffer.rs             # Traditional ring buffer
│   └── lock_free_ring_buffer.rs   # Lock-free implementation
├── events/                        # Event system
│   ├── mod.rs                     # Event system exports
│   ├── event_types.rs             # Event definitions
│   ├── event_bus.rs               # EventBus implementation
│   ├── dispatcher.rs              # Event dispatcher
│   ├── lock_free_dispatcher.rs    # Lock-free dispatcher
│   └── lock_free_event_bus.rs     # Lock-free event bus
├── handlers/                      # Event handlers
│   ├── mod.rs                     # Handler exports
│   ├── market_data.rs             # Market data processing
│   ├── trading.rs                 # Trading event handling
│   ├── errors.rs                  # Error handling
│   └── global.rs                  # Global event monitoring
├── orderbook/                     # Order book management
│   ├── mod.rs                     # OrderBook exports
│   ├── manager.rs                 # OrderBookManager
│   ├── data_structures.rs         # Data types
│   └── analysis.rs                # Market analysis
├── websocket/                     # WebSocket layer
│   ├── mod.rs                     # WebSocket exports
│   ├── manager.rs                 # WebSocketManager
│   └── connection.rs              # Connection handling
├── gui/                           # GUI components
│   ├── mod.rs                     # GUI exports
│   ├── egui_app.rs                # Main application
│   ├── unified_orderbook_widget.rs # Order book display
│   ├── orderbook_widget.rs        # Legacy widget
│   └── debug_window.rs            # Debug interface
├── app/                           # Application layer
│   ├── mod.rs                     # App exports
│   └── reactive_app.rs            # Main application logic
├── monitoring/                    # Performance monitoring
│   └── mod.rs                     # Monitoring systems
├── lib.rs                         # Library interface
└── main.rs                        # Application entry point
```

### 6. Performance Optimizations

#### 6.1 Memory Management
- **Zero-Copy Operations**: Minimal data copying between components
- **Cache-Line Alignment**: 64-byte alignment for critical data structures
- **Memory Prefetching**: CPU cache prefetching hints for predictable access patterns
- **Object Pooling**: Reuse of frequently allocated objects

#### 6.2 CPU Optimizations
- **Bit-Mask Operations**: Power-of-2 sizing for fast modulo operations
- **Branch Prediction**: Optimized conditional logic for hot paths
- **CPU Affinity**: Core binding for consistent performance
- **SIMD Instructions**: Vectorized operations where applicable

#### 6.3 Lock-Free Algorithms
- **Compare-and-Swap (CAS)**: Atomic operations for thread-safe updates
- **Memory Ordering**: Precise memory ordering semantics (Acquire/Release)
- **ABA Problem Prevention**: Proper handling of pointer reuse scenarios
- **Wait-Free Guarantees**: Bounded execution time for critical operations

### 7. Configuration Management

**Config Structure**
```rust
pub struct Config {
    pub symbol: String,              // Trading pair (default: "BTCUSDT")
    pub event_buffer_size: usize,    // Event buffer capacity
    pub max_reconnect_attempts: u32, // WebSocket reconnection limit
    pub max_visible_rows: usize,     // UI display rows (3000)
    pub price_precision: f64,        // Price aggregation (0.01 USD)
}
```

**Builder Pattern**
```rust
let config = Config::new("BTCUSDT".to_string())
    .with_buffer_size(65536)
    .with_max_reconnects(5)
    .with_max_visible_rows(3000)
    .with_price_precision(0.01);
```

### 8. Error Handling and Recovery

#### 8.1 Error Categories
- **Network Errors**: WebSocket disconnections, timeout handling
- **Data Errors**: Malformed JSON, invalid market data
- **System Errors**: Memory allocation failures, resource exhaustion
- **Application Errors**: Logic errors, state inconsistencies

#### 8.2 Recovery Strategies
- **Automatic Reconnection**: Exponential backoff with jitter
- **Circuit Breaker**: Temporary suspension during repeated failures
- **Graceful Degradation**: Continued operation with reduced functionality
- **State Recovery**: Rebuilding application state after errors

### 9. Monitoring and Observability

#### 9.1 Performance Metrics
- **Event Processing Rate**: Events per second throughput
- **Latency Percentiles**: P50, P95, P99 processing times
- **Memory Usage**: Heap allocation and buffer utilization
- **Network Statistics**: Message rates, connection stability

#### 9.2 Health Monitoring
- **WebSocket Health**: Connection status, message flow
- **Buffer Utilization**: Ring buffer fill levels
- **Error Rates**: Error frequency and categorization
- **System Resources**: CPU usage, memory consumption

### 10. Build and Deployment

#### 10.1 Build Configuration
```toml
[package]
name = "binance-futures"
version = "0.1.0"
edition = "2021"

[dependencies]
egui = "0.27"                    # GUI framework
eframe = "0.27"                  # Native window management
tungstenite = "0.24"             # WebSocket client
serde_json = "1.0"               # JSON processing
ordered-float = "4.5"            # Ordered floating point
core_affinity = "0.8"            # CPU affinity control
```

#### 10.2 Performance Build
```bash
# Release build with optimizations
cargo build --release

# Profile-guided optimization (PGO)
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" cargo build --release
# Run application to generate profile data
RUSTFLAGS="-Cprofile-use=/tmp/pgo-data" cargo build --release
```

#### 10.3 System Configuration
```bash
# CPU performance mode
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Disable transparent huge pages
echo never | sudo tee /sys/kernel/mm/transparent_hugepage/enabled

# Network interrupt affinity
echo 1 | sudo tee /proc/irq/*/smp_affinity
```

### 11. Technical Decisions and Rationale

#### 11.1 Choice of Rust
- **Memory Safety**: Zero-cost abstractions with compile-time guarantees
- **Performance**: Native performance comparable to C/C++
- **Concurrency**: Safe concurrency primitives and lock-free programming
- **Ecosystem**: Rich ecosystem for systems programming and GUI development

#### 11.2 egui GUI Framework
- **Performance**: Immediate mode GUI with minimal overhead
- **Cross-Platform**: Native performance on Windows, macOS, and Linux
- **Simplicity**: Easy integration with Rust applications
- **Real-Time**: Suitable for high-frequency data visualization

#### 11.3 Event-Driven Architecture
- **Scalability**: Easy to add new event types and handlers
- **Testability**: Components can be tested in isolation
- **Maintainability**: Clear separation of concerns
- **Performance**: Efficient event processing with minimal overhead

#### 11.4 Lock-Free Design
- **Latency**: Eliminates lock contention and priority inversion
- **Throughput**: Higher throughput under concurrent load
- **Predictability**: More predictable performance characteristics
- **Scalability**: Better scaling with multiple cores

### 12. Future Architecture Considerations

#### 12.1 Horizontal Scaling
- **Multi-Symbol Support**: Parallel processing of multiple trading pairs
- **Distributed Processing**: Event processing across multiple nodes
- **Load Balancing**: Dynamic load distribution based on market activity

#### 12.2 Advanced Features
- **Machine Learning Integration**: Real-time model inference
- **Historical Data**: Time-series database integration
- **Strategy Engine**: Pluggable trading strategy framework
- **Risk Management**: Real-time risk monitoring and controls

---

**Document Version:** 1.0
**Last Updated:** 2025-06-23
**Architecture Review:** Quarterly or upon major system changes
