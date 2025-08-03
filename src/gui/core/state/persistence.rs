/// 状态持久化实现

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::ComponentStateSnapshot;
use super::super::component::{ComponentId, ComponentError, ComponentResult};

/// 状态持久化接口
#[async_trait]
pub trait StatePersistence: Send + Sync {
    /// 保存状态快照
    async fn save_snapshot(&self, snapshot: &ComponentStateSnapshot) -> ComponentResult<()>;
    
    /// 加载状态快照
    async fn load_snapshot(&self, component_id: &ComponentId, version: Option<u64>) -> ComponentResult<Option<ComponentStateSnapshot>>;
    
    /// 列出组件的所有快照
    async fn list_snapshots(&self, component_id: &ComponentId) -> ComponentResult<Vec<ComponentStateSnapshot>>;
    
    /// 删除状态快照
    async fn delete_snapshot(&self, component_id: &ComponentId, version: u64) -> ComponentResult<()>;
    
    /// 清理过期快照
    async fn cleanup_expired_snapshots(&self, max_age: std::time::Duration) -> ComponentResult<u64>;
}

/// 内存状态持久化实现
pub struct MemoryStatePersistence {
    /// 快照存储
    snapshots: Arc<RwLock<HashMap<ComponentId, Vec<ComponentStateSnapshot>>>>,
    /// 最大快照数量 (每个组件)
    max_snapshots_per_component: usize,
}

impl MemoryStatePersistence {
    /// 创建新的内存状态持久化
    pub fn new(max_snapshots_per_component: usize) -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            max_snapshots_per_component,
        }
    }
}

#[async_trait]
impl StatePersistence for MemoryStatePersistence {
    async fn save_snapshot(&self, snapshot: &ComponentStateSnapshot) -> ComponentResult<()> {
        let mut snapshots = self.snapshots.write().await;
        let component_snapshots = snapshots.entry(snapshot.component_id.clone()).or_insert_with(Vec::new);
        
        // 添加新快照
        component_snapshots.push(snapshot.clone());
        
        // 按版本排序
        component_snapshots.sort_by_key(|s| s.version);
        
        // 限制快照数量
        if component_snapshots.len() > self.max_snapshots_per_component {
            component_snapshots.drain(0..component_snapshots.len() - self.max_snapshots_per_component);
        }
        
        Ok(())
    }
    
    async fn load_snapshot(&self, component_id: &ComponentId, version: Option<u64>) -> ComponentResult<Option<ComponentStateSnapshot>> {
        let snapshots = self.snapshots.read().await;
        
        if let Some(component_snapshots) = snapshots.get(component_id) {
            match version {
                Some(v) => {
                    // 查找特定版本
                    Ok(component_snapshots.iter().find(|s| s.version == v).cloned())
                }
                None => {
                    // 返回最新版本
                    Ok(component_snapshots.last().cloned())
                }
            }
        } else {
            Ok(None)
        }
    }
    
    async fn list_snapshots(&self, component_id: &ComponentId) -> ComponentResult<Vec<ComponentStateSnapshot>> {
        let snapshots = self.snapshots.read().await;
        Ok(snapshots.get(component_id).cloned().unwrap_or_default())
    }
    
    async fn delete_snapshot(&self, component_id: &ComponentId, version: u64) -> ComponentResult<()> {
        let mut snapshots = self.snapshots.write().await;
        
        if let Some(component_snapshots) = snapshots.get_mut(component_id) {
            component_snapshots.retain(|s| s.version != version);
        }
        
        Ok(())
    }
    
    async fn cleanup_expired_snapshots(&self, max_age: std::time::Duration) -> ComponentResult<u64> {
        let mut snapshots = self.snapshots.write().await;
        let cutoff_time = std::time::SystemTime::now() - max_age;
        let mut cleaned_count = 0;
        
        for (_, component_snapshots) in snapshots.iter_mut() {
            let original_len = component_snapshots.len();
            component_snapshots.retain(|s| s.timestamp > cutoff_time);
            cleaned_count += (original_len - component_snapshots.len()) as u64;
        }
        
        Ok(cleaned_count)
    }
}

/// 文件状态持久化实现
pub struct FileStatePersistence {
    /// 存储目录
    storage_dir: std::path::PathBuf,
    /// 最大快照数量 (每个组件)
    max_snapshots_per_component: usize,
}

