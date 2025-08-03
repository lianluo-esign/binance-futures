import { BinanceWebSocketManager, BinanceStreamType } from '../binance-websocket-manager';
import { BinanceDepthStream, BinanceTradeStream, TradeData } from '../../types';

// Mock WebSocket (reuse from websocket-manager.test.ts)
class MockWebSocket {
  public onopen: ((event: Event) => void) | null = null;
  public onclose: ((event: CloseEvent) => void) | null = null;
  public onmessage: ((event: MessageEvent) => void) | null = null;
  public onerror: ((event: Event) => void) | null = null;
  public readyState: number = WebSocket.CONNECTING;
  
  constructor(public url: string) {
    setTimeout(() => {
      this.readyState = WebSocket.OPEN;
      if (this.onopen) {
        this.onopen(new Event('open'));
      }
    }, 10);
  }
  
  send(data: string | ArrayBufferLike | Blob | ArrayBufferView): void {
    // Mock send - simulate subscription confirmation
    try {
      const message = JSON.parse(data as string);
      if (message.method === 'SUBSCRIBE' || message.method === 'UNSUBSCRIBE') {
        // Simulate immediate response
        setTimeout(() => {
          if (this.onmessage) {
            this.onmessage(new MessageEvent('message', {
              data: JSON.stringify({
                result: null,
                id: message.id
              })
            }));
          }
        }, 5);
      }
    } catch (e) {
      // Ignore non-JSON messages
    }
  }
  
  close(code?: number, reason?: string): void {
    this.readyState = WebSocket.CLOSED;
    if (this.onclose) {
      this.onclose(new CloseEvent('close', { code: code || 1000, reason }));
    }
  }
  
  // Helper methods for testing
  simulateMessage(data: any): void {
    if (this.onmessage) {
      this.onmessage(new MessageEvent('message', { data: JSON.stringify(data) }));
    }
  }
  
  simulateStreamData(stream: string, data: any): void {
    if (this.onmessage) {
      this.onmessage(new MessageEvent('message', {
        data: JSON.stringify({ stream, data })
      }));
    }
  }
  
  simulateError(): void {
    if (this.onerror) {
      this.onerror(new Event('error'));
    }
  }
}

// Mock global WebSocket
(global as any).WebSocket = MockWebSocket;

