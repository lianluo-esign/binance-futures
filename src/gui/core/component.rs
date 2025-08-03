/// GUI组件基础trait和类型定义
/// 
/// 提供统一的组件接口，支持:
/// - 生命周期管理 (创建、更新、销毁)
/// - 状态管理 (本地状态 + 共享状态)
/// - 事件处理 (输入事件 + 自定义事件)
/// - 渲染控制 (可见性、层级、样式)

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

/// 组件唯一标识符
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComponentId(pub String);

impl ComponentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl fmt::Display for ComponentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 组件类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentType {
    /// 订单簿组件
    OrderBook,
    /// 图表组件
    Chart,
    /// 状态栏组件
    StatusBar,
    /// 调试窗口组件
    DebugWindow,
    /// 工具栏组件
    Toolbar,
    /// 侧边栏组件
    Sidebar,
    /// 弹出窗口组件
    PopupWindow,
    /// 自定义组件
    Custom(String),
}

/// 组件状态枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentState {
    /// 未初始化
    Uninitialized,
    /// 正在初始化
    Initializing,
    /// 活跃状态
    Active,
    /// 暂停状态
    Paused,
    /// 隐藏状态
    Hidden,
    /// 错误状态
    Error(String),
    /// 正在销毁
    Destroying,
    /// 已销毁
    Destroyed,
}

/// 组件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfig {
    /// 组件ID
    pub id: ComponentId,
    /// 组件类型
    pub component_type: ComponentType,
    /// 组件名称
    pub name: String,
    /// 初始可见性
    pub visible: bool,
    /// 层级 (z-index)
    pub z_index: i32,
    /// 位置
    pub position: Option<egui::Pos2>,
    /// 大小
    pub size: Option<egui::Vec2>,
    /// 自定义属性
    pub properties: HashMap<String, serde_json::Value>,
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self {
            id: ComponentId::generate(),
            component_type: ComponentType::Custom("default".to_string()),
            name: "Unnamed Component".to_string(),
            visible: true,
            z_index: 0,
            position: None,
            size: None,
            properties: HashMap::new(),
        }
    }
}

/// 渲染上下文
pub struct RenderContext<'a> {
    /// egui上下文
    pub ctx: &'a egui::Context,
    /// UI对象
    pub ui: &'a mut egui::Ui,
    /// 渲染区域
    pub rect: egui::Rect,
    /// 当前帧时间
    pub frame_time: std::time::Duration,
    /// 鼠标位置
    pub mouse_pos: egui::Pos2,
    /// 是否获得焦点
    pub has_focus: bool,
}

/// 更新上下文
pub struct UpdateContext {
    /// 自上次更新的时间
    pub delta_time: std::time::Duration,
    /// 当前时间戳
    pub timestamp: std::time::Instant,
    /// 组件是否可见
    pub is_visible: bool,
    /// 窗口大小
    pub window_size: egui::Vec2,
    /// 系统事件
    pub system_events: Vec<SystemEvent>,
}

/// 系统事件
#[derive(Debug, Clone)]
pub enum SystemEvent {
    /// 窗口调整大小
    WindowResized(egui::Vec2),
    /// 应用程序获得焦点
    AppFocused,
    /// 应用程序失去焦点
    AppUnfocused,
    /// 主题更改
    ThemeChanged(String),
    /// 语言更改
    LanguageChanged(String),
}

/// 组件错误类型
#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    #[error("组件初始化失败: {0}")]
    InitializationFailed(String),
    
    #[error("组件渲染错误: {0}")]
    RenderError(String),
    
    #[error("组件更新错误: {0}")]
    UpdateError(String),
    
    #[error("配置错误: {0}")]
    ConfigError(String),
    
    #[error("状态错误: {0}")]
    StateError(String),
    
    #[error("通信错误: {0}")]
    CommunicationError(String),
    
    #[error("资源错误: {0}")]
    ResourceError(String),
}

/// 组件操作结果
pub type ComponentResult<T> = Result<T, ComponentError>;

/// GUI组件基础trait
/// 
/// 所有GUI组件都必须实现此trait，提供统一的接口
#[async_trait]
pub trait GUIComponent: Send + Sync {
    /// 获取组件ID
    fn id(&self) -> &ComponentId;
    
