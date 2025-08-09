// Provider Selection System - Provider选择启动系统
//
// 本模块实现了应用启动时的Provider选择功能，包括：
// - 读取配置中的可用Provider列表
// - 提供键盘导航的选择界面
// - 动态启动选中的Provider
// - 统一的Provider管理接口
//
// 设计原则：
// 1. 模块化设计：选择器、启动器、配置管理分离
// 2. 可扩展性：支持新Provider类型的添加
// 3. 统一接口：所有Provider通过相同接口与系统交互
// 4. 配置驱动：通过配置文件控制可用选项

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::config::provider_config::{ProviderConfig, BinanceWebSocketConfig};
use crate::config::provider_mapping::{get_provider_mapping, ProviderMapping};
use super::{AnyProvider, ProviderError, ProviderResult};
use super::binance_market_provider::BinanceProvider;
use super::gzip_historical_provider::{HistoricalDataProvider, HistoricalDataConfig};
use super::gzip_provider::{GzipProvider, GzipProviderConfig};

/// Provider选择器主结构
/// 负责管理可用Provider列表和用户选择
#[derive(Debug, Clone)]
pub struct ProviderSelector {
    /// 可用的Provider选项列表
    pub available_providers: Vec<ProviderOption>,
    /// 当前选中的Provider索引
    pub selected_index: usize,
    /// 配置文件路径
    config_path: PathBuf,
    /// Provider映射缓存
    provider_mappings: HashMap<String, ProviderMapping>,
}

/// 单个Provider选项
#[derive(Debug, Clone)]
pub struct ProviderOption {
    /// Provider名称（用于显示）
    pub name: String,
    /// Provider类型标识
    pub provider_type: String,
    /// 配置文件路径
    pub config_file: Option<String>,
    /// 是否已启用
    pub enabled: bool,
    /// Provider描述信息
    pub description: String,
    /// Provider状态（健康检查等）
    pub status: ProviderOptionStatus,
}

/// Provider选项状态
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderOptionStatus {
    /// 未检查
    Unknown,
    /// 可用
    Available,
    /// 配置错误
    ConfigError(String),
    /// 连接失败
    ConnectionError(String),
    /// 不支持
    Unsupported(String),
}

impl ProviderSelector {
    /// 创建新的Provider选择器
    pub fn new<P: Into<PathBuf>>(config_path: P) -> ProviderResult<Self> {
        let config_path = config_path.into();
        let mut selector = Self {
            available_providers: Vec::new(),
            selected_index: 0,
            config_path,
            provider_mappings: HashMap::new(),
        };
        
        selector.load_provider_mappings()?;
        selector.load_available_providers()?;
        
        Ok(selector)
    }
    
    /// 加载Provider映射信息
    fn load_provider_mappings(&mut self) -> ProviderResult<()> {
        use crate::config::provider_mapping::PROVIDER_MAPPINGS;
        
        for (name, mapping) in PROVIDER_MAPPINGS.iter() {
            self.provider_mappings.insert(
                name.to_string(), 
                mapping.clone()
            );
        }
        
        log::info!("已加载 {} 个Provider映射", self.provider_mappings.len());
        Ok(())
    }
    
    /// 从配置文件加载可用Provider列表
    fn load_available_providers(&mut self) -> ProviderResult<()> {
        // 读取主配置文件
        let config_content = std::fs::read_to_string(&self.config_path)
            .map_err(|e| ProviderError::configuration(
                format!("无法读取配置文件 {}: {}", 
                       self.config_path.display(), e)
            ))?;
        
        // 解析TOML配置
        let config: toml::Value = toml::from_str(&config_content)
            .map_err(|e| ProviderError::configuration(
                format!("配置文件解析失败: {}", e)
            ))?;
        
        // 提取active providers列表
        let active_providers = config
            .get("providers")
            .and_then(|providers| providers.get("active"))
            .and_then(|v| v.as_array())
            .ok_or_else(|| ProviderError::configuration(
                "配置文件中缺少 'providers.active' 字段或格式不正确".to_string()
            ))?;
        
        // 解析每个active provider
        for provider_name in active_providers {
            let name = provider_name.as_str()
                .ok_or_else(|| ProviderError::configuration(
                    "Provider名称必须为字符串".to_string()
                ))?;
            
            match self.create_provider_option(name) {
                Ok(option) => {
                    self.available_providers.push(option);
                    log::info!("已加载Provider选项: {}", name);
                }
                Err(e) => {
                    log::warn!("加载Provider选项失败 {}: {}", name, e);
                    // 创建错误状态的选项
                    let error_option = ProviderOption {
                        name: name.to_string(),
                        provider_type: "Unknown".to_string(),
                        config_file: None,
                        enabled: false,
                        description: format!("加载失败: {}", e),
                        status: ProviderOptionStatus::ConfigError(e.to_string()),
                    };
                    self.available_providers.push(error_option);
                }
            }
        }
        
        if self.available_providers.is_empty() {
            return Err(ProviderError::configuration(
                "没有找到可用的Provider选项".to_string()
            ));
        }
        
        log::info!("总计加载 {} 个Provider选项", self.available_providers.len());
        Ok(())
    }
    
