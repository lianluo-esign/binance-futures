/// GUI模块 - 提供Windows图形界面支持
pub mod egui_app;
pub mod orderbook_widget;
pub mod trade_footprint_widget;
pub mod unified_orderbook_widget;
pub mod debug_window;
pub mod time_footprint_data;
pub mod time_footprint_chart;

pub use egui_app::TradingGUI;
pub use orderbook_widget::OrderBookWidget;
pub use trade_footprint_widget::TradeFootprintWidget;
pub use unified_orderbook_widget::UnifiedOrderBookWidget;
pub use debug_window::DebugWindow;
pub use time_footprint_data::TimeFootprintData;
pub use time_footprint_chart::TimeFootprintChart;
