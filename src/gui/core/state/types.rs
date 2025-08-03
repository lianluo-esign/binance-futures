/// 状态管理相关类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::super::component::ComponentId;

/// 状态变更事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeEvent {
    /// 组件ID
    pub component_id: ComponentId,
    /// 变更类型
    pub change_type: StateChangeType,
    /// 变更前的状态
    pub old_state: Option<serde_json::Value>,
    /// 变更后的状态
    pub new_state: serde_json::Value,
    /// 变更时间
    pub timestamp: std::time::SystemTime,
    /// 变更原因
    pub reason: String,
}

/// 状态变更类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateChangeType {
    /// 状态创建
    Created,
    /// 状态更新
    Updated,
    /// 状态删除
    Deleted,
    /// 状态重置
    Reset,
    /// 批量更新
    BatchUpdate,
}

/// 组件状态快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStateSnapshot {
    /// 组件ID
    pub component_id: ComponentId,
    /// 状态数据
    pub state_data: serde_json::Value,
    /// 快照时间
    pub timestamp: std::time::SystemTime,
    /// 快照版本
    pub version: u64,
    /// 快照标签
    pub tags: HashMap<String, String>,
}

impl ComponentStateSnapshot {
    /// 创建新的状态快照
    pub fn new(
        component_id: ComponentId,
        state_data: serde_json::Value,
        version: u64,
    ) -> Self {
        Self {
            component_id,
            state_data,
            timestamp: std::time::SystemTime::now(),
            version,
            tags: HashMap::new(),
        }
    }
    
    /// 添加标签
    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }
    
    /// 获取状态数据的特定字段
    pub fn get_field(&self, field_path: &str) -> Option<&serde_json::Value> {
        let mut current = &self.state_data;
        
        for part in field_path.split('.') {
            match current {
                serde_json::Value::Object(map) => {
                    current = map.get(part)?;
                }
                serde_json::Value::Array(arr) => {
                    if let Ok(index) = part.parse::<usize>() {
                        current = arr.get(index)?;
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        
        Some(current)
    }
    
    /// 设置状态数据的特定字段
    pub fn set_field(&mut self, field_path: &str, value: serde_json::Value) -> Result<(), String> {
        let parts: Vec<&str> = field_path.split('.').collect();
        if parts.is_empty() {
            return Err("字段路径不能为空".to_string());
        }
        
        self.set_field_recursive(&mut self.state_data, &parts, 0, value)
    }
    
    fn set_field_recursive(
        &self,
        current: &mut serde_json::Value,
        parts: &[&str],
        index: usize,
        value: serde_json::Value,
    ) -> Result<(), String> {
        if index >= parts.len() {
            return Err("索引超出范围".to_string());
        }
        
        let part = parts[index];
        
        if index == parts.len() - 1 {
            // 最后一个部分，设置值
            match current {
                serde_json::Value::Object(ref mut map) => {
                    map.insert(part.to_string(), value);
                    Ok(())
                }
                serde_json::Value::Array(ref mut arr) => {
                    if let Ok(arr_index) = part.parse::<usize>() {
                        if arr_index < arr.len() {
                            arr[arr_index] = value;
                            Ok(())
                        } else {
                            Err(format!("数组索引 {} 超出范围", arr_index))
                        }
                    } else {
                        Err("无效的数组索引".to_string())
                    }
                }
                _ => Err("无法在非对象/数组类型上设置字段".to_string()),
            }
        } else {
            // 中间部分，递归设置
            match current {
                serde_json::Value::Object(ref mut map) => {
                    let next_value = map.entry(part.to_string()).or_insert(serde_json::json!({}));
                    self.set_field_recursive(next_value, parts, index + 1, value)
                }
                serde_json::Value::Array(ref mut arr) => {
                    if let Ok(arr_index) = part.parse::<usize>() {
                        if arr_index < arr.len() {
                            self.set_field_recursive(&mut arr[arr_index], parts, index + 1, value)
                        } else {
                            Err(format!("数组索引 {} 超出范围", arr_index))
                        }
                    } else {
                        Err("无效的数组索引".to_string())
                    }
                }
                _ => Err("无法在非对象/数组类型上设置字段".to_string()),
            }
        }
    }
}