    /// 获取组件类型
    fn component_type(&self) -> &ComponentType;
    
    /// 获取组件名称
    fn name(&self) -> &str;
    
    /// 获取当前状态
    fn state(&self) -> ComponentState;
    
    /// 异步初始化组件
    async fn initialize(&mut self, config: ComponentConfig) -> ComponentResult<()>;
    
    /// 异步更新组件逻辑
    async fn update(&mut self, ctx: UpdateContext) -> ComponentResult<()>;
    
    /// 同步渲染组件UI
    fn render(&mut self, ctx: RenderContext) -> ComponentResult<()>;
    
    /// 处理组件事件
    async fn handle_event(&mut self, event: ComponentEvent) -> ComponentResult<()>;
    
    /// 异步清理资源
    async fn cleanup(&mut self) -> ComponentResult<()>;
    
    /// 获取组件配置
    fn config(&self) -> &ComponentConfig;
    
    /// 更新组件配置
    async fn update_config(&mut self, config: ComponentConfig) -> ComponentResult<()>;
    
    /// 序列化组件状态
    fn serialize_state(&self) -> ComponentResult<serde_json::Value>;
    
    /// 反序列化组件状态
    async fn deserialize_state(&mut self, state: serde_json::Value) -> ComponentResult<()>;
    
    /// 获取组件能力 (可选功能)
    fn capabilities(&self) -> ComponentCapabilities {
        ComponentCapabilities::default()
    }
    
    /// 转换为Any trait以支持向下转型
    fn as_any(&self) -> &dyn Any;
    
    /// 转换为可变Any trait
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// 组件能力标志
#[derive(Debug, Clone)]
pub struct ComponentCapabilities {
    /// 支持拖拽
    pub draggable: bool,
    /// 支持调整大小
    pub resizable: bool,
    /// 支持最小化
    pub minimizable: bool,
    /// 支持最大化
    pub maximizable: bool,
    /// 支持关闭
    pub closable: bool,
    /// 支持状态持久化
    pub persistable: bool,
    /// 支持键盘输入
    pub keyboard_input: bool,
    /// 支持鼠标输入
    pub mouse_input: bool,
    /// 支持多线程更新
    pub threaded_update: bool,
}

impl Default for ComponentCapabilities {
    fn default() -> Self {
        Self {
            draggable: false,
            resizable: false,
            minimizable: false,
            maximizable: false,
            closable: false,
            persistable: false,
            keyboard_input: true,
            mouse_input: true,
            threaded_update: false,
        }
    }
}

/// 组件事件
#[derive(Debug, Clone)]
pub enum ComponentEvent {
    /// 鼠标点击事件
    MouseClick {
        button: egui::PointerButton,
        pos: egui::Pos2,
        modifiers: egui::Modifiers,
    },
    /// 鼠标悬停事件
    MouseHover {
        pos: egui::Pos2,
        delta: egui::Vec2,
    },
    /// 键盘事件
    KeyboardInput {
        key: egui::Key,
        pressed: bool,
        modifiers: egui::Modifiers,
    },
    /// 窗口事件
    WindowEvent {
        event: WindowEvent,
    },
    /// 数据更新事件
    DataUpdate {
        data_type: String,
        data: serde_json::Value,
    },
    /// 配置更改事件
    ConfigChanged {
        config: ComponentConfig,
    },
    /// 状态更改事件
    StateChanged {
        old_state: ComponentState,
        new_state: ComponentState,
    },
    /// 自定义事件
    Custom {
        event_type: String,
        data: serde_json::Value,
    },
}

/// 窗口事件
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// 窗口调整大小
    Resized(egui::Vec2),
    /// 窗口移动
    Moved(egui::Pos2),
    /// 窗口获得焦点
    Focused,
    /// 窗口失去焦点
    Unfocused,
    /// 窗口最小化
    Minimized,
    /// 窗口最大化
    Maximized,
    /// 窗口恢复
    Restored,
    /// 窗口关闭请求
    CloseRequested,
}

