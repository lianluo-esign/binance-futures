/// 渲染队列实现

use std::collections::VecDeque;
use super::command::RenderCommand;
use super::super::component::{ComponentError, ComponentResult};

/// 渲染队列
/// 
/// 收集和管理渲染命令，支持批量处理和优化
pub struct RenderQueue {
    /// 渲染命令列表
    commands: VecDeque<RenderCommand>,
    /// 队列容量限制
    max_capacity: usize,
    /// 批处理大小
    batch_size: usize,
    /// 是否启用z-index排序
    enable_z_sorting: bool,
}

impl RenderQueue {
    /// 创建新的渲染队列
    pub fn new(max_capacity: usize, batch_size: usize) -> Self {
        Self {
            commands: VecDeque::with_capacity(max_capacity),
            max_capacity,
            batch_size,
            enable_z_sorting: true,
        }
    }
    
    /// 添加渲染命令
    pub fn push(&mut self, command: RenderCommand) -> ComponentResult<()> {
        if self.commands.len() >= self.max_capacity {
            return Err(ComponentError::ResourceError(
                "渲染队列已满".to_string()
            ));
        }
        
        self.commands.push_back(command);
        Ok(())
    }
    
    /// 批量添加渲染命令
    pub fn push_batch(&mut self, commands: Vec<RenderCommand>) -> ComponentResult<()> {
        if self.commands.len() + commands.len() > self.max_capacity {
            return Err(ComponentError::ResourceError(
                "渲染队列容量不足".to_string()
            ));
        }
        
        for command in commands {
            self.commands.push_back(command);
        }
        
        Ok(())
    }
    
    /// 获取下一批渲染命令
    pub fn pop_batch(&mut self) -> Vec<RenderCommand> {
        let batch_size = self.batch_size.min(self.commands.len());
        let mut batch = Vec::with_capacity(batch_size);
        
        for _ in 0..batch_size {
            if let Some(command) = self.commands.pop_front() {
                batch.push(command);
            }
        }
        
        // 如果启用z-index排序，对批次进行排序
        if self.enable_z_sorting {
            batch.sort_by_key(|cmd| cmd.z_index());
        }
        
        batch
    }
    
    /// 清空队列
    pub fn clear(&mut self) {
        self.commands.clear();
    }
    
    /// 获取队列长度
    pub fn len(&self) -> usize {
        self.commands.len()
    }
    
    /// 检查队列是否为空
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
    
    /// 设置是否启用z-index排序
    pub fn set_z_sorting(&mut self, enabled: bool) {
        self.enable_z_sorting = enabled;
    }
    
    /// 获取容量使用率
    pub fn usage_ratio(&self) -> f32 {
        self.commands.len() as f32 / self.max_capacity as f32
    }
    
    /// 预留容量
    pub fn reserve(&mut self, additional: usize) {
        self.commands.reserve(additional);
    }
}