    /// 创建单个Provider选项
    fn create_provider_option(&self, name: &str) -> ProviderResult<ProviderOption> {
        // 查找Provider映射
        let mapping = self.provider_mappings.get(name)
            .ok_or_else(|| ProviderError::configuration(
                format!("未找到Provider映射: {}", name)
            ))?;
        
        // 构建配置文件路径
        let config_file_path = format!("configs/providers/{}.toml", name);
        let full_config_path = self.config_path.parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(&config_file_path);
        
        // 检查配置文件是否存在
        let (config_file, status, description) = if full_config_path.exists() {
            // 验证配置文件
            match self.validate_provider_config(&full_config_path, mapping) {
                Ok(desc) => (
                    Some(config_file_path),
                    ProviderOptionStatus::Available,
                    desc
                ),
                Err(e) => (
                    Some(config_file_path),
                    ProviderOptionStatus::ConfigError(e.to_string()),
                    format!("配置验证失败: {}", e)
                )
            }
        } else {
            (
                None,
                ProviderOptionStatus::ConfigError("配置文件不存在".to_string()),
                format!("配置文件未找到: {}", config_file_path)
            )
        };
        
        Ok(ProviderOption {
            name: name.to_string(),
            provider_type: mapping.provider_type.to_string(),
            config_file,
            enabled: matches!(status, ProviderOptionStatus::Available),
            description,
            status,
        })
    }
    
    /// 验证Provider配置文件
    fn validate_provider_config(
        &self, 
        config_path: &std::path::Path,
        mapping: &ProviderMapping
    ) -> ProviderResult<String> {
        let config_content = std::fs::read_to_string(config_path)
            .map_err(|e| ProviderError::configuration(
                format!("无法读取配置文件: {}", e)
            ))?;
        
        let _config: toml::Value = toml::from_str(&config_content)
            .map_err(|e| ProviderError::configuration(
                format!("配置文件格式错误: {}", e)
            ))?;
        
        // 根据Provider类型进行具体验证
        let description = match mapping.provider_type {
            "BinanceWebSocket" => {
                // 验证Binance配置
                let _binance_config: BinanceWebSocketConfig = toml::from_str(&config_content)
                    .map_err(|e| ProviderError::configuration(
                        format!("Binance配置验证失败: {}", e)
                    ))?;
                "Binance WebSocket实时数据源 - 获取实时市场数据"
            }
            "GzipProvider" => {
                // 验证Gzip配置
                match toml::from_str::<GzipProviderConfig>(&config_content) {
                    Ok(_config) => {
                        log::info!("Gzip配置验证成功");
                        "Gzip历史数据源 - 读取压缩的历史数据文件"
                    }
                    Err(e) => {
                        log::error!("Gzip配置验证失败: {}", e);
                        return Err(ProviderError::configuration(
                            format!("Gzip配置验证失败: {}", e)
                        ));
                    }
                }
            }
            "HistoricalFile" => {
                // 验证历史数据配置
                let _historical_config: HistoricalDataConfig = toml::from_str(&config_content)
                    .map_err(|e| ProviderError::configuration(
                        format!("历史数据配置验证失败: {}", e)
                    ))?;
                "历史数据文件源 - 按时间顺序播放历史数据"
            }
            _ => {
                return Err(ProviderError::configuration(
                    format!("不支持的Provider类型: {}", mapping.provider_type)
                ));
            }
        };
        
        Ok(description.to_string())
    }
    
