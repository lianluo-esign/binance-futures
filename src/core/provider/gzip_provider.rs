// Gzip Data Provider - 压缩历史数据文件提供者
//
// 本文件实现了读取gzip压缩历史数据文件的Provider，负责：
// - 读取data目录下的.gz压缩文件
// - 解压缩并解析纳秒时间戳 + JSON格式数据
// - 支持可控播放速度和时间跳转
// - 提供播放控制功能
// - 与现有Provider系统完全兼容
//
// 数据格式：每行包含纳秒时间戳 + 空格 + JSON数据
// 示例：1754092800000006004 {"stream":"btcfdusd@bookTicker","data":{...}}

use super::{
    DataProvider, ControllableProvider, ProviderType, ProviderStatus, EventKind, 
    PerformanceMetrics, PlaybackInfo, HistoricalDataFormat,
    error::{ProviderError, ProviderResult},
};
use crate::config::ProviderIdentity;
use crate::events::EventType;

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Gzip数据Provider配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GzipProviderConfig {
    /// 数据目录路径
    pub data_dir: PathBuf,
    
    /// 文件名模式（例如: "btcfdusd_*.gz"）
    pub file_pattern: String,
    
    /// 播放配置
    pub playback_config: PlaybackConfig,
    
    /// 缓冲配置
    pub buffer_config: BufferConfig,
    
    /// 符号过滤
    pub symbol_filter: Option<String>,
    
    /// 事件类型过滤
    pub event_filter: Vec<String>,
}

impl Default for GzipProviderConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("data"),
            file_pattern: "*.gz".to_string(),
            playback_config: PlaybackConfig::default(),
            buffer_config: BufferConfig::default(),
            symbol_filter: None,
            event_filter: vec![],
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
    
    /// 时间过滤
    pub start_timestamp: Option<u64>,
    pub end_timestamp: Option<u64>,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            initial_speed: 1.0,
            auto_start: true,
            loop_enabled: false,
            max_speed: 1000.0,
            min_speed: 0.1,
            start_timestamp: None,
            end_timestamp: None,
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
            event_buffer_size: 10000,
            prefetch_lines: 1000,
            memory_limit_mb: 500,
        }
    }
}

/// 压缩数据记录
#[derive(Debug, Clone)]
pub struct GzipRecord {
    /// 纳秒时间戳
    pub timestamp_ns: u64,
    /// 毫秒时间戳（转换后）
    pub timestamp_ms: u64,
    /// 原始JSON数据
    pub data: Value,
    /// 事件类型
    pub event_type: EventKind,
    /// 原始行数据（用于调试）
    pub raw_line: String,
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

/// Gzip数据Provider实现
pub struct GzipProvider {
    /// 配置
    config: GzipProviderConfig,
    
    /// 当前文件读取器
    current_reader: Option<BufReader<GzDecoder<File>>>,
    
    /// 当前文件路径
    current_file_path: Option<PathBuf>,
    
    /// 所有数据文件列表
    data_files: Vec<PathBuf>,
    
    /// 当前文件索引
    current_file_index: usize,
    
    /// 事件缓冲区
    event_buffer: VecDeque<GzipRecord>,
    
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
    total_bytes_read: u64,
    events_sent: u64,
    
    /// 时间控制 - 全新的定时系统
    last_event_timestamp_ns: Option<u64>,  // 上一个事件的纳秒时间戳
    playback_start_time: Option<Instant>,  // 播放开始的实时时间
    virtual_start_timestamp_ns: Option<u64>, // 虚拟播放开始时的事件时间戳
    next_event_due_time: Option<Instant>,  // 下一个事件应该发送的实时时间
    
    /// 性能监控
    performance_window_start: Instant,
    performance_events_count: u64,
    
    /// 文件统计
    total_file_size: u64,
    current_file_progress: u64,
}

impl std::fmt::Debug for GzipProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GzipProvider")
            .field("config", &self.config)
            .field("playback_state", &self.playback_state)
            .field("playback_info", &self.playback_info)
            .field("current_file_index", &self.current_file_index)
            .field("total_files", &self.data_files.len())
            .field("total_records_read", &self.total_records_read)
            .field("events_sent", &self.events_sent)
            .finish()
    }
}

