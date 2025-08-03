import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';
import {
  ConnectionStatus,
  ConnectionInfo,
  PerformanceMetrics,
  ConnectionState,
} from '../types';

// Default values
const DEFAULT_MAX_RECONNECT_ATTEMPTS = 5;
const PERFORMANCE_UPDATE_INTERVAL = 1000; // 1 second

const createDefaultPerformanceMetrics = (): PerformanceMetrics => ({
  messagesPerSecond: 0,
  latency: 0,
  memoryUsage: 0,
  connectionUptime: 0,
  totalMessages: 0,
  errorCount: 0,
  lastMessageTime: 0,
});

const createDefaultConnectionInfo = (url: string): ConnectionInfo => ({
  status: 'disconnected',
  url,
  connectedAt: null,
  lastError: null,
  reconnectAttempts: 0,
  maxReconnectAttempts: DEFAULT_MAX_RECONNECT_ATTEMPTS,
});

export const useConnectionStore = create<ConnectionState>()(
  subscribeWithSelector((set, get) => {
    // Performance metrics tracking
    let performanceInterval: NodeJS.Timeout | null = null;
    let messageCountBuffer: number[] = [];
    let latencyBuffer: number[] = [];

    const startPerformanceTracking = () => {
      if (performanceInterval) clearInterval(performanceInterval);
      
      performanceInterval = setInterval(() => {
        const state = get();
        const now = Date.now();

        // Calculate messages per second from buffer
        const messagesInLastSecond = messageCountBuffer.length;
        messageCountBuffer = [];

        // Calculate average latency from buffer
        const avgLatency = latencyBuffer.length > 0 
          ? latencyBuffer.reduce((sum, lat) => sum + lat, 0) / latencyBuffer.length 
          : 0;
        latencyBuffer = [];

        // Calculate connection uptime
        let totalUptime = 0;
        for (const connection of state.connections.values()) {
          if (connection.connectedAt) {
            totalUptime += now - connection.connectedAt;
          }
        }

        // Estimate memory usage (rough calculation)
        const estimatedMemoryUsage = 
          state.connections.size * 1024 + // ~1KB per connection
          state.activeStreams.size * 512 + // ~512B per stream
          state.subscriptions.size * 256; // ~256B per subscription

        set({
          globalMetrics: {
            ...state.globalMetrics,
            messagesPerSecond: messagesInLastSecond,
            latency: avgLatency,
            memoryUsage: estimatedMemoryUsage,
            connectionUptime: totalUptime,
            lastMessageTime: now,
          },
        });
      }, PERFORMANCE_UPDATE_INTERVAL);
    };

    // Start performance tracking on store creation
    startPerformanceTracking();

    return {
      // Connection info
      connections: new Map<string, ConnectionInfo>(),
      globalStatus: 'disconnected',

      // WebSocket management
      activeStreams: new Set<string>(),
      subscriptions: new Map<string, string[]>(),

      // Performance tracking
      globalMetrics: createDefaultPerformanceMetrics(),

      // Actions
      addConnection: (id: string, info: ConnectionInfo) => {
        const state = get();
        const newConnections = new Map(state.connections);
        newConnections.set(id, info);

        // Update global status based on all connections
        const statuses = Array.from(newConnections.values()).map(conn => conn.status);
        let globalStatus: ConnectionStatus = 'disconnected';
        
        if (statuses.some(status => status === 'connected')) {
          globalStatus = 'connected';
        } else if (statuses.some(status => status === 'connecting' || status === 'reconnecting')) {
          globalStatus = 'connecting';
        } else if (statuses.some(status => status === 'error')) {
          globalStatus = 'error';
        }

        set({
          connections: newConnections,
          globalStatus,
        });
      },

      updateConnection: (id: string, updates: Partial<ConnectionInfo>) => {
        const state = get();
        const existing = state.connections.get(id);
        if (!existing) return;

        const newConnections = new Map(state.connections);
        const updatedConnection = { ...existing, ...updates };
        newConnections.set(id, updatedConnection);

        // Update global status
        const statuses = Array.from(newConnections.values()).map(conn => conn.status);
        let globalStatus: ConnectionStatus = 'disconnected';
        
        if (statuses.some(status => status === 'connected')) {
          globalStatus = 'connected';
        } else if (statuses.some(status => status === 'connecting' || status === 'reconnecting')) {
          globalStatus = 'connecting';
        } else if (statuses.some(status => status === 'error')) {
          globalStatus = 'error';
        }

        set({
          connections: newConnections,
          globalStatus,
        });
      },

      removeConnection: (id: string) => {
        const state = get();
        const newConnections = new Map(state.connections);
        newConnections.delete(id);

        // Update global status
        const statuses = Array.from(newConnections.values()).map(conn => conn.status);
        let globalStatus: ConnectionStatus = 'disconnected';
        
        if (statuses.some(status => status === 'connected')) {
          globalStatus = 'connected';
        } else if (statuses.some(status => status === 'connecting' || status === 'reconnecting')) {
          globalStatus = 'connecting';
        } else if (statuses.some(status => status === 'error')) {
          globalStatus = 'error';
        }

        set({
          connections: newConnections,
          globalStatus,
        });
      },

      addStream: (stream: string) => {
        const state = get();
        const newStreams = new Set(state.activeStreams);
        newStreams.add(stream);
        set({ activeStreams: newStreams });
      },

      removeStream: (stream: string) => {
        const state = get();
        const newStreams = new Set(state.activeStreams);
        newStreams.delete(stream);
        set({ activeStreams: newStreams });
      },

      subscribe: (symbol: string, streams: string[]) => {
        const state = get();
        const newSubscriptions = new Map(state.subscriptions);
        newSubscriptions.set(symbol, streams);

        // Add streams to active streams
        const newActiveStreams = new Set(state.activeStreams);
        streams.forEach(stream => newActiveStreams.add(stream));

        set({
          subscriptions: newSubscriptions,
          activeStreams: newActiveStreams,
        });
      },

      unsubscribe: (symbol: string) => {
        const state = get();
        const streams = state.subscriptions.get(symbol);
        if (!streams) return;

        const newSubscriptions = new Map(state.subscriptions);
        newSubscriptions.delete(symbol);

        // Remove streams from active streams if no other symbols use them
        const newActiveStreams = new Set(state.activeStreams);
        streams.forEach(stream => {
          // Check if any other subscription uses this stream
          let streamInUse = false;
          for (const [otherSymbol, otherStreams] of newSubscriptions.entries()) {
            if (otherSymbol !== symbol && otherStreams.includes(stream)) {
              streamInUse = true;
              break;
            }
          }
          if (!streamInUse) {
            newActiveStreams.delete(stream);
          }
        });

        set({
          subscriptions: newSubscriptions,
          activeStreams: newActiveStreams,
        });
      },

      updateGlobalMetrics: (metrics: Partial<PerformanceMetrics>) => {
        const state = get();
        
        // Track message count for messages per second calculation
        if (metrics.totalMessages && metrics.totalMessages > state.globalMetrics.totalMessages) {
          messageCountBuffer.push(metrics.totalMessages - state.globalMetrics.totalMessages);
        }

        // Track latency for average calculation
        if (metrics.latency && metrics.latency > 0) {
          latencyBuffer.push(metrics.latency);
        }

        set({
          globalMetrics: {
            ...state.globalMetrics,
            ...metrics,
            lastMessageTime: Date.now(),
          },
        });
      },

      reset: () => {
        if (performanceInterval) {
          clearInterval(performanceInterval);
        }
        messageCountBuffer = [];
        latencyBuffer = [];
        
        set({
          connections: new Map(),
          globalStatus: 'disconnected',
          activeStreams: new Set(),
          subscriptions: new Map(),
          globalMetrics: createDefaultPerformanceMetrics(),
        });

        // Restart performance tracking
        startPerformanceTracking();
      },
    };
  })
);

