pub mod ring_buffer;
pub mod lock_free_ring_buffer;
pub mod cpu_affinity;
pub mod provider;

pub use ring_buffer::RingBuffer;
pub use lock_free_ring_buffer::{LockFreeRingBuffer, SharedLockFreeRingBuffer, create_shared_lock_free_ring_buffer};
pub use cpu_affinity::{init_cpu_affinity, get_cpu_manager, check_affinity_status};

// Provider模块的公共API导出
pub use provider::{
    DataProvider, ProviderType, ProviderStatus, ProviderFactory,
    ConfigurableProvider, ControllableProvider,
    ProviderManager,
    ProviderError, ProviderResult,
    EventKind, PerformanceMetrics, PlaybackInfo,
    ExchangeId,
};

// 从子模块导出具体类型
pub use provider::manager::{ProviderMetadata, ProviderManagerConfig, ProviderManagerStatus};