    /// 获取当前选中的Provider选项
    pub fn get_selected_option(&self) -> Option<&ProviderOption> {
        self.available_providers.get(self.selected_index)
    }
    
    /// 移动选择到下一个Provider
    pub fn select_next(&mut self) {
        if !self.available_providers.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.available_providers.len();
        }
    }
    
    /// 移动选择到上一个Provider
    pub fn select_previous(&mut self) {
        if !self.available_providers.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.available_providers.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }
    
    /// 直接设置选中索引
    pub fn select_index(&mut self, index: usize) -> bool {
        if index < self.available_providers.len() {
            self.selected_index = index;
            true
        } else {
            false
        }
    }
    
    /// 获取可用Provider数量
    pub fn len(&self) -> usize {
        self.available_providers.len()
    }
    
    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.available_providers.is_empty()
    }
    
    /// 获取所有Provider选项的引用
    pub fn get_all_options(&self) -> &[ProviderOption] {
        &self.available_providers
    }
    
    /// 启动选中的Provider
    pub fn launch_selected_provider(&self) -> ProviderResult<AnyProvider> {
        let option = self.get_selected_option()
            .ok_or_else(|| ProviderError::state(
                "没有选中的Provider",
                "no_selection",
                "provider_selected",
                "launch_provider"
            ))?;
        
        if !option.enabled {
            return Err(ProviderError::state(
                &format!("Provider未启用: {}", option.description),
                "disabled",
                "enabled",
                "launch_provider"
            ));
        }
        
        self.launch_provider_by_name(&option.name)
    }
    
    /// 根据名称启动Provider
    pub fn launch_provider_by_name(&self, name: &str) -> ProviderResult<AnyProvider> {
        let mapping = self.provider_mappings.get(name)
            .ok_or_else(|| ProviderError::configuration(
                format!("未找到Provider映射: {}", name)
            ))?;
        
        // 构建配置文件路径
        let config_file_path = format!("configs/providers/{}.toml", name);
        let full_config_path = self.config_path.parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(&config_file_path);
        
        // 读取配置文件
        let config_content = std::fs::read_to_string(&full_config_path)
            .map_err(|e| ProviderError::configuration(
                format!("无法读取配置文件 {}: {}", full_config_path.display(), e)
            ))?;
        
        // 根据Provider类型创建实例
        let provider = match mapping.provider_type {
            "BinanceWebSocket" => {
                let config: BinanceWebSocketConfig = toml::from_str(&config_content)
                    .map_err(|e| ProviderError::configuration(
                        format!("Binance配置解析失败: {}", e)
                    ))?;
                AnyProvider::Binance(BinanceProvider::new(config))
            }
            "GzipProvider" => {
                let config: GzipProviderConfig = toml::from_str(&config_content)
                    .map_err(|e| ProviderError::configuration(
                        format!("Gzip配置解析失败: {}", e)
                    ))?;
                AnyProvider::Gzip(GzipProvider::new(config))
            }
            "HistoricalFile" => {
                let config: HistoricalDataConfig = toml::from_str(&config_content)
                    .map_err(|e| ProviderError::configuration(
                        format!("历史数据配置解析失败: {}", e)
                    ))?;
                AnyProvider::Historical(HistoricalDataProvider::new(config))
            }
            _ => {
                return Err(ProviderError::configuration(
                    format!("不支持的Provider类型: {}", mapping.provider_type)
                ));
            }
        };
        
        log::info!("成功创建Provider: {} (类型: {})", name, mapping.provider_type);
        Ok(provider)
    }
    
    /// 刷新Provider状态（重新验证配置等）
    pub fn refresh_provider_status(&mut self) -> ProviderResult<()> {
        // 先收集需要更新的Provider信息
        let mut updates = Vec::new();
        
        for (index, option) in self.available_providers.iter().enumerate() {
            if let Some(config_file) = &option.config_file {
                let full_config_path = self.config_path.parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .join(config_file);
                
                if let Some(mapping) = self.provider_mappings.get(&option.name) {
                    let (status, enabled, description) = match self.validate_provider_config(&full_config_path, mapping) {
                        Ok(description) => (
                            ProviderOptionStatus::Available,
                            true,
                            description
                        ),
                        Err(e) => (
                            ProviderOptionStatus::ConfigError(e.to_string()),
                            false,
                            format!("配置验证失败: {}", e)
                        )
                    };
                    
                    updates.push((index, status, enabled, description));
                }
            }
        }
        
        // 应用更新
        for (index, status, enabled, description) in updates {
            if let Some(option) = self.available_providers.get_mut(index) {
                option.status = status;
                option.enabled = enabled;
                option.description = description;
            }
        }
        
        Ok(())
    }
}