impl GzipProvider {
    /// 创建新的Gzip数据Provider
    pub fn new(config: GzipProviderConfig) -> Self {
        // 初始化播放信息
        let playback_info = PlaybackInfo::new(
            config.playback_config.start_timestamp.unwrap_or(0),
            config.playback_config.end_timestamp.unwrap_or(u64::MAX),
        );

        // 初始化状态
        let provider_type = ProviderType::HistoricalData { 
            format: HistoricalDataFormat::Compressed 
        };
        let mut status = ProviderStatus::new(provider_type);
        
        // 设置压缩数据指标
        status.provider_metrics = super::types::ProviderMetrics::Historical {
            file_progress: 0.0,
            playback_speed: config.playback_config.initial_speed,
            current_timestamp: 0,
            total_events: 0,
            processed_events: 0,
            file_path: config.data_dir.to_string_lossy().to_string(),
        };

        // 默认支持的事件类型
        let supported_events = vec![
            EventKind::BookTicker,
            EventKind::Trade,
            EventKind::DepthUpdate,
            EventKind::TickPrice,
        ];

        let now = Instant::now();

        Self {
            config,
            current_reader: None,
            current_file_path: None,
            data_files: Vec::new(),
            current_file_index: 0,
            event_buffer: VecDeque::with_capacity(10000),
            status,
            playback_state: PlaybackState::Stopped,
            playback_info,
            supported_events,
            total_records_read: 0,
            total_bytes_read: 0,
            events_sent: 0,
            last_event_timestamp_ns: None,
            playback_start_time: None,
            virtual_start_timestamp_ns: None,
            next_event_due_time: None,
            performance_window_start: now,
            performance_events_count: 0,
            total_file_size: 0,
            current_file_progress: 0,
        }
    }

    /// 扫描数据目录，获取所有匹配的.gz文件
    fn scan_data_files(&mut self) -> ProviderResult<()> {
        if !self.config.data_dir.exists() {
            return Err(ProviderError::configuration(
                format!("数据目录不存在: {}", self.config.data_dir.display())
            ));
        }

        let mut files = Vec::new();
        let entries = std::fs::read_dir(&self.config.data_dir)
            .map_err(|e| ProviderError::configuration(
                format!("无法读取数据目录: {} - {}", self.config.data_dir.display(), e)
            ))?;

        for entry in entries {
            let entry = entry.map_err(|e| ProviderError::internal(
                format!("读取目录条目失败: {}", e),
                "GzipProvider"
            ))?;

            let path = entry.path();
            
            // 检查是否为.gz文件
            if path.is_file() && 
               path.extension().and_then(|s| s.to_str()) == Some("gz") &&
               self.matches_pattern(&path) {
                files.push(path);
            }
        }

        // 按文件名排序，确保按时间顺序播放
        files.sort();
        
        if files.is_empty() {
            return Err(ProviderError::configuration(
                format!("数据目录中没有找到匹配的.gz文件: {}", self.config.data_dir.display())
            ));
        }

        // 计算总文件大小
        self.total_file_size = files.iter()
            .filter_map(|path| std::fs::metadata(path).ok())
            .map(|metadata| metadata.len())
            .sum();

        self.data_files = files;
        
        log::info!("发现 {} 个数据文件，总大小: {} MB", 
                  self.data_files.len(),
                  self.total_file_size / (1024 * 1024));
        
        Ok(())
    }

    /// 检查文件是否匹配模式
    fn matches_pattern(&self, path: &Path) -> bool {
        // 简单的模式匹配，支持通配符
        let filename = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        if self.config.file_pattern == "*" || self.config.file_pattern == "*.gz" {
            return true;
        }

        // 更复杂的模式匹配可以在这里实现
        filename.contains(&self.config.file_pattern.replace("*", "").replace(".gz", ""))
    }

    /// 打开下一个数据文件
    fn open_next_file(&mut self) -> ProviderResult<bool> {
        // 关闭当前文件
        self.current_reader = None;
        self.current_file_path = None;

        // 检查是否还有更多文件
        if self.current_file_index >= self.data_files.len() {
            if self.config.playback_config.loop_enabled {
                // 重新开始时重置时间控制系统
                self.current_file_index = 0;
                self.reset_timing_base();
                log::info!("重新开始播放数据文件");
            } else {
                // 播放完成
                self.playback_state = PlaybackState::Completed;
                log::info!("所有数据文件播放完成");
                return Ok(false);
            }
        }

        let file_path = &self.data_files[self.current_file_index];
        self.current_file_index += 1;

        // 打开文件
        let file = File::open(file_path)
            .map_err(|e| ProviderError::configuration(
                format!("无法打开数据文件: {} - {}", file_path.display(), e)
            ))?;

        // 创建gzip解码器和缓冲读取器
        let decoder = GzDecoder::new(file);
        let reader = BufReader::new(decoder);
        
        self.current_reader = Some(reader);
        self.current_file_path = Some(file_path.clone());
        self.current_file_progress = 0;

        log::info!("打开数据文件: {} ({}/{})", 
                  file_path.display(),
                  self.current_file_index,
                  self.data_files.len());

        Ok(true)
    }

