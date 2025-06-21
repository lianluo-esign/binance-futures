use super::connection::{WebSocketConnection, WebSocketConfig, ConnectionStats};
use serde_json::Value;
use tungstenite::Message;

/// WebSocket管理器 - 高级接口
pub struct WebSocketManager {
    connection: WebSocketConnection,
    message_buffer: Vec<Value>,
    stats: ManagerStats,
}

#[derive(Debug, Clone, Default)]
pub struct ManagerStats {
    pub total_json_messages: u64,
    pub json_parse_errors: u64,
    pub messages_buffered: u64,
    pub last_message_time: Option<u64>,
    pub connection_errors: u64,
    pub consecutive_errors: u32,
    pub ping_errors: u64,
    pub total_reconnects: u64,
}

/// WebSocket健康状态
#[derive(Debug, Clone)]
pub struct WebSocketHealthStatus {
    pub is_healthy: bool,
    pub is_connected: bool,
    pub consecutive_errors: u32,
    pub total_errors: u64,
    pub total_reconnects: u64,
    pub last_error: Option<String>,
    pub connection_duration: Option<std::time::Duration>,
    pub messages_per_second: f64,
}

impl WebSocketManager {
    pub fn new(config: WebSocketConfig) -> Self {
        Self {
            connection: WebSocketConnection::new(config),
            message_buffer: Vec::new(),
            stats: ManagerStats::default(),
        }
    }