impl FileStatePersistence {
    /// 创建新的文件状态持久化
    pub fn new(storage_dir: std::path::PathBuf, max_snapshots_per_component: usize) -> ComponentResult<Self> {
        // 确保存储目录存在
        std::fs::create_dir_all(&storage_dir)
            .map_err(|e| ComponentError::ResourceError(format!("创建存储目录失败: {}", e)))?;
        
        Ok(Self {
            storage_dir,
            max_snapshots_per_component,
        })
    }
    
    /// 获取组件的快照文件路径
    fn get_snapshot_path(&self, component_id: &ComponentId, version: u64) -> std::path::PathBuf {
        self.storage_dir.join(format!("{}_{}.json", component_id, version))
    }
    
    /// 获取组件的快照索引文件路径
    fn get_index_path(&self, component_id: &ComponentId) -> std::path::PathBuf {
        self.storage_dir.join(format!("{}_index.json", component_id))
    }
}

#[async_trait]
impl StatePersistence for FileStatePersistence {
    async fn save_snapshot(&self, snapshot: &ComponentStateSnapshot) -> ComponentResult<()> {
        let snapshot_path = self.get_snapshot_path(&snapshot.component_id, snapshot.version);
        let index_path = self.get_index_path(&snapshot.component_id);
        
        // 保存快照文件
        let snapshot_json = serde_json::to_string_pretty(snapshot)
            .map_err(|e| ComponentError::ResourceError(format!("序列化快照失败: {}", e)))?;
        
        tokio::fs::write(&snapshot_path, snapshot_json).await
            .map_err(|e| ComponentError::ResourceError(format!("写入快照文件失败: {}", e)))?;
        
        // 更新索引文件
        let mut index: Vec<u64> = if tokio::fs::metadata(&index_path).await.is_ok() {
            let index_content = tokio::fs::read_to_string(&index_path).await
                .map_err(|e| ComponentError::ResourceError(format!("读取索引文件失败: {}", e)))?;
            
            serde_json::from_str(&index_content)
                .map_err(|e| ComponentError::ResourceError(format!("解析索引文件失败: {}", e)))?
        } else {
            Vec::new()
        };
        
        // 添加新版本并排序
        if !index.contains(&snapshot.version) {
            index.push(snapshot.version);
            index.sort();
        }
        
        // 限制快照数量
        if index.len() > self.max_snapshots_per_component {
            let to_remove = index.drain(0..index.len() - self.max_snapshots_per_component).collect::<Vec<_>>();
            
            // 删除多余的快照文件
            for version in to_remove {
                let old_path = self.get_snapshot_path(&snapshot.component_id, version);
                if tokio::fs::metadata(&old_path).await.is_ok() {
                    let _ = tokio::fs::remove_file(&old_path).await;
                }
            }
        }
        
        // 保存更新后的索引
        let index_json = serde_json::to_string_pretty(&index)
            .map_err(|e| ComponentError::ResourceError(format!("序列化索引失败: {}", e)))?;
        
        tokio::fs::write(&index_path, index_json).await
            .map_err(|e| ComponentError::ResourceError(format!("写入索引文件失败: {}", e)))?;
        
        Ok(())
    }
    
    async fn load_snapshot(&self, component_id: &ComponentId, version: Option<u64>) -> ComponentResult<Option<ComponentStateSnapshot>> {
        let index_path = self.get_index_path(component_id);
        
        // 读取索引文件
        if tokio::fs::metadata(&index_path).await.is_err() {
            return Ok(None);
        }
        
        let index_content = tokio::fs::read_to_string(&index_path).await
            .map_err(|e| ComponentError::ResourceError(format!("读取索引文件失败: {}", e)))?;
        
        let index: Vec<u64> = serde_json::from_str(&index_content)
            .map_err(|e| ComponentError::ResourceError(format!("解析索引文件失败: {}", e)))?;
        
        // 确定要加载的版本
        let target_version = match version {
            Some(v) => {
                if index.contains(&v) {
                    v
                } else {
                    return Ok(None);
                }
            }
            None => {
                if let Some(&latest) = index.last() {
                    latest
                } else {
                    return Ok(None);
                }
            }
        };
        
        // 加载快照文件
        let snapshot_path = self.get_snapshot_path(component_id, target_version);
        
        if tokio::fs::metadata(&snapshot_path).await.is_err() {
            return Ok(None);
        }
        
        let snapshot_content = tokio::fs::read_to_string(&snapshot_path).await
            .map_err(|e| ComponentError::ResourceError(format!("读取快照文件失败: {}", e)))?;
        
        let snapshot: ComponentStateSnapshot = serde_json::from_str(&snapshot_content)
            .map_err(|e| ComponentError::ResourceError(format!("解析快照文件失败: {}", e)))?;
        
        Ok(Some(snapshot))
    }
    
