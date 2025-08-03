pub mod ring_buffer;
pub mod lock_free_ring_buffer;
pub mod cache_optimized_ring_buffer;
pub mod performance_config;
pub mod adaptive_ring_buffer;

pub use ring_buffer::RingBuffer;
pub use lock_free_ring_buffer::{LockFreeRingBuffer, SharedLockFreeRingBuffer, create_shared_lock_free_ring_buffer};
pub use cache_optimized_ring_buffer::CacheOptimizedRingBuffer;
pub use performance_config::{PerformanceConfig, PerformanceMetrics, AdaptivePerformanceTuner, PerformanceGrade};
pub use adaptive_ring_buffer::{AdaptiveRingBuffer, BufferStatsSnapshot};

// Backward-compatible type alias for gradual migration
pub type OptimizedLockFreeRingBuffer<T> = CacheOptimizedRingBuffer<T>;
