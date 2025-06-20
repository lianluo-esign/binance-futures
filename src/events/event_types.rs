use serde_json::Value;

/// 事件类型枚举
#[derive(Debug, Clone)]
pub enum EventType {
    TickPrice(Value),
    DepthUpdate(Value),
    Trade(Value),
    BookTicker(Value),
    Signal(Value),
    OrderRequest(Value),
    PositionUpdate(Value),
    OrderCancel(Value),
    OrderStopLoss(Value),
    OrderTakeProfit(Value),
    RiskEvent(Value),
    WebSocketError(String),
}

impl EventType {
    /// 获取事件类型的字符串表示
    pub fn type_name(&self) -> &'static str {
        match self {
            EventType::TickPrice(_) => "TickPrice",
            EventType::DepthUpdate(_) => "DepthUpdate",
            EventType::Trade(_) => "Trade",
            EventType::BookTicker(_) => "BookTicker",
            EventType::Signal(_) => "Signal",
            EventType::OrderRequest(_) => "OrderRequest",
            EventType::PositionUpdate(_) => "PositionUpdate",
            EventType::OrderCancel(_) => "OrderCancel",
            EventType::OrderStopLoss(_) => "OrderStopLoss",
            EventType::OrderTakeProfit(_) => "OrderTakeProfit",
            EventType::RiskEvent(_) => "RiskEvent",
            EventType::WebSocketError(_) => "WebSocketError",
        }
    }

    /// 检查是否为市场数据事件
    pub fn is_market_data(&self) -> bool {
        matches!(self, 
            EventType::TickPrice(_) | 
            EventType::DepthUpdate(_) | 
            EventType::Trade(_) | 
            EventType::BookTicker(_)
        )
    }

    /// 检查是否为交易事件
    pub fn is_trading_event(&self) -> bool {
        matches!(self,
            EventType::OrderRequest(_) |
            EventType::PositionUpdate(_) |
            EventType::OrderCancel(_) |
            EventType::OrderStopLoss(_) |
            EventType::OrderTakeProfit(_)
        )
    }

    /// 检查是否为信号事件
    pub fn is_signal(&self) -> bool {
        matches!(self, EventType::Signal(_))
    }

    /// 检查是否为错误事件
    pub fn is_error(&self) -> bool {
        matches!(self, EventType::WebSocketError(_))
    }
}

/// 事件结构体
#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub timestamp: u64,
    pub source: String,
    pub priority: EventPriority,
}

/// 事件优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl EventPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventPriority::Critical => "Critical",
            EventPriority::High => "High",
            EventPriority::Normal => "Normal",
            EventPriority::Low => "Low",
        }
    }
}

impl Event {
    pub fn new(event_type: EventType, source: String) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let priority = match &event_type {
            EventType::WebSocketError(_) | EventType::RiskEvent(_) => EventPriority::Critical,
            EventType::OrderRequest(_) | EventType::OrderCancel(_) | 
            EventType::OrderStopLoss(_) | EventType::OrderTakeProfit(_) => EventPriority::High,
            EventType::Signal(_) => EventPriority::High,
            EventType::TickPrice(_) | EventType::Trade(_) => EventPriority::Normal,
            EventType::DepthUpdate(_) | EventType::BookTicker(_) => EventPriority::Normal,
            EventType::PositionUpdate(_) => EventPriority::Normal,
        };

        Self {
            event_type,
            timestamp,
            source,
            priority,
        }
    }

    pub fn with_priority(mut self, priority: EventPriority) -> Self {
        self.priority = priority;
        self
    }

    /// 检查事件是否过期（基于时间戳）
    pub fn is_expired(&self, max_age_ms: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        now.saturating_sub(self.timestamp) > max_age_ms
    }
}
