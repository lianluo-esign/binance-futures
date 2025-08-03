import { act, renderHook } from '@testing-library/react';
import {
  useConnectionStore,
  getConnectionHealth,
  getReconnectionInfo,
  getStreamInfo,
  formatConnectionStatus,
  formatLatency,
  formatUptime,
} from '../connection-store';
import { ConnectionInfo } from '../../types';

// Mock timers for performance tracking
jest.useFakeTimers();

describe('Connection Store', () => {
  beforeEach(() => {
    // Reset store state before each test
    useConnectionStore.getState().reset();
  });

  afterEach(() => {
    jest.clearAllTimers();
  });

  describe('Initial State', () => {
    it('should have correct initial state', () => {
      const { result } = renderHook(() => useConnectionStore());
      const state = result.current;

      expect(state.connections.size).toBe(0);
      expect(state.globalStatus).toBe('disconnected');
      expect(state.activeStreams.size).toBe(0);
      expect(state.subscriptions.size).toBe(0);
      expect(state.globalMetrics.messagesPerSecond).toBe(0);
      expect(state.globalMetrics.latency).toBe(0);
      expect(state.globalMetrics.totalMessages).toBe(0);
    });
  });

  describe('Connection Management', () => {
    it('should add connection', () => {
      const { result } = renderHook(() => useConnectionStore());

      const connectionInfo: ConnectionInfo = {
        status: 'connecting',
        url: 'wss://stream.binance.com:9443/ws',
        connectedAt: null,
        lastError: null,
        reconnectAttempts: 0,
        maxReconnectAttempts: 5,
      };

      act(() => {
        result.current.addConnection('binance-main', connectionInfo);
      });

      const state = result.current;
      expect(state.connections.size).toBe(1);
      expect(state.connections.get('binance-main')).toEqual(connectionInfo);
      expect(state.globalStatus).toBe('connecting');
    });

    it('should update connection status', () => {
      const { result } = renderHook(() => useConnectionStore());

      const connectionInfo: ConnectionInfo = {
        status: 'connecting',
        url: 'wss://stream.binance.com:9443/ws',
        connectedAt: null,
        lastError: null,
        reconnectAttempts: 0,
        maxReconnectAttempts: 5,
      };

      act(() => {
        result.current.addConnection('binance-main', connectionInfo);
      });

      act(() => {
        result.current.updateConnection('binance-main', {
          status: 'connected',
          connectedAt: Date.now(),
        });
      });

      const state = result.current;
      const connection = state.connections.get('binance-main');
      expect(connection?.status).toBe('connected');
      expect(connection?.connectedAt).toBeDefined();
      expect(state.globalStatus).toBe('connected');
    });

    it('should remove connection', () => {
      const { result } = renderHook(() => useConnectionStore());

      const connectionInfo: ConnectionInfo = {
        status: 'connected',
        url: 'wss://stream.binance.com:9443/ws',
        connectedAt: Date.now(),
        lastError: null,
        reconnectAttempts: 0,
        maxReconnectAttempts: 5,
      };

      act(() => {
        result.current.addConnection('binance-main', connectionInfo);
      });

      expect(result.current.connections.size).toBe(1);

      act(() => {
        result.current.removeConnection('binance-main');
      });

      const state = result.current;
      expect(state.connections.size).toBe(0);
      expect(state.globalStatus).toBe('disconnected');
    });

    it('should calculate global status correctly with multiple connections', () => {
      const { result } = renderHook(() => useConnectionStore());

      // Add connected connection
      act(() => {
        result.current.addConnection('conn1', {
          status: 'connected',
          url: 'wss://test1.com',
          connectedAt: Date.now(),
          lastError: null,
          reconnectAttempts: 0,
          maxReconnectAttempts: 5,
        });
      });

      expect(result.current.globalStatus).toBe('connected');

      // Add error connection - should still be connected overall
      act(() => {
        result.current.addConnection('conn2', {
          status: 'error',
          url: 'wss://test2.com',
          connectedAt: null,
          lastError: 'Connection failed',
          reconnectAttempts: 3,
          maxReconnectAttempts: 5,
        });
      });

      expect(result.current.globalStatus).toBe('connected');

      // Update first connection to error - should be error overall
      act(() => {
        result.current.updateConnection('conn1', { status: 'error' });
      });

      expect(result.current.globalStatus).toBe('error');
    });
  });

  describe('Stream Management', () => {
    it('should add and remove streams', () => {
      const { result } = renderHook(() => useConnectionStore());

      act(() => {
        result.current.addStream('btcusdt@depth');
      });

      expect(result.current.activeStreams.has('btcusdt@depth')).toBe(true);

      act(() => {
        result.current.removeStream('btcusdt@depth');
      });

      expect(result.current.activeStreams.has('btcusdt@depth')).toBe(false);
    });

    it('should manage subscriptions', () => {
      const { result } = renderHook(() => useConnectionStore());

      const streams = ['btcusdt@depth', 'btcusdt@trade'];

      act(() => {
        result.current.subscribe('BTCUSDT', streams);
      });

      const state = result.current;
      expect(state.subscriptions.get('BTCUSDT')).toEqual(streams);
      expect(state.activeStreams.has('btcusdt@depth')).toBe(true);
      expect(state.activeStreams.has('btcusdt@trade')).toBe(true);
    });

    it('should unsubscribe and clean up streams', () => {
      const { result } = renderHook(() => useConnectionStore());

      // Subscribe to two symbols with overlapping streams
      act(() => {
        result.current.subscribe('BTCUSDT', ['btcusdt@depth', 'btcusdt@trade']);
        result.current.subscribe('ETHUSDT', ['ethusdt@depth', 'btcusdt@depth']); // Shared stream
      });

      expect(result.current.subscriptions.size).toBe(2);
      expect(result.current.activeStreams.size).toBe(3);

      // Unsubscribe BTCUSDT
      act(() => {
        result.current.unsubscribe('BTCUSDT');
      });

      const state = result.current;
      expect(state.subscriptions.size).toBe(1);
      expect(state.activeStreams.has('btcusdt@trade')).toBe(false); // Should be removed
      expect(state.activeStreams.has('btcusdt@depth')).toBe(true); // Should remain (used by ETHUSDT)
      expect(state.activeStreams.has('ethusdt@depth')).toBe(true);
    });
  });

  describe('Performance Metrics', () => {
    it('should update global metrics', () => {
      const { result } = renderHook(() => useConnectionStore());

      const metrics = {
        messagesPerSecond: 100,
        latency: 50,
        totalMessages: 1000,
        errorCount: 5,
      };

      act(() => {
        result.current.updateGlobalMetrics(metrics);
      });

      const state = result.current;
      expect(state.globalMetrics.messagesPerSecond).toBe(100);
      expect(state.globalMetrics.latency).toBe(50);
      expect(state.globalMetrics.totalMessages).toBe(1000);
      expect(state.globalMetrics.errorCount).toBe(5);
    });

    it('should track performance metrics over time', () => {
      const { result } = renderHook(() => useConnectionStore());

      // Update metrics multiple times
      act(() => {
        result.current.updateGlobalMetrics({ totalMessages: 100, latency: 30 });
      });

      act(() => {
        result.current.updateGlobalMetrics({ totalMessages: 150, latency: 40 });
      });

      // Fast-forward time to trigger performance calculation
      act(() => {
        jest.advanceTimersByTime(1000);
      });

      const state = result.current;
      expect(state.globalMetrics.messagesPerSecond).toBeGreaterThan(0);
    });
  });

  describe('Store Reset', () => {
    it('should reset all state', () => {
      const { result } = renderHook(() => useConnectionStore());

      // Add some data
      act(() => {
        result.current.addConnection('test', {
          status: 'connected',
          url: 'wss://test.com',
          connectedAt: Date.now(),
          lastError: null,
          reconnectAttempts: 0,
          maxReconnectAttempts: 5,
        });
        result.current.subscribe('BTCUSDT', ['btcusdt@depth']);
        result.current.updateGlobalMetrics({ totalMessages: 100 });
      });

      expect(result.current.connections.size).toBe(1);
      expect(result.current.subscriptions.size).toBe(1);

      // Reset
      act(() => {
        result.current.reset();
      });

      const state = result.current;
      expect(state.connections.size).toBe(0);
      expect(state.globalStatus).toBe('disconnected');
      expect(state.activeStreams.size).toBe(0);
      expect(state.subscriptions.size).toBe(0);
      expect(state.globalMetrics.totalMessages).toBe(0);
    });
  });
});

