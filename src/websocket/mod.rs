pub mod manager;
pub mod connection;
pub mod exchange_trait;
pub mod exchanges;
pub mod lock_free_threaded_multi_exchange_manager;

pub use manager::WebSocketManager;
pub use connection::{WebSocketConnection, WebSocketConfig, ConnectionStats};
pub use exchange_trait::{
    ExchangeWebSocketManager, ExchangeConfig, ExchangeConnectionState, 
    ExchangeStats, StandardizedMarketData
};
pub use exchanges::okx::OkxWebSocketManager;
pub use exchanges::bybit::BybitWebSocketManager;
pub use lock_free_threaded_multi_exchange_manager::{
    LockFreeThreadedMultiExchangeManager, LockFreeThreadedMultiExchangeManagerBuilder,
    LockFreeThreadedMultiExchangeConfig, ExchangeType, create_lock_free_threaded_manager
};
