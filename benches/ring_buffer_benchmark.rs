use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use flow_sight::core::{LockFreeRingBuffer, CacheOptimizedRingBuffer};
use std::sync::Arc;
use std::thread;

fn benchmark_single_thread_push_pop(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_thread_push_pop");
    group.throughput(Throughput::Elements(1000));
    
    group.bench_function("LockFreeRingBuffer", |b| {
        b.iter(|| {
            let buffer = LockFreeRingBuffer::new(1024);
            for i in 0..1000 {
                let _ = buffer.try_push(black_box(i));
            }
            for _ in 0..1000 {
                let _ = buffer.try_pop();
            }
        });
    });
    
    group.bench_function("CacheOptimizedRingBuffer", |b| {
        b.iter(|| {
            let buffer = CacheOptimizedRingBuffer::new(1024);
            for i in 0..1000 {
                let _ = buffer.try_push(black_box(i));
            }
            for _ in 0..1000 {
                let _ = buffer.try_pop();
            }
        });
    });
    
    group.finish();
}

fn benchmark_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_access");
    group.throughput(Throughput::Elements(10000));
    
    group.bench_function("LockFreeRingBuffer", |b| {
        b.iter(|| {
            let buffer = Arc::new(LockFreeRingBuffer::new(1024));
            let buffer_clone = buffer.clone();
            
            let producer = thread::spawn(move || {
                for i in 0..5000 {
                    while buffer_clone.try_push(black_box(i)).is_err() {
                        thread::yield_now();
                    }
                }
            });
            
            let consumer = thread::spawn(move || {
                let mut count = 0;
                while count < 5000 {
                    if buffer.try_pop().is_some() {
                        count += 1;
                    } else {
                        thread::yield_now();
                    }
                }
            });
            
            producer.join().unwrap();
            consumer.join().unwrap();
        });
    });
    
    group.bench_function("CacheOptimizedRingBuffer", |b| {
        b.iter(|| {
            let buffer = Arc::new(CacheOptimizedRingBuffer::new(1024));
            let buffer_clone = buffer.clone();
            
            let producer = thread::spawn(move || {
                for i in 0..5000 {
                    while buffer_clone.try_push(black_box(i)).is_err() {
                        thread::yield_now();
                    }
                }
            });
            
            let consumer = thread::spawn(move || {
                let mut count = 0;
                while count < 5000 {
                    if buffer.try_pop().is_some() {
                        count += 1;
                    } else {
                        thread::yield_now();
                    }
                }
            });
            
            producer.join().unwrap();
            consumer.join().unwrap();
        });
    });
    
    group.finish();
}

fn benchmark_high_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("high_contention");
    group.throughput(Throughput::Elements(20000));
    
    group.bench_function("LockFreeRingBuffer", |b| {
        b.iter(|| {
            let buffer = Arc::new(LockFreeRingBuffer::new(512)); // Smaller buffer for more contention
            
            let mut handles = Vec::new();
            
            // 4 producers
            for thread_id in 0..4 {
                let buffer_clone = buffer.clone();
                handles.push(thread::spawn(move || {
                    for i in 0..1250 { // 4 * 1250 = 5000 total
                        let value = thread_id * 10000 + i;
                        while buffer_clone.try_push(black_box(value)).is_err() {
                            thread::yield_now();
                        }
                    }
                }));
            }
            
            // 2 consumers
            for _ in 0..2 {
                let buffer_clone = buffer.clone();
                handles.push(thread::spawn(move || {
                    let mut count = 0;
                    while count < 2500 { // 2 * 2500 = 5000 total
                        if buffer_clone.try_pop().is_some() {
                            count += 1;
                        } else {
                            thread::yield_now();
                        }
                    }
                }));
            }
            
            for handle in handles {
                handle.join().unwrap();
            }
        });
    });
    
    group.bench_function("CacheOptimizedRingBuffer", |b| {
        b.iter(|| {
            let buffer = Arc::new(CacheOptimizedRingBuffer::new(512)); // Smaller buffer for more contention
            
            let mut handles = Vec::new();
            
            // 4 producers
            for thread_id in 0..4 {
                let buffer_clone = buffer.clone();
                handles.push(thread::spawn(move || {
                    for i in 0..1250 { // 4 * 1250 = 5000 total
                        let value = thread_id * 10000 + i;
                        while buffer_clone.try_push(black_box(value)).is_err() {
                            thread::yield_now();
                        }
                    }
                }));
            }
            
            // 2 consumers
            for _ in 0..2 {
                let buffer_clone = buffer.clone();
                handles.push(thread::spawn(move || {
                    let mut count = 0;
                    while count < 2500 { // 2 * 2500 = 5000 total
                        if buffer_clone.try_pop().is_some() {
                            count += 1;
                        } else {
                            thread::yield_now();
                        }
                    }
                }));
            }
            
            for handle in handles {
                handle.join().unwrap();
            }
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_single_thread_push_pop,
    benchmark_concurrent_access,
    benchmark_high_contention
);
criterion_main!(benches);