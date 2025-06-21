use std::io;
use std::net::TcpStream;
use tungstenite::{
    client::IntoClientRequest,
    stream::MaybeTlsStream,
    Message, WebSocket,
};

/// WebSocket连接配置
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub symbol: String,
    pub streams: Vec<String>,
    pub reconnect_attempts: u32,
    pub max_reconnect_attempts: u32,
    pub reconnect_delay_ms: u64,
    pub ping_interval_ms: u64,
    pub connection_timeout_ms: u64,
}

impl WebSocketConfig {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol: symbol.clone(),
            streams: vec![
                format!("{}@depth20@100ms", symbol.to_lowercase()),
                format!("{}@trade", symbol.to_lowercase()),
                format!("{}@bookTicker", symbol.to_lowercase()),
            ],
            reconnect_attempts: 0,
            max_reconnect_attempts: 5,
            reconnect_delay_ms: 1000,
            ping_interval_ms: 30000,
            connection_timeout_ms: 10000,
        }
    }

    pub fn build_url(&self) -> String {
        format!(
            "wss://fstream.binance.com/stream?streams={}",
            self.streams.join("/")
        )
    }
}

/// WebSocket连接状态
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// 连接统计信息
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub state: ConnectionState,
    pub total_messages_received: u64,
    pub total_bytes_received: u64,
    pub connection_duration: Option<std::time::Duration>,
    pub reconnect_attempts: u32,
    pub max_reconnect_attempts: u32,
    pub last_error: Option<String>,
    pub last_ping_elapsed: std::time::Duration,
    pub last_pong_elapsed: std::time::Duration,
    pub reconnect_scheduled: bool,
}

/// WebSocket连接包装器
pub struct WebSocketConnection {
    socket: Option<WebSocket<MaybeTlsStream<TcpStream>>>,
    config: WebSocketConfig,
    state: ConnectionState,
    last_ping: std::time::Instant,
    last_pong: std::time::Instant,
    connection_start: Option<std::time::Instant>,
    total_messages_received: u64,
    total_bytes_received: u64,
    last_error: Option<String>,
    // 非阻塞重连相关字段
    last_reconnect_attempt: Option<std::time::Instant>,
    reconnect_scheduled: bool,
}

impl WebSocketConnection {
    pub fn new(config: WebSocketConfig) -> Self {
        Self {
            socket: None,
            config,
            state: ConnectionState::Disconnected,
            last_ping: std::time::Instant::now(),
            last_pong: std::time::Instant::now(),
            connection_start: None,
            total_messages_received: 0,
            total_bytes_received: 0,
            last_error: None,
            last_reconnect_attempt: None,
            reconnect_scheduled: false,
        }
    }

    /// 建立WebSocket连接
    pub fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.state = ConnectionState::Connecting;

        let url = self.config.build_url();
        // 连接信息写入日志文件，不输出到控制台
        log::info!("正在连接到: {}", url);
        log::info!("订阅的流: {:?}", self.config.streams);

        let request = url.into_client_request()?;
        let (socket, response) = tungstenite::client::connect(request)?;

        log::info!("WebSocket连接响应状态: {}", response.status());

        // 设置非阻塞模式
        self.set_nonblocking(&socket)?;

        self.socket = Some(socket);
        self.state = ConnectionState::Connected;
        self.connection_start = Some(std::time::Instant::now());
        self.config.reconnect_attempts = 0;
        self.last_error = None;