describe('BinanceWebSocketManager', () => {
  let manager: BinanceWebSocketManager;
  
  beforeEach(() => {
    manager = new BinanceWebSocketManager({
      reconnectInterval: 100,
      maxReconnectAttempts: 3,
      heartbeatInterval: 1000,
    });
  });
  
  afterEach(() => {
    manager.destroy();
  });

  describe('Initialization', () => {
    it('should initialize with correct default URL', () => {
      const info = manager.getConnectionInfo();
      expect(info.url).toBe('wss://stream.binance.com:9443/ws');
    });

    it('should use testnet URL when specified', () => {
      const testnetManager = new BinanceWebSocketManager({ testnet: true });
      const info = testnetManager.getConnectionInfo();
      expect(info.url).toBe('wss://testnet.binance.vision/ws');
      testnetManager.destroy();
    });

    it('should use custom base URL when provided', () => {
      const customUrl = 'wss://custom.example.com/ws';
      const customManager = new BinanceWebSocketManager({ baseUrl: customUrl });
      const info = customManager.getConnectionInfo();
      expect(info.url).toBe(customUrl);
      customManager.destroy();
    });
  });

  describe('Stream Subscription', () => {
    beforeEach(async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
    });

    it('should subscribe to symbol streams', async () => {
      const confirmationHandler = jest.fn();
      manager.on('subscription-confirmed', confirmationHandler);

      await manager.subscribeToSymbol('BTCUSDT', ['depth', 'trade']);
      
      await new Promise(resolve => setTimeout(resolve, 50));
      
      expect(confirmationHandler).toHaveBeenCalled();
      
      const subscriptions = manager.getSubscriptions();
      expect(subscriptions.has('btcusdt')).toBe(true);
      expect(subscriptions.get('btcusdt')?.streams).toEqual(['depth', 'trade']);
    });

    it('should build correct stream names', async () => {
      const sendSpy = jest.spyOn(manager, 'send');
      
      await manager.subscribeToSymbol('ETHUSDT', ['depth', 'trade', 'ticker']);
      
      expect(sendSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          method: 'SUBSCRIBE',
          params: ['ethusdt@depth', 'ethusdt@trade', 'ethusdt@ticker']
        })
      );
    });

    it('should unsubscribe from symbol streams', async () => {
      await manager.subscribeToSymbol('BTCUSDT', ['depth', 'trade']);
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const sendSpy = jest.spyOn(manager, 'send');
      
      await manager.unsubscribeFromSymbol('BTCUSDT');
      
      expect(sendSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          method: 'UNSUBSCRIBE',
          params: ['btcusdt@depth', 'btcusdt@trade']
        })
      );
      
      const subscriptions = manager.getSubscriptions();
      expect(subscriptions.has('btcusdt')).toBe(false);
    });

    it('should subscribe to multiple symbols at once', async () => {
      const subscriptions = [
        { symbol: 'BTCUSDT', streams: ['depth', 'trade'] as BinanceStreamType[] },
        { symbol: 'ETHUSDT', streams: ['ticker'] as BinanceStreamType[] },
      ];
      
      await manager.subscribeToMultipleSymbols(subscriptions);
      
      const activeSubscriptions = manager.getSubscriptions();
      expect(activeSubscriptions.has('btcusdt')).toBe(true);
      expect(activeSubscriptions.has('ethusdt')).toBe(true);
    });

    it('should clear all subscriptions', async () => {
      await manager.subscribeToSymbol('BTCUSDT', ['depth']);
      await manager.subscribeToSymbol('ETHUSDT', ['trade']);
      
      await manager.clearAllSubscriptions();
      
      const subscriptions = manager.getSubscriptions();
      expect(subscriptions.size).toBe(0);
    });
  });

  describe('Message Processing', () => {
    beforeEach(async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
    });

    it('should process depth update messages', (done) => {
      const mockDepthData: BinanceDepthStream = {
        e: 'depthUpdate',
        E: 1234567890,
        s: 'BTCUSDT',
        U: 1,
        u: 2,
        b: [['50000.00', '1.5']],
        a: [['50100.00', '2.0']],
      };

      manager.on('depth-update', (data) => {
        expect(data).toEqual(mockDepthData);
        done();
      });

      const ws = (manager as any).ws as MockWebSocket;
      ws.simulateMessage(mockDepthData);
    });

    it('should process trade messages', (done) => {
      const mockTradeData: BinanceTradeStream = {
        e: 'trade',
        E: 1234567890,
        s: 'BTCUSDT',
        t: 12345,
        p: '50000.00',
        q: '0.1',
        b: 88,
        a: 50,
        T: 1234567890,
        m: true,
        M: true,
      };

      manager.on('processed-trade', (trade: TradeData) => {
        expect(trade.price).toBe(50000.00);
        expect(trade.quantity).toBe(0.1);
        expect(trade.isBuyerMaker).toBe(true);
        expect(trade.tradeId).toBe(12345);
        expect(trade.symbol).toBe('BTCUSDT');
        done();
      });

      const ws = (manager as any).ws as MockWebSocket;
      ws.simulateMessage(mockTradeData);
    });

    it('should process stream data format', (done) => {
      const mockDepthData: BinanceDepthStream = {
        e: 'depthUpdate',
        E: 1234567890,
        s: 'BTCUSDT',
        U: 1,
        u: 2,
        b: [['50000.00', '1.5']],
        a: [['50100.00', '2.0']],
      };

      manager.on('depth-update', (data) => {
        expect(data).toEqual(mockDepthData);
        done();
      });

      const ws = (manager as any).ws as MockWebSocket;
      ws.simulateStreamData('btcusdt@depth', mockDepthData);
    });

    it('should handle subscription responses', async () => {
      const confirmationHandler = jest.fn();
      manager.on('subscription-confirmed', confirmationHandler);

      // First add a pending subscription to simulate real scenario
      (manager as any).pendingSubscriptions.set(1, { method: 'SUBSCRIBE', params: ['btcusdt@depth'] });

      const ws = (manager as any).ws as MockWebSocket;
      ws.simulateMessage({
        result: null,
        id: 1
      });

      // Wait for message processing
      await new Promise(resolve => setTimeout(resolve, 10));

      expect(confirmationHandler).toHaveBeenCalledWith(1, null);
    });

    it('should handle subscription errors', async () => {
      const errorHandler = jest.fn();
      manager.on('subscription-error', errorHandler);

      // First add a pending subscription to simulate real scenario
      (manager as any).pendingSubscriptions.set(1, { method: 'SUBSCRIBE', params: ['invalid@depth'] });

      const ws = (manager as any).ws as MockWebSocket;
      ws.simulateMessage({
        error: { code: -1, msg: 'Invalid symbol' },
        id: 1
      });

      // Wait for message processing
      await new Promise(resolve => setTimeout(resolve, 10));

      expect(errorHandler).toHaveBeenCalledWith(1, { code: -1, msg: 'Invalid symbol' });
    });
  });

  describe('Reconnection with Subscriptions', () => {
    it('should restore subscriptions after reconnection', async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      // Subscribe to some streams
      await manager.subscribeToSymbol('BTCUSDT', ['depth', 'trade']);
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const subscriptionsBefore = manager.getSubscriptions();
      expect(subscriptionsBefore.size).toBe(1);
      
      // Simulate reconnection
      const sendSpy = jest.spyOn(manager, 'send');
      await manager.reconnectWithSubscriptions();
      
      await new Promise(resolve => setTimeout(resolve, 100));
      
      // Should have restored subscriptions
      const subscriptionsAfter = manager.getSubscriptions();
      expect(subscriptionsAfter.size).toBe(1);
      expect(subscriptionsAfter.has('btcusdt')).toBe(true);
    });
  });

  describe('Statistics and Info', () => {
    beforeEach(async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
    });

    it('should provide Binance-specific statistics', async () => {
      await manager.subscribeToSymbol('BTCUSDT', ['depth', 'trade']);
      
      const stats = manager.getBinanceStats();
      
      expect(stats.subscriptionCount).toBe(1);
      expect(stats.subscribedSymbols).toContain('btcusdt');
      expect(stats.isTestnet).toBe(false);
      expect(typeof stats.pendingSubscriptions).toBe('number');
    });

    it('should track pending subscriptions', async () => {
      const statsBefore = manager.getBinanceStats();
      const pendingBefore = statsBefore.pendingSubscriptions;
      
      // Start subscription (don't wait for completion)
      manager.subscribeToSymbol('BTCUSDT', ['depth']);
      
      const statsAfter = manager.getBinanceStats();
      expect(statsAfter.pendingSubscriptions).toBeGreaterThan(pendingBefore);
    });
  });

  describe('Error Handling', () => {
    beforeEach(async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
    });

    it('should handle malformed messages gracefully', () => {
      const errorHandler = jest.fn();
      manager.on('error', errorHandler);

      const ws = (manager as any).ws as MockWebSocket;
      
      // This should not throw an error
      expect(() => {
        ws.simulateMessage('invalid json');
      }).not.toThrow();
    });

    it('should handle unknown stream types', () => {
      const ws = (manager as any).ws as MockWebSocket;
      
      // This should not throw an error
      expect(() => {
        ws.simulateMessage({
          e: 'unknownEvent',
          data: 'some data'
        });
      }).not.toThrow();
    });

    it('should throw error when subscribing without connection', async () => {
      manager.disconnect();
      
      await expect(
        manager.subscribeToSymbol('BTCUSDT', ['depth'])
      ).rejects.toThrow('WebSocket not connected');
    });
  });

  describe('Stream Type Validation', () => {
    it('should throw error for invalid stream type', async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      await expect(
        manager.subscribeToSymbol('BTCUSDT', ['invalid' as any])
      ).rejects.toThrow('Unknown stream type: invalid');
    });

    it('should handle all valid stream types', async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const validStreams: BinanceStreamType[] = ['depth', 'trade', 'ticker', 'bookTicker'];
      
      for (const stream of validStreams) {
        await expect(
          manager.subscribeToSymbol('BTCUSDT', [stream])
        ).resolves.not.toThrow();
        
        await manager.unsubscribeFromSymbol('BTCUSDT');
      }
    });
  });

  describe('Manual Retry Functionality', () => {
    it('should support manual retry check for Binance connections', async () => {
      // Force manager into error state after max attempts
      (manager as any).reconnectAttempts = 3;
      (manager as any).setConnectionStatus('error');

      expect(manager.canManualRetryBinance()).toBe(true);
    });

    it('should perform manual retry with subscription restoration', async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      // Subscribe to some streams first
      await manager.subscribeToSymbol('BTCUSDT', ['depth', 'trade']);
      await new Promise(resolve => setTimeout(resolve, 50));
      
      // Force into error state
      (manager as any).reconnectAttempts = 3;
      (manager as any).setConnectionStatus('error');
      
      const subscriptionsBefore = manager.getSubscriptions();
      expect(subscriptionsBefore.size).toBe(1);
      
      // Perform manual retry with subscriptions
      await manager.manualRetryWithSubscriptions();
      await new Promise(resolve => setTimeout(resolve, 100));
      
      // Should have restored subscriptions
      const subscriptionsAfter = manager.getSubscriptions();
      expect(subscriptionsAfter.size).toBe(1);
      expect(subscriptionsAfter.has('btcusdt')).toBe(true);
    });

    it('should not perform manual retry when not in error state', async () => {
      const reconnectSpy = jest.spyOn(manager, 'reconnectWithSubscriptions');
      
      await manager.manualRetryWithSubscriptions();
      
      expect(reconnectSpy).not.toHaveBeenCalled();
    });
  });

  describe('Resource Cleanup', () => {
    it('should clean up Binance-specific resources on destroy', async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      await manager.subscribeToSymbol('BTCUSDT', ['depth', 'trade']);
      
      expect(manager.getSubscriptions().size).toBe(1);
      expect((manager as any).pendingSubscriptions.size).toBeGreaterThanOrEqual(0);
      
      manager.destroy();
      
      expect(manager.getSubscriptions().size).toBe(0);
      expect((manager as any).pendingSubscriptions.size).toBe(0);
    });
  });

  describe('Message Format Validation', () => {
    beforeEach(async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
    });

    it('should validate BinanceDepthStream format', (done) => {
      const validDepthData: BinanceDepthStream = {
        e: 'depthUpdate',
        E: 1234567890,
        s: 'BTCUSDT',
        U: 1,
        u: 2,
        b: [['50000.00', '1.5']],
        a: [['50100.00', '2.0']],
      };

      manager.on('depth-update', (data) => {
        expect(data.e).toBe('depthUpdate');
        expect(Array.isArray(data.b)).toBe(true);
        expect(Array.isArray(data.a)).toBe(true);
        done();
      });

      const ws = (manager as any).ws as MockWebSocket;
      ws.simulateMessage(validDepthData);
    });

    it('should validate BinanceTradeStream format', (done) => {
      const validTradeData: BinanceTradeStream = {
        e: 'trade',
        E: 1234567890,
        s: 'BTCUSDT',
        t: 12345,
        p: '50000.00',
        q: '0.1',
        b: 88,
        a: 50,
        T: 1234567890,
        m: true,
        M: true,
      };

      manager.on('trade', (data) => {
        expect(data.e).toBe('trade');
        expect(typeof data.p).toBe('string');
        expect(typeof data.q).toBe('string');
        expect(typeof data.m).toBe('boolean');
        done();
      });

      const ws = (manager as any).ws as MockWebSocket;
      ws.simulateMessage(validTradeData);
    });
  });
});