    /// 从当前文件读取下一批记录
    fn read_next_batch(&mut self) -> ProviderResult<()> {
        // 确保有打开的文件
        if self.current_reader.is_none() {
            if !self.open_next_file()? {
                return Ok(()); // 没有更多文件
            }
        }

        let mut lines_read = 0;
        let mut line = String::new();

        while lines_read < self.config.buffer_config.prefetch_lines &&
              self.event_buffer.len() < self.config.buffer_config.event_buffer_size {
            
            line.clear();
            
            let bytes_read = match self.current_reader.as_mut().unwrap().read_line(&mut line) {
                Ok(0) => {
                    // 当前文件结束，尝试打开下一个文件
                    if !self.open_next_file()? {
                        break; // 没有更多文件
                    }
                    continue;
                }
                Ok(bytes) => bytes,
                Err(e) => {
                    return Err(ProviderError::internal(
                        format!("读取文件失败: {}", e),
                        "GzipProvider"
                    ));
                }
            };

            self.current_file_progress += bytes_read as u64;
            self.total_bytes_read += bytes_read as u64;

            // 解析记录
            let line_trimmed = line.trim();
            if !line_trimmed.is_empty() {
                match self.parse_line(line_trimmed) {
                    Ok(Some(record)) => {
                        // 应用过滤器
                        if self.should_include_record(&record) {
                            self.event_buffer.push_back(record);
                        }
                    }
                    Ok(None) => {
                        // 跳过空记录
                    }
                    Err(e) => {
                        log::warn!("解析记录失败: {} - 跳过行: {}", e, line_trimmed);
                        continue; // 跳过错误的行，继续处理
                    }
                }
            }

            lines_read += 1;
            self.total_records_read += 1;
        }

        // 更新进度
        self.update_progress();
        
        Ok(())
    }

    /// 解析数据行：纳秒时间戳 + JSON
    fn parse_line(&self, line: &str) -> ProviderResult<Option<GzipRecord>> {
        // 查找第一个空格，分离时间戳和JSON
        let space_pos = line.find(' ')
            .ok_or_else(|| ProviderError::data_parsing(
                "无法找到时间戳分隔符".to_string(),
                "timestamp_separator"
            ))?;

        let timestamp_str = &line[..space_pos];
        let json_str = &line[space_pos + 1..];

        // 解析纳秒时间戳
        let timestamp_ns = timestamp_str.parse::<u64>()
            .map_err(|e| ProviderError::data_parsing(
                format!("时间戳解析失败: {}", e),
                "timestamp"
            ))?;

        // 转换为毫秒时间戳
        let timestamp_ms = timestamp_ns / 1_000_000;

        // 解析JSON数据
        let data: Value = serde_json::from_str(json_str)
            .map_err(|e| ProviderError::data_parsing(
                format!("JSON解析失败: {}", e),
                "json"
            ))?;

        // 推断事件类型
        let event_type = self.infer_event_type(&data);

        Ok(Some(GzipRecord {
            timestamp_ns,
            timestamp_ms,
            data,
            event_type,
            raw_line: line.to_string(),
        }))
    }

    /// 推断事件类型
    fn infer_event_type(&self, data: &Value) -> EventKind {
        // 从stream字段推断事件类型
        if let Some(stream) = data.get("stream").and_then(|s| s.as_str()) {
            if stream.contains("@bookTicker") {
                return EventKind::BookTicker;
            } else if stream.contains("@trade") {
                return EventKind::Trade;
            } else if stream.contains("@depth") {
                return EventKind::DepthUpdate;
            }
        }

        // 从data字段内的e字段推断
        if let Some(data_obj) = data.get("data") {
            if let Some(event_type) = data_obj.get("e").and_then(|e| e.as_str()) {
                match event_type {
                    "bookTicker" => return EventKind::BookTicker,
                    "depthUpdate" => return EventKind::DepthUpdate,
                    "trade" => return EventKind::Trade,
                    _ => {}
                }
            }
        }

        // 默认为TickPrice
        EventKind::TickPrice
    }

