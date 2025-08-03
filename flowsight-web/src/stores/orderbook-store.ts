import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';
import {
  OrderFlow,
  MarketSnapshot,
  TradeData,
  ConnectionStatus,
  PerformanceMetrics,
  OrderBookState,
} from '../types';

// Default values
const DEFAULT_SYMBOL = 'BTCUSDT';
const DEFAULT_PRICE_STEP = 0.5;
const DEFAULT_MAX_LEVELS = 40;
const DEFAULT_MAX_TRADE_HISTORY = 1000;
const DATA_CLEANUP_INTERVAL = 5 * 60 * 1000; // 5 minutes

const createDefaultPerformanceMetrics = (): PerformanceMetrics => ({
  messagesPerSecond: 0,
  latency: 0,
  memoryUsage: 0,
  connectionUptime: 0,
  totalMessages: 0,
  errorCount: 0,
  lastMessageTime: 0,
});

const createDefaultMarketSnapshot = (symbol: string): MarketSnapshot => ({
  symbol,
  bestBid: null,
  bestAsk: null,
  currentPrice: null,
  spread: 0,
  realizedVolatility: 0,
  jumpSignal: 0,
  orderBookImbalance: 0,
  volumeWeightedMomentum: 0,
  timestamp: Date.now(),
});

export const useOrderBookStore = create<OrderBookState>()(
  subscribeWithSelector((set, get) => {
    // Cleanup interval for automatic data management
    let cleanupInterval: NodeJS.Timeout | null = null;

    const startCleanup = () => {
      if (cleanupInterval) clearInterval(cleanupInterval);
      cleanupInterval = setInterval(() => {
        const state = get();
        const now = Date.now();
        const cutoffTime = now - DATA_CLEANUP_INTERVAL;

        // Clean up old trade history
        const filteredTrades = state.tradeHistory.filter(
          (trade) => trade.timestamp > cutoffTime
        );

        // Clean up old order flows (keep only recent ones)
        const activeOrderFlows = new Map<number, OrderFlow>();
        for (const [price, orderFlow] of state.orderFlows.entries()) {
          if (orderFlow.timestamp > cutoffTime) {
            activeOrderFlows.set(price, orderFlow);
          }
        }

        set({
          tradeHistory: filteredTrades,
          orderFlows: activeOrderFlows,
        });
      }, DATA_CLEANUP_INTERVAL);
    };

    // Start cleanup on store creation
    startCleanup();

    return {
      // Core data
      orderFlows: new Map<number, OrderFlow>(),
      marketSnapshot: createDefaultMarketSnapshot(DEFAULT_SYMBOL),
      tradeHistory: [],
      currentPrice: null,

      // Connection and performance
      connectionStatus: 'disconnected',
      performanceMetrics: createDefaultPerformanceMetrics(),

      // Configuration
      symbol: DEFAULT_SYMBOL,
      priceStep: DEFAULT_PRICE_STEP,
      maxLevels: DEFAULT_MAX_LEVELS,
      maxTradeHistory: DEFAULT_MAX_TRADE_HISTORY,

      // Actions
      updateOrderBook: (bids: [string, string][], asks: [string, string][]) => {
        const state = get();
        const now = Date.now();
        const newOrderFlows = new Map(state.orderFlows);

        // Process bids
        bids.forEach(([priceStr, quantityStr]) => {
          const price = parseFloat(priceStr);
          const quantity = parseFloat(quantityStr);

          // Round price to price step
          const roundedPrice = Math.round(price / state.priceStep) * state.priceStep;

          if (quantity === 0) {
            // Remove level if quantity is 0
            newOrderFlows.delete(roundedPrice);
          } else {
            const existing = newOrderFlows.get(roundedPrice);
            const orderFlow: OrderFlow = {
              price: roundedPrice,
              bidVolume: quantity,
              askVolume: existing?.askVolume || 0,
              activeBuyVolume: existing?.activeBuyVolume || 0,
              activeSellVolume: existing?.activeSellVolume || 0,
              historicalBuyVolume: existing?.historicalBuyVolume || 0,
              historicalSellVolume: existing?.historicalSellVolume || 0,
              timestamp: now,
            };
            newOrderFlows.set(roundedPrice, orderFlow);
          }
        });

        // Process asks
        asks.forEach(([priceStr, quantityStr]) => {
          const price = parseFloat(priceStr);
          const quantity = parseFloat(quantityStr);

          // Round price to price step
          const roundedPrice = Math.round(price / state.priceStep) * state.priceStep;

          if (quantity === 0) {
            // Remove level if quantity is 0 and no bid volume
            const existing = newOrderFlows.get(roundedPrice);
            if (existing && existing.bidVolume === 0) {
              newOrderFlows.delete(roundedPrice);
            } else if (existing) {
              newOrderFlows.set(roundedPrice, { ...existing, askVolume: 0, timestamp: now });
            }
          } else {
            const existing = newOrderFlows.get(roundedPrice);
            const orderFlow: OrderFlow = {
              price: roundedPrice,
              bidVolume: existing?.bidVolume || 0,
              askVolume: quantity,
              activeBuyVolume: existing?.activeBuyVolume || 0,
              activeSellVolume: existing?.activeSellVolume || 0,
              historicalBuyVolume: existing?.historicalBuyVolume || 0,
              historicalSellVolume: existing?.historicalSellVolume || 0,
              timestamp: now,
            };
            newOrderFlows.set(roundedPrice, orderFlow);
          }
        });

        // Update market snapshot with best bid/ask
        const sortedPrices = Array.from(newOrderFlows.keys()).sort((a, b) => b - a);
        let bestBid: number | null = null;
        let bestAsk: number | null = null;

        for (const price of sortedPrices) {
          const orderFlow = newOrderFlows.get(price)!;
          if (orderFlow.bidVolume > 0 && bestBid === null) {
            bestBid = price;
          }
          if (orderFlow.askVolume > 0 && bestAsk === null) {
            bestAsk = price;
          }
          if (bestBid !== null && bestAsk !== null) break;
        }

        // Calculate spread and order book imbalance
        const spread = bestBid && bestAsk ? bestAsk - bestBid : 0;
        const totalBidVolume = Array.from(newOrderFlows.values()).reduce(
          (sum, flow) => sum + flow.bidVolume,
          0
        );
        const totalAskVolume = Array.from(newOrderFlows.values()).reduce(
          (sum, flow) => sum + flow.askVolume,
          0
        );
        const orderBookImbalance =
          totalBidVolume + totalAskVolume > 0
            ? (totalBidVolume - totalAskVolume) / (totalBidVolume + totalAskVolume)
            : 0;

        set({
          orderFlows: newOrderFlows,
          marketSnapshot: {
            ...state.marketSnapshot!,
            bestBid,
            bestAsk,
            spread,
            orderBookImbalance,
            timestamp: now,
          },
        });
      },

      addTrade: (trade: TradeData) => {
        const state = get();
        const now = Date.now();

        // Add to trade history with size limit
        const newTradeHistory = [trade, ...state.tradeHistory].slice(0, state.maxTradeHistory);

        // Update current price
        const currentPrice = trade.price;

        // Update order flows with active volume
        const newOrderFlows = new Map(state.orderFlows);
        const roundedPrice = Math.round(trade.price / state.priceStep) * state.priceStep;
        const existing = newOrderFlows.get(roundedPrice);

        if (existing) {
          const updatedFlow: OrderFlow = {
            ...existing,
            activeBuyVolume: trade.isBuyerMaker ? existing.activeBuyVolume : existing.activeBuyVolume + trade.quantity,
            activeSellVolume: trade.isBuyerMaker ? existing.activeSellVolume + trade.quantity : existing.activeSellVolume,
            historicalBuyVolume: trade.isBuyerMaker ? existing.historicalBuyVolume : existing.historicalBuyVolume + trade.quantity,
            historicalSellVolume: trade.isBuyerMaker ? existing.historicalSellVolume + trade.quantity : existing.historicalSellVolume,
            timestamp: now,
          };
          newOrderFlows.set(roundedPrice, updatedFlow);
        } else {
          // Create new order flow for this price level
          const newFlow: OrderFlow = {
            price: roundedPrice,
            bidVolume: 0,
            askVolume: 0,
            activeBuyVolume: trade.isBuyerMaker ? 0 : trade.quantity,
            activeSellVolume: trade.isBuyerMaker ? trade.quantity : 0,
            historicalBuyVolume: trade.isBuyerMaker ? 0 : trade.quantity,
            historicalSellVolume: trade.isBuyerMaker ? trade.quantity : 0,
            timestamp: now,
          };
          newOrderFlows.set(roundedPrice, newFlow);
        }

        set({
          tradeHistory: newTradeHistory,
          currentPrice,
          orderFlows: newOrderFlows,
          marketSnapshot: {
            ...state.marketSnapshot!,
            currentPrice,
            timestamp: now,
          },
        });
      },

      updateMarketSnapshot: (snapshot: Partial<MarketSnapshot>) => {
        const state = get();
        set({
          marketSnapshot: {
            ...state.marketSnapshot!,
            ...snapshot,
            timestamp: Date.now(),
          },
        });
      },

      setConnectionStatus: (status: ConnectionStatus) => {
        set({ connectionStatus: status });
      },

      updatePerformanceMetrics: (metrics: Partial<PerformanceMetrics>) => {
        const state = get();
        set({
          performanceMetrics: {
            ...state.performanceMetrics,
            ...metrics,
            lastMessageTime: Date.now(),
          },
        });
      },

      setSymbol: (symbol: string) => {
        set({
          symbol,
          orderFlows: new Map(),
          tradeHistory: [],
          currentPrice: null,
          marketSnapshot: createDefaultMarketSnapshot(symbol),
        });
      },

      setPriceStep: (step: number) => {
        set({ priceStep: step });
      },

      clearData: () => {
        const state = get();
        set({
          orderFlows: new Map(),
          tradeHistory: [],
          currentPrice: null,
          marketSnapshot: createDefaultMarketSnapshot(state.symbol),
          performanceMetrics: createDefaultPerformanceMetrics(),
        });
      },

      cleanup: () => {
        if (cleanupInterval) {
          clearInterval(cleanupInterval);
          cleanupInterval = null;
        }
      },
    };
  })
);

