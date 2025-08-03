/// 状态管理器实现

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use super::types::{StateChangeEvent, StateChangeType, ComponentStateSnapshot};
use super::persistence::StatePersistence;
use super::super::component::{ComponentId, ComponentError, ComponentResult};

/// 状态管理器配置
#[derive(Debug, Clone)]
pub struct StateManagerConfig {
    /// 自动快照间隔
    pub auto_snapshot_interval: Option<std::time::Duration>,
    /// 事件通道缓冲区大小
    pub event_channel_buffer: usize,
    /// 启用状态验证
    pub enable_validation: bool,
    /// 最大状态历史记录数
    pub max_history_size: usize,
}

impl Default for StateManagerConfig {
    fn default() -> Self {
        Self {
            auto_snapshot_interval: Some(std::time::Duration::from_secs(300)), // 5分钟自动快照
            event_channel_buffer: 1000,
            enable_validation: true,
            max_history_size: 100,
        }
    }
}

/// 状态管理器
/// 
/// 统一管理组件状态的创建、更新、持久化和同步
pub struct StateManager {
    /// 当前状态存储
    current_states: Arc<RwLock<HashMap<ComponentId, serde_json::Value>>>,
    /// 状态版本计数器
    version_counters: Arc<RwLock<HashMap<ComponentId, u64>>>,
    /// 状态变更事件发布器
    change_event_sender: broadcast::Sender<StateChangeEvent>,
    /// 状态持久化实现
    persistence: Arc<dyn StatePersistence>,
    /// 管理器配置
    config: StateManagerConfig,
    /// 状态变更历史
    change_history: Arc<RwLock<Vec<StateChangeEvent>>>,
}