    /// 检查记录是否应该包含
    fn should_include_record(&self, record: &GzipRecord) -> bool {
        // 时间过滤
        if let Some(start_time) = self.config.playback_config.start_timestamp {
            if record.timestamp_ms < start_time {
                return false;
            }
        }
        
        if let Some(end_time) = self.config.playback_config.end_timestamp {
            if record.timestamp_ms > end_time {
                return false;
            }
        }

        // 符号过滤
        if let Some(ref symbol_filter) = self.config.symbol_filter {
            if let Some(stream) = record.data.get("stream").and_then(|s| s.as_str()) {
                if !stream.to_lowercase().contains(&symbol_filter.to_lowercase()) {
                    return false;
                }
            }
        }

        // 事件类型过滤
        if !self.config.event_filter.is_empty() {
            let event_str = record.event_type.as_str();
            if !self.config.event_filter.iter().any(|filter| 
                filter.eq_ignore_ascii_case(event_str)) {
                return false;
            }
        }

        true
    }

    /// 更新进度信息
    fn update_progress(&mut self) {
        let overall_progress = if self.total_file_size > 0 {
            self.total_bytes_read as f64 / self.total_file_size as f64
        } else {
            0.0
        };

        // 更新播放信息
        if let Some(record) = self.event_buffer.back() {
            self.playback_info.update_timestamp(record.timestamp_ms);
        }

        // 更新Provider指标
        if let super::types::ProviderMetrics::Historical {
            ref mut file_progress,
            ref mut processed_events,
            ref mut total_events,
            ref mut current_timestamp,
            ..
        } = self.status.provider_metrics {
            *file_progress = overall_progress;
            *processed_events = self.events_sent;
            *total_events = self.total_records_read; // 这是一个估算
            *current_timestamp = self.last_event_timestamp_ns.map(|ns| ns / 1_000_000).unwrap_or(0);
        }

        self.status.update_timestamp();
    }

    /// 将压缩记录转换为EventType
    fn convert_record_to_event(&self, record: &GzipRecord) -> EventType {
        match record.event_type {
            EventKind::BookTicker => EventType::BookTicker(record.data.clone()),
            EventKind::DepthUpdate => EventType::DepthUpdate(record.data.clone()),
            EventKind::Trade => EventType::Trade(record.data.clone()),
            EventKind::TickPrice => EventType::TickPrice(record.data.clone()),
            _ => EventType::TickPrice(record.data.clone()),
        }
    }

    /// 检查是否应该发送事件（基于播放速度的现实时间算法）
    /// 
    /// 算法逻辑：
    /// - speed = 0: 立即发送所有事件（无延迟模式）
    /// - speed > 0: 基于原始市场时间间隔，按速度倍数缩放
    ///   例如：两个事件原本间隔100ms，2x速度下间隔50ms，0.5x速度下间隔200ms
    fn should_send_event(&mut self, record: &GzipRecord) -> bool {
        // 速度为0表示无延迟模式，立即发送
        if self.playback_info.playback_speed == 0.0 {
            return true;
        }

        let now = Instant::now();
        
        // 如果已经到了预定发送时间，则发送
        if let Some(due_time) = self.next_event_due_time {
            if now >= due_time {
                return true;
            } else {
                return false;
            }
        }
        
        // 第一个事件立即发送
        true
    }
    
    /// 更新下一个事件的预定发送时间
    /// 
    /// 此方法在成功发送事件后调用，计算下一个事件应该发送的时间
    fn update_next_event_timing(&mut self, current_record: &GzipRecord, next_record: Option<&GzipRecord>) {
        // 速度为0的情况下不需要时间控制
        if self.playback_info.playback_speed == 0.0 {
            self.next_event_due_time = None;
            self.last_event_timestamp_ns = Some(current_record.timestamp_ns);
            return;
        }
        
        let now = Instant::now();
        
        // 初始化播放时间基准
        if self.playback_start_time.is_none() {
            self.playback_start_time = Some(now);
            self.virtual_start_timestamp_ns = Some(current_record.timestamp_ns);
        }
        
        // 如果有下一个事件，计算其应该发送的时间
        if let Some(next_record) = next_record {
            if let (Some(start_time), Some(virtual_start)) = (self.playback_start_time, self.virtual_start_timestamp_ns) {
                // 计算下一个事件与播放开始时间的虚拟时间差（纳秒）
                let virtual_time_diff_ns = next_record.timestamp_ns.saturating_sub(virtual_start);
                
                // 按播放速度缩放时间差
                let scaled_time_diff_ns = if self.playback_info.playback_speed > 0.0 {
                    (virtual_time_diff_ns as f64 / self.playback_info.playback_speed) as u64
                } else {
                    0 // 避免除零
                };
                
                // 计算下一个事件应该发送的实际时间
                let scaled_duration = Duration::from_nanos(scaled_time_diff_ns);
                self.next_event_due_time = Some(start_time + scaled_duration);
            }
        } else {
            // 没有下一个事件，清除预定时间
            self.next_event_due_time = None;
        }
        
        self.last_event_timestamp_ns = Some(current_record.timestamp_ns);
    }
    
