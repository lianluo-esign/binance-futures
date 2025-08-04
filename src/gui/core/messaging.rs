/// GUI消息传递和事件系统
/// 
/// 提供组件间通信机制:
/// - 事件总线 (EventBus)
/// - 消息通道 (MessageChannel) 
/// - 事件过滤器 (EventFilter)
/// - 异步消息处理

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};

use super::component::{ComponentId, ComponentError, ComponentResult, ComponentEvent};
use super::service::{ServiceId, ServiceMessage};

/// 事件通道
/// 
/// 为组件提供发布-订阅消息通信能力
pub struct EventChannel {
    /// 广播发送器
    sender: broadcast::Sender<ComponentEvent>,
    /// 事件过滤器
    filters: Arc<RwLock<HashMap<ComponentId, Vec<EventFilter>>>>,
    /// 订阅者计数
    subscriber_count: Arc<RwLock<usize>>,
    /// 通道配置
    config: EventChannelConfig,
}

/// 事件通道配置
#[derive(Debug, Clone)]
pub struct EventChannelConfig {
    /// 缓冲区大小
    pub buffer_size: usize,
    /// 是否启用历史事件
    pub enable_history: bool,
    /// 最大历史事件数
    pub max_history_size: usize,
    /// 事件过期时间
    pub event_ttl: std::time::Duration,
}

impl Default for EventChannelConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1000,
            enable_history: true,
            max_history_size: 100,
            event_ttl: std::time::Duration::from_secs(60),
        }
    }
}

impl EventChannel {
    /// 创建新的事件通道
    pub fn new(config: EventChannelConfig) -> Self {
        let (sender, _) = broadcast::channel(config.buffer_size);
        
        Self {
            sender,
            filters: Arc::new(RwLock::new(HashMap::new())),
            subscriber_count: Arc::new(RwLock::new(0)),
            config,
        }
    }
    
    /// 订阅事件
    pub async fn subscribe(&self, component_id: ComponentId) -> ComponentResult<EventSubscription> {
        let receiver = self.sender.subscribe();
        
        // 增加订阅者计数
        {
            let mut count = self.subscriber_count.write().await;
            *count += 1;
        }
        
        log::debug!("组件 {} 订阅事件通道", component_id);
        
        Ok(EventSubscription {
            component_id,
            receiver,
            channel: self.sender.clone(),
        })
    }
    
    /// 发布事件
    pub async fn publish(&self, event: ComponentEvent) -> ComponentResult<usize> {
        match self.sender.send(event.clone()) {
            Ok(subscriber_count) => {
                log::trace!("事件发布给 {} 个订阅者: {:?}", subscriber_count, event);
                Ok(subscriber_count)
            }
            Err(_) => {
                Err(ComponentError::CommunicationError(
                    "事件发布失败: 没有活跃的接收者".to_string()
                ))
            }
        }
    }
    
    /// 添加事件过滤器
    pub async fn add_filter(&self, component_id: ComponentId, filter: EventFilter) {
        let mut filters = self.filters.write().await;
        filters.entry(component_id).or_insert_with(Vec::new).push(filter);
    }
    
    /// 移除事件过滤器
    pub async fn remove_filters(&self, component_id: &ComponentId) {
        let mut filters = self.filters.write().await;
        filters.remove(component_id);
    }
    
    /// 获取订阅者数量
    pub async fn subscriber_count(&self) -> usize {
        *self.subscriber_count.read().await
    }
}

/// 事件订阅
pub struct EventSubscription {
    /// 订阅组件ID
    component_id: ComponentId,
    /// 事件接收器
    receiver: broadcast::Receiver<ComponentEvent>,
    /// 通道引用 (用于取消订阅)
    channel: broadcast::Sender<ComponentEvent>,
}

impl EventSubscription {
    /// 接收事件 (非阻塞)
    pub async fn try_recv(&mut self) -> ComponentResult<ComponentEvent> {
        match self.receiver.try_recv() {
            Ok(event) => Ok(event),
            Err(broadcast::error::TryRecvError::Empty) => {
                Err(ComponentError::CommunicationError("没有新事件".to_string()))
            }
            Err(broadcast::error::TryRecvError::Closed) => {
                Err(ComponentError::CommunicationError("事件通道已关闭".to_string()))
            }
            Err(broadcast::error::TryRecvError::Lagged(count)) => {
                log::warn!("组件 {} 事件接收滞后 {} 个事件", self.component_id, count);
                Err(ComponentError::CommunicationError(
                    format!("事件接收滞后 {} 个事件", count)
                ))
            }
        }
    }
    
    /// 接收事件 (阻塞)
    pub async fn recv(&mut self) -> ComponentResult<ComponentEvent> {
        match self.receiver.recv().await {
            Ok(event) => Ok(event),
            Err(broadcast::error::RecvError::Closed) => {
                Err(ComponentError::CommunicationError("事件通道已关闭".to_string()))
            }
            Err(broadcast::error::RecvError::Lagged(count)) => {
                log::warn!("组件 {} 事件接收滞后 {} 个事件", self.component_id, count);
                Err(ComponentError::CommunicationError(
                    format!("事件接收滞后 {} 个事件", count)
                ))
            }
        }
    }
}

