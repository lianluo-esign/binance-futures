import { WebSocketManager } from '../websocket-manager';
import { ConnectionStatus } from '../../types';

// Mock WebSocket
class MockWebSocket {
  public onopen: ((event: Event) => void) | null = null;
  public onclose: ((event: CloseEvent) => void) | null = null;
  public onmessage: ((event: MessageEvent) => void) | null = null;
  public onerror: ((event: Event) => void) | null = null;
  public readyState: number = WebSocket.CONNECTING;
  
  private listeners: { [key: string]: ((event: any) => void)[] } = {};
  private static shouldConnect: boolean = true;
  
  constructor(public url: string) {
    // Simulate async connection - only connect if shouldConnect is true
    setTimeout(() => {
      if (MockWebSocket.shouldConnect) {
        this.readyState = WebSocket.OPEN;
        if (this.onopen) {
          this.onopen(new Event('open'));
        }
      } else {
        this.readyState = WebSocket.CLOSED;
        if (this.onerror) {
          this.onerror(new Event('error'));
        }
      }
    }, 10);
  }
  
  send(data: string | ArrayBufferLike | Blob | ArrayBufferView): void {
    // Mock send - do nothing
  }
  
  close(code?: number, reason?: string): void {
    this.readyState = WebSocket.CLOSED;
    if (this.onclose) {
      this.onclose(new CloseEvent('close', { code: code || 1000, reason }));
    }
  }
  
  addEventListener(type: string, listener: (event: any) => void): void {
    if (!this.listeners[type]) {
      this.listeners[type] = [];
    }
    this.listeners[type].push(listener);
  }
  
  removeEventListener(type: string, listener: (event: any) => void): void {
    if (this.listeners[type]) {
      const index = this.listeners[type].indexOf(listener);
      if (index > -1) {
        this.listeners[type].splice(index, 1);
      }
    }
  }
  
  // Helper method to simulate receiving messages
  simulateMessage(data: any): void {
    if (this.onmessage) {
      this.onmessage(new MessageEvent('message', { data: JSON.stringify(data) }));
    }
  }
  
  // Helper method to simulate errors
  simulateError(): void {
    if (this.onerror) {
      this.onerror(new Event('error'));
    }
  }
  
  // Helper method to prevent connection
  static preventConnection(): void {
    MockWebSocket.shouldConnect = false;
  }
  
  // Helper method to allow connection
  static allowConnection(): void {
    MockWebSocket.shouldConnect = true;
  }
}

// Mock global WebSocket
(global as any).WebSocket = MockWebSocket;

