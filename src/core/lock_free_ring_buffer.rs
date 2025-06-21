use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::mem::MaybeUninit;
use std::cell::UnsafeCell;

/// 高性能无锁环形缓冲区
/// 
/// 这个实现使用原子操作来实现无锁的单生产者多消费者（SPMC）或多生产者单消费者（MPSC）模式
/// 专为高频事件处理设计，避免了互斥锁的开销
pub struct LockFreeRingBuffer<T> {
    /// 缓冲区数据存储 - 使用UnsafeCell提供内部可变性
    buffer: UnsafeCell<Vec<MaybeUninit<T>>>,
    /// 缓冲区容量（必须是2的幂）
    capacity: usize,
    /// 容量掩码，用于快速取模运算
    mask: usize,
    /// 写入位置（生产者索引）
    write_pos: AtomicUsize,
    /// 读取位置（消费者索引）
    read_pos: AtomicUsize,
    /// 已提交的写入位置，用于确保数据完整性
    committed_write_pos: AtomicUsize,
}

impl<T> LockFreeRingBuffer<T> {
    /// 创建新的无锁环形缓冲区
    /// 
    /// # 参数
    /// * `capacity` - 缓冲区容量，会被向上舍入到最近的2的幂
    pub fn new(capacity: usize) -> Self {
        // 确保容量是2的幂，以便使用位运算优化
        let capacity = capacity.next_power_of_two().max(2);
        let mask = capacity - 1;
        
        // 初始化缓冲区，使用MaybeUninit避免不必要的初始化
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(MaybeUninit::uninit());
        }
        
        Self {
            buffer: UnsafeCell::new(buffer),
            capacity,
            mask,
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
            committed_write_pos: AtomicUsize::new(0),
        }
    }
    
    /// 尝试推入一个元素（非阻塞）
    /// 
    /// # 返回值
    /// * `Ok(())` - 成功推入
    /// * `Err(T)` - 缓冲区已满，返回原始元素
    pub fn try_push(&self, item: T) -> Result<(), T> {
        let current_write = self.write_pos.load(Ordering::Relaxed);
        let current_read = self.read_pos.load(Ordering::Acquire);
        
        // 检查缓冲区是否已满
        // 我们保留一个空槽位来区分满和空的状态
        if (current_write + 1) & self.mask == current_read & self.mask {
            return Err(item); // 缓冲区已满
        }
        
        // 尝试获取写入位置
        let next_write = (current_write + 1) & self.mask;
        match self.write_pos.compare_exchange_weak(
            current_write,
            next_write,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                // 成功获取写入位置，现在写入数据
                unsafe {
                    let buffer = &mut *self.buffer.get();
                    let slot = &mut buffer[current_write & self.mask];
                    slot.write(item);
                }
                
                // 提交写入，确保数据对读取者可见
                // 使用Release语义确保写入操作在提交之前完成
                self.committed_write_pos.store(next_write, Ordering::Release);
                
                Ok(())
            }
            Err(_) => {
                // 写入位置被其他线程抢占，重试
                Err(item)
            }
        }
    }
    
    /// 尝试弹出一个元素（非阻塞）
    /// 
    /// # 返回值
    /// * `Some(T)` - 成功弹出元素
    /// * `None` - 缓冲区为空
    pub fn try_pop(&self) -> Option<T> {
        let current_read = self.read_pos.load(Ordering::Relaxed);
        let committed_write = self.committed_write_pos.load(Ordering::Acquire);
        
        // 检查是否有可读取的数据
        if current_read == committed_write {
            return None; // 缓冲区为空
        }
        
        // 尝试获取读取位置
        let next_read = (current_read + 1) & self.mask;
        match self.read_pos.compare_exchange_weak(
            current_read,
            next_read,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                // 成功获取读取位置，现在读取数据
                unsafe {
                    let buffer = &*self.buffer.get();
                    let slot = &buffer[current_read & self.mask];
                    Some(slot.assume_init_read())
                }
            }
            Err(_) => {
                // 读取位置被其他线程抢占，重试
                None
            }
        }
    }
    
    /// 获取当前缓冲区中的元素数量
    pub fn len(&self) -> usize {
        let write_pos = self.committed_write_pos.load(Ordering::Acquire);
        let read_pos = self.read_pos.load(Ordering::Acquire);
        
        // 处理环形缓冲区的回绕
        if write_pos >= read_pos {
            write_pos - read_pos
        } else {
            self.capacity - (read_pos - write_pos)
        }
    }
    
    /// 检查缓冲区是否为空
    pub fn is_empty(&self) -> bool {
        let write_pos = self.committed_write_pos.load(Ordering::Acquire);
        let read_pos = self.read_pos.load(Ordering::Acquire);
        write_pos == read_pos
    }
    
    /// 检查缓冲区是否已满
    pub fn is_full(&self) -> bool {
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let read_pos = self.read_pos.load(Ordering::Acquire);
        (write_pos + 1) & self.mask == read_pos & self.mask
    }
    
    /// 获取缓冲区容量
    pub fn capacity(&self) -> usize {
        self.capacity - 1 // 减1因为我们保留一个空槽位
    }
    
    /// 清空缓冲区
    /// 
    /// 注意：这个操作不是原子的，应该在确保没有并发访问时调用
    pub fn clear(&self) {
        // 先读取所有剩余元素以确保正确释放内存
        while self.try_pop().is_some() {}
        
        // 重置位置
        self.write_pos.store(0, Ordering::Relaxed);
        self.read_pos.store(0, Ordering::Relaxed);
        self.committed_write_pos.store(0, Ordering::Relaxed);
    }
}

