import { useEffect, useRef } from 'react';
import { useConnectionStore } from '../stores/connection-store';
import { BinanceWebSocketManager } from '../services/binance-websocket-manager';

interface UseLayoutWebSocketOptions {
  symbol?: string;
  autoConnect?: boolean;
  streams?: string[];
}

export const useLayoutWebSocket = (options: UseLayoutWebSocketOptions = {}) => {
  const {
    symbol = 'BTCUSDT',
    autoConnect = true,
    streams = ['depth', 'trade', 'ticker']
  } = options;

  const wsManagerRef = useRef<BinanceWebSocketManager | null>(null);
  const { 
    addConnection, 
    updateConnection, 
    removeConnection,
    subscribe,
    updateGlobalMetrics 
  } = useConnectionStore();

  // Initialize WebSocket manager
  useEffect(() => {
    if (!autoConnect) return;

    const wsManager = new BinanceWebSocketManager({
      symbol,
      streams,
      reconnectInterval: 1000,
      maxReconnectAttempts: 5,
      heartbeatInterval: 30000,
    });

    wsManagerRef.current = wsManager;

    // Add connection to store
    addConnection('binance-main', {
      status: 'connecting',
      url: wsManager.getConnectionUrl(),
      connectedAt: null,
      lastError: null,
      reconnectAttempts: 0,
      maxReconnectAttempts: 5,
    });

    // Subscribe to streams
    subscribe(symbol, streams);

    // Set up event listeners
    wsManager.on('connecting', () => {
      updateConnection('binance-main', { 
        status: 'connecting',
        connectedAt: null 
      });
    });

    wsManager.on('connected', () => {
      updateConnection('binance-main', { 
        status: 'connected',
        connectedAt: Date.now(),
        lastError: null,
        reconnectAttempts: 0
      });
    });

    wsManager.on('disconnected', () => {
      updateConnection('binance-main', { 
        status: 'disconnected',
        connectedAt: null 
      });
    });

    wsManager.on('error', (error: Error) => {
      updateConnection('binance-main', { 
        status: 'error',
        lastError: error.message 
      });
    });

    wsManager.on('reconnecting', (attempt: number) => {
      updateConnection('binance-main', { 
        status: 'reconnecting',
        reconnectAttempts: attempt 
      });
    });

    wsManager.on('message', (data: any) => {
      // Update performance metrics
      updateGlobalMetrics({
        totalMessages: (wsManagerRef.current?.getMetrics().totalMessages || 0) + 1,
        latency: wsManagerRef.current?.getMetrics().latency || 0,
        lastMessageTime: Date.now(),
      });
    });

    // Connect
    wsManager.connect();

    // Cleanup function
    return () => {
      if (wsManagerRef.current) {
        wsManagerRef.current.disconnect();
        wsManagerRef.current.removeAllListeners();
      }
      removeConnection('binance-main');
    };
  }, [symbol, autoConnect, addConnection, updateConnection, removeConnection, subscribe, updateGlobalMetrics]);

  // Handle visibility change to pause/resume connection
  useEffect(() => {
    const handleVisibilityChange = () => {
      if (!wsManagerRef.current) return;

      if (document.hidden) {
        // Page is hidden, consider pausing or reducing frequency
        console.log('Page hidden, WebSocket remains active');
      } else {
        // Page is visible, ensure connection is active
        if (wsManagerRef.current.getConnectionStatus() === 'disconnected') {
          wsManagerRef.current.connect();
        }
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, []);

  // Handle window beforeunload to cleanup connections
  useEffect(() => {
    const handleBeforeUnload = () => {
      if (wsManagerRef.current) {
        wsManagerRef.current.disconnect();
      }
    };

    window.addEventListener('beforeunload', handleBeforeUnload);
    
    return () => {
      window.removeEventListener('beforeunload', handleBeforeUnload);
    };
  }, []);

  return {
    wsManager: wsManagerRef.current,
    isConnected: wsManagerRef.current?.getConnectionStatus() === 'connected',
    reconnect: () => wsManagerRef.current?.connect(),
    disconnect: () => wsManagerRef.current?.disconnect(),
    changeSymbol: (newSymbol: string) => {
      if (wsManagerRef.current) {
        wsManagerRef.current.changeSymbol(newSymbol);
        subscribe(newSymbol, streams);
      }
    },
  };
};