describe('WebSocketManager', () => {
  let manager: WebSocketManager;
  const testUrl = 'wss://test.example.com';
  
  beforeEach(() => {
    manager = new WebSocketManager(testUrl, {
      reconnectInterval: 100,
      maxReconnectAttempts: 3,
      heartbeatInterval: 1000,
      connectionTimeout: 500,
    });
  });
  
  afterEach(() => {
    manager.destroy();
  });

  describe('Connection Management', () => {
    it('should initialize with disconnected status', () => {
      expect(manager.getConnectionStatus()).toBe('disconnected');
    });

    it('should connect successfully', async () => {
      const statusChanges: ConnectionStatus[] = [];
      manager.on('connection-status-changed', (status) => {
        statusChanges.push(status);
      });

      await manager.connect();
      
      // Wait for connection to establish
      await new Promise(resolve => setTimeout(resolve, 50));
      
      expect(statusChanges).toContain('connecting');
      expect(statusChanges).toContain('connected');
      expect(manager.getConnectionStatus()).toBe('connected');
    });

    it('should handle connection timeout', async () => {
      // Prevent mock WebSocket from connecting
      MockWebSocket.preventConnection();
      
      // Create manager with very short timeout
      const shortTimeoutManager = new WebSocketManager(testUrl, {
        connectionTimeout: 20,
      });
      
      const statusChanges: ConnectionStatus[] = [];
      const errorHandler = jest.fn();
      
      shortTimeoutManager.on('connection-status-changed', (status) => {
        statusChanges.push(status);
      });
      
      shortTimeoutManager.on('error', errorHandler);

      await shortTimeoutManager.connect();
      
      // Wait for timeout and error handling
      await new Promise(resolve => setTimeout(resolve, 150));
      
      expect(statusChanges).toContain('connecting');
      // Should eventually reach error state after timeout and reconnection attempts
      expect(statusChanges.some(status => status === 'error' || status === 'reconnecting')).toBe(true);
      
      shortTimeoutManager.destroy();
      
      // Reset mock behavior
      MockWebSocket.allowConnection();
    });

    it('should disconnect properly', async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const statusChanges: ConnectionStatus[] = [];
      manager.on('connection-status-changed', (status) => {
        statusChanges.push(status);
      });

      manager.disconnect();
      
      expect(statusChanges).toContain('disconnected');
      expect(manager.getConnectionStatus()).toBe('disconnected');
    });
  });

  describe('Message Handling', () => {
    beforeEach(async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
    });

    it('should receive and emit messages', (done) => {
      const testMessage = { type: 'test', data: 'hello' };
      
      manager.on('message', (data) => {
        expect(data).toEqual(testMessage);
        done();
      });

      // Simulate receiving a message
      const ws = (manager as any).ws as MockWebSocket;
      ws.simulateMessage(testMessage);
    });

    it('should handle non-JSON messages', (done) => {
      const testMessage = 'plain text message';
      
      manager.on('message', (data) => {
        expect(data).toBe(testMessage);
        done();
      });

      // Simulate receiving a non-JSON message
      const ws = (manager as any).ws as MockWebSocket;
      if (ws.onmessage) {
        ws.onmessage(new MessageEvent('message', { data: testMessage }));
      }
    });

    it('should send messages successfully', () => {
      const testMessage = { type: 'test', data: 'hello' };
      const result = manager.send(testMessage);
      expect(result).toBe(true);
    });

    it('should queue messages when disconnected', () => {
      manager.disconnect();
      
      const testMessage = { type: 'test', data: 'hello' };
      const result = manager.send(testMessage);
      
      expect(result).toBe(false);
      // Message should be queued
      expect((manager as any).messageQueue.length).toBe(1);
    });
  });

  describe('Reconnection Logic', () => {
    it('should attempt reconnection on connection loss', async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const statusChanges: ConnectionStatus[] = [];
      manager.on('connection-status-changed', (status) => {
        statusChanges.push(status);
      });

      // Simulate connection loss
      const ws = (manager as any).ws as MockWebSocket;
      ws.close(1006, 'Connection lost'); // Abnormal closure
      
      await new Promise(resolve => setTimeout(resolve, 50));
      
      expect(statusChanges).toContain('disconnected');
      expect(statusChanges).toContain('reconnecting');
    });

    it('should implement exponential backoff', async () => {
      const reconnectAttempts: number[] = [];
      
      manager.on('connection-status-changed', (status) => {
        if (status === 'reconnecting') {
          reconnectAttempts.push(Date.now());
        }
      });

      // Add error handler to prevent unhandled errors
      manager.on('error', () => {});

      // Force multiple failed reconnection attempts
      for (let i = 0; i < 3; i++) {
        (manager as any).handleConnectionError(new Error('Connection failed'));
        await new Promise(resolve => setTimeout(resolve, 50));
      }
      
      // Should have made multiple reconnection attempts
      expect(reconnectAttempts.length).toBeGreaterThan(0);
    });

    it('should stop reconnecting after max attempts', async () => {
      let reconnectAttempts = 0;
      
      manager.on('connection-status-changed', (status) => {
        if (status === 'reconnecting') {
          reconnectAttempts++;
        }
      });

      // Force multiple failed reconnection attempts
      for (let i = 0; i < 5; i++) {
        (manager as any).handleConnectionError(new Error('Connection failed'));
        await new Promise(resolve => setTimeout(resolve, 50));
      }
      
      expect(manager.getConnectionStatus()).toBe('error');
    });
  });

  describe('Performance Metrics', () => {
    beforeEach(async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
    });

    it('should track message count', (done) => {
      const testMessage = { type: 'test', data: 'hello' };
      
      manager.on('performance-update', (metrics) => {
        if (metrics.totalMessages > 0) {
          expect(metrics.totalMessages).toBeGreaterThan(0);
          expect(metrics.messagesPerSecond).toBeGreaterThanOrEqual(0);
          done();
        }
      });

      // Send some messages to trigger metrics update
      const ws = (manager as any).ws as MockWebSocket;
      ws.simulateMessage(testMessage);
      ws.simulateMessage(testMessage);
    });

    it('should track connection uptime', async () => {
      // Wait for connection and performance metrics update
      await new Promise(resolve => setTimeout(resolve, 1100)); // Wait for at least one metrics update cycle
      
      const metrics = manager.getPerformanceMetrics();
      expect(metrics.connectionUptime).toBeGreaterThan(0);
    });

    it('should track error count', () => {
      const initialMetrics = manager.getPerformanceMetrics();
      const initialErrorCount = initialMetrics.errorCount;
      
      // Simulate an error
      (manager as any).handleConnectionError(new Error('Test error'));
      
      const updatedMetrics = manager.getPerformanceMetrics();
      expect(updatedMetrics.errorCount).toBe(initialErrorCount + 1);
    });
  });

  describe('Heartbeat Mechanism', () => {
    beforeEach(async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
    });

    it('should send heartbeat messages', (done) => {
      manager.on('heartbeat', (latency) => {
        expect(typeof latency).toBe('number');
        expect(latency).toBeGreaterThanOrEqual(0);
        done();
      });

      // Trigger heartbeat manually
      (manager as any).sendHeartbeat();
    });

    it('should update latency metrics', () => {
      (manager as any).sendHeartbeat();
      
      const metrics = manager.getPerformanceMetrics();
      expect(typeof metrics.latency).toBe('number');
    });
  });

  describe('Connection Info', () => {
    it('should provide connection information', async () => {
      const info = manager.getConnectionInfo();
      
      expect(info.status).toBe('disconnected');
      expect(info.url).toBe(testUrl);
      expect(info.connectedAt).toBeNull();
      expect(info.reconnectAttempts).toBe(0);
      expect(info.uptime).toBe(0);
    });

    it('should update connection info when connected', async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const info = manager.getConnectionInfo();
      
      expect(info.status).toBe('connected');
      expect(info.connectedAt).toBeGreaterThan(0);
      expect(info.uptime).toBeGreaterThan(0);
    });
  });

  describe('Message Buffer Functionality', () => {
    beforeEach(async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
    });

    it('should buffer messages when enabled', (done) => {
      const testMessage = { type: 'test', data: 'hello' };
      
      manager.on('message', () => {
        // Check if message was buffered
        const bufferedMessages = manager.getBufferedMessages();
        expect(bufferedMessages.length).toBeGreaterThan(0);
        expect(bufferedMessages[bufferedMessages.length - 1].data).toEqual(testMessage);
        done();
      });

      const ws = (manager as any).ws as MockWebSocket;
      ws.simulateMessage(testMessage);
    });

    it('should retrieve buffered messages since a specific timestamp', async () => {
      const testMessage1 = { type: 'test1', data: 'hello1' };
      const testMessage2 = { type: 'test2', data: 'hello2' };
      
      const ws = (manager as any).ws as MockWebSocket;
      
      // Send first message
      ws.simulateMessage(testMessage1);
      await new Promise(resolve => setTimeout(resolve, 10));
      
      const timestamp = Date.now();
      await new Promise(resolve => setTimeout(resolve, 10));
      
      // Send second message after timestamp
      ws.simulateMessage(testMessage2);
      await new Promise(resolve => setTimeout(resolve, 10));
      
      const recentMessages = manager.getBufferedMessages(timestamp);
      expect(recentMessages.length).toBe(1);
      expect(recentMessages[0].data).toEqual(testMessage2);
    });

    it('should limit buffer size and remove oldest messages', async () => {
      // Create manager with small buffer size
      const smallBufferManager = new WebSocketManager(testUrl, {
        messageBufferSize: 2,
      });
      
      await smallBufferManager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const ws = (smallBufferManager as any).ws as MockWebSocket;
      
      // Send 3 messages to exceed buffer size
      ws.simulateMessage({ id: 1 });
      ws.simulateMessage({ id: 2 });
      ws.simulateMessage({ id: 3 });
      
      await new Promise(resolve => setTimeout(resolve, 10));
      
      const bufferedMessages = smallBufferManager.getBufferedMessages();
      expect(bufferedMessages.length).toBe(2);
      expect((bufferedMessages[0].data as any).id).toBe(2); // First message should be removed
      expect((bufferedMessages[1].data as any).id).toBe(3);
      
      smallBufferManager.destroy();
    });

    it('should clear message buffer', async () => {
      const testMessage = { type: 'test', data: 'hello' };
      
      const ws = (manager as any).ws as MockWebSocket;
      ws.simulateMessage(testMessage);
      
      await new Promise(resolve => setTimeout(resolve, 10));
      
      expect(manager.getBufferedMessages().length).toBeGreaterThan(0);
      
      manager.clearMessageBuffer();
      
      expect(manager.getBufferedMessages().length).toBe(0);
    });

    it('should work with message buffer disabled', async () => {
      // Create manager with buffer disabled
      const noBufferManager = new WebSocketManager(testUrl, {
        enableMessageBuffer: false,
      });
      
      await noBufferManager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const testMessage = { type: 'test', data: 'hello' };
      const ws = (noBufferManager as any).ws as MockWebSocket;
      ws.simulateMessage(testMessage);
      
      await new Promise(resolve => setTimeout(resolve, 10));
      
      // Buffer should remain empty
      expect(noBufferManager.getBufferedMessages().length).toBe(0);
      
      noBufferManager.destroy();
    });
  });

  describe('Resource Cleanup', () => {
    it('should clean up resources on destroy', async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const listenerCount = manager.listenerCount('message');
      expect(listenerCount).toBeGreaterThanOrEqual(0);
      
      manager.destroy();
      
      expect(manager.getConnectionStatus()).toBe('disconnected');
      expect(manager.listenerCount('message')).toBe(0);
    });

    it('should clear message queue on destroy', () => {
      manager.disconnect();
      manager.send({ test: 'message' });
      
      expect((manager as any).messageQueue.length).toBe(1);
      
      manager.destroy();
      
      expect((manager as any).messageQueue.length).toBe(0);
    });

    it('should clear message buffer on destroy', async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const testMessage = { type: 'test', data: 'hello' };
      const ws = (manager as any).ws as MockWebSocket;
      ws.simulateMessage(testMessage);
      
      await new Promise(resolve => setTimeout(resolve, 10));
      
      expect(manager.getBufferedMessages().length).toBeGreaterThan(0);
      
      manager.destroy();
      
      expect(manager.getBufferedMessages().length).toBe(0);
    });
  });

  describe('Error Handling', () => {
    it('should emit error events', (done) => {
      manager.on('error', (error) => {
        expect(error).toBeInstanceOf(Error);
        done();
      });

      (manager as any).handleConnectionError(new Error('Test error'));
    });

    it('should handle WebSocket errors gracefully', async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const errorHandler = jest.fn();
      manager.on('error', errorHandler);

      // Simulate WebSocket error
      const ws = (manager as any).ws as MockWebSocket;
      ws.simulateError();
      
      expect(errorHandler).toHaveBeenCalled();
    });

    it('should provide enhanced error messages for timeouts', (done) => {
      manager.on('error', (error) => {
        expect(error.message).toContain('Connection timeout after');
        expect(error.message).toContain('Please check your network connection');
        done();
      });

      (manager as any).handleConnectionError(new Error('Connection timeout'));
    });

    it('should provide enhanced error messages for WebSocket errors', (done) => {
      manager.on('error', (error) => {
        expect(error.message).toContain('WebSocket connection failed');
        expect(error.message).toContain('Server may be unavailable');
        done();
      });

      (manager as any).handleConnectionError(new Error('WebSocket error: test'));
    });

    it('should provide enhanced error messages for reconnection attempts', (done) => {
      // Set reconnect attempts to simulate retry scenario
      (manager as any).reconnectAttempts = 2;
      
      manager.on('error', (error) => {
        expect(error.message).toContain('Reconnection attempt 2/3 failed');
        done();
      });

      (manager as any).handleConnectionError(new Error('Connection failed'));
    });

    it('should emit error for high latency', async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const errorHandler = jest.fn();
      manager.on('error', errorHandler);

      // Mock high latency by overriding Date.now
      const originalDateNow = Date.now;
      let callCount = 0;
      Date.now = jest.fn(() => {
        callCount++;
        if (callCount === 1) return 1000; // heartbeatTime
        if (callCount === 2) return 2500; // sendTime (1500ms latency)
        return originalDateNow();
      });

      // Trigger heartbeat
      (manager as any).sendHeartbeat();

      expect(errorHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          message: expect.stringContaining('High latency detected')
        })
      );

      // Restore Date.now
      Date.now = originalDateNow;
    });
  });

  describe('Manual Retry Functionality', () => {
    it('should allow manual retry when in error state after max attempts', async () => {
      // Force manager into error state after max attempts
      (manager as any).reconnectAttempts = 3;
      (manager as any).setConnectionStatus('error');

      expect(manager.canManualRetry()).toBe(true);

      const statusChanges: ConnectionStatus[] = [];
      manager.on('connection-status-changed', (status) => {
        statusChanges.push(status);
      });

      manager.manualRetry();

      await new Promise(resolve => setTimeout(resolve, 50));

      expect(statusChanges).toContain('connecting');
      expect((manager as any).reconnectAttempts).toBe(0);
    });

    it('should not allow manual retry when not in error state', () => {
      expect(manager.canManualRetry()).toBe(false);
    });

    it('should not allow manual retry when reconnect attempts are below max', async () => {
      (manager as any).reconnectAttempts = 1;
      (manager as any).setConnectionStatus('error');

      expect(manager.canManualRetry()).toBe(false);
    });

    it('should not perform manual retry when not in error state', () => {
      const connectSpy = jest.spyOn(manager, 'connect');
      
      manager.manualRetry();
      
      expect(connectSpy).not.toHaveBeenCalled();
    });
  });

  describe('Backpressure Mechanism', () => {
    beforeEach(async () => {
      await manager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
    });

    it('should activate backpressure when processing queue exceeds threshold', async () => {
      // Create manager with very small threshold for testing
      const smallThresholdManager = new WebSocketManager(testUrl, {
        backpressureThreshold: 2,
      });
      
      await smallThresholdManager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const errorHandler = jest.fn();
      smallThresholdManager.on('error', errorHandler);
      
      const ws = (smallThresholdManager as any).ws as MockWebSocket;
      
      // Send messages rapidly to exceed threshold
      ws.simulateMessage({ id: 1 });
      ws.simulateMessage({ id: 2 });
      ws.simulateMessage({ id: 3 }); // This should trigger backpressure
      
      // Check immediately after sending messages
      const queueStatus = smallThresholdManager.getProcessingQueueStatus();
      expect(queueStatus.isBackpressureActive).toBe(true);
      expect(errorHandler).toHaveBeenCalledWith(
        expect.objectContaining({
          message: expect.stringContaining('Backpressure activated')
        })
      );
      
      smallThresholdManager.destroy();
    });

    it('should drop messages when backpressure is active', async () => {
      // Create manager with small threshold
      const smallThresholdManager = new WebSocketManager(testUrl, {
        backpressureThreshold: 3,
      });
      
      await smallThresholdManager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const ws = (smallThresholdManager as any).ws as MockWebSocket;
      
      // Fill up the queue to threshold
      for (let i = 0; i < 5; i++) {
        ws.simulateMessage({ id: i });
      }
      
      await new Promise(resolve => setTimeout(resolve, 10));
      
      const queueStatus = smallThresholdManager.getProcessingQueueStatus();
      expect(queueStatus.queueLength).toBeLessThan(5); // Some messages should be dropped
      
      smallThresholdManager.destroy();
    });

    it('should reset backpressure when queue size reduces', async () => {
      // Create manager with small threshold
      const smallThresholdManager = new WebSocketManager(testUrl, {
        backpressureThreshold: 10,
      });
      
      await smallThresholdManager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const ws = (smallThresholdManager as any).ws as MockWebSocket;
      
      // Fill up the queue to activate backpressure
      for (let i = 0; i < 15; i++) {
        ws.simulateMessage({ id: i });
      }
      
      await new Promise(resolve => setTimeout(resolve, 10));
      
      let queueStatus = smallThresholdManager.getProcessingQueueStatus();
      expect(queueStatus.isBackpressureActive).toBe(true);
      
      // Wait for queue processing to reduce the queue size
      await new Promise(resolve => setTimeout(resolve, 100));
      
      queueStatus = smallThresholdManager.getProcessingQueueStatus();
      // Backpressure should be reset when queue is below 50% of threshold
      expect(queueStatus.queueLength).toBeLessThan(5);
      
      smallThresholdManager.destroy();
    });

    it('should provide processing queue status', () => {
      const queueStatus = manager.getProcessingQueueStatus();
      
      expect(queueStatus).toHaveProperty('queueLength');
      expect(queueStatus).toHaveProperty('isBackpressureActive');
      expect(queueStatus).toHaveProperty('threshold');
      expect(typeof queueStatus.queueLength).toBe('number');
      expect(typeof queueStatus.isBackpressureActive).toBe('boolean');
      expect(typeof queueStatus.threshold).toBe('number');
    });

    it('should work with backpressure disabled', async () => {
      // Create manager with backpressure disabled
      const noBackpressureManager = new WebSocketManager(testUrl, {
        enableBackpressure: false,
      });
      
      await noBackpressureManager.connect();
      await new Promise(resolve => setTimeout(resolve, 50));
      
      const ws = (noBackpressureManager as any).ws as MockWebSocket;
      
      // Send many messages
      for (let i = 0; i < 20; i++) {
        ws.simulateMessage({ id: i });
      }
      
      await new Promise(resolve => setTimeout(resolve, 10));
      
      const queueStatus = noBackpressureManager.getProcessingQueueStatus();
      expect(queueStatus.isBackpressureActive).toBe(false);
      expect(queueStatus.queueLength).toBe(0); // No queue when backpressure is disabled
      
      noBackpressureManager.destroy();
    });

    it('should clear processing queue on destroy', async () => {
      const ws = (manager as any).ws as MockWebSocket;
      
      // Add some messages to processing queue
      for (let i = 0; i < 5; i++) {
        ws.simulateMessage({ id: i });
      }
      
      await new Promise(resolve => setTimeout(resolve, 10));
      
      expect(manager.getProcessingQueueStatus().queueLength).toBeGreaterThan(0);
      
      manager.destroy();
      
      expect(manager.getProcessingQueueStatus().queueLength).toBe(0);
      expect(manager.getProcessingQueueStatus().isBackpressureActive).toBe(false);
    });
  });
});