/// 事件过滤器
#[derive(Debug, Clone)]
pub enum EventFilter {
    /// 按事件类型过滤
    EventType(EventTypeFilter),
    /// 按组件ID过滤
    ComponentId(ComponentId),
    /// 按自定义条件过滤
    Custom(CustomFilter),
    /// 组合过滤器
    And(Vec<EventFilter>),
    /// 或过滤器
    Or(Vec<EventFilter>),
    /// 非过滤器
    Not(Box<EventFilter>),
}

/// 事件类型过滤器
#[derive(Debug, Clone, PartialEq)]
pub enum EventTypeFilter {
    MouseClick,
    MouseHover,
    KeyboardInput,
    WindowEvent,
    DataUpdate,
    ConfigChanged,
    StateChanged,
    Custom(String),
}

/// 自定义过滤器
#[derive(Debug, Clone)]
pub struct CustomFilter {
    /// 过滤器名称
    pub name: String,
    /// 过滤器条件 (序列化的条件表达式)
    pub condition: serde_json::Value,
}

impl EventFilter {
    /// 检查事件是否匹配过滤器
    pub fn matches(&self, event: &ComponentEvent) -> bool {
        match self {
            EventFilter::EventType(filter) => {
                match (filter, event) {
                    (EventTypeFilter::MouseClick, ComponentEvent::MouseClick { .. }) => true,
                    (EventTypeFilter::MouseHover, ComponentEvent::MouseHover { .. }) => true,
                    (EventTypeFilter::KeyboardInput, ComponentEvent::KeyboardInput { .. }) => true,
                    (EventTypeFilter::WindowEvent, ComponentEvent::WindowEvent { .. }) => true,
                    (EventTypeFilter::DataUpdate, ComponentEvent::DataUpdate { .. }) => true,
                    (EventTypeFilter::ConfigChanged, ComponentEvent::ConfigChanged { .. }) => true,
                    (EventTypeFilter::StateChanged, ComponentEvent::StateChanged { .. }) => true,
                    (EventTypeFilter::Custom(name), ComponentEvent::Custom { event_type, .. }) => {
                        name == event_type
                    }
                    _ => false,
                }
            }
            EventFilter::ComponentId(_) => {
                // 需要在事件中包含组件ID信息才能过滤
                true // 简化实现，实际需要在事件中添加source_component_id字段
            }
            EventFilter::Custom(_) => {
                // 自定义过滤器需要实现条件评估逻辑
                true // 简化实现
            }
            EventFilter::And(filters) => {
                filters.iter().all(|f| f.matches(event))
            }
            EventFilter::Or(filters) => {
                filters.iter().any(|f| f.matches(event))
            }
            EventFilter::Not(filter) => {
                !filter.matches(event)
            }
        }
    }
}

/// 消息总线
/// 
/// 全局消息传递中心，连接组件和服务
pub struct MessageBus {
    /// 组件事件通道
    component_channel: Arc<EventChannel>,
    /// 服务消息通道
    service_sender: mpsc::UnboundedSender<ServiceMessage>,
    service_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<ServiceMessage>>>>,
    /// 消息处理器映射
    handlers: Arc<RwLock<HashMap<String, Box<dyn MessageHandler + Send + Sync>>>>,
    /// 运行状态
    is_running: Arc<RwLock<bool>>,
    /// 消息统计
    stats: Arc<RwLock<MessageBusStats>>,
}

/// 消息总线统计
#[derive(Debug, Default)]
struct MessageBusStats {
    component_events_processed: u64,
    service_messages_processed: u64,
    errors_count: u64,
    last_activity: Option<std::time::SystemTime>,
}

