/// CPU亲和性测试示例
/// 
/// 演示如何使用CPU亲和性功能来优化程序性能
/// 运行方式:
/// 1. 默认绑定到CPU核心1: `cargo run --example test_cpu_affinity`
/// 2. 指定CPU核心: `cargo run --example test_cpu_affinity -- --cpu-core 0`

use binance_futures::{init_logging, init_cpu_affinity, check_affinity_status, get_cpu_manager};
use std::{env, thread, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    init_logging();
    
    println!("=== CPU亲和性测试示例 ===");
    println!("测试程序将演示CPU绑定功能");
    
    // 解析命令行参数
    let cpu_core = env::args()
        .position(|arg| arg == "--cpu-core")
        .and_then(|pos| env::args().nth(pos + 1))
        .and_then(|core_str| core_str.parse::<usize>().ok());
    
    println!("目标CPU核心: {}", cpu_core.unwrap_or(1));
    
    // 设置CPU亲和性
    match init_cpu_affinity(cpu_core) {
        Ok(()) => {
            println!("✅ CPU亲和性设置成功!");
            
            // 显示详细状态
            check_affinity_status();
            
            // 验证绑定是否有效
            if let Some(manager) = get_cpu_manager() {
                println!("\n🧪 开始CPU绑定验证测试...");
                
                // 运行一些计算密集型任务来测试绑定效果
                run_performance_test(manager.target_core());
                
            } else {
                println!("⚠️ 无法获取CPU管理器实例");
            }
        }
        Err(e) => {
            eprintln!("❌ CPU亲和性设置失败: {}", e);
            println!("\n可能的解决方案:");
            println!("1. 以管理员权限运行程序");
            println!("2. 检查目标CPU核心是否存在");
            println!("3. 检查系统是否支持CPU亲和性设置");
            return Err(e.into());
        }
    }
    
    println!("\n✅ 测试完成!");
    Ok(())
}

/// 运行性能测试来验证CPU绑定效果
fn run_performance_test(target_core: usize) {
    use std::time::Instant;
    
    println!("🔄 运行性能测试 (绑定到CPU核心 {})...", target_core);
    
    // 测试1: CPU密集型计算
    let start = Instant::now();
    let mut sum = 0u64;
    for i in 0..10_000_000 {
        sum = sum.wrapping_add(i * i);
    }
    let cpu_duration = start.elapsed();
    
    println!("  CPU密集型计算耗时: {:?} (结果: {})", cpu_duration, sum);
    
    // 测试2: 内存访问密集型操作
    let start = Instant::now();
    let mut vec = Vec::with_capacity(1_000_000);
    for i in 0..1_000_000 {
        vec.push(i);
    }
    let sum: usize = vec.iter().sum();
    let memory_duration = start.elapsed();
    
    println!("  内存访问密集型计算耗时: {:?} (结果: {})", memory_duration, sum);
    
    // 测试3: 短时间内多次上下文切换
    println!("  开始上下文切换测试...");
    let start = Instant::now();
    for _ in 0..1000 {
        thread::sleep(Duration::from_nanos(1000)); // 1微秒
    }
    let context_duration = start.elapsed();
    
    println!("  上下文切换测试耗时: {:?}", context_duration);
    
    // 显示当前CPU亲和性状态
    if let Some(manager) = get_cpu_manager() {
        println!("\n📊 测试期间CPU绑定状态:");
        if let Some(current_core) = manager.get_current_affinity() {
            println!("  当前运行核心: {:?}", current_core);
            println!("  目标核心: {}", manager.target_core());
            println!("  绑定状态: {}", if manager.is_bound() { "✅ 已绑定" } else { "❌ 未绑定" });
        }
    }
}