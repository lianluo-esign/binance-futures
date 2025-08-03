/// 渲染管理器实现

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::command::{RenderCommand, RenderTarget};
use super::queue::RenderQueue;
use super::pipeline::{RenderPipeline, RenderPipelineConfig, RenderStats};
use super::super::component::{ComponentError, ComponentResult};

/// 渲染管理器配置
#[derive(Debug, Clone)]
pub struct RenderManagerConfig {
    /// 队列最大容量
    pub queue_max_capacity: usize,
    /// 批处理大小
    pub batch_size: usize,
    /// 渲染线程数
    pub render_threads: u32,
    /// 启用性能监控
    pub enable_profiling: bool,
    /// 渲染管道配置
    pub pipeline_config: RenderPipelineConfig,
}

impl Default for RenderManagerConfig {
    fn default() -> Self {
        Self {
            queue_max_capacity: 10000,
            batch_size: 100,
            render_threads: 1,
            enable_profiling: true,
            pipeline_config: RenderPipelineConfig::default(),
        }
    }
}

/// 渲染管理器
/// 
/// 统一管理所有渲染相关的资源和操作
pub struct RenderManager {
    /// 渲染队列 (按目标分组)
    render_queues: Arc<RwLock<HashMap<RenderTarget, RenderQueue>>>,
    /// 渲染管道
    pipeline: Arc<RwLock<RenderPipeline>>,
    /// 渲染器配置
    config: RenderManagerConfig,
    /// 运行状态
    is_running: Arc<RwLock<bool>>,
}

impl RenderManager {
    /// 创建新的渲染管理器
    pub fn new(config: RenderManagerConfig) -> Self {
        let pipeline = RenderPipeline::new(config.pipeline_config.clone());
        
        Self {
            render_queues: Arc::new(RwLock::new(HashMap::new())),
            pipeline: Arc::new(RwLock::new(pipeline)),
            config,
            is_running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// 启动渲染管理器
    pub async fn start(&self) -> ComponentResult<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(ComponentError::StateError("渲染管理器已在运行".to_string()));
        }
        
        *is_running = true;
        log::info!("渲染管理器启动成功");
        Ok(())
    }
    
    /// 停止渲染管理器
    pub async fn stop(&self) -> ComponentResult<()> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        // 清空所有队列
        let mut queues = self.render_queues.write().await;
        for (_, queue) in queues.iter_mut() {
            queue.clear();
        }
        
        log::info!("渲染管理器停止");
        Ok(())
    }
    
    /// 提交渲染命令
    pub async fn submit_command(&self, command: RenderCommand) -> ComponentResult<()> {
        let target = command.target().clone();
        let mut queues = self.render_queues.write().await;
        
        // 获取或创建目标队列
        let queue = queues.entry(target.clone()).or_insert_with(|| {
            RenderQueue::new(self.config.queue_max_capacity, self.config.batch_size)
        });
        
        queue.push(command)?;
        Ok(())
    }
    
    /// 批量提交渲染命令
    pub async fn submit_commands(&self, commands: Vec<RenderCommand>) -> ComponentResult<()> {
        let mut queues = self.render_queues.write().await;
        
        // 按目标分组命令
        let mut grouped_commands: HashMap<RenderTarget, Vec<RenderCommand>> = HashMap::new();
        for command in commands {
            let target = command.target().clone();
            grouped_commands.entry(target).or_insert_with(Vec::new).push(command);
        }
        
        // 提交到相应队列
        for (target, target_commands) in grouped_commands {
            let queue = queues.entry(target).or_insert_with(|| {
                RenderQueue::new(self.config.queue_max_capacity, self.config.batch_size)
            });
            
            queue.push_batch(target_commands)?;
        }
        
        Ok(())
    }
    
    /// 处理渲染队列
    pub async fn process_queues(&self, ctx: &eframe::egui::Context) -> ComponentResult<()> {
        let mut queues = self.render_queues.write().await;
        let mut pipeline = self.pipeline.write().await;
        
        for (target, queue) in queues.iter_mut() {
            while !queue.is_empty() {
                let batch = queue.pop_batch();
                if !batch.is_empty() {
                    pipeline.execute_batch(&batch, ctx)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// 获取渲染统计
    pub async fn get_stats(&self) -> RenderStats {
        let pipeline = self.pipeline.read().await;
        pipeline.get_stats().clone()
    }
    
    /// 重置渲染统计
    pub async fn reset_stats(&self) {
        let mut pipeline = self.pipeline.write().await;
        pipeline.reset_stats();
    }
    
    /// 获取队列状态信息
    pub async fn get_queue_info(&self) -> HashMap<String, QueueInfo> {
        let queues = self.render_queues.read().await;
        let mut info = HashMap::new();
        
        for (target, queue) in queues.iter() {
            let target_name = match target {
                RenderTarget::MainWindow => "主窗口".to_string(),
                RenderTarget::ChildWindow(name) => format!("子窗口: {}", name),
                RenderTarget::OffscreenBuffer(name) => format!("离屏缓冲: {}", name),
                RenderTarget::Texture(name) => format!("纹理: {}", name),
            };
            
            info.insert(target_name, QueueInfo {
                length: queue.len(),
                usage_ratio: queue.usage_ratio(),
            });
        }
        
        info
    }
    
    /// 清空所有队列
    pub async fn clear_all_queues(&self) {
        let mut queues = self.render_queues.write().await;
        for (_, queue) in queues.iter_mut() {
            queue.clear();
        }
        log::debug!("所有渲染队列已清空");
    }
    
    /// 预留队列容量
    pub async fn reserve_queue_capacity(&self, target: RenderTarget, additional: usize) {
        let mut queues = self.render_queues.write().await;
        if let Some(queue) = queues.get_mut(&target) {
            queue.reserve(additional);
        }
    }
}

/// 队列信息
#[derive(Debug, Clone)]
pub struct QueueInfo {
    /// 队列长度
    pub length: usize,
    /// 使用率
    pub usage_ratio: f32,
}