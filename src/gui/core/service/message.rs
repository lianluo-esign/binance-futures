/// 服务消息系统

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use super::types::{ServiceId, ServiceMessageType, MessagePriority};
use super::super::component::{ComponentId, ComponentError, ComponentResult};

/// 服务消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMessage {
    /// 消息ID
    pub id: String,
    /// 消息类型
    pub message_type: ServiceMessageType,
    /// 发送者服务ID
    pub sender: ServiceId,
    /// 接收者服务ID (None表示广播)
    pub receiver: Option<ServiceId>,
    /// 消息主题
    pub topic: String,
    /// 消息内容
    pub payload: serde_json::Value,
    /// 消息优先级
    pub priority: MessagePriority,
    /// 创建时间
    pub created_at: std::time::SystemTime,
    /// 过期时间 (可选)
    pub expires_at: Option<std::time::SystemTime>,
    /// 相关组件ID (可选)
    pub component_id: Option<ComponentId>,
}

impl ServiceMessage {
    /// 创建新消息
    pub fn new(
        sender: ServiceId,
        receiver: Option<ServiceId>,
        topic: String,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type: ServiceMessageType::Event,
            sender,
            receiver,
            topic,
            payload,
            priority: MessagePriority::Normal,
            created_at: std::time::SystemTime::now(),
            expires_at: None,
            component_id: None,
        }
    }
    
    /// 创建请求消息
    pub fn request(
        sender: ServiceId,
        receiver: ServiceId,
        topic: String,
        payload: serde_json::Value,
    ) -> Self {
        let mut msg = Self::new(sender, Some(receiver), topic, payload);
        msg.message_type = ServiceMessageType::Request;
        msg
    }
    
    /// 创建响应消息
    pub fn response(
        sender: ServiceId,
        receiver: ServiceId,
        topic: String,
        payload: serde_json::Value,
    ) -> Self {
        let mut msg = Self::new(sender, Some(receiver), topic, payload);
        msg.message_type = ServiceMessageType::Response;
        msg
    }
    
    /// 创建广播事件
    pub fn broadcast_event(
        sender: ServiceId,
        topic: String,
        payload: serde_json::Value,
    ) -> Self {
        let mut msg = Self::new(sender, None, topic, payload);
        msg.message_type = ServiceMessageType::Event;
        msg
    }
    
    /// 设置优先级
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// 设置过期时间
    pub fn with_expiry(mut self, expiry: std::time::Duration) -> Self {
        self.expires_at = Some(std::time::SystemTime::now() + expiry);
        self
    }
    
    /// 关联组件
    pub fn with_component(mut self, component_id: ComponentId) -> Self {
        self.component_id = Some(component_id);
        self
    }
    
    /// 检查消息是否过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            std::time::SystemTime::now() > expires_at
        } else {
            false
        }
    }
}

/// 消息路由器
/// 
/// 负责在服务间路由消息
pub struct MessageRouter {
    /// 消息通道映射
    channels: Arc<RwLock<HashMap<ServiceId, mpsc::UnboundedSender<ServiceMessage>>>>,
    /// 消息统计
    stats: Arc<RwLock<MessageStats>>,
}

/// 消息统计
#[derive(Debug, Default)]
pub struct MessageStats {
    pub total_sent: u64,
    pub total_received: u64,
    pub total_dropped: u64,
    pub last_activity: Option<std::time::SystemTime>,
}

impl MessageRouter {
    /// 创建新的消息路由器
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(MessageStats::default())),
        }
    }
    
    /// 注册服务消息通道
    pub async fn register_channel(
        &self,
        service_id: ServiceId,
        sender: mpsc::UnboundedSender<ServiceMessage>,
    ) {
        let mut channels = self.channels.write().await;
        channels.insert(service_id, sender);
    }
    
    /// 路由消息
    pub async fn route_message(&self, message: ServiceMessage) -> ComponentResult<()> {
        // 检查消息是否过期
        if message.is_expired() {
            let mut stats = self.stats.write().await;
            stats.total_dropped += 1;
            return Err(ComponentError::CommunicationError("消息已过期".to_string()));
        }
        
        let channels = self.channels.read().await;
        
        match &message.receiver {
            Some(receiver_id) => {
                // 点对点消息
                if let Some(sender) = channels.get(receiver_id) {
                    if let Err(_) = sender.send(message) {
                        return Err(ComponentError::CommunicationError(
                            format!("发送消息到服务 {} 失败", receiver_id)
                        ));
                    }
                } else {
                    return Err(ComponentError::CommunicationError(
                        format!("服务 {} 的消息通道未找到", receiver_id)
                    ));
                }
            }
            None => {
                // 广播消息
                let sender_id = &message.sender;
                for (service_id, sender) in channels.iter() {
                    if service_id != sender_id {
                        if let Err(_) = sender.send(message.clone()) {
                            log::warn!("广播消息到服务 {} 失败", service_id);
                        }
                    }
                }
            }
        }
        
        // 更新统计
        {
            let mut stats = self.stats.write().await;
            stats.total_sent += 1;
            stats.last_activity = Some(std::time::SystemTime::now());
        }
        
        Ok(())
    }
    
    /// 获取消息统计
    pub async fn get_stats(&self) -> MessageStats {
        let stats = self.stats.read().await;
        MessageStats {
            total_sent: stats.total_sent,
            total_received: stats.total_received,
            total_dropped: stats.total_dropped,
            last_activity: stats.last_activity,
        }
    }
}