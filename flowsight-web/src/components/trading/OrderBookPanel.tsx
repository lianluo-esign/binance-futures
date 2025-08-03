import React from 'react';

interface OrderBookPanelProps {
  className?: string;
}

export const OrderBookPanel: React.FC<OrderBookPanelProps> = ({ className = '' }) => {
  return (
    <div className={`bg-gray-800 border border-gray-700 rounded-lg p-4 ${className}`}>
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-white text-lg font-semibold">Order Book</h2>
        <div className="text-gray-400 text-sm">BTCUSDT</div>
      </div>
      
      <div className="space-y-2">
        {/* Header */}
        <div className="grid grid-cols-3 text-gray-400 text-xs font-medium border-b border-gray-700 pb-2">
          <div className="text-left">Price</div>
          <div className="text-right">Size</div>
          <div className="text-right">Total</div>
        </div>
        
        {/* Placeholder content */}
        <div className="text-gray-500 text-center py-8">
          Order book data will be displayed here
        </div>
      </div>
    </div>
  );
};