impl StateManager {
    /// 创建新的状态管理器
    pub fn new(persistence: Arc<dyn StatePersistence>, config: StateManagerConfig) -> Self {
        let (change_event_sender, _) = broadcast::channel(config.event_channel_buffer);
        
        Self {
            current_states: Arc::new(RwLock::new(HashMap::new())),
            version_counters: Arc::new(RwLock::new(HashMap::new())),
            change_event_sender,
            persistence,
            config,
            change_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// 设置组件状态
    pub async fn set_state(
        &self,
        component_id: ComponentId,
        state: serde_json::Value,
        reason: String,
    ) -> ComponentResult<()> {
        // 验证状态数据
        if self.config.enable_validation {
            self.validate_state(&state).await?;
        }
        
        let old_state = {
            let mut current_states = self.current_states.write().await;
            current_states.insert(component_id.clone(), state.clone())
        };
        
        // 增加版本计数器
        let version = {
            let mut version_counters = self.version_counters.write().await;
            let counter = version_counters.entry(component_id.clone()).or_insert(0);
            *counter += 1;
            *counter
        };
        
        // 创建状态变更事件
        let change_event = StateChangeEvent {
            component_id: component_id.clone(),
            change_type: if old_state.is_some() { StateChangeType::Updated } else { StateChangeType::Created },
            old_state,
            new_state: state.clone(),
            timestamp: std::time::SystemTime::now(),
            reason,
        };
        
        // 添加到历史记录
        self.add_to_history(change_event.clone()).await;
        
        // 发布状态变更事件
        if let Err(_) = self.change_event_sender.send(change_event) {
            log::warn!("发送状态变更事件失败 - 没有活跃的监听器");
        }
        
        // 创建快照
        let snapshot = ComponentStateSnapshot::new(component_id, state, version);
        self.persistence.save_snapshot(&snapshot).await?;
        
        Ok(())
    }
    
    /// 获取组件状态
    pub async fn get_state(&self, component_id: &ComponentId) -> Option<serde_json::Value> {
        let current_states = self.current_states.read().await;
        current_states.get(component_id).cloned()
    }
    
    /// 批量设置状态
    pub async fn set_states(
        &self,
        states: HashMap<ComponentId, serde_json::Value>,
        reason: String,
    ) -> ComponentResult<()> {
        for (component_id, state) in states {
            self.set_state(component_id, state, reason.clone()).await?;
        }
        
        // 创建批量更新事件
        let batch_event = StateChangeEvent {
            component_id: ComponentId::new("__batch__"),
            change_type: StateChangeType::BatchUpdate,
            old_state: None,
            new_state: serde_json::json!({"count": states.len()}),
            timestamp: std::time::SystemTime::now(),
            reason,
        };
        
        self.add_to_history(batch_event.clone()).await;
        if let Err(_) = self.change_event_sender.send(batch_event) {
            log::warn!("发送批量状态变更事件失败");
        }
        
        Ok(())
    }
    
    /// 删除组件状态
    pub async fn remove_state(&self, component_id: &ComponentId, reason: String) -> ComponentResult<()> {
        let old_state = {
            let mut current_states = self.current_states.write().await;
            current_states.remove(component_id)
        };
        
        if let Some(old_state) = old_state {
            // 创建删除事件
            let change_event = StateChangeEvent {
                component_id: component_id.clone(),
                change_type: StateChangeType::Deleted,
                old_state: Some(old_state),
                new_state: serde_json::Value::Null,
                timestamp: std::time::SystemTime::now(),
                reason,
            };
            
            self.add_to_history(change_event.clone()).await;
            
            if let Err(_) = self.change_event_sender.send(change_event) {
                log::warn!("发送状态删除事件失败 - 没有活跃的监听器");
            }
        }
        
        // 清理版本计数器
        {
            let mut version_counters = self.version_counters.write().await;
            version_counters.remove(component_id);
        }
        
        Ok(())
    }
    
    /// 重置组件状态
    pub async fn reset_state(&self, component_id: &ComponentId, reason: String) -> ComponentResult<()> {
        // 尝试从持久化存储加载初始状态
        if let Some(snapshot) = self.persistence.load_snapshot(component_id, Some(1)).await? {
            self.set_state(component_id.clone(), snapshot.state_data, reason).await?;
        } else {
            // 如果没有初始状态，则删除当前状态
            self.remove_state(component_id, reason).await?;
        }
        
        Ok(())
    }
    
    /// 订阅状态变更事件
    pub fn subscribe_changes(&self) -> broadcast::Receiver<StateChangeEvent> {
        self.change_event_sender.subscribe()
    }
    
    /// 创建状态快照
    pub async fn create_snapshot(&self, component_id: &ComponentId) -> ComponentResult<Option<ComponentStateSnapshot>> {
        if let Some(state) = self.get_state(component_id).await {
            let version = {
                let version_counters = self.version_counters.read().await;
                version_counters.get(component_id).copied().unwrap_or(0)
            };
            
            let snapshot = ComponentStateSnapshot::new(component_id.clone(), state, version)
                .with_tag("type".to_string(), "manual".to_string())
                .with_tag("timestamp".to_string(), 
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        .to_string()
                );
            
            self.persistence.save_snapshot(&snapshot).await?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }
    
    /// 恢复到指定快照
    pub async fn restore_from_snapshot(&self, component_id: &ComponentId, version: Option<u64>) -> ComponentResult<bool> {
        if let Some(snapshot) = self.persistence.load_snapshot(component_id, version).await? {
            self.set_state(
                component_id.clone(),
                snapshot.state_data,
                format!("从快照版本 {} 恢复", snapshot.version),
            ).await?;
            
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// 列出组件的所有快照
    pub async fn list_snapshots(&self, component_id: &ComponentId) -> ComponentResult<Vec<ComponentStateSnapshot>> {
        self.persistence.list_snapshots(component_id).await
    }
    
    /// 清理过期快照
    pub async fn cleanup_expired_snapshots(&self, max_age: std::time::Duration) -> ComponentResult<u64> {
        self.persistence.cleanup_expired_snapshots(max_age).await
    }
    
    /// 获取状态变更历史
    pub async fn get_change_history(&self, component_id: Option<&ComponentId>, limit: Option<usize>) -> Vec<StateChangeEvent> {
        let history = self.change_history.read().await;
        let mut filtered: Vec<StateChangeEvent> = if let Some(id) = component_id {
            history.iter()
                .filter(|event| &event.component_id == id)
                .cloned()
                .collect()
        } else {
            history.clone()
        };
        
        // 按时间倒序排列
        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        if let Some(limit) = limit {
            filtered.truncate(limit);
        }
        
        filtered
    }
    
    /// 获取所有组件的当前状态
    pub async fn get_all_states(&self) -> HashMap<ComponentId, serde_json::Value> {
        let current_states = self.current_states.read().await;
        current_states.clone()
    }
    
    /// 启动自动快照定时器
    pub async fn start_auto_snapshot(&self) -> ComponentResult<()> {
        if let Some(interval) = self.config.auto_snapshot_interval {
            let persistence = self.persistence.clone();
            let current_states = self.current_states.clone();
            let version_counters = self.version_counters.clone();
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(interval);
                
                loop {
                    interval.tick().await;
                    
                    // 为所有组件创建自动快照
                    let states = current_states.read().await;
                    let versions = version_counters.read().await;
                    
                    for (component_id, state) in states.iter() {
                        let version = versions.get(component_id).copied().unwrap_or(0);
                        let snapshot = ComponentStateSnapshot::new(
                            component_id.clone(),
                            state.clone(),
                            version
                        ).with_tag("type".to_string(), "auto".to_string());
                        
                        if let Err(e) = persistence.save_snapshot(&snapshot).await {
                            log::warn!("自动快照保存失败 {}: {}", component_id, e);
                        }
                    }
                }
            });
        }
        
        Ok(())
    }
    
    /// 验证状态数据
    async fn validate_state(&self, state: &serde_json::Value) -> ComponentResult<()> {
        // 基本验证：检查状态是否为合法JSON
        if state.is_null() {
            return Err(ComponentError::StateError("状态不能为null".to_string()));
        }
        
        // 可以添加更多验证逻辑
        Ok(())
    }
    
    /// 添加到变更历史
    async fn add_to_history(&self, event: StateChangeEvent) {
        let mut history = self.change_history.write().await;
        history.push(event);
        
        // 限制历史记录大小
        if history.len() > self.config.max_history_size {
            history.drain(0..history.len() - self.config.max_history_size);
        }
    }
}