    /// 重置播放时间基准（当速度改变或暂停/恢复时调用）
    fn reset_timing_base(&mut self) {
        let now = Instant::now();
        self.playback_start_time = Some(now);
        self.next_event_due_time = None;
        
        // 如果有当前事件，将其作为新的虚拟开始时间
        if let Some(current_record) = self.event_buffer.front() {
            self.virtual_start_timestamp_ns = Some(current_record.timestamp_ns);
        }
    }
}

impl DataProvider for GzipProvider {
    type Error = ProviderError;

    fn initialize(&mut self) -> ProviderResult<()> {
        log::info!("初始化Gzip Data Provider: {}", 
                  self.config.data_dir.display());

        // 扫描数据文件
        self.scan_data_files()?;

        // 打开第一个文件
        if !self.open_next_file()? {
            return Err(ProviderError::configuration(
                "没有可用的数据文件".to_string()
            ));
        }

        // 预读取一些数据
        self.read_next_batch()?;

        self.status.is_running = false;
        self.playback_state = PlaybackState::Stopped;

        log::info!("Gzip Data Provider初始化完成，发现 {} 个文件，预加载 {} 条记录", 
                  self.data_files.len(),
                  self.event_buffer.len());
        Ok(())
    }

    fn start(&mut self) -> ProviderResult<()> {
        log::info!("启动Gzip Data Provider");

        if self.config.playback_config.auto_start {
            self.playback_state = PlaybackState::Playing;
            self.playback_start_time = Some(Instant::now());
        } else {
            self.playback_state = PlaybackState::Paused;
        }

        self.status.is_running = true;
        self.status.is_connected = true; // 历史数据总是"连接"的
        self.performance_window_start = Instant::now();

        log::info!("Gzip Data Provider启动完成，播放状态: {:?}", 
                  self.playback_state);
        Ok(())
    }

    fn stop(&mut self) -> ProviderResult<()> {
        log::info!("停止Gzip Data Provider");

        self.playback_state = PlaybackState::Stopped;
        self.status.is_running = false;
        self.playback_start_time = None;

        log::info!("Gzip Data Provider已停止");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.current_reader.is_some() && self.status.is_running
    }

    fn read_events(&mut self) -> ProviderResult<Vec<EventType>> {
        if !matches!(self.playback_state, PlaybackState::Playing) {
            return Ok(vec![]); // 非播放状态返回空列表
        }

        let mut events = Vec::new();

        // 从缓冲区读取准备好的事件
        loop {
            // 先检查是否还有事件，并克隆记录以避免借用冲突
            let should_send = if let Some(record) = self.event_buffer.front().cloned() {
                self.should_send_event(&record)
            } else {
                false
            };
            
            if !should_send {
                break; // 没有更多事件或者还没到发送时间
            }
            
            // 从缓冲区取出事件
            let record = self.event_buffer.pop_front().unwrap();
            let event = self.convert_record_to_event(&record);
            events.push(event);

            // 更新下一个事件的定时
            // 创建一个临时的 Option，避免借用冲突
            let next_record_copy = self.event_buffer.front().cloned();
            self.update_next_event_timing(&record, next_record_copy.as_ref());
            
            // 更新统计
            self.events_sent += 1;
            self.performance_events_count += 1;
            self.status.record_event();
        }

        // 如果缓冲区快空了，尝试读取更多数据
        if self.event_buffer.len() < self.config.buffer_config.prefetch_lines / 2 {
            if let Err(e) = self.read_next_batch() {
                log::warn!("读取压缩数据失败: {}", e);
                if self.event_buffer.is_empty() {
                    self.playback_state = PlaybackState::Error;
                    return Err(e);
                }
            }
        }

        // 检查是否播放完成
        if self.playback_state == PlaybackState::Completed && self.event_buffer.is_empty() {
            log::info!("压缩数据播放完成");
        }

        self.update_progress();

        Ok(events)
    }

    fn get_status(&self) -> ProviderStatus {
        self.status.clone()
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::HistoricalData { format: HistoricalDataFormat::Compressed }
    }

    fn supported_events(&self) -> &[EventKind] {
        &self.supported_events
    }

