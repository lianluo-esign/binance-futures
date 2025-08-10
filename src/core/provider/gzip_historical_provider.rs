// Historical Data Provider - 历史数据文件提供者
//
// 本文件实现了历史数据文件的Provider，负责：
// - 读取本地历史数据文件
// - 支持多种格式（CSV、JSON、二进制等）
// - 可控播放速度和时间跳转
// - 支持回测模式
// - 提供播放控制功能
//
// 设计原则：
// 1. 格式无关：支持多种历史数据格式
// 2. 播放控制：支持暂停、恢复、跳转、调速
// 3. 内存优化：流式读取，避免一次性加载大文件
// 4. 时间准确：严格按照历史时间戳播放

use super::{
    DataProvider, ControllableProvider, ProviderType, ProviderStatus, EventKind, 
    PerformanceMetrics, PlaybackInfo, HistoricalDataFormat,
    error::{ProviderError, ProviderResult},
};
use crate::events::EventType;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::Instant;

/// 历史数据Provider配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataConfig {
    /// 数据文件路径
    pub file_path: PathBuf,
    
    /// 数据格式
    pub format: HistoricalDataFormat,
    
    /// 播放配置
    pub playback_config: PlaybackConfig,
    
    /// 缓冲配置
    pub buffer_config: BufferConfig,
    
    /// 时间配置
    pub time_config: TimeConfig,
}

impl Default for HistoricalDataConfig {
    fn default() -> Self {
        Self {
            file_path: PathBuf::from("data/historical_data.json"),
            format: HistoricalDataFormat::JSON,
            playback_config: PlaybackConfig::default(),
            buffer_config: BufferConfig::default(),
            time_config: TimeConfig::default(),
        }
    }
}

/// 播放控制配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackConfig {
    /// 初始播放速度倍数
    pub initial_speed: f64,
    
    /// 是否自动开始播放
    pub auto_start: bool,
    
    /// 是否循环播放
    pub loop_enabled: bool,
    
    /// 最大播放速度
    pub max_speed: f64,
    
    /// 最小播放速度
    pub min_speed: f64,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            initial_speed: 1.0,
            auto_start: true,
            loop_enabled: false,
            max_speed: 100.0,
            min_speed: 0.1,
        }
    }
}

/// 缓冲配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferConfig {
    /// 事件缓冲区大小
    pub event_buffer_size: usize,
    
    /// 预读取行数
    pub prefetch_lines: usize,
    
    /// 内存限制（MB）
    pub memory_limit_mb: usize,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            event_buffer_size: 1000,
            prefetch_lines: 100,
            memory_limit_mb: 100,
        }
    }
}

/// 时间配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeConfig {
    /// 时间戳字段名
    pub timestamp_field: String,
    
    /// 时间戳格式
    pub timestamp_format: TimestampFormat,
    
    /// 时区偏移（小时）
    pub timezone_offset: i8,
    
    /// 起始时间（可选，用于过滤）
    pub start_time: Option<u64>,
    
    /// 结束时间（可选，用于过滤）
    pub end_time: Option<u64>,
}

impl Default for TimeConfig {
    fn default() -> Self {
        Self {
            timestamp_field: "timestamp".to_string(),
            timestamp_format: TimestampFormat::Milliseconds,
            timezone_offset: 0,
            start_time: None,
            end_time: None,
        }
    }
}

/// 时间戳格式
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TimestampFormat {
    /// 毫秒时间戳
    Milliseconds,
    /// 秒时间戳
    Seconds,
    /// ISO 8601字符串格式
    ISO8601,
    /// 自定义格式字符串
    Custom,
}

/// 历史数据记录
#[derive(Debug, Clone)]
pub struct HistoricalRecord {
    /// 时间戳（毫秒）
    pub timestamp: u64,
    /// 原始数据
    pub data: Value,
    /// 事件类型
    pub event_type: EventKind,
}

/// 播放状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackState {
    /// 停止
    Stopped,
    /// 播放中
    Playing,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 错误状态
    Error,
}

/// 历史数据Provider实现
pub struct HistoricalDataProvider {
    /// 配置
    config: HistoricalDataConfig,
    
    /// 文件读取器
    file_reader: Option<BufReader<File>>,
    
    /// 事件缓冲区
    event_buffer: VecDeque<HistoricalRecord>,
    
    /// Provider状态
    status: ProviderStatus,
    
    /// 播放状态
    playback_state: PlaybackState,
    
    /// 播放信息
    playback_info: PlaybackInfo,
    