// Utility functions for data aggregation
export const getOrderBookLevels = (orderFlows: Map<number, OrderFlow>, maxLevels: number = 40) => {
  const levels = Array.from(orderFlows.values())
    .filter((flow) => flow.bidVolume > 0 || flow.askVolume > 0)
    .sort((a, b) => b.price - a.price)
    .slice(0, maxLevels);

  return levels;
};

export const getBidLevels = (orderFlows: Map<number, OrderFlow>, maxLevels: number = 20) => {
  return Array.from(orderFlows.values())
    .filter((flow) => flow.bidVolume > 0)
    .sort((a, b) => b.price - a.price)
    .slice(0, maxLevels);
};

export const getAskLevels = (orderFlows: Map<number, OrderFlow>, maxLevels: number = 20) => {
  return Array.from(orderFlows.values())
    .filter((flow) => flow.askVolume > 0)
    .sort((a, b) => a.price - b.price)
    .slice(0, maxLevels);
};

export const getSpreadInfo = (orderFlows: Map<number, OrderFlow>) => {
  const bidLevels = getBidLevels(orderFlows, 1);
  const askLevels = getAskLevels(orderFlows, 1);

  const bestBid = bidLevels.length > 0 ? bidLevels[0].price : null;
  const bestAsk = askLevels.length > 0 ? askLevels[0].price : null;
  const spread = bestBid && bestAsk ? bestAsk - bestBid : 0;
  const midPrice = bestBid && bestAsk ? (bestBid + bestAsk) / 2 : null;

  return {
    bestBid,
    bestAsk,
    spread,
    midPrice,
    spreadPercent: midPrice ? (spread / midPrice) * 100 : 0,
  };
};

export const getTotalVolume = (orderFlows: Map<number, OrderFlow>) => {
  let totalBidVolume = 0;
  let totalAskVolume = 0;
  let totalActiveBuyVolume = 0;
  let totalActiveSellVolume = 0;

  for (const flow of orderFlows.values()) {
    totalBidVolume += flow.bidVolume;
    totalAskVolume += flow.askVolume;
    totalActiveBuyVolume += flow.activeBuyVolume;
    totalActiveSellVolume += flow.activeSellVolume;
  }

  return {
    totalBidVolume,
    totalAskVolume,
    totalActiveBuyVolume,
    totalActiveSellVolume,
    totalVolume: totalBidVolume + totalAskVolume,
    totalActiveVolume: totalActiveBuyVolume + totalActiveSellVolume,
  };
};
