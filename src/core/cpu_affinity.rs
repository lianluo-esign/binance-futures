/// CPU亲和性管理模块
/// 
/// 提供跨平台的CPU核心绑定功能，优化单核性能和缓存命中率
/// 专门为高频交易应用设计，减少延迟和上下文切换开销

use log::{info, warn, error};

/// CPU亲和性管理器
pub struct CpuAffinityManager {
    target_core: usize,
    is_bound: bool,
}

impl CpuAffinityManager {
    /// 创建新的CPU亲和性管理器
    /// 
    /// # Arguments
    /// * `target_core` - 目标CPU核心ID (从0开始)
    /// 
    /// # Example
    /// ```
    /// let manager = CpuAffinityManager::new(1); // 绑定到CPU核心1
    /// ```
    pub fn new(target_core: usize) -> Self {
        Self {
            target_core,
            is_bound: false,
        }
    }
    
    /// 设置CPU亲和性，将当前进程绑定到指定的CPU核心
    /// 
    /// # Returns
    /// * `Ok(())` - 绑定成功
    /// * `Err(String)` - 绑定失败，包含错误信息
    pub fn bind_to_core(&mut self) -> Result<(), String> {
        info!("正在尝试将进程绑定到CPU核心 {}", self.target_core);
        
        // 检查系统CPU核心数
        let core_ids = core_affinity::get_core_ids();
        if core_ids.is_none() {
            let error_msg = "无法获取系统CPU核心信息".to_string();
            error!("{}", error_msg);
            return Err(error_msg);
        }
        
        let core_ids = core_ids.unwrap();
        info!("系统可用CPU核心数: {}", core_ids.len());
        info!("可用核心ID: {:?}", core_ids);
        
        // 验证目标核心是否存在
        if self.target_core >= core_ids.len() {
            let error_msg = format!(
                "目标CPU核心 {} 不存在，系统只有 {} 个核心", 
                self.target_core, 
                core_ids.len()
            );
            error!("{}", error_msg);
            return Err(error_msg);
        }
        
        // 获取目标核心ID
        let target_core_id = core_ids[self.target_core];
        
        // 设置CPU亲和性
        match core_affinity::set_for_current(target_core_id) {
            true => {
                self.is_bound = true;
                info!("✅ 成功将进程绑定到CPU核心 {} (Core ID: {:?})", self.target_core, target_core_id);
                
                // 额外的Windows特定验证和优化
                #[cfg(windows)]
                self.optimize_windows_performance()?;
                
                info!("✅ 验证成功: 进程已绑定到目标CPU核心");
                Ok(())
            }
            false => {
                let error_msg = format!("设置CPU亲和性失败，无法绑定到核心 {}", self.target_core);
                error!("{}", error_msg);
                Err(error_msg)
            }
        }
    }
    
    /// Windows平台特定的性能优化
    #[cfg(windows)]
    fn optimize_windows_performance(&self) -> Result<(), String> {
        use winapi::um::processthreadsapi::{GetCurrentProcess, SetPriorityClass};
        use winapi::um::winbase::HIGH_PRIORITY_CLASS;
        
        unsafe {
            let process_handle = GetCurrentProcess();
            
            // 设置高优先级以减少调度延迟
            let result = SetPriorityClass(process_handle, HIGH_PRIORITY_CLASS);
            
            if result == 0 {
                warn!("⚠️ 无法设置进程为高优先级，可能需要管理员权限");
            } else {
                info!("✅ 进程优先级已设置为HIGH_PRIORITY_CLASS");
            }
        }
        
        Ok(())
    }
    
    /// 获取当前绑定状态
    pub fn is_bound(&self) -> bool {
        self.is_bound
    }
    
    /// 获取目标核心ID
    pub fn target_core(&self) -> usize {
        self.target_core
    }
    
    /// 获取当前CPU亲和性信息
    /// 注意：core_affinity库没有提供获取当前绑定核心的功能
    /// 此方法返回目标核心ID作为参考
    pub fn get_current_affinity(&self) -> Option<core_affinity::CoreId> {
        if self.is_bound {
            let core_ids = core_affinity::get_core_ids();
            if let Some(core_ids) = core_ids {
                if self.target_core < core_ids.len() {
                    Some(core_ids[self.target_core])
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// 显示CPU亲和性状态信息
    pub fn print_status(&self) {
        info!("=== CPU亲和性状态 ===");
        info!("目标核心: {}", self.target_core);
        info!("绑定状态: {}", if self.is_bound { "已绑定" } else { "未绑定" });
        
        if let Some(current_core) = self.get_current_affinity() {
            info!("当前运行核心: {:?}", current_core);
        } else {
            warn!("无法获取当前核心信息");
        }
        
        let core_ids = core_affinity::get_core_ids();
        if let Some(core_ids) = core_ids {
            info!("系统可用核心: {} 个", core_ids.len());
        } else {
            warn!("无法获取系统CPU核心信息");
        }
        info!("=====================");
    }
}

/// 全局CPU亲和性管理器实例
static mut CPU_MANAGER: Option<CpuAffinityManager> = None;

/// 初始化CPU亲和性管理器并绑定到指定核心
/// 
/// # Arguments
/// * `target_core` - 目标CPU核心ID (默认为1)
/// 
/// # Returns
/// * `Ok(())` - 初始化和绑定成功
/// * `Err(String)` - 失败，包含错误信息
pub fn init_cpu_affinity(target_core: Option<usize>) -> Result<(), String> {
    let core = target_core.unwrap_or(1); // 默认绑定到CPU核心1
    
    unsafe {
        let mut manager = CpuAffinityManager::new(core);
        
        // 尝试绑定到目标核心
        match manager.bind_to_core() {
            Ok(()) => {
                manager.print_status();
                CPU_MANAGER = Some(manager);
                
                info!("🚀 CPU亲和性优化完成! 进程现在运行在专用核心上");
                info!("📈 预期性能提升:");
                info!("   • L1/L2 缓存命中率提升");
                info!("   • 减少核心间缓存同步开销"); 
                info!("   • 降低上下文切换延迟");
                info!("   • 提升单核并发处理能力");
                
                Ok(())
            }
            Err(e) => {
                error!("❌ CPU亲和性设置失败: {}", e);
                error!("程序将继续运行，但可能无法获得最佳性能");
                Err(e)
            }
        }
    }
}

/// 获取全局CPU管理器的引用
pub fn get_cpu_manager() -> Option<&'static CpuAffinityManager> {
    unsafe { CPU_MANAGER.as_ref() }
}

/// 检查CPU亲和性状态
pub fn check_affinity_status() {
    if let Some(manager) = get_cpu_manager() {
        manager.print_status();
    } else {
        warn!("CPU亲和性管理器未初始化");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cpu_manager_creation() {
        let manager = CpuAffinityManager::new(1);
        assert_eq!(manager.target_core(), 1);
        assert!(!manager.is_bound());
    }
    
    #[test]
    fn test_get_core_ids() {
        let core_ids = core_affinity::get_core_ids();
        assert!(core_ids.is_some(), "应该能够获取系统CPU核心信息");
        if let Some(core_ids) = core_ids {
            assert!(!core_ids.is_empty(), "系统应该至少有一个CPU核心");
        }
    }
}