    /// 支持的事件类型
    supported_events: Vec<EventKind>,
    
    /// 统计信息
    total_records_read: u64,
    current_position: u64,
    events_sent: u64,
    
    /// 时间控制
    last_event_timestamp: Option<u64>,
    playback_start_time: Option<Instant>,
    virtual_time_offset: u64,
    
    /// 性能监控
    performance_window_start: Instant,
    performance_events_count: u64,
    
    /// 文件信息
    file_size: u64,
    estimated_total_records: u64,
}

impl std::fmt::Debug for HistoricalDataProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoricalDataProvider")
            .field("config", &self.config)
            .field("playback_state", &self.playback_state)
            .field("playback_info", &self.playback_info)
            .field("total_records_read", &self.total_records_read)
            .field("events_sent", &self.events_sent)
            .finish()
    }
}

impl HistoricalDataProvider {
    /// 创建新的历史数据Provider
    pub fn new(config: HistoricalDataConfig) -> Self {
        // 初始化播放信息
        let mut playback_info = PlaybackInfo::new(
            config.time_config.start_time.unwrap_or(0),
            config.time_config.end_time.unwrap_or(u64::MAX),
        );
        
        // 设置正确的初始播放速度
        playback_info.playback_speed = config.playback_config.initial_speed;

        // 初始化状态
        let provider_type = ProviderType::HistoricalData { 
            format: config.format 
        };
        let mut status = ProviderStatus::new(provider_type);
        
        // 设置历史数据指标
        status.provider_metrics = super::types::ProviderMetrics::Historical {
            file_progress: 0.0,
            playback_speed: config.playback_config.initial_speed,
            current_timestamp: 0,
            total_events: 0,
            processed_events: 0,
            file_path: config.file_path.to_string_lossy().to_string(),
        };

        // 默认支持的事件类型
        let supported_events = vec![
            EventKind::TickPrice,
            EventKind::DepthUpdate,
            EventKind::Trade,
            EventKind::BookTicker,
        ];

        let now = Instant::now();

        Self {
            config,
            file_reader: None,
            event_buffer: VecDeque::with_capacity(1000),
            status,
            playback_state: PlaybackState::Stopped,
            playback_info,
            supported_events,
            total_records_read: 0,
            current_position: 0,
            events_sent: 0,
            last_event_timestamp: None,
            playback_start_time: None,
            virtual_time_offset: 0,
            performance_window_start: now,
            performance_events_count: 0,
            file_size: 0,
            estimated_total_records: 0,
        }
    }

    /// 打开并初始化文件
    fn open_file(&mut self) -> ProviderResult<()> {
        let file = File::open(&self.config.file_path)
            .map_err(|e| ProviderError::configuration(
                format!("无法打开历史数据文件: {} - {}", 
                       self.config.file_path.display(), e)
            ))?;

        // 获取文件大小
        self.file_size = file.metadata()
            .map_err(|e| ProviderError::configuration(
                format!("无法获取文件元数据: {}", e)
            ))?
            .len();

        // 估算总记录数（粗略估计，每行平均100字节）
        self.estimated_total_records = self.file_size / 100;

        self.file_reader = Some(BufReader::new(file));
        
        log::info!("历史数据文件已打开: {} ({} bytes)", 
                  self.config.file_path.display(),
                  self.file_size);

        Ok(())
    }

