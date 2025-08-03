import { EventEmitter } from 'events';
import { ConnectionStatus, PerformanceMetrics, WebSocketConnectionConfig } from '../types';

export interface WebSocketManagerEvents {
  'connection-status-changed': (status: ConnectionStatus) => void;
  'message': (data: unknown) => void;
  'error': (error: Error) => void;
  'performance-update': (metrics: PerformanceMetrics) => void;
  'heartbeat': (latency: number) => void;
}

export interface WebSocketManagerOptions {
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  heartbeatInterval?: number;
  messageQueueSize?: number;
  connectionTimeout?: number;
  enableMessageBuffer?: boolean;
  messageBufferSize?: number;
  enableBackpressure?: boolean;
  backpressureThreshold?: number;
}

export class WebSocketManager extends EventEmitter {
  private ws: WebSocket | null = null;
  private url: string;
  private connectionStatus: ConnectionStatus = 'disconnected';
  private reconnectAttempts = 0;
  private reconnectTimer: NodeJS.Timeout | null = null;
  private heartbeatTimer: NodeJS.Timeout | null = null;
  private connectionTimer: NodeJS.Timeout | null = null;
  private lastHeartbeatTime = 0;
  private connectedAt: number | null = null;
  private messageQueue: unknown[] = [];
  private messageBuffer: { data: unknown; timestamp: number }[] = [];
  private processingQueue: unknown[] = [];
  private isBackpressureActive = false;
  
  // Configuration
  private readonly options: Required<WebSocketManagerOptions>;
  
  // Performance metrics
  private performanceMetrics: PerformanceMetrics = {
    messagesPerSecond: 0,
    latency: 0,
    memoryUsage: 0,
    connectionUptime: 0,
    totalMessages: 0,
    errorCount: 0,
    lastMessageTime: 0,
  };
  
  private messageCount = 0;
  private lastMetricsUpdate = Date.now();

  constructor(url: string, options: WebSocketManagerOptions = {}) {
    super();
    this.url = url;
    this.options = {
      reconnectInterval: options.reconnectInterval ?? 1000,
      maxReconnectAttempts: options.maxReconnectAttempts ?? 5,
      heartbeatInterval: options.heartbeatInterval ?? 30000,
      messageQueueSize: options.messageQueueSize ?? 1000,
      connectionTimeout: options.connectionTimeout ?? 10000,
      enableMessageBuffer: options.enableMessageBuffer ?? true,
      messageBufferSize: options.messageBufferSize ?? 5000,
      enableBackpressure: options.enableBackpressure ?? true,
      backpressureThreshold: options.backpressureThreshold ?? 1000,
    };
    
    // Start performance metrics tracking
    this.startPerformanceTracking();
  }

  /**
   * Connect to the WebSocket server
   */
  public async connect(): Promise<void> {
    if (this.connectionStatus === 'connecting' || this.connectionStatus === 'connected') {
      return;
    }

    this.setConnectionStatus('connecting');
    this.clearTimers();

    try {
      this.ws = new WebSocket(this.url);
      this.setupWebSocketEventHandlers();
      
      // Set connection timeout
      this.connectionTimer = setTimeout(() => {
        if (this.connectionStatus === 'connecting') {
          this.handleConnectionError(new Error('Connection timeout'));
        }
      }, this.options.connectionTimeout);
      
    } catch (error) {
      this.handleConnectionError(error as Error);
    }
  }

  /**
   * Disconnect from the WebSocket server
   */
  public disconnect(): void {
    this.clearTimers();
    this.reconnectAttempts = 0;
    
    if (this.ws) {
      this.ws.close(1000, 'Manual disconnect');
      this.ws = null;
    }
    
    this.setConnectionStatus('disconnected');
    this.connectedAt = null;
  }

  /**
   * Send a message through the WebSocket connection
   */
  public send(data: unknown): boolean {
    if (this.connectionStatus !== 'connected' || !this.ws) {
      // Queue message if not connected
      if (this.messageQueue.length < this.options.messageQueueSize) {
        this.messageQueue.push(data);
      }
      return false;
    }

    try {
      const message = typeof data === 'string' ? data : JSON.stringify(data);
      this.ws.send(message);
      return true;
    } catch (error) {
      this.emit('error', error as Error);
      return false;
    }
  }

  /**
   * Get current connection status
   */
  public getConnectionStatus(): ConnectionStatus {
    return this.connectionStatus;
  }

  /**
   * Get current performance metrics
   */
  public getPerformanceMetrics(): PerformanceMetrics {
    return { ...this.performanceMetrics };
  }

  /**
   * Get connection info
   */
  public getConnectionInfo() {
    return {
      status: this.connectionStatus,
      url: this.url,
      connectedAt: this.connectedAt,
      reconnectAttempts: this.reconnectAttempts,
      maxReconnectAttempts: this.options.maxReconnectAttempts,
      uptime: this.connectedAt ? Date.now() - this.connectedAt : 0,
    };
  }