        // 连接成功信息写入日志文件，不输出到控制台
        log::info!("WebSocket连接成功: {} - 开始监听消息", self.config.symbol);
        Ok(())
    }

    /// 设置非阻塞模式
    fn set_nonblocking(&self, socket: &WebSocket<MaybeTlsStream<TcpStream>>) -> Result<(), Box<dyn std::error::Error>> {
        let stream = socket.get_ref();
        match stream {
            MaybeTlsStream::Plain(tcp_stream) => {
                tcp_stream.set_nonblocking(true)?;
            }
            MaybeTlsStream::NativeTls(tls_stream) => {
                tls_stream.get_ref().set_nonblocking(true)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// 读取消息
    pub fn read_message(&mut self) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        if let Some(ref mut socket) = self.socket {
            match socket.read() {
                Ok(message) => {
                    self.total_messages_received += 1;

                    match &message {
                        Message::Text(text) => {
                            self.total_bytes_received += text.len() as u64;
                            // 添加调试日志，但限制频率
                            if self.total_messages_received % 100 == 1 {
                                log::info!("收到文本消息 #{}: {} 字节", self.total_messages_received, text.len());
                            }
                        }
                        Message::Binary(data) => {
                            self.total_bytes_received += data.len() as u64;
                            log::info!("收到二进制消息: {} 字节", data.len());
                        }
                        Message::Pong(_) => {
                            self.last_pong = std::time::Instant::now();
                            log::debug!("收到Pong消息");
                        }
                        Message::Close(_) => {
                            self.state = ConnectionState::Disconnected;
                            // 连接关闭信息写入日志文件，不输出到控制台
                            log::warn!("WebSocket连接已关闭");
                        }
                        Message::Ping(_) => {
                            log::debug!("收到Ping消息");
                        }
                        _ => {
                            log::debug!("收到其他类型消息");
                        }
                    }

                    Ok(Some(message))
                }
                Err(tungstenite::Error::Io(ref e)) if e.kind() == io::ErrorKind::WouldBlock => {
                    // 非阻塞模式下没有数据可读
                    Ok(None)
                }
                Err(e) => {
                    self.state = ConnectionState::Failed;
                    self.last_error = Some(e.to_string());
                    Err(Box::new(e))
                }
            }
        } else {
            Ok(None)
        }
    }

    /// 发送消息
    pub fn send_message(&mut self, message: Message) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut socket) = self.socket {
            socket.send(message)?;
            Ok(())
        } else {
            Err("WebSocket未连接".into())
        }
    }

    /// 发送Ping消息
    pub fn send_ping(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_ping).as_millis() > self.config.ping_interval_ms as u128 {
            self.send_message(Message::Ping(vec![]))?;
            self.last_ping = now;
            // Ping消息不输出到控制台，避免干扰UI
        }
        Ok(())
    }

    /// 检查连接健康状态 - 增强版本
    pub fn check_health(&mut self) -> bool {
        let now = std::time::Instant::now();

        // 检查连接状态
        match self.state {
            ConnectionState::Connected => {
                // 检查Pong响应超时
                let pong_timeout_ms = self.config.ping_interval_ms * 3;
                if now.duration_since(self.last_pong).as_millis() > pong_timeout_ms as u128 {
                    // Pong超时警告写入日志文件，不输出到控制台
                    log::warn!("Pong响应超时 ({}ms)，连接可能已断开", pong_timeout_ms);
                    self.state = ConnectionState::Failed;
                    self.last_error = Some("Pong响应超时".to_string());
                    return false;
                }

                // 检查是否长时间没有收到消息
                if let Some(connection_start) = self.connection_start {
                    let connection_duration = now.duration_since(connection_start);
                    if connection_duration.as_secs() > 10 && self.total_messages_received == 0 {
                        log::warn!("连接已建立{}秒但未收到任何消息", connection_duration.as_secs());
                        self.state = ConnectionState::Failed;
                        self.last_error = Some("长时间未收到消息".to_string());
                        return false;
                    }
                }

                true
            }
            ConnectionState::Connecting => {
                // 检查连接超时
                if let Some(connection_start) = self.connection_start {
                    if now.duration_since(connection_start).as_millis() > self.config.connection_timeout_ms as u128 {
                        log::warn!("连接超时 ({}ms)", self.config.connection_timeout_ms);
                        self.state = ConnectionState::Failed;
                        self.last_error = Some("连接超时".to_string());
                        return false;
                    }
                }
                false // 连接中，暂时不可用
            }
            ConnectionState::Reconnecting => {
                // 重连状态下不可用，但不是错误
                false
            }
            ConnectionState::Failed | ConnectionState::Disconnected => {
                false
            }
        }
    }

    /// 断开连接
    pub fn disconnect(&mut self) {
        if let Some(mut socket) = self.socket.take() {
            let _ = socket.close(None);
        }
        self.state = ConnectionState::Disconnected;
        self.connection_start = None;
        // 连接断开信息写入日志文件，不输出到控制台
        log::info!("WebSocket连接已断开");
    }

    /// 尝试重连 - 非阻塞版本
    pub fn attempt_reconnect(&mut self) -> bool {
        if self.config.reconnect_attempts >= self.config.max_reconnect_attempts {
            // 重连失败错误写入日志文件，不输出到控制台
            log::error!("重连次数已达上限: {}", self.config.max_reconnect_attempts);
            self.reconnect_scheduled = false;
            return false;
        }

        let now = std::time::Instant::now();

        // 检查是否需要调度重连
        if !self.reconnect_scheduled {
            self.reconnect_scheduled = true;
            self.last_reconnect_attempt = Some(now);
            self.state = ConnectionState::Reconnecting;

            // 重连调度信息写入日志文件，不输出到控制台
            log::info!("调度重连 ({}/{}), 将在{}ms后执行",
                self.config.reconnect_attempts + 1,
                self.config.max_reconnect_attempts,
                self.config.reconnect_delay_ms);
            return false; // 还未到重连时间
        }

        // 检查是否到了重连时间
        if let Some(last_attempt) = self.last_reconnect_attempt {
            if now.duration_since(last_attempt).as_millis() < self.config.reconnect_delay_ms as u128 {
                return false; // 还未到重连时间
            }
        }

        // 执行重连
        self.config.reconnect_attempts += 1;
        self.reconnect_scheduled = false;

        // 重连尝试信息写入日志文件，不输出到控制台
        log::info!("执行重连 ({}/{})", self.config.reconnect_attempts, self.config.max_reconnect_attempts);

        match self.connect() {
            Ok(()) => {
                // 重连成功信息写入日志文件，不输出到控制台
                log::info!("重连成功");
                self.last_reconnect_attempt = None;
                true
            }
            Err(e) => {
                // 重连失败错误写入日志文件，不输出到控制台
                log::error!("重连失败: {}", e);
                self.last_error = Some(e.to_string());
                self.last_reconnect_attempt = Some(now); // 重新调度下次重连
                self.reconnect_scheduled = true;
                false
            }
        }
    }

    // Getter方法
    pub fn state(&self) -> &ConnectionState {
        &self.state
    }

    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }

    pub fn should_reconnect(&self) -> bool {
        (matches!(self.state, ConnectionState::Failed | ConnectionState::Disconnected) ||
         (self.state == ConnectionState::Reconnecting && self.reconnect_scheduled)) &&
        self.config.reconnect_attempts < self.config.max_reconnect_attempts
    }

    pub fn connection_duration(&self) -> Option<std::time::Duration> {
        self.connection_start.map(|start| start.elapsed())
    }

    /// 获取详细的连接统计信息
    pub fn get_connection_stats(&self) -> ConnectionStats {
        ConnectionStats {
            state: self.state.clone(),
            total_messages_received: self.total_messages_received,
            total_bytes_received: self.total_bytes_received,
            connection_duration: self.connection_duration(),
            reconnect_attempts: self.config.reconnect_attempts,
            max_reconnect_attempts: self.config.max_reconnect_attempts,
            last_error: self.last_error.clone(),
            last_ping_elapsed: self.last_ping.elapsed(),
            last_pong_elapsed: self.last_pong.elapsed(),
            reconnect_scheduled: self.reconnect_scheduled,
        }
    }

    /// 重置连接统计信息
    pub fn reset_stats(&mut self) {
        self.total_messages_received = 0;
        self.total_bytes_received = 0;
        self.config.reconnect_attempts = 0;
        self.last_error = None;
        self.last_reconnect_attempt = None;
        self.reconnect_scheduled = false;
    }

    pub fn stats(&self) -> ConnectionStats {
        ConnectionStats {
            state: self.state.clone(),
            total_messages_received: self.total_messages_received,
            total_bytes_received: self.total_bytes_received,
            connection_duration: self.connection_duration(),
            reconnect_attempts: self.config.reconnect_attempts,
            max_reconnect_attempts: self.config.max_reconnect_attempts,
            last_error: self.last_error.clone(),
            last_ping_elapsed: self.last_ping.elapsed(),
            last_pong_elapsed: self.last_pong.elapsed(),
            reconnect_scheduled: self.reconnect_scheduled,
        }
    }
}


