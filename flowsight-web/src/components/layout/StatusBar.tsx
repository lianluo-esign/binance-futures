import React from 'react';
import { useConnectionStore } from '../../stores/connection-store';
import type { ConnectionStatus } from '../../types';

interface StatusBarProps {
  className?: string;
}

export const StatusBar: React.FC<StatusBarProps> = ({ className = '' }) => {
  const { globalStatus, globalMetrics, connections } = useConnectionStore();

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'connected':
        return 'text-green-500';
      case 'connecting':
      case 'reconnecting':
        return 'text-yellow-500';
      case 'error':
        return 'text-red-500';
      case 'disconnected':
      default:
        return 'text-gray-500';
    }
  };

  const getStatusIndicator = (status: string) => {
    switch (status) {
      case 'connected':
        return '●';
      case 'connecting':
      case 'reconnecting':
        return '◐';
      case 'error':
        return '●';
      case 'disconnected':
      default:
        return '○';
    }
  };

  const formatConnectionStatus = (status: ConnectionStatus): string => {
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

  const formatLatency = (latency: number): string => {
    if (latency < 1) return '<1ms';
    if (latency < 1000) return `${Math.round(latency)}ms`;
    return `${(latency / 1000).toFixed(1)}s`;
  };

  const formatUptime = (uptime: number): string => {
    const seconds = Math.floor(uptime / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);

    if (days > 0) return `${days}d ${hours % 24}h`;
    if (hours > 0) return `${hours}h ${minutes % 60}m`;
    if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
    return `${seconds}s`;
  };

  const formatMemoryUsage = (bytes: number): string => {
    if (bytes < 1024) return `${bytes}B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
  };

  return (
    <div className={`bg-gray-900 text-white px-4 py-2 flex items-center justify-between text-sm border-b border-gray-700 ${className}`}>
      {/* Left side - Connection status */}
      <div className="flex items-center space-x-6">
        <div className="flex items-center space-x-2">
          <span className={`${getStatusColor(globalStatus)} font-mono`}>
            {getStatusIndicator(globalStatus)}
          </span>
          <span className="text-gray-300">
            {formatConnectionStatus(globalStatus)}
          </span>
          <span className="text-gray-500">
            ({connections.size} connection{connections.size !== 1 ? 's' : ''})
          </span>
        </div>

        {/* Performance metrics */}
        <div className="flex items-center space-x-4 text-gray-400">
          <div className="flex items-center space-x-1">
            <span className="text-gray-500">Latency:</span>
            <span className={globalMetrics.latency > 500 ? 'text-yellow-500' : 'text-gray-300'}>
              {formatLatency(globalMetrics.latency)}
            </span>
          </div>
          
          <div className="flex items-center space-x-1">
            <span className="text-gray-500">Msg/s:</span>
            <span className="text-gray-300 font-mono">
              {globalMetrics.messagesPerSecond.toLocaleString()}
            </span>
          </div>

          <div className="flex items-center space-x-1">
            <span className="text-gray-500">Memory:</span>
            <span className="text-gray-300 font-mono">
              {formatMemoryUsage(globalMetrics.memoryUsage)}
            </span>
          </div>
        </div>
      </div>

      {/* Right side - Additional metrics */}
      <div className="flex items-center space-x-4 text-gray-400">
        <div className="flex items-center space-x-1">
          <span className="text-gray-500">Uptime:</span>
          <span className="text-gray-300 font-mono">
            {formatUptime(globalMetrics.connectionUptime)}
          </span>
        </div>

        <div className="flex items-center space-x-1">
          <span className="text-gray-500">Total:</span>
          <span className="text-gray-300 font-mono">
            {globalMetrics.totalMessages.toLocaleString()}
          </span>
        </div>

        {globalMetrics.errorCount > 0 && (
          <div className="flex items-center space-x-1">
            <span className="text-gray-500">Errors:</span>
            <span className="text-red-400 font-mono">
              {globalMetrics.errorCount}
            </span>
          </div>
        )}

        {/* Current time */}
        <div className="text-gray-500 font-mono">
          {new Date().toLocaleTimeString()}
        </div>
      </div>
    </div>
  );
};