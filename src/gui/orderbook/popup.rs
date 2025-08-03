/// 弹出窗口管理模块

use eframe::egui;

/// 弹出窗口类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupType {
    TradingSignal,
    QuantitativeBacktest,
    Settings,
    About,
    Help,
}

/// 弹出窗口状态
#[derive(Debug, Clone)]
pub struct PopupState {
    pub is_open: bool,
    pub size: egui::Vec2,
    pub position: Option<egui::Pos2>,
    pub resizable: bool,
    pub movable: bool,
}

impl Default for PopupState {
    fn default() -> Self {
        Self {
            is_open: false,
            size: egui::Vec2::new(600.0, 400.0),
            position: None,
            resizable: true,
            movable: true,
        }
    }
}

/// 弹出窗口管理器
pub struct PopupManager {
    /// 窗口状态映射
    popup_states: std::collections::HashMap<PopupType, PopupState>,
    /// 交易信号配置
    trading_signal_config: TradingSignalConfig,
    /// 量化回测配置
    backtest_config: BacktestConfig,
    /// 应用设置
    app_settings: AppSettings,
}

/// 交易信号配置
#[derive(Debug, Clone)]
pub struct TradingSignalConfig {
    pub enabled: bool,
    pub signal_types: Vec<SignalType>,
    pub threshold: f64,
    pub time_frame: TimeFrame,
}

/// 信号类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalType {
    VolumeSpike,
    PriceBreakout,
    OrderImbalance,
    DeltaAnomaly,
}

/// 时间框架
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeFrame {
    Seconds5,
    Seconds30,
    Minutes1,
    Minutes5,
    Minutes15,
}

/// 量化回测配置
#[derive(Debug, Clone)]
pub struct BacktestConfig {
    pub strategy_name: String,
    pub start_date: String,
    pub end_date: String,
    pub initial_capital: f64,
    pub commission_rate: f64,
    pub slippage: f64,
}

/// 应用设置
#[derive(Debug, Clone)]
pub struct AppSettings {
    pub theme: Theme,
    pub language: Language,
    pub auto_scroll: bool,
    pub price_precision: f64,
    pub update_interval: u64,
}

/// 主题设置
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
    Auto,
}

/// 语言设置
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Language {
    Chinese,
    English,
}

impl PopupManager {
    pub fn new() -> Self {
        Self {
            popup_states: std::collections::HashMap::new(),
            trading_signal_config: TradingSignalConfig::default(),
            backtest_config: BacktestConfig::default(),
            app_settings: AppSettings::default(),
        }
    }
    
    /// 打开弹出窗口
    pub fn open_popup(&mut self, popup_type: PopupType) {
        let state = self.popup_states.entry(popup_type.clone()).or_insert_with(|| {
            PopupState {
                size: self.get_default_size(&popup_type),
                ..Default::default()
            }
        });
        state.is_open = true;
    }
    
    /// 关闭弹出窗口
    pub fn close_popup(&mut self, popup_type: &PopupType) {
        if let Some(state) = self.popup_states.get_mut(popup_type) {
            state.is_open = false;
        }
    }
    
    /// 检查弹出窗口是否打开
    pub fn is_popup_open(&self, popup_type: &PopupType) -> bool {
        self.popup_states.get(popup_type)
            .map(|state| state.is_open)
            .unwrap_or(false)
    }
    
    /// 渲染所有弹出窗口
    pub fn render_popups(&mut self, ctx: &egui::Context) {
        let popup_types = vec![
            PopupType::TradingSignal,
            PopupType::QuantitativeBacktest,
            PopupType::Settings,
            PopupType::About,
            PopupType::Help,
        ];
        
        for popup_type in popup_types {
            if self.is_popup_open(&popup_type) {
                self.render_popup(ctx, popup_type);
            }
        }
    }
    
    /// 渲染单个弹出窗口
    fn render_popup(&mut self, ctx: &egui::Context, popup_type: PopupType) {
        let state = self.popup_states.get_mut(&popup_type).unwrap();
        
        let mut window = egui::Window::new(self.get_window_title(&popup_type))
            .open(&mut state.is_open)
            .default_size(state.size)
            .resizable(state.resizable)
            .movable(state.movable);
        
        if let Some(pos) = state.position {
            window = window.default_pos(pos);
        }
        
        window.show(ctx, |ui| {
            match popup_type {
                PopupType::TradingSignal => self.render_trading_signal_window(ui),
                PopupType::QuantitativeBacktest => self.render_backtest_window(ui),
                PopupType::Settings => self.render_settings_window(ui),
                PopupType::About => self.render_about_window(ui),
                PopupType::Help => self.render_help_window(ui),
            }
        });
    }
    
