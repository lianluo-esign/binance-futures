/// 服务相关的基础类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// 服务唯一标识符
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceId(pub String);

impl ServiceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl fmt::Display for ServiceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 服务状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceState {
    /// 未启动
    Stopped,
    /// 正在启动
    Starting,
    /// 运行中
    Running,
    /// 正在停止
    Stopping,
    /// 暂停
    Paused,
    /// 错误状态
    Error(String),
}

/// 服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// 服务ID
    pub id: ServiceId,
    /// 服务名称
    pub name: String,
    /// 服务描述
    pub description: String,
    /// 服务版本
    pub version: String,
    /// 依赖服务列表
    pub dependencies: Vec<ServiceId>,
    /// 优先级 (数字越小优先级越高)
    pub priority: u32,
    /// 是否自动启动
    pub auto_start: bool,
    /// 消息队列大小
    pub message_queue_size: usize,
    /// 自定义配置
    pub properties: HashMap<String, serde_json::Value>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            id: ServiceId::generate(),
            name: "Unnamed Service".to_string(),
            description: "No description".to_string(),
            version: "1.0.0".to_string(),
            dependencies: Vec::new(),
            priority: 100,
            auto_start: true,
            message_queue_size: 1000,
            properties: HashMap::new(),
        }
    }
}

/// 服务消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceMessageType {
    /// 请求消息
    Request,
    /// 响应消息
    Response,
    /// 事件通知
    Event,
    /// 命令消息
    Command,
    /// 系统消息
    System,
}

/// 消息优先级
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MessagePriority {
    /// 低优先级
    Low = 0,
    /// 普通优先级
    Normal = 1,
    /// 高优先级
    High = 2,
    /// 紧急优先级
    Critical = 3,
}