impl Default for ProviderSelector {
    fn default() -> Self {
        // 使用默认配置路径
        Self::new("config.toml").unwrap_or_else(|_| Self {
            available_providers: Vec::new(),
            selected_index: 0,
            config_path: PathBuf::from("config.toml"),
            provider_mappings: HashMap::new(),
        })
    }
}

impl std::fmt::Display for ProviderOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status_icon = match self.status {
            ProviderOptionStatus::Available => "✓",
            ProviderOptionStatus::ConfigError(_) => "✗",
            ProviderOptionStatus::ConnectionError(_) => "⚠",
            ProviderOptionStatus::Unsupported(_) => "?",
            ProviderOptionStatus::Unknown => "○",
        };
        
        write!(f, "{} {} - {}", status_icon, self.name, self.description)
    }
}

impl std::fmt::Display for ProviderOptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderOptionStatus::Unknown => write!(f, "未知"),
            ProviderOptionStatus::Available => write!(f, "可用"),
            ProviderOptionStatus::ConfigError(e) => write!(f, "配置错误: {}", e),
            ProviderOptionStatus::ConnectionError(e) => write!(f, "连接错误: {}", e),
            ProviderOptionStatus::Unsupported(e) => write!(f, "不支持: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};
    
    fn create_test_config() -> (NamedTempFile, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        
        // 创建主配置文件
        let mut main_config = NamedTempFile::new().unwrap();
        writeln!(main_config, r#"
active = ["binance_market_provider", "gzip_historical_provider"]
        "#).unwrap();
        
        // 创建Provider配置目录
        let providers_dir = temp_dir.path().join("configs").join("providers");
        std::fs::create_dir_all(&providers_dir).unwrap();
        
        // 创建Binance配置文件
        let binance_config_path = providers_dir.join("binance_market_provider.toml");
        std::fs::write(&binance_config_path, r#"
base_url = "wss://stream.binance.com:9443/ws"
symbol = "BTCUSDT"
reconnect_interval = 5000
max_reconnect_attempts = 10
        "#).unwrap();
        
        // 创建Gzip配置文件
        let gzip_config_path = providers_dir.join("gzip_historical_provider.toml");
        std::fs::write(&gzip_config_path, r#"
file_path = "data/test.json.gz"
compression_level = 6
buffer_size = 8192
        "#).unwrap();
        
        (main_config, temp_dir)
    }
    
    #[test]
    fn test_provider_selector_creation() {
        let (config_file, _temp_dir) = create_test_config();
        let selector = ProviderSelector::new(config_file.path());
        
        assert!(selector.is_ok());
        let selector = selector.unwrap();
        assert!(!selector.is_empty());
        assert_eq!(selector.len(), 2);
    }
    
    #[test]
    fn test_provider_selection() {
        let (config_file, _temp_dir) = create_test_config();
        let mut selector = ProviderSelector::new(config_file.path()).unwrap();
        
        // 测试初始选择
        assert_eq!(selector.selected_index, 0);
        
        // 测试向前选择
        selector.select_next();
        assert_eq!(selector.selected_index, 1);
        
        // 测试循环
        selector.select_next();
        assert_eq!(selector.selected_index, 0);
        
        // 测试向后选择
        selector.select_previous();
        assert_eq!(selector.selected_index, 1);
    }
    
    #[test]
    fn test_provider_option_display() {
        let option = ProviderOption {
            name: "test_provider".to_string(),
            provider_type: "TestType".to_string(),
            config_file: Some("test.toml".to_string()),
            enabled: true,
            description: "Test provider".to_string(),
            status: ProviderOptionStatus::Available,
        };
        
        let display = format!("{}", option);
        assert!(display.contains("✓"));
        assert!(display.contains("test_provider"));
        assert!(display.contains("Test provider"));
    }
}