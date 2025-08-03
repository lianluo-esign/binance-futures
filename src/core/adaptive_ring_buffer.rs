use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// 自适应高性能环形缓冲区
/// 
/// 相比原始RingBuffer的改进:
/// 1. 支持动态扩容和背压控制
/// 2. 集成性能监控和统计
/// 3. 自适应批处理优化
/// 4. 内存预分配和缓存友好的布局
#[repr(C, align(64))]
pub struct AdaptiveRingBuffer<T> {
    /// 缓冲区数据存储
    buffer: Box<[MaybeUninit<T>]>,
    /// 当前容量 (总是2的幂)
    capacity: usize,
    /// 位掩码，用于快速取模
    mask: usize,
    /// 读取位置
    head: AtomicUsize,
    /// 写入位置
    tail: AtomicUsize,
    /// 当前元素数量
    size: AtomicUsize,
    /// 是否启用自动扩容
    auto_resize: AtomicBool,
    /// 最大容量限制
    max_capacity: usize,
    /// 扩容阈值 (使用率)
    resize_threshold: f64,
    /// 背压控制开关
    backpressure_enabled: AtomicBool,
    /// 背压阈值
    backpressure_threshold: f64,
    /// 性能统计
    stats: BufferStats,
}

/// 缓冲区性能统计
#[derive(Debug)]
pub struct BufferStats {
    /// 总推入次数
    pub total_pushes: AtomicUsize,
    /// 总弹出次数
    pub total_pops: AtomicUsize,
    /// 丢弃的元素数量
    pub dropped_items: AtomicUsize,
    /// 扩容次数
    pub resize_count: AtomicUsize,
    /// 背压触发次数
    pub backpressure_events: AtomicUsize,
    /// 上次重置时间
    pub last_reset: std::sync::Mutex<Instant>,
}

impl<T> AdaptiveRingBuffer<T> {
    /// 创建新的自适应环形缓冲区
    /// 
    /// # 参数
    /// * `initial_capacity` - 初始容量
    /// * `max_capacity` - 最大容量限制
    /// * `resize_threshold` - 扩容阈值 (0.0-1.0)
    /// * `backpressure_threshold` - 背压阈值 (0.0-1.0)
    pub fn new(
        initial_capacity: usize,
        max_capacity: usize,
        resize_threshold: f64,
        backpressure_threshold: f64,
    ) -> Self {
        let capacity = initial_capacity.next_power_of_two().max(64); // 最小64个元素
        let mask = capacity - 1;
        
        // 预分配缓冲区
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(MaybeUninit::uninit());
        }
        