    /// 从文件读取下一批记录
    fn read_next_batch(&mut self) -> ProviderResult<()> {
        let reader = self.file_reader.as_mut()
            .ok_or_else(|| ProviderError::state(
                "文件读取器未初始化",
                "uninitialized",
                "initialized", 
                "read_batch"
            ))?;

        let mut lines_read = 0;
        let mut line = String::new();

        while lines_read < self.config.buffer_config.prefetch_lines &&
              self.event_buffer.len() < self.config.buffer_config.event_buffer_size {
            
            line.clear();
            
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // 文件结束
                    if self.config.playback_config.loop_enabled {
                        // 重置到文件开头
                        reader.seek(SeekFrom::Start(0))
                            .map_err(|e| ProviderError::internal(
                                format!("文件重置失败: {}", e),
                                "HistoricalDataProvider"
                            ))?;
                        
                        self.current_position = 0;
                        self.virtual_time_offset = self.last_event_timestamp.unwrap_or(0);
                        
                        log::info!("历史数据文件重新开始播放");
                        continue;
                    } else {
                        // 播放完成
                        self.playback_state = PlaybackState::Completed;
                        break;
                    }
                }
                Ok(bytes_read) => {
                    self.current_position += bytes_read as u64;
                    
                    // 解析记录
                    let line_trimmed = line.trim();
                    let record_opt = if line_trimmed.is_empty() {
                        None
                    } else {
                        match self.config.format {
                            HistoricalDataFormat::JSON => {
                                match serde_json::from_str::<Value>(line_trimmed) {
                                    Ok(data) => {
                                        // 提取时间戳
                                        let timestamp = if let Some(timestamp_value) = data.get(&self.config.time_config.timestamp_field) {
                                            match self.config.time_config.timestamp_format {
                                                TimestampFormat::Milliseconds => {
                                                    timestamp_value.as_u64().unwrap_or(0)
                                                }
                                                TimestampFormat::Seconds => {
                                                    timestamp_value.as_u64().map(|t| t * 1000).unwrap_or(0)
                                                }
                                                _ => 0, // 其他格式暂未实现
                                            }
                                        } else {
                                            continue; // 跳过没有时间戳的记录
                                        };
                                        
                                        // 推断事件类型
                                        let event_type = if let Some(event_type) = data.get("e").and_then(|e| e.as_str()) {
                                            match event_type {
                                                "bookTicker" => EventKind::BookTicker,
                                                "depthUpdate" => EventKind::DepthUpdate,
                                                "trade" => EventKind::Trade,
                                                "kline" => EventKind::TickPrice,
                                                "24hrTicker" => EventKind::TickPrice,
                                                _ => EventKind::TickPrice,
                                            }
                                        } else if data.get("price").is_some() {
                                            EventKind::TickPrice
                                        } else if data.get("bids").is_some() || data.get("asks").is_some() {
                                            EventKind::DepthUpdate
                                        } else {
                                            EventKind::TickPrice
                                        };
                                        
                                        Some(HistoricalRecord {
                                            timestamp,
                                            data,
                                            event_type,
                                        })
                                    }
                                    Err(_) => {
                                        continue; // 跳过解析失败的行
                                    }
                                }
                            }
                            _ => {
                                continue; // 暂时跳过其他格式
                            }
                        }
                    };
                    
                    if let Some(record) = record_opt {
                        // 检查时间过滤
                        let should_include = {
                            // 检查时间过滤
                            if let Some(start_time) = self.config.time_config.start_time {
                                if record.timestamp < start_time {
                                    false
                                } else if let Some(end_time) = self.config.time_config.end_time {
                                    record.timestamp <= end_time
                                } else {
                                    true
                                }
                            } else if let Some(end_time) = self.config.time_config.end_time {
                                record.timestamp <= end_time
                            } else {
                                true
                            }
                        };
                        
                        if should_include {
                            self.event_buffer.push_back(record);
                        }
                    }
                    
                    lines_read += 1;
                    self.total_records_read += 1;
                }
                Err(e) => {
                    return Err(ProviderError::internal(
                        format!("读取文件失败: {}", e),
                        "HistoricalDataProvider"
                    ));
                }
            }
        }

        // 更新进度
        self.update_progress();
        
        Ok(())
    }

    /// 解析一行数据
    #[allow(dead_code)]
    fn parse_line(&self, line: &str) -> ProviderResult<Option<HistoricalRecord>> {
        if line.trim().is_empty() {
            return Ok(None);
        }

        match self.config.format {
            HistoricalDataFormat::JSON => self.parse_json_line(line),
            HistoricalDataFormat::CSV => self.parse_csv_line(line),
            HistoricalDataFormat::Binary => {
                Err(ProviderError::configuration(
                    "二进制格式暂未实现".to_string()
                ))
            }
            HistoricalDataFormat::Compressed => {
                Err(ProviderError::configuration(
                    "压缩格式暂未实现".to_string()
                ))
            }
        }
    }

    /// 解析JSON行
    #[allow(dead_code)]
    fn parse_json_line(&self, line: &str) -> ProviderResult<Option<HistoricalRecord>> {
        let data: Value = serde_json::from_str(line)
            .map_err(|e| ProviderError::data_parsing(
                format!("JSON解析失败: {}", e),
                "JSON"
            ))?;

        // 提取时间戳
        let timestamp = self.extract_timestamp(&data)?;
        
        // 推断事件类型
        let event_type = self.infer_event_type(&data);

        Ok(Some(HistoricalRecord {
            timestamp,
            data,
            event_type,
        }))
    }

    /// 解析CSV行
    #[allow(dead_code)]
    fn parse_csv_line(&self, _line: &str) -> ProviderResult<Option<HistoricalRecord>> {
        // TODO: 实现CSV解析
        // 这里需要根据具体的CSV格式进行解析
        Err(ProviderError::configuration(
            "CSV格式暂未实现".to_string()
        ))
    }

    /// 提取时间戳
    #[allow(dead_code)]
    fn extract_timestamp(&self, data: &Value) -> ProviderResult<u64> {
        let timestamp_value = data.get(&self.config.time_config.timestamp_field)
            .ok_or_else(|| ProviderError::data_parsing(
                format!("缺少时间戳字段: {}", self.config.time_config.timestamp_field),
                "timestamp"
            ))?;

        match self.config.time_config.timestamp_format {
            TimestampFormat::Milliseconds => {
                timestamp_value.as_u64()
                    .ok_or_else(|| ProviderError::data_parsing(
                        "时间戳格式错误，期望毫秒时间戳".to_string(),
                        "timestamp"
                    ))
            }
            TimestampFormat::Seconds => {
                timestamp_value.as_u64()
                    .map(|t| t * 1000) // 转换为毫秒
                    .ok_or_else(|| ProviderError::data_parsing(
                        "时间戳格式错误，期望秒时间戳".to_string(),
                        "timestamp"
                    ))
            }
            TimestampFormat::ISO8601 => {
                // TODO: 实现ISO8601解析
                Err(ProviderError::configuration(
                    "ISO8601格式暂未实现".to_string()
                ))
            }
            TimestampFormat::Custom => {
                // TODO: 实现自定义格式解析
                Err(ProviderError::configuration(
                    "自定义时间格式暂未实现".to_string()
                ))
            }
        }
    }

    /// 推断事件类型
    #[allow(dead_code)]
    fn infer_event_type(&self, data: &Value) -> EventKind {
        // 尝试从数据中推断事件类型
        if let Some(event_type) = data.get("e").and_then(|e| e.as_str()) {
            match event_type {
                "bookTicker" => EventKind::BookTicker,
                "depthUpdate" => EventKind::DepthUpdate,
                "trade" => EventKind::Trade,
                "kline" => EventKind::TickPrice,
                "24hrTicker" => EventKind::TickPrice,
                _ => EventKind::TickPrice, // 默认
            }
        } else if data.get("price").is_some() {
            EventKind::TickPrice
        } else if data.get("bids").is_some() || data.get("asks").is_some() {
            EventKind::DepthUpdate
        } else {
            EventKind::TickPrice // 默认
        }
    }


    /// 更新进度信息
    fn update_progress(&mut self) {
        let progress = if self.file_size > 0 {
            self.current_position as f64 / self.file_size as f64
        } else {
            0.0
        };

        // 更新播放信息
        if let Some(record) = self.event_buffer.back() {
            self.playback_info.update_timestamp(record.timestamp);
        }
        
        // 更新Provider状态的metrics
        self.status.provider_metrics = super::types::ProviderMetrics::Historical {
            file_progress: progress,
            playback_speed: self.playback_info.playback_speed,
            current_timestamp: self.playback_info.current_timestamp,
            total_events: self.estimated_total_records,
            processed_events: self.events_sent,
            file_path: self.config.file_path.to_string_lossy().to_string(),
        };
        
        // 更新自定义元数据
        use std::collections::HashMap;
        let mut metadata = HashMap::new();
        metadata.insert("file_path".to_string(), serde_json::Value::String(
            self.config.file_path.to_string_lossy().to_string()
        ));
        metadata.insert("file_size".to_string(), serde_json::Value::Number(
            serde_json::Number::from(self.file_size)
        ));
        metadata.insert("format".to_string(), serde_json::Value::String(
            format!("{:?}", self.config.format)
        ));
        metadata.insert("playback_state".to_string(), serde_json::Value::String(
            format!("{:?}", self.playback_state)
        ));
        self.status.custom_metadata = Some(metadata);

        // 更新Provider指标
        if let super::types::ProviderMetrics::Historical {
            ref mut file_progress,
            ref mut processed_events,
            ref mut total_events,
            ref mut current_timestamp,
            ..
        } = self.status.provider_metrics {
            *file_progress = progress;
            *processed_events = self.events_sent;
            *total_events = self.estimated_total_records;
            *current_timestamp = self.last_event_timestamp.unwrap_or(0);
        }

        self.status.update_timestamp();
    }

    /// 将历史记录转换为EventType
    fn convert_record_to_event(&self, record: &HistoricalRecord) -> EventType {
        match record.event_type {
            EventKind::BookTicker => EventType::BookTicker(record.data.clone()),
            EventKind::DepthUpdate => EventType::DepthUpdate(record.data.clone()),
            EventKind::Trade => EventType::Trade(record.data.clone()),
            EventKind::TickPrice => EventType::TickPrice(record.data.clone()),
            _ => EventType::TickPrice(record.data.clone()),
        }
    }

}

