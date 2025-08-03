import { WebSocketManager, WebSocketManagerOptions } from './websocket-manager';
import {
  BinanceDepthStream,
  BinanceTradeStream,
  BinanceTickerStream,
  BinanceBookTickerStream,
  TradeData,
  OrderFlow,
  isBinanceDepthStream,
  isBinanceTradeStream,
} from '../types';

export interface BinanceStreamSubscription {
  symbol: string;
  streams: BinanceStreamType[];
}

export type BinanceStreamType = 'depth' | 'trade' | 'ticker' | 'bookTicker';

export interface BinanceWebSocketManagerEvents {
  'depth-update': (data: BinanceDepthStream) => void;
  'trade': (data: BinanceTradeStream) => void;
  'ticker': (data: BinanceTickerStream) => void;
  'book-ticker': (data: BinanceBookTickerStream) => void;
  'processed-trade': (trade: TradeData) => void;
  'processed-depth': (bids: [string, string][], asks: [string, string][]) => void;
  'subscription-confirmed': (id: number, result: unknown) => void;
  'subscription-error': (id: number, error: unknown) => void;
}

export interface BinanceWebSocketManagerOptions extends WebSocketManagerOptions {
  baseUrl?: string;
  testnet?: boolean;
}

export class BinanceWebSocketManager extends WebSocketManager {
  private static readonly BINANCE_WS_URL = 'wss://stream.binance.com:9443/ws';
  private static readonly BINANCE_TESTNET_WS_URL = 'wss://testnet.binance.vision/ws';
  
  private subscriptions = new Map<string, BinanceStreamSubscription>();
  private subscriptionId = 1;
  private pendingSubscriptions = new Map<number, { method: string; params: string[] }>();
  private isTestnet: boolean;

  constructor(options: BinanceWebSocketManagerOptions = {}) {
    const { baseUrl, testnet = false, ...wsOptions } = options;
    
    const url = baseUrl || (testnet ? 
      BinanceWebSocketManager.BINANCE_TESTNET_WS_URL : 
      BinanceWebSocketManager.BINANCE_WS_URL
    );
    
    super(url, wsOptions);
    this.isTestnet = testnet;
    
    // Set up message processing
    this.on('message', this.processMessage.bind(this));
  }

  /**
   * Subscribe to Binance streams for a symbol
   */
  public async subscribeToSymbol(
    symbol: string, 
    streams: BinanceStreamType[] = ['depth', 'trade']
  ): Promise<void> {
    const normalizedSymbol = symbol.toLowerCase();
    const streamNames = this.buildStreamNames(normalizedSymbol, streams);
    
    // Store subscription info
    this.subscriptions.set(normalizedSymbol, { symbol: normalizedSymbol, streams });
    
    // Subscribe to streams
    await this.subscribeToStreams(streamNames);
  }

  /**
   * Unsubscribe from streams for a symbol
   */
  public async unsubscribeFromSymbol(symbol: string): Promise<void> {
    const normalizedSymbol = symbol.toLowerCase();
    const subscription = this.subscriptions.get(normalizedSymbol);
    
    if (!subscription) {
      return;
    }
    
    const streamNames = this.buildStreamNames(normalizedSymbol, subscription.streams);
    await this.unsubscribeFromStreams(streamNames);
    
    this.subscriptions.delete(normalizedSymbol);
  }

  /**
   * Subscribe to multiple symbols at once
   */
  public async subscribeToMultipleSymbols(
    subscriptions: BinanceStreamSubscription[]
  ): Promise<void> {
    const allStreams: string[] = [];
    
    for (const subscription of subscriptions) {
      const normalizedSymbol = subscription.symbol.toLowerCase();
      const streamNames = this.buildStreamNames(normalizedSymbol, subscription.streams);
      allStreams.push(...streamNames);
      
      // Store subscription info
      this.subscriptions.set(normalizedSymbol, {
        symbol: normalizedSymbol,
        streams: subscription.streams,
      });
    }
    
    await this.subscribeToStreams(allStreams);
  }

  /**
   * Get current subscriptions
   */
  public getSubscriptions(): Map<string, BinanceStreamSubscription> {
    return new Map(this.subscriptions);
  }

  /**
   * Clear all subscriptions
   */
  public async clearAllSubscriptions(): Promise<void> {
    const allStreams: string[] = [];
    
    for (const subscription of this.subscriptions.values()) {
      const streamNames = this.buildStreamNames(subscription.symbol, subscription.streams);
      allStreams.push(...streamNames);
    }
    
    if (allStreams.length > 0) {
      await this.unsubscribeFromStreams(allStreams);
    }
    
    this.subscriptions.clear();
  }

  /**
   * Reconnect and restore subscriptions
   */
  public async reconnectWithSubscriptions(): Promise<void> {
    const currentSubscriptions = Array.from(this.subscriptions.values());
    
    // Reconnect
    this.reconnect();
    
    // Wait for connection
    await new Promise<void>((resolve) => {
      const checkConnection = () => {
        if (this.getConnectionStatus() === 'connected') {
          resolve();
        } else {
          setTimeout(checkConnection, 100);
        }
      };
      checkConnection();
    });
    
    // Restore subscriptions
    if (currentSubscriptions.length > 0) {
      await this.subscribeToMultipleSymbols(currentSubscriptions);
    }
  }

