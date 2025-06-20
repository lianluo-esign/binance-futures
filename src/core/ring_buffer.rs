use std::mem::MaybeUninit;

/// 高性能循环缓冲区
/// 使用repr(C)确保内存布局可预测，align(64)对齐到缓存行大小
#[repr(C, align(64))]
#[derive(Debug)]
pub struct RingBuffer<T> {
    // 使用裸指针数组避免Option<T>的开销
    buffer: Box<[MaybeUninit<T>]>,
    // 确保容量是2的幂，用于位掩码优化
    capacity: usize,
    // 位掩码，用于替代模运算
    mask: usize,
    // 读取位置
    head: usize,
    // 写入位置
    tail: usize,
    // 当前元素数量
    size: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        // 确保容量是2的幂
        let capacity = capacity.next_power_of_two();
        let mask = capacity - 1;
        
        // 创建未初始化的缓冲区
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(MaybeUninit::uninit());
        }
        
        Self {
            buffer: buffer.into_boxed_slice(),
            capacity,
            mask,
            head: 0,
            tail: 0,
            size: 0,
        }
    }

    #[inline(always)]
    pub fn push(&mut self, item: T) -> bool {
        if self.size == self.capacity {
            // 缓冲区满，覆盖最旧的数据
            self.head = (self.head + 1) & self.mask; // 使用位与代替模运算
        } else {
            self.size += 1;
        }
        
        // 使用ptr::write避免移动和复制
        unsafe {
            self.buffer[self.tail].as_mut_ptr().write(item);
        }
        
        // 预取下一个可能访问的缓存行
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let next_idx = (self.tail + 1) & self.mask;
            let ptr = self.buffer.as_ptr().add(next_idx);
            std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0);
        }
        
        self.tail = (self.tail + 1) & self.mask; // 使用位与代替模运算
        true
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.size == 0 {
            return None;
        }
        
        // 安全地取出元素
        let item = unsafe {
            let item_ptr = self.buffer[self.head].as_ptr();
            let item = std::ptr::read(item_ptr);
            item
        };
        
        // 预取下一个可能访问的缓存行
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let next_idx = (self.head + 1) & self.mask;
            let ptr = self.buffer.as_ptr().add(next_idx);
            std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0);
        }
        
        self.head = (self.head + 1) & self.mask;
        self.size -= 1;
        Some(item)
    }
    
    // 批量操作：一次性推入多个元素
    #[inline]
    pub fn push_batch(&mut self, items: &[T]) -> usize
    where
        T: Clone,
    {
        let mut count = 0;
        for item in items {
            if self.push(item.clone()) {
                count += 1;
            } else {
                break;
            }
        }
        count
    }
    
    // 批量操作：一次性弹出多个元素
    #[inline]
    pub fn pop_batch(&mut self, max_items: usize) -> Vec<T> {
        let mut result = Vec::with_capacity(max_items.min(self.size));
        for _ in 0..max_items {
            if let Some(item) = self.pop() {
                result.push(item);
            } else {
                break;
            }
        }
        result
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.size
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.size == self.capacity
    }
    
    // 清空缓冲区但不释放内存
    pub fn clear(&mut self) {
        // 如果T需要Drop，我们需要手动调用它
        if std::mem::needs_drop::<T>() {
            while let Some(_) = self.pop() {}
        } else {
            self.head = 0;
            self.tail = 0;
            self.size = 0;
        }
    }
}

// 为RingBuffer实现Drop，确保所有元素都被正确释放
impl<T> Drop for RingBuffer<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

// 为RingBuffer实现Clone，如果T实现了Clone
impl<T: Clone> Clone for RingBuffer<T> {
    fn clone(&self) -> Self {
        let mut new_buffer = Self::new(self.capacity);
        
        // 复制所有元素
        if self.size > 0 {
            let mut idx = self.head;
            for _ in 0..self.size {
                unsafe {
                    let item = std::ptr::read(self.buffer[idx].as_ptr()).clone();
                    new_buffer.push(item);
                }
                idx = (idx + 1) & self.mask;
            }
        }
        
        new_buffer
    }
}

// 实现Send和Sync，使RingBuffer可以在多线程环境中使用
unsafe impl<T: Send> Send for RingBuffer<T> {}
unsafe impl<T: Sync> Sync for RingBuffer<T> {}