describe('Connection Utility Functions', () => {
  describe('getConnectionHealth', () => {
    it('should calculate connection health correctly', () => {
      const connections = new Map([
        ['conn1', { status: 'connected' } as ConnectionInfo],
        ['conn2', { status: 'connected' } as ConnectionInfo],
        ['conn3', { status: 'error' } as ConnectionInfo],
        ['conn4', { status: 'disconnected' } as ConnectionInfo],
      ]);

      const health = getConnectionHealth(connections);

      expect(health.totalConnections).toBe(4);
      expect(health.connectedCount).toBe(2);
      expect(health.errorCount).toBe(1);
      expect(health.healthPercentage).toBe(50);
      expect(health.hasErrors).toBe(true);
    });

    it('should handle empty connections', () => {
      const connections = new Map();
      const health = getConnectionHealth(connections);

      expect(health.totalConnections).toBe(0);
      expect(health.connectedCount).toBe(0);
      expect(health.errorCount).toBe(0);
      expect(health.healthPercentage).toBe(0);
      expect(health.hasErrors).toBe(false);
    });
  });

  describe('getReconnectionInfo', () => {
    it('should calculate reconnection info correctly', () => {
      const connections = new Map([
        ['conn1', { status: 'reconnecting', reconnectAttempts: 2, maxReconnectAttempts: 5 } as ConnectionInfo],
        ['conn2', { status: 'reconnecting', reconnectAttempts: 5, maxReconnectAttempts: 5 } as ConnectionInfo],
        ['conn3', { status: 'connected', reconnectAttempts: 0, maxReconnectAttempts: 5 } as ConnectionInfo],
      ]);

      const info = getReconnectionInfo(connections);

      expect(info.reconnectingCount).toBe(2);
      expect(info.totalReconnectAttempts).toBe(7);
      expect(info.maxAttemptsReached).toBe(true);
      expect(info.reconnectingConnections).toHaveLength(2);
    });
  });

  describe('getStreamInfo', () => {
    it('should calculate stream info correctly', () => {
      const activeStreams = new Set(['btcusdt@depth', 'btcusdt@trade', 'ethusdt@depth']);
      const subscriptions = new Map([
        ['BTCUSDT', ['btcusdt@depth', 'btcusdt@trade']],
        ['ETHUSDT', ['ethusdt@depth']],
      ]);

      const info = getStreamInfo(activeStreams, subscriptions);

      expect(info.totalStreams).toBe(3);
      expect(info.totalSubscriptions).toBe(2);
      expect(info.streamsPerSymbol.get('BTCUSDT')).toBe(2);
      expect(info.streamsPerSymbol.get('ETHUSDT')).toBe(1);
      expect(info.averageStreamsPerSymbol).toBe(1.5);
    });
  });

  describe('formatConnectionStatus', () => {
    it('should format connection status correctly', () => {
      expect(formatConnectionStatus('connecting')).toBe('Connecting...');
      expect(formatConnectionStatus('connected')).toBe('Connected');
      expect(formatConnectionStatus('disconnected')).toBe('Disconnected');
      expect(formatConnectionStatus('error')).toBe('Connection Error');
      expect(formatConnectionStatus('reconnecting')).toBe('Reconnecting...');
    });
  });

  describe('formatLatency', () => {
    it('should format latency correctly', () => {
      expect(formatLatency(0.5)).toBe('<1ms');
      expect(formatLatency(25)).toBe('25ms');
      expect(formatLatency(150)).toBe('150ms');
      expect(formatLatency(1500)).toBe('1.5s');
      expect(formatLatency(2000)).toBe('2.0s');
    });
  });

  describe('formatUptime', () => {
    it('should format uptime correctly', () => {
      expect(formatUptime(30 * 1000)).toBe('30s');
      expect(formatUptime(90 * 1000)).toBe('1m 30s');
      expect(formatUptime(3600 * 1000)).toBe('1h 0m');
      expect(formatUptime(3690 * 1000)).toBe('1h 1m');
      expect(formatUptime(25 * 3600 * 1000)).toBe('1d 1h');
      expect(formatUptime(48 * 3600 * 1000)).toBe('2d 0h');
    });
  });
});