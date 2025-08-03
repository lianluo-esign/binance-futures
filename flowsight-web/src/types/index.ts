// ============================================================================
// CORE TRADING DATA TYPES
// ============================================================================

export interface OrderFlow {
  price: number;
  bidVolume: number;
  askVolume: number;
  activeBuyVolume: number;
  activeSellVolume: number;
  historicalBuyVolume: number;
  historicalSellVolume: number;
  timestamp: number;
}

export interface MarketSnapshot {
  symbol: string;
  bestBid: number | null;
  bestAsk: number | null;
  currentPrice: number | null;
  spread: number;
  realizedVolatility: number;
  jumpSignal: number;
  orderBookImbalance: number;
  volumeWeightedMomentum: number;
  timestamp: number;
}

export interface TradeData {
  price: number;
  quantity: number;
  timestamp: number;
  isBuyerMaker: boolean;
  tradeId?: number;
  symbol?: string;
}

export interface PricePoint {
  timestamp: number;
  price: number;
}

export interface VolumePoint {
  timestamp: number;
  volume: number;
  isBuy: boolean;
}

// ============================================================================
// FOOTPRINT CHART TYPES
// ============================================================================

export interface FootprintLevel {
  price: number;
  buyVolume: number;
  sellVolume: number;
  totalVolume: number;
  netVolume: number; // buyVolume - sellVolume
  volumeIntensity: number; // 0-1 scale for color intensity
}

export interface FootprintCandle {
  timestamp: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  buyVolume: number;
  sellVolume: number;
  levels: Map<number, FootprintLevel>;
  tradeCount: number;
}

export interface FootprintData {
  candles: FootprintCandle[];
  timeframe: number; // in milliseconds (e.g., 5 * 60 * 1000 for 5 minutes)
  priceStep: number; // price aggregation step
  symbol: string;
  lastUpdate: number;
}

// ============================================================================
// WEBSOCKET MESSAGE INTERFACES
// ============================================================================

export interface WebSocketMessage {
  stream: string;
  data: unknown;
  timestamp: number;
}

export interface BinanceDepthStream {
  e: 'depthUpdate';
  E: number; // Event time
  s: string; // Symbol
  U: number; // First update ID in event
  u: number; // Final update ID in event
  b: [string, string][]; // Bids [price, quantity]
  a: [string, string][]; // Asks [price, quantity]
}

export interface BinanceTradeStream {
  e: 'trade';
  E: number; // Event time
  s: string; // Symbol
  t: number; // Trade ID
  p: string; // Price
  q: string; // Quantity
  b: number; // Buyer order ID
  a: number; // Seller order ID
  T: number; // Trade time
  m: boolean; // Is buyer maker
  M: boolean; // Ignore (always true for trade events)
}

export interface BinanceTickerStream {
  e: '24hrTicker';
  E: number; // Event time
  s: string; // Symbol
  p: string; // Price change
  P: string; // Price change percent
  w: string; // Weighted average price
  x: string; // Previous day's close price
  c: string; // Current day's close price
  Q: string; // Close trade's quantity
  b: string; // Best bid price
  B: string; // Best bid quantity
  a: string; // Best ask price
  A: string; // Best ask quantity
  o: string; // Open price
  h: string; // High price
  l: string; // Low price
  v: string; // Total traded base asset volume
  q: string; // Total traded quote asset volume
  O: number; // Statistics open time
  C: number; // Statistics close time
  F: number; // First trade ID
  L: number; // Last trade ID
  n: number; // Total number of trades
}

export interface BinanceBookTickerStream {
  u: number; // Order book updateId
  s: string; // Symbol
  b: string; // Best bid price
  B: string; // Best bid qty
  a: string; // Best ask price
  A: string; // Best ask qty
}

export interface WebSocketConnectionConfig {
  url: string;
  streams: string[];
  reconnectInterval: number;
  maxReconnectAttempts: number;
  heartbeatInterval: number;
}

// ============================================================================
// CHART-RELATED INTERFACES
// ============================================================================

export interface ChartDimensions {
  width: number;
  height: number;
  margin: {
    top: number;
    right: number;
    bottom: number;
    left: number;
  };
}

export interface ChartProps {
  width: number;
  height: number;
  data: unknown[];
  className?: string;
  onDataPointClick?: (dataPoint: unknown) => void;
  onDataPointHover?: (dataPoint: unknown | null) => void;
}

export interface LineChartDataPoint {
  timestamp: number;
  price: number;
  volume?: number;
  isBuy?: boolean;
}

export interface LineChartProps extends ChartProps {
  data: LineChartDataPoint[];
  showVolumeDots?: boolean;
  timeWindow?: number; // in milliseconds
  priceRange?: {
    min: number;
    max: number;
  };
  colors?: {
    line: string;
    buyDots: string;
    sellDots: string;
    currentPrice: string;
  };
}

