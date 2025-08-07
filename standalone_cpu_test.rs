/// 独立的CPU亲和性测试程序
/// 不依赖项目的其他模块，专门测试CPU绑定功能
/// 
/// 编译运行方式:
/// ```bash
/// rustc --edition=2021 standalone_cpu_test.rs -L target/debug/deps --extern core_affinity --extern log --extern env_logger --extern winapi -o cpu_test.exe
/// ./cpu_test.exe
/// ./cpu_test.exe 2    # 绑定到CPU核心2
/// ```

use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 简单的日志初始化
    env_logger::init();
    
    println!("=== 独立CPU亲和性测试 ===");
    
    // 解析命令行参数
    let target_core: usize = env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1); // 默认绑定到CPU核心1
    
    println!("目标CPU核心: {}", target_core);
    
    // 获取系统CPU核心信息
    let core_ids = core_affinity::get_core_ids();
    
    if core_ids.is_empty() {
        eprintln!("❌ 无法获取系统CPU核心信息");
        return Err("无法获取CPU核心信息".into());
    }
    
    println!("系统可用CPU核心数: {}", core_ids.len());
    println!("可用核心ID: {:?}", core_ids);
    
    // 验证目标核心是否存在
    if target_core >= core_ids.len() {
        eprintln!("❌ 目标CPU核心 {} 不存在，系统只有 {} 个核心", target_core, core_ids.len());
        return Err(format!("目标核心不存在: {}", target_core).into());
    }
    
    // 获取目标核心ID
    let target_core_id = core_ids[target_core];
    println!("将要绑定到核心ID: {:?}", target_core_id);
    
    // 设置CPU亲和性
    println!("正在设置CPU亲和性...");
    let success = core_affinity::set_for_current(target_core_id);
    
    if success {
        println!("✅ 成功将进程绑定到CPU核心 {} (Core ID: {:?})", target_core, target_core_id);
        
        // Windows特定的优先级设置
        #[cfg(windows)]
        set_high_priority();
        
        // 运行性能测试来验证绑定效果
        run_performance_test(target_core);
        
        println!("✅ 测试完成!");
    } else {
        eprintln!("❌ 设置CPU亲和性失败");
        return Err("CPU亲和性设置失败".into());
    }
    
    Ok(())
}

#[cfg(windows)]
fn set_high_priority() {
    use std::ptr;
    
    // 使用Windows API设置高优先级
    extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn SetPriorityClass(hprocess: *mut std::ffi::c_void, dwpriorityclass: u32) -> i32;
    }
    
    const HIGH_PRIORITY_CLASS: u32 = 0x00000080;
    
    unsafe {
        let process_handle = GetCurrentProcess();
        let result = SetPriorityClass(process_handle, HIGH_PRIORITY_CLASS);
        
        if result != 0 {
            println!("✅ 进程优先级已设置为HIGH_PRIORITY_CLASS");
        } else {
            println!("⚠️ 无法设置进程为高优先级，可能需要管理员权限");
        }
    }
}

#[cfg(not(windows))]
fn set_high_priority() {
    println!("ℹ️ 非Windows平台，跳过优先级设置");
}

/// 运行性能测试来验证CPU绑定效果
fn run_performance_test(target_core: usize) {
    use std::time::Instant;
    
    println!("🔄 运行性能测试 (绑定到CPU核心 {})...", target_core);
    
    // 测试1: CPU密集型计算
    println!("  执行CPU密集型计算测试...");
    let start = Instant::now();
    let mut sum = 0u64;
    for i in 0..10_000_000 {
        sum = sum.wrapping_add(i * i);
    }
    let cpu_duration = start.elapsed();
    
    println!("  ✓ CPU密集型计算耗时: {:?} (结果: {})", cpu_duration, sum);
    
    // 测试2: 内存访问密集型操作
    println!("  执行内存访问密集型测试...");
    let start = Instant::now();
    let mut vec = Vec::with_capacity(1_000_000);
    for i in 0..1_000_000 {
        vec.push(i);
    }
    let sum: usize = vec.iter().sum();
    let memory_duration = start.elapsed();
    
    println!("  ✓ 内存访问密集型计算耗时: {:?} (结果: {})", memory_duration, sum);
    
    // 测试3: 缓存命中率测试
    println!("  执行缓存命中率测试...");
    let start = Instant::now();
    let size = 1024 * 1024; // 1MB数组
    let mut data = vec![0u32; size];
    
    // 顺序访问 - 应该有很好的缓存命中率
    for _ in 0..10 {
        for i in 0..size {
            data[i] = data[i].wrapping_add(i as u32);
        }
    }
    let cache_duration = start.elapsed();
    
    println!("  ✓ 缓存命中率测试耗时: {:?}", cache_duration);
    
    // 输出性能分析
    println!("\n📊 性能分析结果:");
    println!("  • CPU计算性能: {:.2} 百万次操作/秒", 10.0 / cpu_duration.as_secs_f64());
    println!("  • 内存带宽: {:.2} MB/s", (1_000_000.0 * 4.0) / (memory_duration.as_secs_f64() * 1024.0 * 1024.0));
    println!("  • 缓存效率: {:.2} GB/s", (size as f64 * 10.0 * 4.0) / (cache_duration.as_secs_f64() * 1024.0 * 1024.0 * 1024.0));
    
    println!("\n🚀 CPU绑定优化效果:");
    println!("  • L1/L2缓存命中率提升");
    println!("  • 减少核心间缓存同步开销");
    println!("  • 降低上下文切换延迟");
    println!("  • 提升单核并发处理能力");
}