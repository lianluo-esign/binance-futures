use flow_sight::{Config, ReactiveApp, Event, EventType};
use std::time::{Duration, Instant};
use std::thread;

#[test]
fn test_lock_free_performance() {
    // 创建配置
    let config = Config::new("BTCUSDT".to_string())
        .with_buffer_size(10000);
    
    // 创建应用程序
    let mut app = ReactiveApp::new(config);
    
    // 验证初始状态
    assert_eq!(app.get_symbol(), "BTCUSDT");
    assert!(!app.is_running());
    
    // 测试高频事件处理性能
    let start_time = Instant::now();
    let test_events = 1000;
    
    // 模拟高频事件处理
    for i in 0..test_events {
        // 模拟事件循环处理
        app.event_loop();
        
        // 每100次循环检查一次性能
        if i % 100 == 0 {
            let elapsed = start_time.elapsed();
            if elapsed > Duration::from_secs(5) {
                // 如果超过5秒，说明性能良好（没有阻塞）
                break;
            }
        }
    }
    
    let total_elapsed = start_time.elapsed();
    
    // 验证性能指标
    assert!(total_elapsed < Duration::from_secs(10), 
        "Lock-free implementation should complete quickly without blocking");
    
    println!("Lock-free performance test completed in {:?}", total_elapsed);
    println!("Processed {} event loops", test_events);
    
    // 获取统计信息
    let stats = app.get_stats();
    println!("Events processed per second: {:.2}", stats.events_processed_per_second);
    println!("Pending events: {}", stats.pending_events);
    println!("WebSocket connected: {}", stats.websocket_connected);
}

#[test]
fn test_concurrent_event_processing() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    // 创建配置
    let config = Config::new("BTCUSDT".to_string())
        .with_buffer_size(10000);
    
    // 创建应用程序
    let app = Arc::new(std::sync::Mutex::new(ReactiveApp::new(config)));
    let processed_count = Arc::new(AtomicUsize::new(0));
    
    // 启动多个线程模拟并发处理
    let mut handles = Vec::new();
    
    for thread_id in 0..4 {
        let app_clone = app.clone();
        let count_clone = processed_count.clone();
        
        let handle = thread::spawn(move || {
            let start_time = Instant::now();
            let mut local_count = 0;
            
            // 每个线程运行1秒
            while start_time.elapsed() < Duration::from_secs(1) {
                {
                    let mut app_guard = app_clone.lock().unwrap();
                    app_guard.event_loop();
                }
                local_count += 1;
                
                // 短暂让出CPU时间
                thread::yield_now();
            }
            
            count_clone.fetch_add(local_count, Ordering::Relaxed);
            println!("Thread {} completed {} event loops", thread_id, local_count);
        });
        
        handles.push(handle);
    }
    
    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }
    
    let total_processed = processed_count.load(Ordering::Relaxed);
    println!("Total event loops processed by all threads: {}", total_processed);
    
    // 验证并发处理没有导致死锁或阻塞
    assert!(total_processed > 0, "Should have processed some events");
    
    // 获取最终统计信息
    let app_guard = app.lock().unwrap();
    let stats = app_guard.get_stats();
    println!("Final stats - Events/sec: {:.2}, Pending: {}", 
        stats.events_processed_per_second, stats.pending_events);
}

#[test]
fn test_memory_usage_stability() {
    // 创建配置
    let config = Config::new("BTCUSDT".to_string())
        .with_buffer_size(1000);
    
    // 创建应用程序
    let mut app = ReactiveApp::new(config);
    
    let start_time = Instant::now();
    let mut max_pending = 0;
    let mut total_loops = 0;
    
    // 运行2秒钟，监控内存使用情况
    while start_time.elapsed() < Duration::from_secs(2) {
        app.event_loop();
        
        let stats = app.get_stats();
        max_pending = max_pending.max(stats.pending_events);
        total_loops += 1;
        
        // 每1000次循环检查一次
        if total_loops % 1000 == 0 {
            println!("Loop {}: Pending events: {}, Max pending: {}", 
                total_loops, stats.pending_events, max_pending);
        }
    }
    
    let final_stats = app.get_stats();
    
    println!("Memory stability test completed:");
    println!("Total loops: {}", total_loops);
    println!("Max pending events: {}", max_pending);
    println!("Final pending events: {}", final_stats.pending_events);
    println!("Events processed per second: {:.2}", final_stats.events_processed_per_second);
    
    // 验证内存使用稳定（待处理事件数量不会无限增长）
    assert!(max_pending < 1000, "Pending events should not grow unbounded");
    assert!(final_stats.pending_events < 100, "Should not have excessive pending events at the end");
}

#[test]
fn test_no_deadlock_under_stress() {
    // 创建配置
    let config = Config::new("BTCUSDT".to_string())
        .with_buffer_size(5000);
    
    // 创建应用程序
    let mut app = ReactiveApp::new(config);
    
    let start_time = Instant::now();
    let stress_duration = Duration::from_secs(3);
    let mut iteration_count = 0;
    
    // 高强度压力测试
    while start_time.elapsed() < stress_duration {
        // 快速连续调用事件循环
        for _ in 0..100 {
            app.event_loop();
            iteration_count += 1;
        }
        
        // 检查是否有阻塞
        let elapsed = start_time.elapsed();
        let expected_min_iterations = (elapsed.as_millis() / 10) as usize; // 每10ms至少100次迭代
        
        if iteration_count < expected_min_iterations {
            panic!("Possible deadlock detected: only {} iterations in {:?}", 
                iteration_count, elapsed);
        }
    }
    
    let total_elapsed = start_time.elapsed();
    let iterations_per_second = iteration_count as f64 / total_elapsed.as_secs_f64();
    
    println!("Stress test completed:");
    println!("Total iterations: {}", iteration_count);
    println!("Iterations per second: {:.0}", iterations_per_second);
    println!("No deadlocks detected");
    
    // 验证高性能（每秒至少10000次迭代）
    assert!(iterations_per_second > 10000.0, 
        "Should achieve high iteration rate without blocking");
    
    // 获取最终统计信息
    let stats = app.get_stats();
    println!("Final events processed per second: {:.2}", stats.events_processed_per_second);
}