export interface CandlestickDataPoint {
  timestamp: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export interface CandlestickChartProps extends ChartProps {
  data: CandlestickDataPoint[];
  colors?: {
    bullish: string;
    bearish: string;
    wick: string;
  };
}

export interface OrderBookChartProps extends ChartProps {
  data: OrderFlow[];
  maxLevels?: number;
  priceStep?: number;
  colors?: {
    bid: string;
    ask: string;
    spread: string;
    activeLevel: string;
  };
}

export interface FootprintChartProps extends ChartProps {
  data: FootprintCandle[];
  priceStep: number;
  colors?: {
    bullishCandle: string;
    bearishCandle: string;
    buyVolume: string;
    sellVolume: string;
    neutralVolume: string;
  };
  volumeIntensityScale?: [number, number]; // min/max for color intensity
}

export interface ChartTooltipData {
  x: number;
  y: number;
  content: React.ReactNode;
  visible: boolean;
}

export interface ChartAxisConfig {
  show: boolean;
  tickCount?: number;
  tickFormat?: (value: number) => string;
  gridLines?: boolean;
  label?: string;
}

export interface ChartConfig {
  xAxis: ChartAxisConfig;
  yAxis: ChartAxisConfig;
  animation?: {
    enabled: boolean;
    duration: number;
    easing: string;
  };
  interaction?: {
    zoom: boolean;
    pan: boolean;
    crosshair: boolean;
  };
}

// ============================================================================
// CONNECTION AND PERFORMANCE TYPES
// ============================================================================

export type ConnectionStatus =
  | 'connecting'
  | 'connected'
  | 'disconnected'
  | 'error'
  | 'reconnecting';

export interface PerformanceMetrics {
  messagesPerSecond: number;
  latency: number;
  memoryUsage: number;
  connectionUptime: number;
  totalMessages: number;
  errorCount: number;
  lastMessageTime: number;
}

export interface ConnectionInfo {
  status: ConnectionStatus;
  url: string;
  connectedAt: number | null;
  lastError: string | null;
  reconnectAttempts: number;
  maxReconnectAttempts: number;
}

// ============================================================================
// STATE MANAGEMENT INTERFACES (ZUSTAND STORES)
// ============================================================================

export interface OrderBookState {
  // Core data
  orderFlows: Map<number, OrderFlow>;
  marketSnapshot: MarketSnapshot | null;
  tradeHistory: TradeData[];
  currentPrice: number | null;
  
  // Connection and performance
  connectionStatus: ConnectionStatus;
  performanceMetrics: PerformanceMetrics;
  
  // Configuration
  symbol: string;
  priceStep: number;
  maxLevels: number;
  maxTradeHistory: number;
  
  // Actions
  updateOrderBook: (bids: [string, string][], asks: [string, string][]) => void;
  addTrade: (trade: TradeData) => void;
  updateMarketSnapshot: (snapshot: Partial<MarketSnapshot>) => void;
  setConnectionStatus: (status: ConnectionStatus) => void;
  updatePerformanceMetrics: (metrics: Partial<PerformanceMetrics>) => void;
  setSymbol: (symbol: string) => void;
  setPriceStep: (step: number) => void;
  clearData: () => void;
  cleanup: () => void;
}

export interface ConnectionState {
  // Connection info
  connections: Map<string, ConnectionInfo>;
  globalStatus: ConnectionStatus;
  
  // WebSocket management
  activeStreams: Set<string>;
  subscriptions: Map<string, string[]>; // symbol -> streams
  
  // Performance tracking
  globalMetrics: PerformanceMetrics;
  
  // Actions
  addConnection: (id: string, info: ConnectionInfo) => void;
  updateConnection: (id: string, updates: Partial<ConnectionInfo>) => void;
  removeConnection: (id: string) => void;
  addStream: (stream: string) => void;
  removeStream: (stream: string) => void;
  subscribe: (symbol: string, streams: string[]) => void;
  unsubscribe: (symbol: string) => void;
  updateGlobalMetrics: (metrics: Partial<PerformanceMetrics>) => void;
  reset: () => void;
}

export interface SettingsState {
  // Display settings
  theme: 'light' | 'dark' | 'auto';
  colorScheme: 'default' | 'high-contrast' | 'colorblind';
  
  // Trading settings
  defaultSymbol: string;
  defaultPriceStep: number;
  maxOrderBookLevels: number;
  chartTimeWindow: number; // in milliseconds
  
  // Chart settings
  showVolumeDots: boolean;
  enableAnimations: boolean;
  showGridLines: boolean;
  autoScale: boolean;
  
  // Performance settings
  maxTradeHistory: number;
  dataCleanupInterval: number; // in milliseconds
  renderThrottleMs: number;
  