  /**
   * Force reconnection
   */
  public reconnect(): void {
    this.disconnect();
    this.reconnectAttempts = 0;
    this.connect();
  }

  /**
   * Manual retry after max reconnection attempts reached (Requirement 8.4)
   */
  public manualRetry(): void {
    if (this.connectionStatus === 'error') {
      this.reconnectAttempts = 0;
      this.connect();
    }
  }

  /**
   * Check if manual retry is available
   */
  public canManualRetry(): boolean {
    return this.connectionStatus === 'error' && this.reconnectAttempts >= this.options.maxReconnectAttempts;
  }

  /**
   * Cleanup resources
   */
  public destroy(): void {
    this.disconnect();
    this.removeAllListeners();
    this.messageQueue = [];
    this.messageBuffer = [];
    this.processingQueue = [];
    this.isBackpressureActive = false;
  }

  private setupWebSocketEventHandlers(): void {
    if (!this.ws) return;

    this.ws.onopen = () => {
      this.handleConnectionOpen();
    };

    this.ws.onmessage = (event) => {
      this.handleMessage(event);
    };

    this.ws.onclose = (event) => {
      this.handleConnectionClose(event);
    };

    this.ws.onerror = (event) => {
      this.handleConnectionError(new Error(`WebSocket error: ${event}`));
    };
  }

  private handleConnectionOpen(): void {
    if (this.connectionTimer) {
      clearTimeout(this.connectionTimer);
      this.connectionTimer = null;
    }

    this.connectedAt = Date.now();
    this.reconnectAttempts = 0;
    this.setConnectionStatus('connected');
    
    // Send queued messages
    this.flushMessageQueue();
    
    // Start heartbeat
    this.startHeartbeat();
  }

  private handleMessage(event: MessageEvent): void {
    try {
      const data = JSON.parse(event.data);
      this.updateMessageMetrics();
      
      // Add to processing queue if backpressure is enabled
      if (this.options.enableBackpressure) {
        this.processingQueue.push(data);
        
        // Check for backpressure (Requirement 2.4)
        if (this.processingQueue.length >= this.options.backpressureThreshold) {
          this.handleBackpressure();
          return; // Drop message to prevent queue overflow
        }
      }
      
      // Buffer message if enabled (Requirement 8.3 - data processing without data loss)
      if (this.options.enableMessageBuffer) {
        this.bufferMessage(data);
      }
      
      this.emit('message', data);
      
      // Start processing queue if backpressure is enabled and not already processing
      if (this.options.enableBackpressure && this.processingQueue.length === 1) {
        setTimeout(() => this.processMessageQueue(), 10); // Small delay to allow queue to build up
      }
    } catch (error) {
      // Handle non-JSON messages
      this.updateMessageMetrics();
      
      if (this.options.enableMessageBuffer) {
        this.bufferMessage(event.data);
      }
      
      this.emit('message', event.data);
    }
  }

  private handleConnectionClose(event: CloseEvent): void {
    this.clearTimers();
    this.ws = null;
    this.connectedAt = null;

    if (event.code === 1000) {
      // Normal closure
      this.setConnectionStatus('disconnected');
    } else {
      // Abnormal closure - attempt reconnection
      this.setConnectionStatus('disconnected');
      this.attemptReconnection();
    }
  }

  private handleConnectionError(error: Error): void {
    this.performanceMetrics.errorCount++;
    this.clearTimers();
    this.ws = null;
    this.connectedAt = null;
    
    // Enhance error message with specific context (Requirement 12.1)
    let enhancedError = error;
    if (error.message.includes('timeout')) {
      enhancedError = new Error(`Connection timeout after ${this.options.connectionTimeout}ms. Please check your network connection.`);
    } else if (error.message.includes('WebSocket error')) {
      enhancedError = new Error(`WebSocket connection failed. Server may be unavailable or network issues detected.`);
    } else if (this.reconnectAttempts > 0) {
      enhancedError = new Error(`Reconnection attempt ${this.reconnectAttempts}/${this.options.maxReconnectAttempts} failed: ${error.message}`);
    }
    
    // Only emit error if there are listeners to prevent unhandled error
    if (this.listenerCount('error') > 0) {
      this.emit('error', enhancedError);
    }
    
    this.setConnectionStatus('error');
    this.attemptReconnection();
  }

  private attemptReconnection(): void {
    if (this.reconnectAttempts >= this.options.maxReconnectAttempts) {
      this.setConnectionStatus('error');
      return;
    }

    this.setConnectionStatus('reconnecting');
    this.reconnectAttempts++;
    
    // Exponential backoff
    const delay = Math.min(
      this.options.reconnectInterval * Math.pow(2, this.reconnectAttempts - 1),
      30000 // Max 30 seconds
    );

    this.reconnectTimer = setTimeout(() => {
      this.connect();
    }, delay);
  }