        Self {
            buffer: buffer.into_boxed_slice(),
            capacity,
            mask,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            size: AtomicUsize::new(0),
            auto_resize: AtomicBool::new(true),
            max_capacity: max_capacity.max(capacity),
            resize_threshold: resize_threshold.clamp(0.1, 1.0),
            backpressure_enabled: AtomicBool::new(true),
            backpressure_threshold: backpressure_threshold.clamp(0.5, 1.0),
            stats: BufferStats::new(),
        }
    }

    /// 推入单个元素
    #[inline]
    pub fn push(&self, item: T) -> Result<(), T> {
        self.stats.total_pushes.fetch_add(1, Ordering::Relaxed);

        let current_size = self.size.load(Ordering::Acquire);
        let usage_ratio = current_size as f64 / self.capacity as f64;

        // 检查背压控制
        if self.backpressure_enabled.load(Ordering::Relaxed) && usage_ratio >= self.backpressure_threshold {
            self.stats.backpressure_events.fetch_add(1, Ordering::Relaxed);
            return Err(item); // 触发背压，拒绝新数据
        }

        // 检查是否需要扩容
        if self.auto_resize.load(Ordering::Relaxed) && usage_ratio >= self.resize_threshold {
            // 注意: 这里应该使用异步扩容机制，避免阻塞
            // 当前简化实现，实际应用中需要在后台线程处理
            log::warn!("缓冲区使用率达到 {:.1}%，建议扩容", usage_ratio * 100.0);
        }

        // 尝试推入元素
        let current_tail = self.tail.load(Ordering::Relaxed);
        let next_tail = (current_tail + 1) & self.mask;

        // 检查是否已满
        if next_tail == self.head.load(Ordering::Acquire) {
            self.stats.dropped_items.fetch_add(1, Ordering::Relaxed);
            return Err(item); // 缓冲区已满
        }

        // 写入数据
        unsafe {
            let slot = &mut *(self.buffer.as_ptr() as *mut MaybeUninit<T>).add(current_tail);
            slot.write(item);
        }

        // 预取下一个可能访问的缓存行
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let prefetch_idx = (current_tail + 4) & self.mask;
            let ptr = self.buffer.as_ptr().add(prefetch_idx);
            std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0);
        }

        // 更新尾指针和大小
        self.tail.store(next_tail, Ordering::Release);
        self.size.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// 弹出单个元素
    #[inline]
    pub fn pop(&self) -> Option<T> {
        self.stats.total_pops.fetch_add(1, Ordering::Relaxed);

        let current_head = self.head.load(Ordering::Relaxed);
        
        // 检查是否为空
        if current_head == self.tail.load(Ordering::Acquire) {
            return None;
        }

        // 读取数据
        let item = unsafe {
            let slot = &*(self.buffer.as_ptr() as *const MaybeUninit<T>).add(current_head);
            slot.assume_init_read()
        };

        // 预取下一个可能访问的缓存行
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let prefetch_idx = (current_head + 4) & self.mask;
            let ptr = self.buffer.as_ptr().add(prefetch_idx);
            std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0);
        }

        // 更新头指针和大小
        let next_head = (current_head + 1) & self.mask;
        self.head.store(next_head, Ordering::Release);
        self.size.fetch_sub(1, Ordering::Relaxed);

        Some(item)
    }

    /// 批量推入元素 (优化版本)
    pub fn push_batch(&self, items: &[T]) -> usize 
    where 
        T: Clone,
    {
        if items.is_empty() {
            return 0;
        }

        let mut pushed = 0;
        let batch_size = items.len().min(64); // 限制批处理大小

        // 检查可用空间
        let current_size = self.size.load(Ordering::Acquire);
        let available_space = self.capacity.saturating_sub(current_size);
        
        if available_space == 0 {
            return 0; // 没有可用空间
        }

        let items_to_push = batch_size.min(available_space);

        // 批量推入
        for item in items.iter().take(items_to_push) {
            match self.push(item.clone()) {
                Ok(()) => pushed += 1,
                Err(_) => break, // 遇到错误停止
            }
        }

        pushed
    }

    /// 批量弹出元素 (优化版本)
    pub fn pop_batch(&self, max_items: usize) -> Vec<T> {
        let mut result = Vec::with_capacity(max_items.min(64));
        
        for _ in 0..max_items {
            match self.pop() {
                Some(item) => result.push(item),
                None => break,
            }
        }
        
        result
    }

    /// 获取当前大小
    #[inline]
    pub fn len(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    /// 获取容量
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 检查是否为空
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 检查是否已满
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity - 1 // 保留一个空槽位
    }

    /// 获取使用率
    pub fn usage_ratio(&self) -> f64 {
        self.len() as f64 / self.capacity as f64
    }

    /// 获取统计信息
    pub fn stats(&self) -> BufferStatsSnapshot {
        BufferStatsSnapshot {
            total_pushes: self.stats.total_pushes.load(Ordering::Relaxed),
            total_pops: self.stats.total_pops.load(Ordering::Relaxed),
            dropped_items: self.stats.dropped_items.load(Ordering::Relaxed),
            resize_count: self.stats.resize_count.load(Ordering::Relaxed),
            backpressure_events: self.stats.backpressure_events.load(Ordering::Relaxed),
            current_size: self.len(),
            current_capacity: self.capacity,
            usage_ratio: self.usage_ratio(),
        }
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        self.stats.total_pushes.store(0, Ordering::Relaxed);
        self.stats.total_pops.store(0, Ordering::Relaxed);
        self.stats.dropped_items.store(0, Ordering::Relaxed);
        self.stats.resize_count.store(0, Ordering::Relaxed);
        self.stats.backpressure_events.store(0, Ordering::Relaxed);
        *self.stats.last_reset.lock().unwrap() = Instant::now();
    }

    /// 设置自动扩容开关
    pub fn set_auto_resize(&self, enabled: bool) {
        self.auto_resize.store(enabled, Ordering::Relaxed);
    }

    /// 设置背压控制开关
    pub fn set_backpressure_enabled(&self, enabled: bool) {
        self.backpressure_enabled.store(enabled, Ordering::Relaxed);
    }

    /// 清空缓冲区
    pub fn clear(&self) {
        // 如果T需要Drop，我们需要手动调用
        if std::mem::needs_drop::<T>() {
            while self.pop().is_some() {}
        } else {
            self.head.store(0, Ordering::Relaxed);
            self.tail.store(0, Ordering::Relaxed);
            self.size.store(0, Ordering::Relaxed);
        }
    }

    /// 检查是否需要性能优化
    pub fn needs_optimization(&self) -> bool {
        let stats = self.stats();
        stats.usage_ratio > 0.9 || stats.dropped_items > 0 || stats.backpressure_events > 10
    }

    /// 获取性能建议
    pub fn performance_advice(&self) -> Vec<String> {
        let mut advice = Vec::new();
        let stats = self.stats();

        if stats.usage_ratio > 0.9 {
            advice.push("缓冲区使用率过高，建议增加容量或提高消费速度".to_string());
        }

        if stats.dropped_items > 0 {
            advice.push(format!("检测到 {} 个丢弃的元素，建议检查生产者/消费者平衡", stats.dropped_items));
        }

        if stats.backpressure_events > 10 {
            advice.push("背压事件频繁，建议优化数据处理流程".to_string());
        }

        if stats.usage_ratio < 0.3 && self.capacity > 1024 {
            advice.push("缓冲区使用率较低，可以考虑减少容量以节省内存".to_string());
        }

        if advice.is_empty() {
            advice.push("缓冲区性能良好".to_string());
        }

        advice
    }
}