impl MessageBus {
    /// 创建新的消息总线
    pub fn new() -> Self {
        let (service_sender, service_receiver) = mpsc::unbounded_channel();
        
        Self {
            component_channel: Arc::new(EventChannel::new(EventChannelConfig::default())),
            service_sender,
            service_receiver: Arc::new(RwLock::new(Some(service_receiver))),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(MessageBusStats::default())),
        }
    }
    
    /// 启动消息总线
    pub async fn start(&self) -> ComponentResult<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(ComponentError::StateError("消息总线已在运行".to_string()));
        }
        
        *is_running = true;
        
        // 启动服务消息处理循环
        self.spawn_service_message_handler().await?;
        
        log::info!("消息总线启动成功");
        Ok(())
    }
    
    /// 停止消息总线
    pub async fn stop(&self) -> ComponentResult<()> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        log::info!("消息总线停止");
        Ok(())
    }
    
    /// 获取组件事件通道
    pub fn component_channel(&self) -> Arc<EventChannel> {
        self.component_channel.clone()
    }
    
    /// 发送服务消息
    pub async fn send_service_message(&self, message: ServiceMessage) -> ComponentResult<()> {
        self.service_sender.send(message)
            .map_err(|e| ComponentError::CommunicationError(format!("发送服务消息失败: {}", e)))?;
        
        // 更新统计
        {
            let mut stats = self.stats.write().await;
            stats.service_messages_processed += 1;
            stats.last_activity = Some(std::time::SystemTime::now());
        }
        
        Ok(())
    }
    
    /// 注册消息处理器
    pub async fn register_handler(
        &self,
        topic: String,
        handler: Box<dyn MessageHandler + Send + Sync>,
    ) {
        let mut handlers = self.handlers.write().await;
        let topic_clone = topic.clone();
        handlers.insert(topic, handler);
        log::debug!("注册消息处理器: {}", topic_clone);
    }
    
    /// 取消注册消息处理器
    pub async fn unregister_handler(&self, topic: &str) {
        let mut handlers = self.handlers.write().await;
        handlers.remove(topic);
        log::debug!("取消注册消息处理器: {}", topic);
    }
    
    /// 获取消息统计
    pub async fn get_stats(&self) -> MessageBusStats {
        let stats = self.stats.read().await;
        MessageBusStats {
            component_events_processed: stats.component_events_processed,
            service_messages_processed: stats.service_messages_processed,
            errors_count: stats.errors_count,
            last_activity: stats.last_activity,
        }
    }
    
    /// 生成服务消息处理协程
    async fn spawn_service_message_handler(&self) -> ComponentResult<()> {
        let mut receiver_guard = self.service_receiver.write().await;
        let receiver = receiver_guard.take().ok_or_else(|| {
            ComponentError::StateError("服务消息接收器已被获取".to_string())
        })?;
        
        let handlers = self.handlers.clone();
        let is_running = self.is_running.clone();
        let stats = self.stats.clone();
        
        tokio::spawn(async move {
            let mut receiver = receiver;
            
            while *is_running.read().await {
                match receiver.recv().await {
                    Some(message) => {
                        // 查找处理器
                        let handlers_guard = handlers.read().await;
                        if let Some(handler) = handlers_guard.get(&message.topic) {
                            if let Err(e) = handler.handle_message(message).await {
                                log::error!("处理服务消息失败: {}", e);
                                
                                // 更新错误统计
                                let mut stats_guard = stats.write().await;
                                stats_guard.errors_count += 1;
                            }
                        } else {
                            log::warn!("未找到主题 '{}' 的处理器", message.topic);
                        }
                    }
                    None => {
                        log::info!("服务消息通道已关闭");
                        break;
                    }
                }
            }
            
            log::info!("服务消息处理协程结束");
        });
        
        Ok(())
    }
}

/// 消息处理器trait
#[async_trait]
pub trait MessageHandler {
    /// 处理消息
    async fn handle_message(&self, message: ServiceMessage) -> ComponentResult<()>;
    
    /// 获取处理器名称
    fn name(&self) -> &str;
    
    /// 获取支持的消息主题
    fn supported_topics(&self) -> Vec<String>;
}

/// 简单消息处理器
pub struct SimpleMessageHandler<F>
where
    F: Fn(ServiceMessage) -> ComponentResult<()> + Send + Sync,
{
    name: String,
    topics: Vec<String>,
    handler_fn: F,
}

impl<F> SimpleMessageHandler<F>
where
    F: Fn(ServiceMessage) -> ComponentResult<()> + Send + Sync,
{
    pub fn new(name: String, topics: Vec<String>, handler_fn: F) -> Self {
        Self {
            name,
            topics,
            handler_fn,
        }
    }
}

#[async_trait]
impl<F> MessageHandler for SimpleMessageHandler<F>
where
    F: Fn(ServiceMessage) -> ComponentResult<()> + Send + Sync,
{
    async fn handle_message(&self, message: ServiceMessage) -> ComponentResult<()> {
        (self.handler_fn)(message)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn supported_topics(&self) -> Vec<String> {
        self.topics.clone()
    }
}

/// 异步消息处理器
pub struct AsyncMessageHandler<F, Fut>
where
    F: Fn(ServiceMessage) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = ComponentResult<()>> + Send,
{
    name: String,
    topics: Vec<String>,
    handler_fn: F,
}

impl<F, Fut> AsyncMessageHandler<F, Fut>
where
    F: Fn(ServiceMessage) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = ComponentResult<()>> + Send,
{
    pub fn new(name: String, topics: Vec<String>, handler_fn: F) -> Self {
        Self {
            name,
            topics,
            handler_fn,
        }
    }
}

#[async_trait]
impl<F, Fut> MessageHandler for AsyncMessageHandler<F, Fut>
where
    F: Fn(ServiceMessage) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = ComponentResult<()>> + Send,
{
    async fn handle_message(&self, message: ServiceMessage) -> ComponentResult<()> {
        (self.handler_fn)(message).await
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn supported_topics(&self) -> Vec<String> {
        self.topics.clone()
    }
}