    /// 连接到WebSocket
    pub fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connection.connect()
    }

    /// 断开WebSocket连接
    pub fn disconnect(&mut self) {
        self.connection.disconnect();
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        self.connection.is_connected()
    }

    /// 检查是否应该重连
    pub fn should_reconnect(&self) -> bool {
        self.connection.should_reconnect()
    }

    /// 尝试重连 - 增强错误处理
    pub fn attempt_reconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.connection.attempt_reconnect() {
            self.stats.total_reconnects += 1;
            self.stats.consecutive_errors = 0; // 重连成功后重置错误计数
            log::info!("WebSocket重连成功 (总重连次数: {})", self.stats.total_reconnects);
            Ok(())
        } else {
            Err("重连失败".into())
        }
    }

    /// 读取消息并解析为JSON - 增强错误处理
    pub fn read_messages(&mut self) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        self.message_buffer.clear();

        // 发送ping保持连接 - 增强错误处理
        if let Err(e) = self.connection.send_ping() {
            self.stats.ping_errors += 1;
            log::warn!("发送ping失败 (总计: {}次): {}", self.stats.ping_errors, e);
            // ping失败不应该中断消息读取
        }

        // 检查连接健康状态
        if !self.connection.check_health() {
            // 连接不健康时返回空消息列表，让上层处理重连
            return Ok(vec![]);
        }

        // 读取所有可用消息 - 增强错误处理和恢复机制
        let mut consecutive_errors = 0;
        const MAX_CONSECUTIVE_ERRORS: u32 = 5;

        loop {
            match self.connection.read_message() {
                Ok(Some(message)) => {
                    consecutive_errors = 0; // 重置错误计数
                    self.stats.consecutive_errors = 0;
                    if let Some(json_value) = self.process_message(message) {
                        self.message_buffer.push(json_value);
                        self.stats.messages_buffered += 1;
                    }
                }
                Ok(None) => break, // 没有更多消息
                Err(e) => {
                    consecutive_errors += 1;
                    self.stats.connection_errors += 1;
                    self.stats.consecutive_errors = consecutive_errors;
                    log::warn!("读取WebSocket消息时出错 (连续: {}, 总计: {}): {}",
                        consecutive_errors, self.stats.connection_errors, e);

                    // 如果连续错误过多，停止读取以避免无限循环
                    if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                        log::error!("连续错误过多 ({}次)，停止读取消息", consecutive_errors);
                        // 标记连接为失败状态，触发重连
                        self.connection.disconnect();
                        break;
                    }

                    // 不使用阻塞延迟，直接跳出循环让主循环处理
                }
            }
        }

        Ok(self.message_buffer.clone())
    }

    /// 处理单个WebSocket消息
    fn process_message(&mut self, message: Message) -> Option<Value> {
        match message {
            Message::Text(text) => {
                self.stats.total_json_messages += 1;
                self.stats.last_message_time = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64
                );

                // 添加调试日志，限制频率
                if self.stats.total_json_messages % 50 == 1 {
                    log::info!("处理JSON消息 #{}: {} 字符", self.stats.total_json_messages, text.len());
                    if text.len() < 200 {
                        log::debug!("消息内容: {}", text);
                    }
                }

                match serde_json::from_str::<Value>(&text) {
                    Ok(json_value) => Some(json_value),
                    Err(e) => {
                        self.stats.json_parse_errors += 1;
                        // JSON解析错误写入日志文件，不输出到控制台
                        log::error!("JSON解析错误: {} - 原始数据: {}", e, text);
                        None
                    }
                }
            }
            Message::Ping(payload) => {
                // 自动响应ping
                let _ = self.connection.send_message(Message::Pong(payload));
                None
            }
            Message::Pong(_) => {
                // Pong消息已在connection层处理
                None
            }
            Message::Close(frame) => {
                // 关闭消息写入日志文件，不输出到控制台
                log::warn!("收到关闭消息: {:?}", frame);
                None
            }
            Message::Binary(_) => {
                // 二进制消息不输出到控制台
                None
            }
            Message::Frame(_) => {
                // 原始帧不输出到控制台
                None
            }
        }
    }

    /// 发送消息
    pub fn send_message(&mut self, message: Message) -> Result<(), Box<dyn std::error::Error>> {
        self.connection.send_message(message)
    }

    /// 发送JSON消息
    pub fn send_json(&mut self, json: &Value) -> Result<(), Box<dyn std::error::Error>> {
        let text = serde_json::to_string(json)?;
        self.send_message(Message::Text(text))
    }

    /// 订阅新的数据流
    pub fn subscribe(&mut self, streams: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        let subscribe_message = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": streams,
            "id": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        });

        self.send_json(&subscribe_message)
    }

    /// 取消订阅数据流
    pub fn unsubscribe(&mut self, streams: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        let unsubscribe_message = serde_json::json!({
            "method": "UNSUBSCRIBE",
            "params": streams,
            "id": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        });

        self.send_json(&unsubscribe_message)
    }

    /// 获取连接统计信息
    pub fn get_stats(&self) -> Option<ConnectionStats> {
        Some(self.connection.stats())
    }

    /// 获取管理器统计信息
    pub fn get_manager_stats(&self) -> &ManagerStats {
        &self.stats
    }

    /// 获取连接统计信息
    pub fn get_connection_stats(&self) -> super::connection::ConnectionStats {
        self.connection.get_connection_stats()
    }

    /// 获取综合健康状态
    pub fn get_health_status(&self) -> WebSocketHealthStatus {
        let connection_stats = self.connection.get_connection_stats();
        let is_healthy = self.is_connected() &&
                        self.stats.consecutive_errors < 3 &&
                        connection_stats.last_pong_elapsed.as_secs() < 60;

        WebSocketHealthStatus {
            is_healthy,
            is_connected: self.is_connected(),
            consecutive_errors: self.stats.consecutive_errors,
            total_errors: self.stats.connection_errors,
            total_reconnects: self.stats.total_reconnects,
            last_error: connection_stats.last_error,
            connection_duration: connection_stats.connection_duration,
            messages_per_second: self.calculate_message_rate(),
        }
    }

    /// 计算消息接收速率
    fn calculate_message_rate(&self) -> f64 {
        if let Some(last_message_time) = self.stats.last_message_time {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            let time_diff = now.saturating_sub(last_message_time);
            if time_diff > 0 && time_diff < 60000 { // 在过去1分钟内
                (self.stats.total_json_messages as f64) / (time_diff as f64 / 1000.0)
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// 重置统计信息
    pub fn reset_stats(&mut self) {
        self.stats = ManagerStats::default();
    }

    /// 获取连接状态描述
    pub fn get_status_description(&self) -> String {
        let conn_stats = self.connection.stats();
        
        format!(
            "状态: {:?}, 消息: {}, 字节: {}, 重连: {}/{}",
            conn_stats.state,
            conn_stats.total_messages_received,
            conn_stats.total_bytes_received,
            conn_stats.reconnect_attempts,
            5 // max_reconnect_attempts from config
        )
    }

    /// 获取延迟信息
    pub fn get_latency_info(&self) -> Option<std::time::Duration> {
        self.connection.connection_duration()
    }

    /// 检查消息处理性能
    pub fn check_performance(&self) -> PerformanceInfo {
        let conn_stats = self.connection.stats();
        let manager_stats = &self.stats;
        
        let messages_per_second = if let Some(duration) = conn_stats.connection_duration {
            if duration.as_secs() > 0 {
                conn_stats.total_messages_received as f64 / duration.as_secs_f64()
            } else {
                0.0
            }
        } else {
            0.0
        };

        let bytes_per_second = if let Some(duration) = conn_stats.connection_duration {
            if duration.as_secs() > 0 {
                conn_stats.total_bytes_received as f64 / duration.as_secs_f64()
            } else {
                0.0
            }
        } else {
            0.0
        };

        PerformanceInfo {
            messages_per_second,
            bytes_per_second,
            json_parse_error_rate: if manager_stats.total_json_messages > 0 {
                manager_stats.json_parse_errors as f64 / manager_stats.total_json_messages as f64
            } else {
                0.0
            },
            connection_uptime: conn_stats.connection_duration,
        }
    }
}

/// 性能信息
#[derive(Debug, Clone)]
pub struct PerformanceInfo {
    pub messages_per_second: f64,
    pub bytes_per_second: f64,
    pub json_parse_error_rate: f64,
    pub connection_uptime: Option<std::time::Duration>,
}