  private startHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
    }

    this.heartbeatTimer = setInterval(() => {
      this.sendHeartbeat();
    }, this.options.heartbeatInterval);
  }

  private sendHeartbeat(): void {
    if (this.connectionStatus !== 'connected') {
      return;
    }

    const heartbeatTime = Date.now();
    this.lastHeartbeatTime = heartbeatTime;
    
    // Send ping message
    const pingMessage = {
      type: 'ping',
      timestamp: heartbeatTime,
    };
    
    if (this.send(pingMessage)) {
      // For real latency measurement, we would wait for pong response
      // For now, we'll measure the time it takes to send the message
      const sendTime = Date.now();
      const latency = sendTime - heartbeatTime;
      
      this.performanceMetrics.latency = latency;
      this.emit('heartbeat', latency);
      
      // Check for connection quality warning (Requirement 8.5)
      if (latency > 1000) {
        this.emit('error', new Error(`High latency detected: ${latency}ms. Connection quality may be poor.`));
      }
    }
  }

  private flushMessageQueue(): void {
    while (this.messageQueue.length > 0 && this.connectionStatus === 'connected') {
      const message = this.messageQueue.shift();
      if (message) {
        this.send(message);
      }
    }
  }

  private bufferMessage(data: unknown): void {
    if (this.messageBuffer.length >= this.options.messageBufferSize) {
      // Remove oldest message to make room
      this.messageBuffer.shift();
    }
    
    this.messageBuffer.push({
      data,
      timestamp: Date.now(),
    });
  }

  /**
   * Get buffered messages (useful for data recovery after reconnection)
   */
  public getBufferedMessages(since?: number): { data: unknown; timestamp: number }[] {
    if (since) {
      return this.messageBuffer.filter(msg => msg.timestamp >= since);
    }
    return [...this.messageBuffer];
  }

  /**
   * Clear message buffer
   */
  public clearMessageBuffer(): void {
    this.messageBuffer = [];
  }

  private handleBackpressure(): void {
    if (!this.isBackpressureActive) {
      this.isBackpressureActive = true;
      this.emit('error', new Error(`Backpressure activated: Processing queue exceeded ${this.options.backpressureThreshold} messages. Dropping messages to prevent overflow.`));
    }
    
    // Drop oldest messages from processing queue to make room
    const dropCount = Math.floor(this.options.backpressureThreshold * 0.1); // Drop 10% of threshold
    this.processingQueue.splice(0, dropCount);
  }

  private processMessageQueue(): void {
    // Process messages from queue in smaller batches to simulate realistic processing delay
    const batchSize = 1; // Process one message at a time to simulate realistic load
    let processed = 0;
    
    while (this.processingQueue.length > 0 && processed < batchSize) {
      this.processingQueue.shift(); // Remove processed message
      processed++;
    }
    
    // Reset backpressure flag if queue is manageable
    if (this.isBackpressureActive && this.processingQueue.length < this.options.backpressureThreshold * 0.5) {
      this.isBackpressureActive = false;
    }
    
    // Continue processing if there are more messages (with delay to simulate realistic processing)
    if (this.processingQueue.length > 0) {
      setTimeout(() => this.processMessageQueue(), 1);
    }
  }

  /**
   * Get current processing queue status
   */
  public getProcessingQueueStatus() {
    return {
      queueLength: this.processingQueue.length,
      isBackpressureActive: this.isBackpressureActive,
      threshold: this.options.backpressureThreshold,
    };
  }

  private updateMessageMetrics(): void {
    this.messageCount++;
    this.performanceMetrics.totalMessages++;
    this.performanceMetrics.lastMessageTime = Date.now();
  }

  private startPerformanceTracking(): void {
    setInterval(() => {
      this.updatePerformanceMetrics();
    }, 1000); // Update every second
  }

  private updatePerformanceMetrics(): void {
    const now = Date.now();
    const timeDiff = now - this.lastMetricsUpdate;
    
    // Calculate messages per second
    this.performanceMetrics.messagesPerSecond = 
      Math.round((this.messageCount * 1000) / timeDiff);
    
    // Update connection uptime
    if (this.connectedAt) {
      this.performanceMetrics.connectionUptime = now - this.connectedAt;
    }
    
    // Estimate memory usage (simplified)
    this.performanceMetrics.memoryUsage = 
      this.messageQueue.length * 100 + // Rough estimate
      this.performanceMetrics.totalMessages * 0.1;
    
    // Reset counters
    this.messageCount = 0;
    this.lastMetricsUpdate = now;
    
    this.emit('performance-update', this.performanceMetrics);
  }

  private setConnectionStatus(status: ConnectionStatus): void {
    if (this.connectionStatus !== status) {
      this.connectionStatus = status;
      this.emit('connection-status-changed', status);
    }
  }

  private clearTimers(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
    
    if (this.connectionTimer) {
      clearTimeout(this.connectionTimer);
      this.connectionTimer = null;
    }
  }
}