    async fn list_snapshots(&self, component_id: &ComponentId) -> ComponentResult<Vec<ComponentStateSnapshot>> {
        let index_path = self.get_index_path(component_id);
        
        if tokio::fs::metadata(&index_path).await.is_err() {
            return Ok(Vec::new());
        }
        
        let index_content = tokio::fs::read_to_string(&index_path).await
            .map_err(|e| ComponentError::ResourceError(format!("读取索引文件失败: {}", e)))?;
        
        let index: Vec<u64> = serde_json::from_str(&index_content)
            .map_err(|e| ComponentError::ResourceError(format!("解析索引文件失败: {}", e)))?;
        
        let mut snapshots = Vec::new();
        
        for version in index {
            if let Ok(Some(snapshot)) = self.load_snapshot(component_id, Some(version)).await {
                snapshots.push(snapshot);
            }
        }
        
        Ok(snapshots)
    }
    
    async fn delete_snapshot(&self, component_id: &ComponentId, version: u64) -> ComponentResult<()> {
        let snapshot_path = self.get_snapshot_path(component_id, version);
        let index_path = self.get_index_path(component_id);
        
        // 删除快照文件
        if tokio::fs::metadata(&snapshot_path).await.is_ok() {
            tokio::fs::remove_file(&snapshot_path).await
                .map_err(|e| ComponentError::ResourceError(format!("删除快照文件失败: {}", e)))?;
        }
        
        // 更新索引文件
        if tokio::fs::metadata(&index_path).await.is_ok() {
            let index_content = tokio::fs::read_to_string(&index_path).await
                .map_err(|e| ComponentError::ResourceError(format!("读取索引文件失败: {}", e)))?;
            
            let mut index: Vec<u64> = serde_json::from_str(&index_content)
                .map_err(|e| ComponentError::ResourceError(format!("解析索引文件失败: {}", e)))?;
            
            index.retain(|&v| v != version);
            
            let index_json = serde_json::to_string_pretty(&index)
                .map_err(|e| ComponentError::ResourceError(format!("序列化索引失败: {}", e)))?;
            
            tokio::fs::write(&index_path, index_json).await
                .map_err(|e| ComponentError::ResourceError(format!("写入索引文件失败: {}", e)))?;
        }
        
        Ok(())
    }
    
    async fn cleanup_expired_snapshots(&self, max_age: std::time::Duration) -> ComponentResult<u64> {
        let cutoff_time = std::time::SystemTime::now() - max_age;
        let mut cleaned_count = 0;
        
        // 遍历存储目录中的所有索引文件
        let mut dir = tokio::fs::read_dir(&self.storage_dir).await
            .map_err(|e| ComponentError::ResourceError(format!("读取存储目录失败: {}", e)))?;
        
        while let Some(entry) = dir.next_entry().await
            .map_err(|e| ComponentError::ResourceError(format!("遍历目录失败: {}", e)))? {
            
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with("_index.json") {
                    // 提取组件ID
                    let component_id_str = file_name.trim_end_matches("_index.json");
                    let component_id = ComponentId::new(component_id_str);
                    
                    // 获取该组件的所有快照
                    let snapshots = self.list_snapshots(&component_id).await?;
                    
                    // 删除过期快照
                    for snapshot in snapshots {
                        if snapshot.timestamp < cutoff_time {
                            if let Err(e) = self.delete_snapshot(&component_id, snapshot.version).await {
                                log::warn!("删除过期快照失败: {}", e);
                            } else {
                                cleaned_count += 1;
                            }
                        }
                    }
                }
            }
        }
        
        Ok(cleaned_count)
    }
}