    fn get_config_info(&self) -> Option<String> {
        Some(format!(
            "Dir: {}, Files: {}, Speed: {:.1}x, Records: {}",
            self.config.data_dir.display(),
            self.data_files.len(),
            self.playback_info.playback_speed,
            self.total_records_read
        ))
    }

    fn health_check(&self) -> bool {
        self.current_reader.is_some() && 
        !matches!(self.playback_state, PlaybackState::Error)
    }

    fn get_performance_metrics(&self) -> Option<PerformanceMetrics> {
        let window_duration = self.performance_window_start.elapsed();
        if window_duration.as_secs() == 0 {
            return None;
        }

        let events_per_second = self.performance_events_count as f64 / window_duration.as_secs_f64();
        let bytes_per_second = self.total_bytes_read as f64 / window_duration.as_secs_f64();

        Some(PerformanceMetrics {
            events_per_second,
            bytes_per_second,
            average_latency_ms: 0.0,
            max_latency_ms: 0.0,
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0.0,
            error_rate: 0.0, // 历史数据很少出错
            window_seconds: window_duration.as_secs(),
        })
    }
}

impl ControllableProvider for GzipProvider {
    fn pause(&mut self) -> ProviderResult<()> {
        if matches!(self.playback_state, PlaybackState::Playing) {
            self.playback_state = PlaybackState::Paused;
            // 暂停时清除定时器，避免时间计算错误
            self.next_event_due_time = None;
            log::info!("压缩数据播放已暂停");
        }
        Ok(())
    }

    fn resume(&mut self) -> ProviderResult<()> {
        if matches!(self.playback_state, PlaybackState::Paused) {
            self.playback_state = PlaybackState::Playing;
            // 恢复时重置时间基准，保证时间计算的连续性
            self.reset_timing_base();
            log::info!("压缩数据播放已恢复");
        }
        Ok(())
    }

    fn set_playback_speed(&mut self, speed: f64) -> ProviderResult<()> {
        // 允许speed=0作为特殊情况（无延迟模式）
        if speed < 0.0 || (speed > 0.0 && speed < self.config.playback_config.min_speed) || 
           speed > self.config.playback_config.max_speed {
            return Err(ProviderError::configuration(
                format!("播放速度超出范围 [0.0, {}-{}]: {}", 
                       self.config.playback_config.min_speed,
                       self.config.playback_config.max_speed,
                       speed)
            ));
        }

        let old_speed = self.playback_info.playback_speed;
        self.playback_info.playback_speed = speed;
        
        // 当速度改变时重置时间基准，确保平滑过渡
        if (old_speed == 0.0) != (speed == 0.0) || speed != old_speed {
            self.reset_timing_base();
        }
        
        if speed == 0.0 {
            log::info!("播放速度已设置为无延迟模式（speed=0）");
        } else {
            log::info!("播放速度已设置为 {:.1}x", speed);
        }
        Ok(())
    }