impl DataProvider for HistoricalDataProvider {
    type Error = ProviderError;

    fn initialize(&mut self) -> ProviderResult<()> {
        log::info!("初始化Historical Data Provider: {}", 
                  self.config.file_path.display());

        // 验证文件存在
        if !self.config.file_path.exists() {
            return Err(ProviderError::configuration(
                format!("历史数据文件不存在: {}", self.config.file_path.display())
            ));
        }

        // 打开文件
        self.open_file()?;

        // 预读取一些数据
        self.read_next_batch()?;

        self.status.is_running = false;
        self.playback_state = PlaybackState::Stopped;

        log::info!("Historical Data Provider初始化完成，预加载 {} 条记录", 
                  self.event_buffer.len());
        Ok(())
    }

    fn start(&mut self) -> ProviderResult<()> {
        log::info!("启动Historical Data Provider");

        if self.config.playback_config.auto_start {
            self.playback_state = PlaybackState::Playing;
            self.playback_start_time = Some(Instant::now());
        } else {
            self.playback_state = PlaybackState::Paused;
        }

        self.status.is_running = true;
        self.status.is_connected = true; // 历史数据总是"连接"的
        self.performance_window_start = Instant::now();

        log::info!("Historical Data Provider启动完成，播放状态: {:?}", 
                  self.playback_state);
        Ok(())
    }

