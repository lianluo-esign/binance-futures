import React from 'react';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';
import { StatusBar } from '../StatusBar';

// Mock the connection store
jest.mock('../../../stores/connection-store', () => ({
  useConnectionStore: jest.fn(),
  formatConnectionStatus: jest.fn((status) => {
    switch (status) {
      case 'connected': return 'Connected';
      case 'connecting': return 'Connecting...';
      case 'disconnected': return 'Disconnected';
      case 'error': return 'Connection Error';
      case 'reconnecting': return 'Reconnecting...';
      default: return 'Unknown';
    }
  }),
  formatLatency: jest.fn((latency) => {
    if (latency < 1) return '<1ms';
    if (latency < 1000) return `${Math.round(latency)}ms`;
    return `${(latency / 1000).toFixed(1)}s`;
  }),
  formatUptime: jest.fn((uptime) => {
    const seconds = Math.floor(uptime / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    
    if (hours > 0) return `${hours}h ${minutes % 60}m`;
    if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
    return `${seconds}s`;
  }),
}));

describe('StatusBar', () => {
  const mockConnectionStore = require('../../../stores/connection-store').useConnectionStore;

  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders connected status correctly', () => {
    mockConnectionStore.mockReturnValue({
      globalStatus: 'connected',
      globalMetrics: {
        messagesPerSecond: 150,
        latency: 45,
        memoryUsage: 2048 * 1024, // 2MB
        connectionUptime: 120000, // 2 minutes
        totalMessages: 5000,
        errorCount: 0,
        lastMessageTime: Date.now(),
      },
      connections: new Map([
        ['binance-main', { status: 'connected' }],
        ['binance-backup', { status: 'connected' }],
      ]),
    });

    render(<StatusBar />);

    expect(screen.getByText('Connected')).toBeInTheDocument();
    expect(screen.getByText('(2 connections)')).toBeInTheDocument();
    expect(screen.getByText('45ms')).toBeInTheDocument();
    expect(screen.getByText('150')).toBeInTheDocument();
    expect(screen.getByText('2.0MB')).toBeInTheDocument();
    expect(screen.getByText('2m 0s')).toBeInTheDocument();
    expect(screen.getByText('5,000')).toBeInTheDocument();
  });

  it('renders disconnected status correctly', () => {
    mockConnectionStore.mockReturnValue({
      globalStatus: 'disconnected',
      globalMetrics: {
        messagesPerSecond: 0,
        latency: 0,
        memoryUsage: 512 * 1024, // 512KB
        connectionUptime: 0,
        totalMessages: 0,
        errorCount: 0,
        lastMessageTime: 0,
      },
      connections: new Map(),
    });

    render(<StatusBar />);

    expect(screen.getByText('Disconnected')).toBeInTheDocument();
    expect(screen.getByText('(0 connections)')).toBeInTheDocument();
    expect(screen.getByText('<1ms')).toBeInTheDocument();
    expect(screen.getAllByText('0')).toHaveLength(2); // Messages per second and total messages
    expect(screen.getByText('512.0KB')).toBeInTheDocument();
  });

  it('renders error status with error count', () => {
    mockConnectionStore.mockReturnValue({
      globalStatus: 'error',
      globalMetrics: {
        messagesPerSecond: 0,
        latency: 1500,
        memoryUsage: 1024 * 1024, // 1MB
        connectionUptime: 30000, // 30 seconds
        totalMessages: 100,
        errorCount: 3,
        lastMessageTime: Date.now() - 5000,
      },
      connections: new Map([
        ['binance-main', { status: 'error' }],
      ]),
    });

    render(<StatusBar />);

    expect(screen.getByText('Connection Error')).toBeInTheDocument();
    expect(screen.getByText('(1 connection)')).toBeInTheDocument();
    expect(screen.getByText('1.5s')).toBeInTheDocument();
    expect(screen.getByText('Errors:')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  it('renders connecting status correctly', () => {
    mockConnectionStore.mockReturnValue({
      globalStatus: 'connecting',
      globalMetrics: {
        messagesPerSecond: 0,
        latency: 0,
        memoryUsage: 256 * 1024, // 256KB
        connectionUptime: 0,
        totalMessages: 0,
        errorCount: 0,
        lastMessageTime: 0,
      },
      connections: new Map([
        ['binance-main', { status: 'connecting' }],
      ]),
    });

    render(<StatusBar />);

    expect(screen.getByText('Connecting...')).toBeInTheDocument();
    expect(screen.getByText('(1 connection)')).toBeInTheDocument();
  });

  it('renders reconnecting status correctly', () => {
    mockConnectionStore.mockReturnValue({
      globalStatus: 'reconnecting',
      globalMetrics: {
        messagesPerSecond: 0,
        latency: 2000,
        memoryUsage: 768 * 1024, // 768KB
        connectionUptime: 60000, // 1 minute
        totalMessages: 50,
        errorCount: 1,
        lastMessageTime: Date.now() - 10000,
      },
      connections: new Map([
        ['binance-main', { status: 'reconnecting' }],
      ]),
    });

    render(<StatusBar />);

    expect(screen.getByText('Reconnecting...')).toBeInTheDocument();
    expect(screen.getByText('2.0s')).toBeInTheDocument();
  });

  it('applies custom className', () => {
    mockConnectionStore.mockReturnValue({
      globalStatus: 'connected',
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
    });

    const { container } = render(<StatusBar className="custom-class" />);
    
    expect(container.firstChild).toHaveClass('custom-class');
  });

  it('shows high latency warning color', () => {
    mockConnectionStore.mockReturnValue({
      globalStatus: 'connected',
      globalMetrics: {
        messagesPerSecond: 100,
        latency: 750, // High latency
        memoryUsage: 1024 * 1024,
        connectionUptime: 60000,
        totalMessages: 1000,
        errorCount: 0,
        lastMessageTime: Date.now(),
      },
      connections: new Map([
        ['binance-main', { status: 'connected' }],
      ]),
    });

    render(<StatusBar />);

    const latencyElement = screen.getByText('750ms');
    expect(latencyElement).toHaveClass('text-yellow-500');
  });

  it('displays current time', () => {
    mockConnectionStore.mockReturnValue({
      globalStatus: 'connected',
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
    });

    render(<StatusBar />);

    // Check if time is displayed (format: HH:MM:SS AM/PM)
    const timeRegex = /\d{1,2}:\d{2}:\d{2}/;
    expect(screen.getByText(timeRegex)).toBeInTheDocument();
  });

  it('formats memory usage correctly for different sizes', () => {
    // Test bytes
    mockConnectionStore.mockReturnValue({
      globalStatus: 'connected',
      globalMetrics: {
        messagesPerSecond: 0,
        latency: 0,
        memoryUsage: 512, // 512 bytes
        connectionUptime: 0,
        totalMessages: 0,
        errorCount: 0,
        lastMessageTime: 0,
      },
      connections: new Map(),
    });

    const { rerender } = render(<StatusBar />);
    expect(screen.getByText('512B')).toBeInTheDocument();

    // Test KB
    mockConnectionStore.mockReturnValue({
      globalStatus: 'connected',
      globalMetrics: {
        messagesPerSecond: 0,
        latency: 0,
        memoryUsage: 1536, // 1.5KB
        connectionUptime: 0,
        totalMessages: 0,
        errorCount: 0,
        lastMessageTime: 0,
      },
      connections: new Map(),
    });

    rerender(<StatusBar />);
    expect(screen.getByText('1.5KB')).toBeInTheDocument();

    // Test MB
    mockConnectionStore.mockReturnValue({
      globalStatus: 'connected',
      globalMetrics: {
        messagesPerSecond: 0,
        latency: 0,
        memoryUsage: 2.5 * 1024 * 1024, // 2.5MB
        connectionUptime: 0,
        totalMessages: 0,
        errorCount: 0,
        lastMessageTime: 0,
      },
      connections: new Map(),
    });

    rerender(<StatusBar />);
    expect(screen.getByText('2.5MB')).toBeInTheDocument();
  });
});