    fn seek_to(&mut self, timestamp: u64) -> ProviderResult<()> {
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
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_gzip_file(dir: &Path, filename: &str, data: &str) -> PathBuf {
        let file_path = dir.join(filename);
        let file = File::create(&file_path).unwrap();
        let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        encoder.write_all(data.as_bytes()).unwrap();
        encoder.finish().unwrap();
        file_path
    }

    #[test]
    fn test_gzip_provider_creation() {
        let config = GzipProviderConfig::default();
        let provider = GzipProvider::new(config);
        
        assert_eq!(provider.playback_state, PlaybackState::Stopped);
        assert_eq!(
            provider.provider_type(),
            ProviderType::HistoricalData { format: HistoricalDataFormat::Compressed }
        );
    }

    #[test]
    fn test_line_parsing() {
        let config = GzipProviderConfig::default();
        let provider = GzipProvider::new(config);
        
        let test_line = r#"1754092800000006004 {"stream":"btcfdusd@bookTicker","data":{"u":30154000299,"s":"BTCFDUSD","b":"113558.12000000","B":"0.00460000","a":"113562.99000000","A":"0.01650000"}}"#;
        
        let result = provider.parse_line(test_line).unwrap();
        assert!(result.is_some());
        
        let record = result.unwrap();
        assert_eq!(record.timestamp_ns, 1754092800000006004);
        assert_eq!(record.timestamp_ms, 1754092800000);
        assert_eq!(record.event_type, EventKind::BookTicker);
    }

    #[test]
    fn test_gzip_file_reading() {
        let temp_dir = TempDir::new().unwrap();
        let test_data = concat!(
            "1754092800000006004 {\"stream\":\"btcfdusd@bookTicker\",\"data\":{\"u\":30154000299,\"s\":\"BTCFDUSD\",\"b\":\"113558.12000000\",\"B\":\"0.00460000\",\"a\":\"113562.99000000\",\"A\":\"0.01650000\"}}\n",
            "1754092800002512902 {\"stream\":\"btcfdusd@trade\",\"data\":{\"e\":\"trade\",\"E\":1754092799975,\"s\":\"BTCFDUSD\",\"t\":1722548214,\"p\":\"113558.13000000\",\"q\":\"0.02000000\",\"T\":1754092799974,\"m\":true,\"M\":true}}\n"
        );
        
        create_test_gzip_file(temp_dir.path(), "test_data.gz", test_data);
        
        let mut config = GzipProviderConfig::default();
        config.data_dir = temp_dir.path().to_path_buf();
        
        let mut provider = GzipProvider::new(config);
        
        // 初始化应该成功
        assert!(provider.initialize().is_ok());
        assert!(provider.event_buffer.len() > 0);
    }
    
    #[test]
    fn test_timing_algorithm_speed_zero() {
        let temp_dir = TempDir::new().unwrap();
        // 创建两个事件，间隔1秒（纳秒时间戳）
        let test_data = concat!(
            "1754092800000000000 {\"stream\":\"btcfdusd@bookTicker\",\"data\":{}}\n",
            "1754092801000000000 {\"stream\":\"btcfdusd@bookTicker\",\"data\":{}}\n"
        );
        
        create_test_gzip_file(temp_dir.path(), "timing_test.gz", test_data);
        
        let mut config = GzipProviderConfig::default();
        config.data_dir = temp_dir.path().to_path_buf();
        config.playback_config.initial_speed = 0.0; // 无延迟模式
        config.playback_config.auto_start = true;
        
        let mut provider = GzipProvider::new(config);
        provider.initialize().unwrap();
        provider.start().unwrap();
        
        let start_time = Instant::now();
        
        // speed=0时，所有事件应该立即可用
        let events1 = provider.read_events().unwrap();
        let events2 = provider.read_events().unwrap();
        
        let elapsed = start_time.elapsed();
        
        // 在speed=0模式下，两个事件都应该在极短时间内可用（< 10ms）
        assert!(!events1.is_empty() || !events2.is_empty());
        assert!(elapsed.as_millis() < 10, 
                "Speed=0 mode should not introduce delays, but took {}ms", 
                elapsed.as_millis());
    }
    
    #[test]
    fn test_timing_algorithm_speed_scaling() {
        let temp_dir = TempDir::new().unwrap();
        // 创建两个事件，间隔100ms（纳秒时间戳）
        let base_time = 1754092800000000000u64;
        let interval_ns = 100_000_000u64; // 100ms in nanoseconds
        
        let test_data = format!(
            "{} {{\"stream\":\"btcfdusd@bookTicker\",\"data\":{{}}}}\n{} {{\"stream\":\"btcfdusd@bookTicker\",\"data\":{{}}}}", 
            base_time, 
            base_time + interval_ns
        );
        
        create_test_gzip_file(temp_dir.path(), "timing_test.gz", &test_data);
        
        // 测试2x速度
        let mut config = GzipProviderConfig::default();
        config.data_dir = temp_dir.path().to_path_buf();
        config.playback_config.initial_speed = 2.0; // 2x速度
        config.playback_config.auto_start = true;
        
        let mut provider = GzipProvider::new(config);
        provider.initialize().unwrap();
        provider.start().unwrap();
        
        let start_time = Instant::now();
        
        // 第一个事件应该立即可用
        let events1 = provider.read_events().unwrap();
        assert!(!events1.is_empty(), "First event should be immediately available");
        
        // 第二个事件应该在约50ms后可用（原100ms / 2x速度）
        std::thread::sleep(Duration::from_millis(45)); // 稍微早一点
        let events2 = provider.read_events().unwrap();
        assert!(events2.is_empty(), "Second event should not be available yet");
        
        std::thread::sleep(Duration::from_millis(10)); // 总共等55ms
        let events3 = provider.read_events().unwrap();
        assert!(!events3.is_empty(), "Second event should now be available");
        
        let total_elapsed = start_time.elapsed();
        assert!(total_elapsed.as_millis() >= 50, 
                "Should take at least 50ms for 2x speed, but took {}ms", 
                total_elapsed.as_millis());
        assert!(total_elapsed.as_millis() < 70, 
                "Should not take much more than 50ms for 2x speed, but took {}ms", 
                total_elapsed.as_millis());
    }
    
    #[test]
    fn test_speed_change_during_playback() {
        let temp_dir = TempDir::new().unwrap();
        // 创建三个事件，每个间隔100ms
        let base_time = 1754092800000000000u64;
        let interval_ns = 100_000_000u64; 
        
        let test_data = format!(
            "{} {{\"stream\":\"btcfdusd@bookTicker\",\"data\":{{}}}}\n{} {{\"stream\":\"btcfdusd@bookTicker\",\"data\":{{}}}}\n{} {{\"stream\":\"btcfdusd@bookTicker\",\"data\":{{}}}}", 
            base_time, 
            base_time + interval_ns,
            base_time + 2 * interval_ns
        );
        
        create_test_gzip_file(temp_dir.path(), "speed_change_test.gz", &test_data);
        
        let mut config = GzipProviderConfig::default();
        config.data_dir = temp_dir.path().to_path_buf();
        config.playback_config.initial_speed = 1.0; // 正常速度
        config.playback_config.auto_start = true;
        
        let mut provider = GzipProvider::new(config);
        provider.initialize().unwrap();
        provider.start().unwrap();
        
        // 第一个事件立即可用
        let events1 = provider.read_events().unwrap();
        assert!(!events1.is_empty());
        
        // 改变速度为2x
        provider.set_playback_speed(2.0).unwrap();
        
        let start_time = Instant::now();
        
        // 第二个事件应该在约50ms后可用（因为速度改变后重置了时间基准）
        std::thread::sleep(Duration::from_millis(45));
        let events2 = provider.read_events().unwrap();
        assert!(events2.is_empty(), "Event should not be ready yet after speed change");
        
        std::thread::sleep(Duration::from_millis(10));
        let events3 = provider.read_events().unwrap();
        assert!(!events3.is_empty(), "Event should be ready now");
        
        let elapsed = start_time.elapsed();
        assert!(elapsed.as_millis() >= 50 && elapsed.as_millis() < 70,
                "Speed change should work correctly, elapsed: {}ms", elapsed.as_millis());
    }
    
    #[test]
    fn test_pause_resume_timing() {
        let temp_dir = TempDir::new().unwrap();
        let base_time = 1754092800000000000u64;
        let interval_ns = 100_000_000u64; // 100ms
        
        let test_data = format!(
            "{} {{\"stream\":\"btcfdusd@bookTicker\",\"data\":{{}}}}\n{} {{\"stream\":\"btcfdusd@bookTicker\",\"data\":{{}}}}", 
            base_time, 
            base_time + interval_ns
        );
        
        create_test_gzip_file(temp_dir.path(), "pause_test.gz", &test_data);
        
        let mut config = GzipProviderConfig::default();
        config.data_dir = temp_dir.path().to_path_buf();
        config.playback_config.initial_speed = 1.0;
        config.playback_config.auto_start = true;
        
        let mut provider = GzipProvider::new(config);
        provider.initialize().unwrap();
        provider.start().unwrap();
        
        // 第一个事件
        let events1 = provider.read_events().unwrap();
        assert!(!events1.is_empty());
        
        // 暂停
        provider.pause().unwrap();
        
        // 暂停时不应该有事件
        std::thread::sleep(Duration::from_millis(110)); // 超过原始间隔
        let events2 = provider.read_events().unwrap();
        assert!(events2.is_empty(), "No events should be available while paused");
        
        // 恢复
        let resume_time = Instant::now();
        provider.resume().unwrap();
        
        // 恢复后第二个事件应该在100ms后可用
        std::thread::sleep(Duration::from_millis(95));
        let events3 = provider.read_events().unwrap();
        assert!(events3.is_empty(), "Event should not be ready yet");
        
        std::thread::sleep(Duration::from_millis(10));
        let events4 = provider.read_events().unwrap();
        assert!(!events4.is_empty(), "Event should be ready after resume timing");
        
        let resume_elapsed = resume_time.elapsed();
        assert!(resume_elapsed.as_millis() >= 100,
                "Resume should respect original timing, elapsed: {}ms", resume_elapsed.as_millis());
    }
}

/// ProviderIdentity implementation - defines the canonical name and type
impl ProviderIdentity for GzipProvider {
    /// Canonical name that MUST be used in configuration files
    const CANONICAL_NAME: &'static str = "gzip_historical_provider";
    
    /// Canonical type identifier 
    const CANONICAL_TYPE: &'static str = "GzipProvider";
    
    /// Human-readable display name
    const DISPLAY_NAME: &'static str = "Gzip Historical Data Provider";
    
    /// Provider version
    const VERSION: &'static str = "1.0.0";
}