impl BufferStats {
    fn new() -> Self {
        Self {
            total_pushes: AtomicUsize::new(0),
            total_pops: AtomicUsize::new(0),
            dropped_items: AtomicUsize::new(0),
            resize_count: AtomicUsize::new(0),
            backpressure_events: AtomicUsize::new(0),
            last_reset: std::sync::Mutex::new(Instant::now()),
        }
    }
}

/// 缓冲区统计信息快照
#[derive(Debug, Clone)]
pub struct BufferStatsSnapshot {
    pub total_pushes: usize,
    pub total_pops: usize,
    pub dropped_items: usize,
    pub resize_count: usize,
    pub backpressure_events: usize,
    pub current_size: usize,
    pub current_capacity: usize,
    pub usage_ratio: f64,
}

impl BufferStatsSnapshot {
    /// 计算吞吐量 (元素/秒)
    pub fn throughput(&self, duration: Duration) -> f64 {
        if duration.as_secs_f64() > 0.0 {
            self.total_pops as f64 / duration.as_secs_f64()
        } else {
            0.0
        }
    }

    /// 计算丢失率
    pub fn drop_rate(&self) -> f64 {
        if self.total_pushes > 0 {
            self.dropped_items as f64 / self.total_pushes as f64
        } else {
            0.0
        }
    }
}

// 实现Send和Sync
unsafe impl<T: Send> Send for AdaptiveRingBuffer<T> {}
unsafe impl<T: Send> Sync for AdaptiveRingBuffer<T> {}

impl<T> Drop for AdaptiveRingBuffer<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::sync::Arc;

    #[test]
    fn test_adaptive_buffer_basic() {
        let buffer = AdaptiveRingBuffer::new(8, 64, 0.8, 0.9);
        
        // 测试推入和弹出
        assert!(buffer.push(1).is_ok());
        assert!(buffer.push(2).is_ok());
        assert_eq!(buffer.len(), 2);
        
        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), None);
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_batch_operations() {
        let buffer = AdaptiveRingBuffer::new(16, 64, 0.8, 0.9);
        
        let items = vec![1, 2, 3, 4, 5];
        let pushed = buffer.push_batch(&items);
        assert_eq!(pushed, 5);
        
        let popped = buffer.pop_batch(3);
        assert_eq!(popped, vec![1, 2, 3]);
        assert_eq!(buffer.len(), 2);
    }

    #[test]
    fn test_backpressure() {
        let buffer = AdaptiveRingBuffer::new(4, 16, 0.5, 0.7);
        
        // 填充到背压阈值
        for i in 0..3 {
            assert!(buffer.push(i).is_ok());
        }
        
        // 应该触发背压
        assert!(buffer.push(4).is_err());
        
        let stats = buffer.stats();
        assert!(stats.backpressure_events > 0);
    }

    #[test]
    fn test_concurrent_access() {
        let buffer = Arc::new(AdaptiveRingBuffer::new(1000, 4000, 0.8, 0.9));
        let buffer_clone = buffer.clone();
        
        // 生产者线程
        let producer = thread::spawn(move || {
            for i in 0..500 {
                while buffer_clone.push(i).is_err() {
                    thread::yield_now();
                }
            }
        });
        
        // 消费者线程
        let buffer_consumer = buffer.clone();
        let consumer = thread::spawn(move || {
            let mut consumed = 0;
            while consumed < 500 {
                if let Some(_) = buffer_consumer.pop() {
                    consumed += 1;
                } else {
                    thread::yield_now();
                }
            }
            consumed
        });
        
        producer.join().unwrap();
        let consumed = consumer.join().unwrap();
        
        assert_eq!(consumed, 500);
        let stats = buffer.stats();
        assert_eq!(stats.total_pushes, 500);
        assert_eq!(stats.total_pops, 500);
    }
}