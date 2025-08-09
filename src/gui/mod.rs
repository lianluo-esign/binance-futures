/// GUI模块 - 提供图形界面支持
pub mod egui_app;
pub mod orderbook_renderer;
pub mod bar_chart;
pub mod price_tracker;
pub mod layout_manager;
pub mod signal_renderer;
pub mod signal_functions;
pub mod volume_profile;
pub mod price_chart;
pub mod provider_selection_ui;

pub use egui_app::TradingGUI;
pub use orderbook_renderer::{OrderBookRenderer, OrderBookRenderData, PriceLevel};
pub use bar_chart::{BarChartRenderer, BarChartData};
pub use price_tracker::{PriceTracker, PriceTrackerConfig};
pub use layout_manager::{LayoutManager, PriceRegion, LayoutStats};
pub use signal_renderer::SignalRenderer;
pub use signal_functions::render_signals;
pub use volume_profile::{VolumeProfileManager, VolumeProfileRenderer, VolumeProfileData, VolumeLevel, VolumeProfileWidget};
pub use price_chart::{PriceChartRenderer, PricePoint, PriceChartStats};
pub use provider_selection_ui::{ProviderSelectionUI, SelectionAction};