  // Export settings
  exportFormat: 'json' | 'csv';
  includeTimestamps: boolean;
  
  // Actions
  setTheme: (theme: 'light' | 'dark' | 'auto') => void;
  setColorScheme: (scheme: 'default' | 'high-contrast' | 'colorblind') => void;
  setDefaultSymbol: (symbol: string) => void;
  setDefaultPriceStep: (step: number) => void;
  setMaxOrderBookLevels: (levels: number) => void;
  setChartTimeWindow: (window: number) => void;
  toggleVolumeDots: () => void;
  toggleAnimations: () => void;
  toggleGridLines: () => void;
  toggleAutoScale: () => void;
  setMaxTradeHistory: (max: number) => void;
  setDataCleanupInterval: (interval: number) => void;
  setRenderThrottleMs: (ms: number) => void;
  setExportFormat: (format: 'json' | 'csv') => void;
  toggleIncludeTimestamps: () => void;
  resetToDefaults: () => void;
  loadFromStorage: () => void;
  saveToStorage: () => void;
}

export interface FootprintState {
  // Core data
  footprintData: FootprintData | null;
  candles: FootprintCandle[];
  
  // Configuration
  timeframe: number; // in milliseconds
  priceStep: number;
  maxCandles: number;
  
  // Processing state
  isProcessing: boolean;
  lastUpdate: number;
  
  // Actions
  updateFootprintData: (data: FootprintData) => void;
  addCandle: (candle: FootprintCandle) => void;
  updateCandle: (timestamp: number, updates: Partial<FootprintCandle>) => void;
  setTimeframe: (timeframe: number) => void;
  setPriceStep: (step: number) => void;
  setMaxCandles: (max: number) => void;
  setProcessing: (processing: boolean) => void;
  clearData: () => void;
  cleanup: () => void;
}

// ============================================================================
// UTILITY AND HELPER TYPES
// ============================================================================

export interface DataExportOptions {
  format: 'json' | 'csv';
  includeTimestamps: boolean;
  dateRange?: {
    start: number;
    end: number;
  };
  dataTypes: ('orderbook' | 'trades' | 'footprint' | 'metrics')[];
}

export interface ErrorInfo {
  code: string;
  message: string;
  timestamp: number;
  context?: Record<string, unknown>;
  stack?: string;
}

export interface SystemHealth {
  memoryUsage: number;
  cpuUsage: number;
  connectionCount: number;
  messageQueueSize: number;
  errorRate: number;
  uptime: number;
}

export interface TechnicalIndicator {
  name: string;
  value: number;
  timestamp: number;
  parameters?: Record<string, unknown>;
}

export interface AlertConfig {
  id: string;
  type: 'price' | 'volume' | 'indicator' | 'connection';
  condition: 'above' | 'below' | 'equals' | 'change';
  threshold: number;
  enabled: boolean;
  message: string;
}

// Type guards for runtime type checking
export const isOrderFlow = (obj: unknown): obj is OrderFlow => {
  return (
    typeof obj === 'object' &&
    obj !== null &&
    typeof (obj as OrderFlow).price === 'number' &&
    typeof (obj as OrderFlow).bidVolume === 'number' &&
    typeof (obj as OrderFlow).askVolume === 'number' &&
    typeof (obj as OrderFlow).timestamp === 'number'
  );
};

export const isTradeData = (obj: unknown): obj is TradeData => {
  return (
    typeof obj === 'object' &&
    obj !== null &&
    typeof (obj as TradeData).price === 'number' &&
    typeof (obj as TradeData).quantity === 'number' &&
    typeof (obj as TradeData).timestamp === 'number' &&
    typeof (obj as TradeData).isBuyerMaker === 'boolean'
  );
};

export const isBinanceDepthStream = (obj: unknown): obj is BinanceDepthStream => {
  return (
    typeof obj === 'object' &&
    obj !== null &&
    (obj as BinanceDepthStream).e === 'depthUpdate' &&
    typeof (obj as BinanceDepthStream).E === 'number' &&
    typeof (obj as BinanceDepthStream).s === 'string' &&
    Array.isArray((obj as BinanceDepthStream).b) &&
    Array.isArray((obj as BinanceDepthStream).a)
  );
};

export const isBinanceTradeStream = (obj: unknown): obj is BinanceTradeStream => {
  return (
    typeof obj === 'object' &&
    obj !== null &&
    (obj as BinanceTradeStream).e === 'trade' &&
    typeof (obj as BinanceTradeStream).E === 'number' &&
    typeof (obj as BinanceTradeStream).s === 'string' &&
    typeof (obj as BinanceTradeStream).p === 'string' &&
    typeof (obj as BinanceTradeStream).q === 'string'
  );
};