// Utility functions for connection management
export const getConnectionHealth = (connections: Map<string, ConnectionInfo>) => {
  const totalConnections = connections.size;
  const connectedCount = Array.from(connections.values()).filter(
    conn => conn.status === 'connected'
  ).length;
  const errorCount = Array.from(connections.values()).filter(
    conn => conn.status === 'error'
  ).length;

  return {
    totalConnections,
    connectedCount,
    errorCount,
    healthPercentage: totalConnections > 0 ? (connectedCount / totalConnections) * 100 : 0,
    hasErrors: errorCount > 0,
  };
};

export const getReconnectionInfo = (connections: Map<string, ConnectionInfo>) => {
  const reconnectingConnections = Array.from(connections.values()).filter(
    conn => conn.status === 'reconnecting'
  );

  const totalReconnectAttempts = reconnectingConnections.reduce(
    (sum, conn) => sum + conn.reconnectAttempts,
    0
  );

  const maxAttemptsReached = reconnectingConnections.some(
    conn => conn.reconnectAttempts >= conn.maxReconnectAttempts
  );

  return {
    reconnectingCount: reconnectingConnections.length,
    totalReconnectAttempts,
    maxAttemptsReached,
    reconnectingConnections,
  };
};

export const getStreamInfo = (activeStreams: Set<string>, subscriptions: Map<string, string[]>) => {
  const totalStreams = activeStreams.size;
  const totalSubscriptions = subscriptions.size;
  const streamsPerSymbol = new Map<string, number>();

  for (const [symbol, streams] of subscriptions.entries()) {
    streamsPerSymbol.set(symbol, streams.length);
  }

  return {
    totalStreams,
    totalSubscriptions,
    streamsPerSymbol,
    averageStreamsPerSymbol: totalSubscriptions > 0 ? totalStreams / totalSubscriptions : 0,
  };
};

export const formatConnectionStatus = (status: ConnectionStatus): string => {
  switch (status) {
    case 'connecting':
      return 'Connecting...';
    case 'connected':
      return 'Connected';
    case 'disconnected':
      return 'Disconnected';
    case 'error':
      return 'Connection Error';
    case 'reconnecting':
      return 'Reconnecting...';
    default:
      return 'Unknown';
  }
};

export const formatLatency = (latency: number): string => {
  if (latency < 1) return '<1ms';
  if (latency < 1000) return `${Math.round(latency)}ms`;
  return `${(latency / 1000).toFixed(1)}s`;
};

export const formatUptime = (uptime: number): string => {
  const seconds = Math.floor(uptime / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) return `${days}d ${hours % 24}h`;
  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
  return `${seconds}s`;
};
