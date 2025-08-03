/// GUI状态管理系统
/// 
/// 提供统一的状态管理机制:
/// - 组件状态快照
/// - 状态变更事件  
/// - 状态持久化
/// - 状态同步

pub mod types;
pub mod persistence;
pub mod manager;

// 重新导出公共类型
pub use types::{StateChangeEvent, StateChangeType, ComponentStateSnapshot};
pub use persistence::{StatePersistence, MemoryStatePersistence, FileStatePersistence};
pub use manager::{StateManager, StateManagerConfig};