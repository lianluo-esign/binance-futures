/// 统一订单簿模块
/// 
/// 将原本单一的大文件拆分为多个专门的模块:
/// - types: 数据类型定义
/// - widget: 主要组件实现
/// - rendering: 渲染逻辑
/// - chart: 价格图表功能
/// - popup: 弹出窗口管理
/// - utils: 工具函数

pub mod types;
pub mod widget;
pub mod rendering;
pub mod chart;
pub mod popup;
pub mod utils;
pub mod aggregated_depth;

// 重新导出主要类型
pub use types::{UnifiedOrderBookRow, AggregatedOrderFlow, SmartScrollInfo};
pub use widget::UnifiedOrderBookWidget;
pub use rendering::{TableRenderer, BarRenderer};
pub use chart::{PriceChart, ChartConfig};
pub use popup::{PopupManager, PopupType};
pub use utils::{ScrollCalculator, DataExtractor};
pub use aggregated_depth::{AggregatedDepthManager, AggregatedPriceLevel};