    fn stop(&mut self) -> ProviderResult<()> {
        log::info!("停止Historical Data Provider");

        self.playback_state = PlaybackState::Stopped;
        self.status.is_running = false;
        self.playback_start_time = None;

        log::info!("Historical Data Provider已停止");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.file_reader.is_some() && self.status.is_running
    }

    fn read_events(&mut self) -> ProviderResult<Vec<EventType>> {
        if !matches!(self.playback_state, PlaybackState::Playing) {
            return Ok(vec![]); // 非播放状态返回空列表
        }

        let mut events = Vec::new();

        // 从缓冲区读取准备好的事件
        loop {
            // 检查是否有可用的记录
            let should_send = if let Some(record) = self.event_buffer.front() {
                // 检查是否应该发送事件（基于播放速度）
                if let Some(last_timestamp) = self.last_event_timestamp {
                    if let Some(start_time) = self.playback_start_time {
                        let real_time_elapsed = start_time.elapsed();
                        let virtual_time_elapsed = ((record.timestamp - last_timestamp) as f64 / self.playback_info.playback_speed) as u64;
                        
                        real_time_elapsed.as_millis() as u64 >= virtual_time_elapsed
                    } else {
                        true
                    }
                } else {
                    true // 第一个事件立即发送
                }
            } else {
                break; // 没有更多记录
            };

            if should_send {
                let record = self.event_buffer.pop_front().unwrap();
                let event = self.convert_record_to_event(&record);
                events.push(event);

                // 更新统计
                self.events_sent += 1;
                self.last_event_timestamp = Some(record.timestamp);
                self.performance_events_count += 1;
                self.status.record_event();
            } else {
                break; // 还没到发送时间
            }
        }

        // 如果缓冲区快空了，尝试读取更多数据
        if self.event_buffer.len() < self.config.buffer_config.prefetch_lines / 2 {
            if let Err(e) = self.read_next_batch() {
                log::warn!("读取历史数据失败: {}", e);
                self.playback_state = PlaybackState::Error;
                return Err(e);
            }
        }

        // 检查是否播放完成
        if self.playback_state == PlaybackState::Completed && self.event_buffer.is_empty() {
            log::info!("历史数据播放完成");
        }

        self.update_progress();

        Ok(events)
    }

    fn get_status(&self) -> ProviderStatus {
        self.status.clone()
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::HistoricalData { format: self.config.format }
    }

    fn supported_events(&self) -> &[EventKind] {
        &self.supported_events
    }

    fn get_config_info(&self) -> Option<String> {
        Some(format!(
            "File: {}, Format: {:?}, Speed: {:.1}x, Records: {}",
            self.config.file_path.display(),
            self.config.format,
            self.playback_info.playback_speed,
            self.total_records_read
        ))
    }

