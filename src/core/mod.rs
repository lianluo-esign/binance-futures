pub mod ring_buffer;
pub mod lock_free_ring_buffer;
pub mod cache_optimized_ring_buffer;

pub use ring_buffer::RingBuffer;
pub use lock_free_ring_buffer::{LockFreeRingBuffer, SharedLockFreeRingBuffer, create_shared_lock_free_ring_buffer};
pub use cache_optimized_ring_buffer::CacheOptimizedRingBuffer;

// Backward-compatible type alias for gradual migration
pub type OptimizedLockFreeRingBuffer<T> = CacheOptimizedRingBuffer<T>;
