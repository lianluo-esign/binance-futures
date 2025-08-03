import React from 'react';

interface FootprintChartProps {
  className?: string;
}

export const FootprintChart: React.FC<FootprintChartProps> = ({ className = '' }) => {
  return (
    <div className={`bg-gray-800 border border-gray-700 rounded-lg p-4 ${className}`}>
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-white text-lg font-semibold">Footprint Chart</h2>
        <div className="flex items-center space-x-2 text-sm">
          <div className="text-gray-400">5min</div>
          <div className="flex items-center space-x-1">
            <div className="w-2 h-2 bg-green-500 rounded-full"></div>
            <span className="text-gray-400 text-xs">Buy Vol</span>
          </div>
          <div className="flex items-center space-x-1">
            <div className="w-2 h-2 bg-red-500 rounded-full"></div>
            <span className="text-gray-400 text-xs">Sell Vol</span>
          </div>
        </div>
      </div>
      
      <div className="h-full flex items-center justify-center">
        <div className="text-gray-500 text-center">
          <div className="mb-2">📊</div>
          <div>Footprint candlestick chart will be displayed here</div>
          <div className="text-xs mt-1">5-minute candles with volume heatmap</div>
        </div>
      </div>
    </div>
  );
};