// 实现Send和Sync，使缓冲区可以在多线程环境中使用
unsafe impl<T: Send> Send for LockFreeRingBuffer<T> {}
unsafe impl<T: Send> Sync for LockFreeRingBuffer<T> {}

impl<T> Drop for LockFreeRingBuffer<T> {
    fn drop(&mut self) {
        // 清理所有剩余的元素
        while self.try_pop().is_some() {}
    }
}

/// 无锁环形缓冲区的共享引用版本
/// 
/// 这个包装器提供了Arc<LockFreeRingBuffer<T>>的便利接口
pub type SharedLockFreeRingBuffer<T> = Arc<LockFreeRingBuffer<T>>;

/// 创建共享的无锁环形缓冲区
pub fn create_shared_lock_free_ring_buffer<T>(capacity: usize) -> SharedLockFreeRingBuffer<T> {
    Arc::new(LockFreeRingBuffer::new(capacity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_basic_operations() {
        let buffer = LockFreeRingBuffer::new(4);
        
        // 测试推入
        assert!(buffer.try_push(1).is_ok());
        assert!(buffer.try_push(2).is_ok());
        assert!(buffer.try_push(3).is_ok());
        
        // 测试长度
        assert_eq!(buffer.len(), 3);
        
        // 测试弹出
        assert_eq!(buffer.try_pop(), Some(1));
        assert_eq!(buffer.try_pop(), Some(2));
        assert_eq!(buffer.try_pop(), Some(3));
        assert_eq!(buffer.try_pop(), None);
        
        // 测试空状态
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_full_buffer() {
        let buffer = LockFreeRingBuffer::new(4);
        
        // 填满缓冲区（容量-1）
        assert!(buffer.try_push(1).is_ok());
        assert!(buffer.try_push(2).is_ok());
        assert!(buffer.try_push(3).is_ok());
        
        // 下一个推入应该失败
        assert!(buffer.try_push(4).is_err());
        assert!(buffer.is_full());
    }

    #[test]
    fn test_concurrent_access() {
        let buffer = Arc::new(LockFreeRingBuffer::new(1000));
        let buffer_clone = buffer.clone();
        
        // 生产者线程
        let producer = thread::spawn(move || {
            for i in 0..500 {
                while buffer_clone.try_push(i).is_err() {
                    thread::yield_now();
                }
            }
        });
        
        // 消费者线程
        let consumer = thread::spawn(move || {
            let mut received = Vec::new();
            while received.len() < 500 {
                if let Some(item) = buffer.try_pop() {
                    received.push(item);
                } else {
                    thread::yield_now();
                }
            }
            received
        });
        
        producer.join().unwrap();
        let received = consumer.join().unwrap();
        
        // 验证所有数据都被正确接收
        assert_eq!(received.len(), 500);
        for i in 0..500 {
            assert!(received.contains(&i));
        }
    }
}