    fn health_check(&self) -> bool {
        self.file_reader.is_some() && 
        !matches!(self.playback_state, PlaybackState::Error)
    }

    fn get_performance_metrics(&self) -> Option<PerformanceMetrics> {
        let window_duration = self.performance_window_start.elapsed();
        if window_duration.as_secs() == 0 {
            return None;
        }

        let events_per_second = self.performance_events_count as f64 / window_duration.as_secs_f64();

        Some(PerformanceMetrics {
            events_received: self.events_sent,
            last_event_time: self.last_event_timestamp,
            error_count: 0, // 历史数据很少出错
            events_per_second,
            bytes_per_second: 0.0, // 文件读取不需要字节统计
            average_latency_ms: 0.0,
            max_latency_ms: 0.0,
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0.0,
            error_rate: 0.0, // 历史数据很少出错
            window_seconds: window_duration.as_secs(),
        })
    }
}

impl ControllableProvider for HistoricalDataProvider {
    fn pause(&mut self) -> ProviderResult<()> {
        if matches!(self.playback_state, PlaybackState::Playing) {
            self.playback_state = PlaybackState::Paused;
            log::info!("历史数据播放已暂停");
        }
        Ok(())
    }

    fn resume(&mut self) -> ProviderResult<()> {
        if matches!(self.playback_state, PlaybackState::Paused) {
            self.playback_state = PlaybackState::Playing;
            self.playback_start_time = Some(Instant::now());
            log::info!("历史数据播放已恢复");
        }
        Ok(())
    }

    fn set_playback_speed(&mut self, speed: f64) -> ProviderResult<()> {
        if speed < self.config.playback_config.min_speed || 
           speed > self.config.playback_config.max_speed {
            return Err(ProviderError::configuration(
                format!("播放速度超出范围 [{}, {}]: {}", 
                       self.config.playback_config.min_speed,
                       self.config.playback_config.max_speed,
                       speed)
            ));
        }

        self.playback_info.playback_speed = speed;
        
        // 重置播放时间基准
        self.playback_start_time = Some(Instant::now());
        
        log::info!("播放速度已设置为 {:.1}x", speed);
        Ok(())
    }

    fn seek_to(&mut self, _timestamp: u64) -> ProviderResult<()> {
        // TODO: 实现文件定位功能
        // 这需要预先建立时间戳索引或者从头扫描文件
        Err(ProviderError::configuration(
            "时间跳转功能暂未实现".to_string()
        ))
    }

    fn get_playback_info(&self) -> Option<PlaybackInfo> {
        Some(self.playback_info.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_json_file() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        
        writeln!(file, r#"{{"timestamp": 1640995200000, "price": 47000.0, "volume": 1.5, "e": "trade"}}"#).unwrap();
        writeln!(file, r#"{{"timestamp": 1640995201000, "price": 47001.0, "volume": 2.0, "e": "trade"}}"#).unwrap();
        writeln!(file, r#"{{"timestamp": 1640995202000, "price": 47002.0, "volume": 1.0, "e": "trade"}}"#).unwrap();
        
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_historical_provider_creation() {
        let config = HistoricalDataConfig::default();
        let provider = HistoricalDataProvider::new(config);
        
        assert_eq!(provider.playback_state, PlaybackState::Stopped);
        assert_eq!(
            provider.provider_type(),
            ProviderType::HistoricalData { format: HistoricalDataFormat::JSON }
        );
    }

    #[test]
    fn test_json_parsing() {
        let test_file = create_test_json_file();
        
        let mut config = HistoricalDataConfig::default();
        config.file_path = test_file.path().to_path_buf();
        
        let mut provider = HistoricalDataProvider::new(config);
        
        // 初始化应该成功
        assert!(provider.initialize().is_ok());
        assert!(provider.event_buffer.len() > 0);
    }

    #[test]
    fn test_playback_control() {
        let mut config = HistoricalDataConfig::default();
        config.playback_config.auto_start = false;
        
        let mut provider = HistoricalDataProvider::new(config);
        
        // 测试暂停和恢复
        assert!(provider.pause().is_ok());
        assert!(provider.resume().is_ok());
        
        // 测试速度设置
        assert!(provider.set_playback_speed(2.0).is_ok());
        assert_eq!(provider.playback_info.playback_speed, 2.0);
        
        // 测试无效速度
        assert!(provider.set_playback_speed(1000.0).is_err());
    }
}