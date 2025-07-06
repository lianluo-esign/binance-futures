pub mod manager;
pub mod connection;
pub mod exchange_trait;
pub mod exchanges;
pub mod multi_exchange_manager;

pub use manager::WebSocketManager;
pub use connection::{WebSocketConnection, WebSocketConfig, ConnectionStats};
pub use exchange_trait::{ExchangeWebSocketManager, ExchangeConfig, ExchangeConnectionState, ExchangeStats};
pub use exchanges::okx::OkxWebSocketManager;
pub use exchanges::bybit::BybitWebSocketManager;
pub use multi_exchange_manager::{
    MultiExchangeManager, MultiExchangeManagerBuilder, MultiExchangeConfig, 
    MultiExchangeStats, ExchangeType
};