    /// 渲染交易信号窗口
    fn render_trading_signal_window(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("交易信号分析");
            ui.add_space(20.0);
            
            // 信号配置
            ui.group(|ui| {
                ui.label("信号配置:");
                ui.checkbox(&mut self.trading_signal_config.enabled, "启用信号");
                
                ui.horizontal(|ui| {
                    ui.label("阈值:");
                    ui.add(egui::Slider::new(&mut self.trading_signal_config.threshold, 0.0..=10.0));
                });
                
                ui.horizontal(|ui| {
                    ui.label("时间框架:");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", self.trading_signal_config.time_frame))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.trading_signal_config.time_frame, TimeFrame::Seconds5, "5秒");
                            ui.selectable_value(&mut self.trading_signal_config.time_frame, TimeFrame::Seconds30, "30秒");
                            ui.selectable_value(&mut self.trading_signal_config.time_frame, TimeFrame::Minutes1, "1分钟");
                            ui.selectable_value(&mut self.trading_signal_config.time_frame, TimeFrame::Minutes5, "5分钟");
                            ui.selectable_value(&mut self.trading_signal_config.time_frame, TimeFrame::Minutes15, "15分钟");
                        });
                });
            });
            
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);
            
            // 信号类型选择
            ui.label("信号类型:");
            for signal_type in [SignalType::VolumeSpike, SignalType::PriceBreakout, 
                               SignalType::OrderImbalance, SignalType::DeltaAnomaly] {
                let mut enabled = self.trading_signal_config.signal_types.contains(&signal_type);
                if ui.checkbox(&mut enabled, format!("{:?}", signal_type)).changed() {
                    if enabled {
                        self.trading_signal_config.signal_types.push(signal_type);
                    } else {
                        self.trading_signal_config.signal_types.retain(|t| t != &signal_type);
                    }
                }
            }
            
            ui.add_space(20.0);
            
            // 实时信号显示
            ui.group(|ui| {
                ui.label("实时信号:");
                ui.label("• 当前无活跃信号");
                ui.label("• 等待数据更新...");
            });
        });
    }
    
    /// 渲染量化回测窗口
    fn render_backtest_window(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("量化回测系统");
            ui.add_space(20.0);
            
            // 回测配置
            ui.group(|ui| {
                ui.label("回测配置:");
                
                ui.horizontal(|ui| {
                    ui.label("策略名称:");
                    ui.text_edit_singleline(&mut self.backtest_config.strategy_name);
                });
                
                ui.horizontal(|ui| {
                    ui.label("起始日期:");
                    ui.text_edit_singleline(&mut self.backtest_config.start_date);
                });
                
                ui.horizontal(|ui| {
                    ui.label("结束日期:");
                    ui.text_edit_singleline(&mut self.backtest_config.end_date);
                });
                
                ui.horizontal(|ui| {
                    ui.label("初始资本:");
                    ui.add(egui::DragValue::new(&mut self.backtest_config.initial_capital)
                        .prefix("$").speed(1000.0));
                });
                
                ui.horizontal(|ui| {
                    ui.label("手续费率:");
                    ui.add(egui::Slider::new(&mut self.backtest_config.commission_rate, 0.0..=0.01)
                        .suffix("%"));
                });
                
                ui.horizontal(|ui| {
                    ui.label("滑点:");
                    ui.add(egui::Slider::new(&mut self.backtest_config.slippage, 0.0..=0.001)
                        .suffix("%"));
                });
            });
            
            ui.add_space(20.0);
            
            // 控制按钮
            ui.horizontal(|ui| {
                if ui.button("开始回测").clicked() {
                    // 启动回测逻辑
                }
                
                if ui.button("停止回测").clicked() {
                    // 停止回测逻辑
                }
                
                if ui.button("导出结果").clicked() {
                    // 导出回测结果
                }
            });
            
            ui.add_space(20.0);
            ui.separator();
            
            // 回测结果展示
            ui.label("回测结果:");
            ui.label("• 总收益率: +0.00%");
            ui.label("• 最大回撤: 0.00%");
            ui.label("• 夏普比率: 0.00");
            ui.label("• 交易次数: 0");
        });
    }
    
    /// 渲染设置窗口
    fn render_settings_window(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading("应用设置");
            ui.add_space(20.0);
            
            // 外观设置
            ui.group(|ui| {
                ui.label("外观设置:");
                
                ui.horizontal(|ui| {
                    ui.label("主题:");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", self.app_settings.theme))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.app_settings.theme, Theme::Dark, "深色");
                            ui.selectable_value(&mut self.app_settings.theme, Theme::Light, "浅色");
                            ui.selectable_value(&mut self.app_settings.theme, Theme::Auto, "自动");
                        });
                });
                
                ui.horizontal(|ui| {
                    ui.label("语言:");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", self.app_settings.language))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.app_settings.language, Language::Chinese, "中文");
                            ui.selectable_value(&mut self.app_settings.language, Language::English, "English");
                        });
                });
            });
            
            ui.add_space(10.0);
            
            // 功能设置
            ui.group(|ui| {
                ui.label("功能设置:");
                
                ui.checkbox(&mut self.app_settings.auto_scroll, "自动滚动跟踪价格");
                
                ui.horizontal(|ui| {
                    ui.label("价格精度:");
                    ui.add(egui::Slider::new(&mut self.app_settings.price_precision, 0.01..=10.0)
                        .logarithmic(true));
                });
                
                ui.horizontal(|ui| {
                    ui.label("更新间隔 (ms):");
                    ui.add(egui::Slider::new(&mut self.app_settings.update_interval, 100..=5000));
                });
            });
            
            ui.add_space(20.0);
            
            // 保存按钮
            ui.horizontal(|ui| {
                if ui.button("保存设置").clicked() {
                    // 保存设置逻辑
                }
                
                if ui.button("恢复默认").clicked() {
                    self.app_settings = AppSettings::default();
                }
            });
        });
    }
    
    /// 渲染关于窗口
    fn render_about_window(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("关于 FlowSight");
            ui.add_space(20.0);
            
            ui.label("FlowSight - 高性能币安期货交易分析系统");
            ui.add_space(10.0);
            ui.label("版本: 1.0.0");
            ui.label("构建日期: 2024-01-01");
            ui.add_space(20.0);
            
            ui.separator();
            ui.add_space(10.0);
            
            ui.label("特性:");
            ui.label("• 实时订单簿数据");
            ui.label("• 高性能渲染引擎");
            ui.label("• 智能价格跟踪");
            ui.label("• 交易信号分析");
            ui.label("• 量化回测系统");
            
            ui.add_space(20.0);
            
            ui.label("技术栈:");
            ui.label("• Rust + egui");
            ui.label("• Tokio 异步运行时");
            ui.label("• WGPU GPU加速");
        });
    }
    
    /// 渲染帮助窗口
    fn render_help_window(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading("使用帮助");
            ui.add_space(20.0);
            
            ui.label("快捷键:");
            ui.group(|ui| {
                ui.label("• F1: 显示/隐藏帮助");
                ui.label("• F2: 打开设置");
                ui.label("• F3: 交易信号窗口");
                ui.label("• F4: 量化回测窗口");
                ui.label("• Ctrl+R: 重置视图");
                ui.label("• 空格: 暂停/恢复自动滚动");
            });
            
            ui.add_space(20.0);
            
            ui.label("操作说明:");
            ui.group(|ui| {
                ui.label("• 鼠标滚轮: 缩放价格精度");
                ui.label("• 拖拽: 手动滚动订单簿");
                ui.label("• 双击价格: 设置价格提醒");
                ui.label("• 右键菜单: 更多操作选项");
            });
            
            ui.add_space(20.0);
            
            ui.label("颜色说明:");
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::GREEN, "绿色");
                    ui.label(": 买单/上涨");
                });
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::RED, "红色");
                    ui.label(": 卖单/下跌");
                });
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::YELLOW, "黄色");
                    ui.label(": 当前价格");
                });
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::BLUE, "蓝色");
                    ui.label(": 成交量");
                });
            });
        });
    }
    
    /// 获取窗口标题
    fn get_window_title(&self, popup_type: &PopupType) -> &'static str {
        match popup_type {
            PopupType::TradingSignal => "交易信号",
            PopupType::QuantitativeBacktest => "量化回测",
            PopupType::Settings => "设置",
            PopupType::About => "关于",
            PopupType::Help => "帮助",
        }
    }
    
    /// 获取默认窗口大小
    fn get_default_size(&self, popup_type: &PopupType) -> egui::Vec2 {
        match popup_type {
            PopupType::TradingSignal => egui::Vec2::new(600.0, 500.0),
            PopupType::QuantitativeBacktest => egui::Vec2::new(800.0, 600.0),
            PopupType::Settings => egui::Vec2::new(500.0, 400.0),
            PopupType::About => egui::Vec2::new(400.0, 300.0),
            PopupType::Help => egui::Vec2::new(500.0, 500.0),
        }
    }
}

// 默认实现
impl Default for TradingSignalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            signal_types: vec![SignalType::VolumeSpike],
            threshold: 1.0,
            time_frame: TimeFrame::Minutes1,
        }
    }
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            strategy_name: "默认策略".to_string(),
            start_date: "2024-01-01".to_string(),
            end_date: "2024-12-31".to_string(),
            initial_capital: 10000.0,
            commission_rate: 0.001,
            slippage: 0.0001,
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            language: Language::Chinese,
            auto_scroll: true,
            price_precision: 0.1,
            update_interval: 1000,
        }
    }
}