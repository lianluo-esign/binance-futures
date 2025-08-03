import React, { useEffect, useRef } from 'react';
import { StatusBar } from './StatusBar';
import { OrderBookPanel } from '../trading/OrderBookPanel';
import { ActiveOrderChart } from '../charts/ActiveOrderChart';
import { FootprintChart } from '../charts/FootprintChart';
import { useConnectionStore } from '../../stores/connection-store';
import { useLayoutWebSocket } from '../../hooks/use-layout-websocket';

interface MainLayoutProps {
  children?: React.ReactNode;
}

export const MainLayout: React.FC<MainLayoutProps> = ({ children }) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const { globalStatus } = useConnectionStore();
  
  // Initialize WebSocket connection for the layout
  const { isConnected, reconnect } = useLayoutWebSocket({
    symbol: 'BTCUSDT',
    autoConnect: true,
    streams: ['depth', 'trade', 'ticker']
  });

  // Handle window resize for responsive layout
  useEffect(() => {
    const handleResize = () => {
      // Force re-render on resize to update panel dimensions
      if (containerRef.current) {
        containerRef.current.style.height = `${window.innerHeight}px`;
      }
    };

    handleResize(); // Initial call
    window.addEventListener('resize', handleResize);
    
    return () => {
      window.removeEventListener('resize', handleResize);
    };
  }, []);

  return (
    <div 
      ref={containerRef}
      className="min-h-screen bg-gray-900 flex flex-col overflow-hidden"
    >
      {/* Status Bar */}
      <StatusBar />

      {/* Main Content Area */}
      <div className="flex-1 overflow-hidden">
        {/* Desktop Layout (768px and above) */}
        <div className="hidden md:grid h-full grid-cols-2 gap-4 p-4">
          {/* Left Panel - Order Book (50% width) */}
          <div className="flex flex-col">
            <OrderBookPanel className="h-full" />
          </div>

          {/* Right Panel - Split into two sections */}
          <div className="flex flex-col gap-4">
            {/* Upper Right - Active Order Chart (45% height) */}
            <div className="flex-[45]">
              <ActiveOrderChart className="h-full" />
            </div>

            {/* Lower Right - Footprint Chart (55% height) */}
            <div className="flex-[55]">
              <FootprintChart className="h-full" />
            </div>
          </div>
        </div>

        {/* Tablet Layout (640px to 768px) */}
        <div className="hidden sm:block md:hidden h-full p-4">
          <div className="grid grid-rows-2 gap-4 h-full">
            {/* Top Row - Order Book and Active Chart */}
            <div className="grid grid-cols-2 gap-4">
              <OrderBookPanel className="h-full" />
              <ActiveOrderChart className="h-full" />
            </div>
            
            {/* Bottom Row - Footprint Chart */}
            <div>
              <FootprintChart className="h-full" />
            </div>
          </div>
        </div>

        {/* Mobile Layout (below 640px) */}
        <div className="block sm:hidden h-full overflow-y-auto">
          <div className="flex flex-col gap-4 p-4 min-h-full">
            {/* Stacked layout for mobile */}
            <div className="min-h-[400px]">
              <OrderBookPanel className="h-full" />
            </div>
            
            <div className="min-h-[300px]">
              <ActiveOrderChart className="h-full" />
            </div>
            
            <div className="min-h-[400px]">
              <FootprintChart className="h-full" />
            </div>
          </div>
        </div>
      </div>

      {/* Connection Status Overlay for disconnected state */}
      {globalStatus === 'disconnected' && (
        <div className="absolute inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-gray-800 border border-gray-600 rounded-lg p-6 text-center">
            <div className="text-gray-400 mb-2">⚠️</div>
            <div className="text-white text-lg font-semibold mb-2">
              Connection Lost
            </div>
            <div className="text-gray-400 text-sm">
              Attempting to reconnect to market data...
            </div>
          </div>
        </div>
      )}

      {/* Error State Overlay */}
      {globalStatus === 'error' && (
        <div className="absolute inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-gray-800 border border-red-600 rounded-lg p-6 text-center">
            <div className="text-red-500 mb-2">❌</div>
            <div className="text-white text-lg font-semibold mb-2">
              Connection Error
            </div>
            <div className="text-gray-400 text-sm mb-4">
              Unable to connect to market data feed
            </div>
            <button 
              className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded text-sm"
              onClick={() => window.location.reload()}
            >
              Retry Connection
            </button>
          </div>
        </div>
      )}

      {children}
    </div>
  );
};