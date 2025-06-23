# Product Requirements Document (PRD)
## Binance Futures Trading Application - FlowSight

### 1. Product Vision and Objectives

**Product Name:** FlowSight
**Version:** 0.1.0
**Target Market:** Professional cryptocurrency traders and quantitative analysts

#### Vision Statement
FlowSight is a high-performance, real-time order flow analysis system for Binance futures trading, designed to provide traders with comprehensive market depth visualization and trade execution insights through an intuitive desktop GUI interface.

#### Primary Objectives
- Deliver real-time order book visualization with sub-millisecond latency
- Provide comprehensive order flow analysis with historical footprint data
- Enable professional traders to make informed decisions through advanced market microstructure analysis
- Maintain system stability and reliability for continuous 24/7 operation

### 2. Current Features

#### 2.1 Core Trading Interface
- **Real-time Order Book Display**: Live BTCUSDT order book with ±40 price levels around current market price
- **Unified Data Visualization**: Merged order book depth and trade footprint data in a single comprehensive table
- **Price Level Aggregation**: 1 USD price level aggregation using floor rounding for cleaner liquidity display
- **Auto-tracking Price Movement**: Synchronized scrolling that automatically centers on current price movements

#### 2.2 GUI Framework (egui-based)
- **Native Desktop Application**: Built with egui framework for responsive, high-performance GUI
- **Full Window Layout**: Responsive design that occupies full window height and width (1200x800 default)
- **Dark Theme Interface**: Professional dark background with optimized color scheme for extended trading sessions
- **Real-time Updates**: 1ms refresh intervals for ultra-low latency data visualization

#### 2.3 Data Visualization Features
- **Volume Bar Charts**: Horizontal bar charts behind order quantities with proportional scaling
- **Color-coded Orders**: Blue for bid orders, dark red for ask orders (non-standard color scheme preference)
- **Active Volume Highlighting**: Green color for active buy volume (主动买单量) with bold formatting
- **Historical Data Integration**: 5-second cumulative trade data with historical footprint analysis

#### 2.4 Technical Performance
- **High-frequency Data Processing**: Event-driven architecture supporting high-frequency market data
- **WebSocket Integration**: Real-time Binance futures API connectivity with automatic reconnection
- **Memory Optimization**: Lock-free ring buffer implementation for optimal performance
- **CPU Affinity Support**: Core affinity settings for dedicated processing power

### 3. User Requirements

#### 3.1 Visual Design Requirements
- **Color Scheme**: Blue for bid orders, dark red for ask orders, green for active buy volume
- **Typography**: Bold formatting for active buy and sell order volume numbers
- **Layout Structure**: 5% header/95% table proportions with fixed table headers
- **No Zebra Striping**: Clean table design without alternating row colors
- **Branding**: Product logo integration at src/image/logo.png with window and taskbar icon support

#### 3.2 Data Display Requirements
- **Price Levels**: Exactly ±40 price levels around current price with dynamic padding
- **Empty Level Handling**: Zero volume placeholders when market data is sparse
- **Column Structure**:
  1. Active sell orders (5s cumulative)
  2. Bid depth
  3. Price (center column)
  4. Ask depth
  5. Active buy orders (5s cumulative)
  6. Historical cumulative buy orders
  7. Historical cumulative sell orders
  8. Order delta
  9. Total volume

#### 3.3 Performance Requirements
- **Update Frequency**: 1ms refresh intervals for both data processing and UI rendering
- **Data Cleaning**: Automatic removal of invalid bid/ask orders based on best prices
- **Latency**: Sub-millisecond event processing with lock-free architecture
- **Stability**: 24/7 operation capability with automatic error recovery

### 4. Functional Requirements

#### 4.1 Real-time Data Processing
- **WebSocket Streams**:
  - `{symbol}@depth20@100ms` - Order book depth updates
  - `{symbol}@trade` - Individual trade data
  - `{symbol}@bookTicker` - Best bid/ask ticker updates
- **Event Processing**: Lock-free event bus architecture with ring buffer optimization
- **Data Aggregation**: Real-time price level aggregation and volume summation

#### 4.2 Order Book Management
- **Price Level Tracking**: Dynamic price level management with automatic centering
- **Volume Calculation**: Real-time bid/ask volume calculations with historical tracking
- **Market State**: Current price tracking with best bid/ask monitoring
- **Data Validation**: Automatic filtering of invalid or stale order data

#### 4.3 User Interface Controls
- **Auto-tracking Toggle**: Enable/disable automatic price following
- **Manual Scrolling**: User-controlled table navigation when auto-tracking disabled
- **Connection Status**: Real-time WebSocket connection status indicator
- **Performance Metrics**: Events per second display in header bar

### 5. Non-Functional Requirements

#### 5.1 Performance Specifications
- **Latency**: < 1ms event processing time
- **Throughput**: Support for 10,000+ events per second
- **Memory Usage**: Efficient memory management with ring buffer recycling
- **CPU Utilization**: Optimized for single-core performance with CPU affinity

#### 5.2 Reliability Requirements
- **Uptime**: 99.9% availability target
- **Error Recovery**: Automatic WebSocket reconnection with exponential backoff
- **Data Integrity**: Guaranteed event ordering and data consistency
- **Graceful Degradation**: Continued operation during temporary network issues

#### 5.3 Usability Requirements
- **Response Time**: Immediate UI feedback (< 16ms for 60fps)
- **Visual Clarity**: High contrast colors for extended use
- **Information Density**: Maximum data visibility without clutter
- **Professional Appearance**: Clean, modern interface suitable for trading floors

### 6. Technical Constraints

#### 6.1 Platform Requirements
- **Operating System**: Windows (primary), with cross-platform Rust compatibility
- **Architecture**: x86_64 with optional CPU-specific optimizations
- **Dependencies**: Minimal external dependencies for security and performance
- **Network**: Stable internet connection for WebSocket data feeds

#### 6.2 Integration Requirements
- **Binance API**: Futures WebSocket API compliance
- **Data Format**: JSON message parsing and validation
- **Time Synchronization**: UTC timestamp handling for global markets
- **Logging**: File-based logging system (no console output to avoid UI interference)

### 7. Success Metrics and Acceptance Criteria

#### 7.1 Performance Metrics
- **Latency**: 99th percentile event processing < 1ms
- **Throughput**: Sustained 5,000+ events/second processing
- **UI Responsiveness**: Consistent 60fps rendering
- **Memory Efficiency**: < 100MB RAM usage under normal operation

#### 7.2 Functional Acceptance
- **Data Accuracy**: 100% order book data integrity
- **Visual Correctness**: Proper color coding and formatting
- **Auto-tracking**: Smooth price following without lag
- **Connection Stability**: < 0.1% WebSocket disconnection rate

#### 7.3 User Experience Metrics
- **Startup Time**: Application launch < 3 seconds
- **Visual Clarity**: All text readable in trading environment lighting
- **Information Accessibility**: All critical data visible without scrolling
- **Professional Standards**: Interface suitable for institutional trading use

### 8. Future Enhancements (Out of Scope)

- Multi-symbol support beyond BTCUSDT
- Historical data replay functionality
- Advanced charting and technical indicators
- Order execution capabilities
- Portfolio management features
- Multi-exchange connectivity

### 9. Risk Considerations

- **Market Data Dependency**: Reliance on Binance API availability
- **Performance Degradation**: Potential issues during high volatility periods
- **Memory Leaks**: Long-running application stability concerns
- **Network Latency**: Geographic distance from Binance servers impact

---

**Document Version:** 1.0
**Last Updated:** 2025-06-23
**Next Review:** Quarterly or upon major feature additions
