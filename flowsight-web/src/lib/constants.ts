// WebSocket configuration
export const WEBSOCKET_CONFIG = {
  RECONNECT_INTERVAL: 1000,
  MAX_RECONNECT_ATTEMPTS: 5,
  HEARTBEAT_INTERVAL: 30000,
  MESSAGE_QUEUE_SIZE: 1000,
} as const;

// Trading configuration
export const TRADING_CONFIG = {
  DEFAULT_SYMBOL: 'BTCUSDT',
  DEFAULT_PRICE_PRECISION: 2,
  DEFAULT_VOLUME_PRECISION: 4,
  ORDERBOOK_LEVELS: 40,
  CHART_TIMEFRAME: 5 * 60 * 1000, // 5 minutes in milliseconds
  DATA_RETENTION_TIME: 5 * 60 * 1000, // 5 minutes
} as const;

// Performance thresholds
export const PERFORMANCE_THRESHOLDS = {
  MAX_MEMORY_USAGE: 200 * 1024 * 1024, // 200MB
  MAX_LATENCY: 1000, // 1 second
  TARGET_FPS: 60,
  MAX_EVENT_QUEUE_SIZE: 1000,
} as const;

// UI configuration
export const UI_CONFIG = {
  PANEL_PROPORTIONS: {
    LEFT: 50, // Order book
    UPPER_RIGHT: 45, // Active order chart
    LOWER_RIGHT: 55, // Footprint chart
  },
  MOBILE_BREAKPOINT: 768,
  TABLET_BREAKPOINT: 1024,
} as const;

// Color scheme
export const COLORS = {
  BUY: '#10b981', // green-500
  SELL: '#ef4444', // red-500
  NEUTRAL: '#6b7280', // gray-500
  BACKGROUND: '#111827', // gray-900
  SURFACE: '#1f2937', // gray-800
  TEXT_PRIMARY: '#f9fafb', // gray-50
  TEXT_SECONDARY: '#d1d5db', // gray-300
} as const;