  private buildStreamNames(symbol: string, streams: BinanceStreamType[]): string[] {
    return streams.map(stream => {
      switch (stream) {
        case 'depth':
          return `${symbol}@depth`;
        case 'trade':
          return `${symbol}@trade`;
        case 'ticker':
          return `${symbol}@ticker`;
        case 'bookTicker':
          return `${symbol}@bookTicker`;
        default:
          throw new Error(`Unknown stream type: ${stream}`);
      }
    });
  }

  private async subscribeToStreams(streams: string[]): Promise<void> {
    if (this.getConnectionStatus() !== 'connected') {
      throw new Error('WebSocket not connected');
    }

    const id = this.subscriptionId++;
    const message = {
      method: 'SUBSCRIBE',
      params: streams,
      id,
    };

    this.pendingSubscriptions.set(id, { method: 'SUBSCRIBE', params: streams });
    this.send(message);
  }

  private async unsubscribeFromStreams(streams: string[]): Promise<void> {
    if (this.getConnectionStatus() !== 'connected') {
      return;
    }

    const id = this.subscriptionId++;
    const message = {
      method: 'UNSUBSCRIBE',
      params: streams,
      id,
    };

    this.pendingSubscriptions.set(id, { method: 'UNSUBSCRIBE', params: streams });
    this.send(message);
  }

  private processMessage(data: unknown): void {
    try {
      // Handle subscription responses
      if (this.isSubscriptionResponse(data)) {
        this.handleSubscriptionResponse(data);
        return;
      }

      // Handle stream data
      if (this.isStreamData(data)) {
        this.handleStreamData(data);
        return;
      }

      // Handle direct stream messages (when using single stream connection)
      this.handleDirectStreamMessage(data);
      
    } catch (error) {
      this.emit('error', new Error(`Failed to process message: ${error}`));
    }
  }

  private isSubscriptionResponse(data: unknown): boolean {
    return (
      typeof data === 'object' &&
      data !== null &&
      'id' in data &&
      typeof (data as any).id === 'number'
    );
  }

  private isStreamData(data: unknown): boolean {
    return (
      typeof data === 'object' &&
      data !== null &&
      'stream' in data &&
      'data' in data
    );
  }

  private handleSubscriptionResponse(data: any): void {
    const { id, result, error } = data;
    const pendingSubscription = this.pendingSubscriptions.get(id);
    
    if (!pendingSubscription) {
      return;
    }
    
    this.pendingSubscriptions.delete(id);
    
    if (error) {
      this.emit('subscription-error', id, error);
    } else {
      this.emit('subscription-confirmed', id, result);
    }
  }

  private handleStreamData(data: any): void {
    const { stream, data: streamData } = data;
    this.handleDirectStreamMessage(streamData);
  }

  private handleDirectStreamMessage(data: unknown): void {
    if (isBinanceDepthStream(data)) {
      this.handleDepthUpdate(data);
    } else if (isBinanceTradeStream(data)) {
      this.handleTradeUpdate(data);
    } else if (this.isBinanceTickerStream(data)) {
      this.handleTickerUpdate(data);
    } else if (this.isBinanceBookTickerStream(data)) {
      this.handleBookTickerUpdate(data);
    }
  }

  private handleDepthUpdate(data: BinanceDepthStream): void {
    this.emit('depth-update', data);
    
    // Process and emit formatted depth data
    this.emit('processed-depth', data.b, data.a);
  }

  private handleTradeUpdate(data: BinanceTradeStream): void {
    this.emit('trade', data);
    
    // Convert to internal TradeData format
    const trade: TradeData = {
      price: parseFloat(data.p),
      quantity: parseFloat(data.q),
      timestamp: data.T,
      isBuyerMaker: data.m,
      tradeId: data.t,
      symbol: data.s,
    };
    
    this.emit('processed-trade', trade);
  }

  private handleTickerUpdate(data: BinanceTickerStream): void {
    this.emit('ticker', data);
  }

  private handleBookTickerUpdate(data: BinanceBookTickerStream): void {
    this.emit('book-ticker', data);
  }

  private isBinanceTickerStream(data: unknown): data is BinanceTickerStream {
    return (
      typeof data === 'object' &&
      data !== null &&
      (data as BinanceTickerStream).e === '24hrTicker'
    );
  }

  private isBinanceBookTickerStream(data: unknown): data is BinanceBookTickerStream {
    return (
      typeof data === 'object' &&
      data !== null &&
      'u' in data &&
      's' in data &&
      'b' in data &&
      'B' in data &&
      'a' in data &&
      'A' in data
    );
  }

  /**
   * Get connection statistics specific to Binance
   */
  public getBinanceStats() {
    return {
      ...this.getConnectionInfo(),
      subscriptionCount: this.subscriptions.size,
      pendingSubscriptions: this.pendingSubscriptions.size,
      isTestnet: this.isTestnet,
      subscribedSymbols: Array.from(this.subscriptions.keys()),
    };
  }

  /**
   * Check if connection supports manual retry (Requirement 8.4)
   */
  public canManualRetryBinance(): boolean {
    return this.canManualRetry();
  }

  /**
   * Manual retry with subscription restoration (Requirement 8.4)
   */
  public async manualRetryWithSubscriptions(): Promise<void> {
    if (this.canManualRetry()) {
      await this.reconnectWithSubscriptions();
    }
  }

  /**
   * Override destroy to clean up Binance-specific resources
   */
  public destroy(): void {
    this.subscriptions.clear();
    this.pendingSubscriptions.clear();
    super.destroy();
  }
}
