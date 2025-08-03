import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import '@testing-library/jest-dom';
import { MainLayout } from '../MainLayout';

// Mock the stores
jest.mock('../../../stores/connection-store', () => ({
  useConnectionStore: jest.fn(() => ({
    globalStatus: 'connected',
    globalMetrics: {
      messagesPerSecond: 100,
      latency: 50,
      memoryUsage: 1024 * 1024,
      connectionUptime: 60000,
      totalMessages: 1000,
      errorCount: 0,
      lastMessageTime: Date.now(),
    },
    connections: new Map([
      ['binance-main', {
        status: 'connected',
        url: 'wss://stream.binance.com:9443/ws',
        connectedAt: Date.now() - 60000,
        lastError: null,
        reconnectAttempts: 0,
        maxReconnectAttempts: 5,
      }]
    ]),
    addConnection: jest.fn(),
    updateConnection: jest.fn(),
    removeConnection: jest.fn(),
    subscribe: jest.fn(),
    updateGlobalMetrics: jest.fn(),
  })),
}));

// Mock the WebSocket hook
jest.mock('../../../hooks/use-layout-websocket', () => ({
  useLayoutWebSocket: jest.fn(() => ({
    isConnected: true,
    reconnect: jest.fn(),
    disconnect: jest.fn(),
    changeSymbol: jest.fn(),
  })),
}));

// Mock the child components
jest.mock('../../trading/OrderBookPanel', () => ({
  OrderBookPanel: ({ className }: { className?: string }) => (
    <div data-testid="order-book-panel" className={className}>
      Order Book Panel
    </div>
  ),
}));

jest.mock('../../charts/ActiveOrderChart', () => ({
  ActiveOrderChart: ({ className }: { className?: string }) => (
    <div data-testid="active-order-chart" className={className}>
      Active Order Chart
    </div>
  ),
}));

jest.mock('../../charts/FootprintChart', () => ({
  FootprintChart: ({ className }: { className?: string }) => (
    <div data-testid="footprint-chart" className={className}>
      Footprint Chart
    </div>
  ),
}));

// Mock window.innerHeight for responsive tests
Object.defineProperty(window, 'innerHeight', {
  writable: true,
  configurable: true,
  value: 1080,
});

describe('MainLayout', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders the main layout with all panels', () => {
    render(<MainLayout />);

    // Check if status bar is present
    expect(screen.getByText('Connected')).toBeInTheDocument();
    
    // Check if all three panels are present
    expect(screen.getByTestId('order-book-panel')).toBeInTheDocument();
    expect(screen.getByTestId('active-order-chart')).toBeInTheDocument();
    expect(screen.getByTestId('footprint-chart')).toBeInTheDocument();
  });

  it('displays connection status correctly', () => {
    render(<MainLayout />);

    // Check connection status indicator
    expect(screen.getByText('Connected')).toBeInTheDocument();
    expect(screen.getByText('(1 connection)')).toBeInTheDocument();
  });

  it('shows performance metrics in status bar', () => {
    render(<MainLayout />);

    // Check if performance metrics are displayed
    expect(screen.getByText('50ms')).toBeInTheDocument(); // Latency
    expect(screen.getByText('100')).toBeInTheDocument(); // Messages per second
    expect(screen.getByText('1.0MB')).toBeInTheDocument(); // Memory usage
  });

  it('handles window resize events', async () => {
    render(<MainLayout />);

    // Simulate window resize
    Object.defineProperty(window, 'innerHeight', {
      writable: true,
      configurable: true,
      value: 800,
    });

    fireEvent(window, new Event('resize'));

    // Wait for the resize handler to execute
    await waitFor(() => {
      // The layout should still be rendered correctly
      expect(screen.getByTestId('order-book-panel')).toBeInTheDocument();
    });
  });

  it('displays disconnected overlay when connection is lost', () => {
    // Mock disconnected state
    const mockUseConnectionStore = require('../../../stores/connection-store').useConnectionStore;
    mockUseConnectionStore.mockReturnValue({
      globalStatus: 'disconnected',
      globalMetrics: {
        messagesPerSecond: 0,
        latency: 0,
        memoryUsage: 0,
        connectionUptime: 0,
        totalMessages: 0,
        errorCount: 0,
        lastMessageTime: 0,
      },
      connections: new Map(),
      addConnection: jest.fn(),
      updateConnection: jest.fn(),
      removeConnection: jest.fn(),
      subscribe: jest.fn(),
      updateGlobalMetrics: jest.fn(),
    });

    render(<MainLayout />);

    expect(screen.getByText('Connection Lost')).toBeInTheDocument();
    expect(screen.getByText('Attempting to reconnect to market data...')).toBeInTheDocument();
  });

  it('displays error overlay when connection has error', () => {
    // Mock error state
    const mockUseConnectionStore = require('../../../stores/connection-store').useConnectionStore;
    mockUseConnectionStore.mockReturnValue({
      globalStatus: 'error',
      globalMetrics: {
        messagesPerSecond: 0,
        latency: 0,
        memoryUsage: 0,
        connectionUptime: 0,
        totalMessages: 0,
        errorCount: 1,
        lastMessageTime: 0,
      },
      connections: new Map(),
      addConnection: jest.fn(),
      updateConnection: jest.fn(),
      removeConnection: jest.fn(),
      subscribe: jest.fn(),
      updateGlobalMetrics: jest.fn(),
    });

    render(<MainLayout />);

    expect(screen.getByText('Connection Error')).toBeInTheDocument();
    expect(screen.getByText('Unable to connect to market data feed')).toBeInTheDocument();
    expect(screen.getByText('Retry Connection')).toBeInTheDocument();
  });

  it('handles retry connection button click', () => {
    // Mock error state
    const mockUseConnectionStore = require('../../../stores/connection-store').useConnectionStore;
    mockUseConnectionStore.mockReturnValue({
      globalStatus: 'error',
      globalMetrics: {
        messagesPerSecond: 0,
        latency: 0,
        memoryUsage: 0,
        connectionUptime: 0,
        totalMessages: 0,
        errorCount: 1,
        lastMessageTime: 0,
      },
      connections: new Map(),
      addConnection: jest.fn(),
      updateConnection: jest.fn(),
      removeConnection: jest.fn(),
      subscribe: jest.fn(),
      updateGlobalMetrics: jest.fn(),
    });

    // Mock window.location.reload
    const mockReload = jest.fn();
    Object.defineProperty(window, 'location', {
      value: { reload: mockReload },
      writable: true,
    });

    render(<MainLayout />);

    const retryButton = screen.getByText('Retry Connection');
    fireEvent.click(retryButton);

    expect(mockReload).toHaveBeenCalled();
  });

  it('renders children when provided', () => {
    render(
      <MainLayout>
        <div data-testid="custom-child">Custom Child Component</div>
      </MainLayout>
    );

    expect(screen.getByTestId('custom-child')).toBeInTheDocument();
  });
});