/// 基础组件实现
/// 
/// 提供组件的默认实现，其他组件可以继承或组合使用
pub struct BaseComponent {
    /// 组件配置
    config: ComponentConfig,
    /// 当前状态
    state: ComponentState,
    /// 创建时间
    created_at: std::time::Instant,
    /// 最后更新时间
    last_update: std::time::Instant,
    /// 本地属性存储
    properties: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    /// 事件处理器
    event_handlers: HashMap<String, Box<dyn Fn(&ComponentEvent) -> ComponentResult<()> + Send + Sync>>,
}

impl BaseComponent {
    /// 创建新的基础组件
    pub fn new(config: ComponentConfig) -> Self {
        let now = std::time::Instant::now();
        
        Self {
            config,
            state: ComponentState::Uninitialized,
            created_at: now,
            last_update: now,
            properties: Arc::new(RwLock::new(HashMap::new())),
            event_handlers: HashMap::new(),
        }
    }
    
    /// 设置属性
    pub async fn set_property(&self, key: String, value: serde_json::Value) -> ComponentResult<()> {
        let mut props = self.properties.write().await;
        props.insert(key, value);
        Ok(())
    }
    
    /// 获取属性
    pub async fn get_property(&self, key: &str) -> Option<serde_json::Value> {
        let props = self.properties.read().await;
        props.get(key).cloned()
    }
    
    /// 设置状态
    pub fn set_state(&mut self, new_state: ComponentState) {
        if self.state != new_state {
            let old_state = std::mem::replace(&mut self.state, new_state);
            log::debug!("组件 {} 状态变更: {:?} -> {:?}", self.config.id, old_state, self.state);
        }
    }
    
    /// 更新最后更新时间
    pub fn touch(&mut self) {
        self.last_update = std::time::Instant::now();
    }
}

#[async_trait]
impl GUIComponent for BaseComponent {
    fn id(&self) -> &ComponentId {
        &self.config.id
    }
    
    fn component_type(&self) -> &ComponentType {
        &self.config.component_type
    }
    
    fn name(&self) -> &str {
        &self.config.name
    }
    
    fn state(&self) -> ComponentState {
        self.state.clone()
    }
    
    async fn initialize(&mut self, config: ComponentConfig) -> ComponentResult<()> {
        self.set_state(ComponentState::Initializing);
        self.config = config;
        self.set_state(ComponentState::Active);
        log::info!("组件 {} 初始化完成", self.config.id);
        Ok(())
    }
    
    async fn update(&mut self, ctx: UpdateContext) -> ComponentResult<()> {
        self.touch();
        // 基础组件无需特殊更新逻辑
        Ok(())
    }
    
    fn render(&mut self, ctx: RenderContext) -> ComponentResult<()> {
        // 基础组件渲染一个简单的占位符
        ctx.ui.label(format!("组件: {}", self.config.name));
        Ok(())
    }
    
    async fn handle_event(&mut self, event: ComponentEvent) -> ComponentResult<()> {
        log::debug!("组件 {} 收到事件: {:?}", self.config.id, event);
        Ok(())
    }
    
    async fn cleanup(&mut self) -> ComponentResult<()> {
        self.set_state(ComponentState::Destroying);
        // 清理资源
        self.set_state(ComponentState::Destroyed);
        log::info!("组件 {} 清理完成", self.config.id);
        Ok(())
    }
    
    fn config(&self) -> &ComponentConfig {
        &self.config
    }
    
    async fn update_config(&mut self, config: ComponentConfig) -> ComponentResult<()> {
        self.config = config;
        Ok(())
    }
    
    fn serialize_state(&self) -> ComponentResult<serde_json::Value> {
        let state_json = serde_json::json!({
            "id": self.config.id,
            "state": self.state,
            "config": self.config,
            "created_at": self.created_at.elapsed().as_secs(),
            "last_update": self.last_update.elapsed().as_secs()
        });
        Ok(state_json)
    }
    
    async fn deserialize_state(&mut self, state: serde_json::Value) -> ComponentResult<()> {
        if let Ok(config) = serde_json::from_value::<ComponentConfig>(state["config"].clone()) {
            self.config = config;
        }
        
        if let Ok(state) = serde_json::from_value::<ComponentState>(state["state"].clone()) {
            self.state = state;
        }
        
        Ok(())
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}