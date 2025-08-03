/// GUI module - 高性能线程分离架构的图形界面
/// 
/// 核心组件:
/// - ThreadedTradingGUI: 主要的线程分离GUI应用
/// - GUIState: 线程安全的UI状态管理
/// - orderbook: 模块化订单簿组件系统
/// - core: 服务化架构核心组件

pub mod threaded_gui;

// 服务化架构核心
pub mod core;

// 模块化订单簿组件
pub mod orderbook;

// 保留原有组件以支持threaded_gui
pub mod debug_window;

// 重新导出核心组件
pub use threaded_gui::{
    ThreadedTradingGUI, 
    GUIState, 
    OrderBookUIData, 
    MarketStats, 
    PerformanceInfo,
    ConnectionStatus,
    UISettings
};

// 导出模块化订单簿组件
pub use orderbook::{
    UnifiedOrderBookWidget,
    types::{UnifiedOrderBookRow, ColorScheme, SmartScrollInfo},
    utils::{ScrollCalculator, DataExtractor, PerformanceTracker, PriceValidator},
    rendering::{TableRenderer, ColumnWidths, BarRenderer, GridRenderer},
    chart::{PriceChart, ChartConfig},
    popup::{PopupManager, PopupType, TradingSignalConfig, BacktestConfig, AppSettings},
};

// 导出服务化架构组件 (暂时注释以解决模块冲突)
// pub use core::service::{
//     GUIServiceManager as GUIServiceManagerInternal,
//     ServiceMessage as GUIServiceMessage,
//     ServiceMessageType, MessagePriority, ServiceHealth as GUIServiceHealth, ServiceStats as GUIServiceStats
// };

// 原有组件
pub use debug_window::DebugWindow;
