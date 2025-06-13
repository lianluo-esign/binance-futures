
use crate::ringbuffer::{Event, EventType, TimerType, EventRingBuffer, TimerManager};
use crate::signal::SignalEventType;

// 扩展 EventType 枚举
#[derive(Clone, Debug)]
pub enum EventType {
    WebSocketData(Value),
    WebSocketConnected,
    WebSocketDisconnected,
    Timer(TimerType),
    Signal(SignalEventType),
    Render,
    Quit,
}

// 事件处理器特征
pub trait EventHandler {
    fn handle_event(&mut self, event: Event) -> Result<(), Box<dyn std::error::Error>>;
}