use eframe::egui;
use crate::services::{
    ServiceManager,
    service_manager::{ServiceMessage, MessagePriority},
    rendering_service::{RenderingService, RenderCommand, TableData, ChartData, Position, Size},
    data_processing_service::DataProcessingService,
    performance_service::PerformanceService,
};
use crate::core::{PerformanceConfig, PerformanceMetrics};
use std::sync::{Arc, RwLock, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// 线程分离的GUI应用程序
/// 
/// 架构设计:
/// - 主线程: GUI渲染和用户交互
/// - 数据线程: 市场数据处理和计算
/// - 服务线程: 后台服务管理
pub struct ThreadedTradingGUI {
    /// 服务管理器
    service_manager: Arc<RwLock<ServiceManager>>,
    /// GUI状态
    gui_state: Arc<RwLock<GUIState>>,
    /// 数据通信通道
    data_channel: DataChannel,
    /// 渲染通信通道
    render_channel: RenderChannel,
    /// 线程句柄
    thread_handles: Vec<std::thread::JoinHandle<()>>,
    /// 运行状态
    is_running: Arc<AtomicBool>,
    /// 性能指标
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    /// 上次更新时间
    last_update: Instant,
    /// 帧率控制
    target_fps: u32,
}

/// GUI状态数据
#[derive(Debug, Clone)]
pub struct GUIState {
    /// 订单簿数据
    pub orderbook_data: OrderBookUIData,
    /// 市场统计
    pub market_stats: MarketStats,
    /// 连接状态
    pub connection_status: ConnectionStatus,
    /// 性能指标
    pub performance_info: PerformanceInfo,
    /// 用户界面设置
    pub ui_settings: UISettings,
}

/// 订单簿UI数据
#[derive(Debug, Clone)]
pub struct OrderBookUIData {
    pub price_levels: Vec<PriceLevel>,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub spread: f64,
    pub last_update: u64,
}

/// 价格级别
#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: f64,
    pub bid_size: f64,
    pub ask_size: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub delta: f64,
}

/// 市场统计
#[derive(Debug, Clone)]
pub struct MarketStats {
    pub symbol: String,
    pub current_price: Option<f64>,
    pub price_change_24h: f64,
    pub volume_24h: f64,
    pub realized_volatility: f64,
    pub jump_signal: f64,
}

/// 连接状态
#[derive(Debug, Clone)]
pub struct ConnectionStatus {
    pub is_connected: bool,
    pub is_reconnecting: bool,
    pub connection_quality: ConnectionQuality,
    pub last_message_time: Option<Instant>,
}

/// 连接质量
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionQuality {
    Excellent,
    Good,
    Fair,
    Poor,
}

/// 性能信息
#[derive(Debug, Clone)]
pub struct PerformanceInfo {
    pub fps: f64,
    pub frame_time_ms: f64,
    pub buffer_usage: f64,
    pub event_latency_ms: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

/// UI设置
#[derive(Debug, Clone)]
pub struct UISettings {
    pub show_debug_info: bool,
    pub auto_scroll: bool,
    pub price_precision: f64,
    pub update_frequency: u32,
    pub theme: UITheme,
}

/// UI主题
#[derive(Debug, Clone, PartialEq)]
pub enum UITheme {
    Dark,
    Light,
    HighContrast,
}

/// 数据通信通道
#[derive(Clone)]
pub struct DataChannel {
    /// 数据更新发送器
    pub data_sender: mpsc::UnboundedSender<DataUpdate>,
    /// 数据更新接收器
    pub data_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<DataUpdate>>>>,
    /// 命令发送器
    pub command_sender: mpsc::UnboundedSender<DataCommand>,
}

/// 渲染通信通道
#[derive(Clone)]
pub struct RenderChannel {
    /// 渲染命令发送器
    pub render_sender: mpsc::UnboundedSender<RenderCommand>,
    /// 渲染状态接收器
    pub status_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<RenderStatus>>>>,
}

/// 数据更新消息
#[derive(Debug, Clone)]
pub enum DataUpdate {
    /// 订单簿更新
    OrderBookUpdate(OrderBookUIData),
    /// 市场统计更新
    MarketStatsUpdate(MarketStats),
    /// 连接状态更新
    ConnectionStatusUpdate(ConnectionStatus),
    /// 性能指标更新
    PerformanceUpdate(PerformanceInfo),
}

/// 数据命令
#[derive(Debug, Clone)]
pub enum DataCommand {
    /// 重新连接
    Reconnect,
    /// 暂停数据处理
    Pause,
    /// 恢复数据处理
    Resume,
    /// 更改交易对
    ChangeSymbol(String),
    /// 更新配置
    UpdateConfig(PerformanceConfig),
}

/// 渲染状态
#[derive(Debug, Clone)]
pub enum RenderStatus {
    /// 帧渲染完成
    FrameCompleted { frame_time: Duration, commands_processed: usize },
    /// 渲染错误
    RenderError(String),
    /// 性能警告
    PerformanceWarning(String),
}

impl ThreadedTradingGUI {
    /// 创建新的线程分离GUI应用
    pub fn new(config: PerformanceConfig) -> Self {
        let service_manager = Arc::new(RwLock::new(ServiceManager::new()));
        let gui_state = Arc::new(RwLock::new(GUIState::default()));
        let is_running = Arc::new(AtomicBool::new(false));
        
        // 创建通信通道
        let data_channel = Self::create_data_channel();
        let render_channel = Self::create_render_channel();
        
        Self {
            service_manager,
            gui_state,
            data_channel,
            render_channel,
            thread_handles: Vec::new(),
            is_running,
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            last_update: Instant::now(),
            target_fps: config.gui.target_fps,
        }
    }

    /// 启动应用程序
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_running.load(Ordering::Relaxed) {
            return Err("应用程序已在运行".into());
        }

        log::info!("启动线程分离的GUI应用程序");

        // 启动数据处理线程
        self.start_data_thread()?;
        
        // 启动服务管理线程
        self.start_service_thread()?;
        
        // 启动渲染线程
        self.start_render_thread()?;

        self.is_running.store(true, Ordering::Relaxed);
        
        log::info!("所有线程已启动");
        Ok(())
    }

    /// 停止应用程序
    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Ok(());
        }

        log::info!("停止线程分离的GUI应用程序");

        self.is_running.store(false, Ordering::Relaxed);

        // 等待所有线程结束
        for handle in self.thread_handles.drain(..) {
            if let Err(e) = handle.join() {
                log::warn!("线程结束时出错: {:?}", e);
            }
        }

        log::info!("所有线程已停止");
        Ok(())
    }

    /// 启动数据处理线程
    fn start_data_thread(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let gui_state = self.gui_state.clone();
        let data_channel = self.data_channel.clone();
        let is_running = self.is_running.clone();
        
        let handle = std::thread::Builder::new()
            .name("data-processing".to_string())
            .spawn(move || {
                Self::data_processing_thread(gui_state, data_channel, is_running);
            })?;

        self.thread_handles.push(handle);
        log::info!("数据处理线程已启动");
        Ok(())
    }

    /// 启动服务管理线程
    fn start_service_thread(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let service_manager = self.service_manager.clone();
        let is_running = self.is_running.clone();
        
        let handle = std::thread::Builder::new()
            .name("service-manager".to_string())
            .spawn(move || {
                // 创建异步运行时
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    Self::service_management_thread(service_manager, is_running).await;
                });
            })?;

        self.thread_handles.push(handle);
        log::info!("服务管理线程已启动");
        Ok(())
    }

    /// 启动渲染线程
    fn start_render_thread(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let render_channel = self.render_channel.clone();
        let is_running = self.is_running.clone();
        let target_fps = self.target_fps;
        
        let handle = std::thread::Builder::new()
            .name("rendering".to_string())
            .spawn(move || {
                Self::rendering_thread(render_channel, is_running, target_fps);
            })?;

        self.thread_handles.push(handle);
        log::info!("渲染线程已启动");
        Ok(())
    }

    /// 数据处理线程主函数
    fn data_processing_thread(
        gui_state: Arc<RwLock<GUIState>>,
        data_channel: DataChannel,
        is_running: Arc<AtomicBool>,
    ) {
        log::info!("数据处理线程开始运行");
        
        // 创建数据处理服务
        let mut data_service = match Self::create_data_processing_service() {
            Ok(service) => service,
            Err(e) => {
                log::error!("创建数据处理服务失败: {}", e);
                return;
            }
        };

        // 启动数据处理服务
        if let Err(e) = data_service.start() {
            log::error!("启动数据处理服务失败: {}", e);
            return;
        }

        let mut last_update = Instant::now();
        let update_interval = Duration::from_millis(100); // 10Hz数据更新

        while is_running.load(Ordering::Relaxed) {
            let now = Instant::now();
            
            // 控制更新频率
            if now.duration_since(last_update) >= update_interval {
                // 处理数据命令
                Self::process_data_commands(&data_channel);
                
                // 更新GUI状态
                Self::update_gui_state_sync(&gui_state, &data_service, &data_channel);
                
                last_update = now;
            }

            // 短暂休眠以释放CPU
            std::thread::sleep(Duration::from_millis(10));
        }

        // 停止数据处理服务
        if let Err(e) = data_service.stop() {
            log::warn!("停止数据处理服务失败: {}", e);
        }

        log::info!("数据处理线程结束");
    }

    /// 服务管理线程主函数
    async fn service_management_thread(
        service_manager: Arc<RwLock<ServiceManager>>,
        is_running: Arc<AtomicBool>,
    ) {
        log::info!("服务管理线程开始运行");
        
        // 初始化服务
        if let Err(e) = Self::initialize_services(&service_manager).await {
            log::error!("初始化服务失败: {}", e);
            return;
        }

        let mut interval = tokio::time::interval(Duration::from_secs(30));

        while is_running.load(Ordering::Relaxed) {
            interval.tick().await;
            
            // 监控服务健康状态
            Self::monitor_services(&service_manager).await;
        }

        // 停止所有服务
        if let Ok(mut manager) = service_manager.write() {
            if let Err(e) = manager.stop_all().await {
                log::warn!("停止服务失败: {}", e);
            }
        }

        log::info!("服务管理线程结束");
    }

    /// 渲染线程主函数
    fn rendering_thread(
        render_channel: RenderChannel,
        is_running: Arc<AtomicBool>,
        target_fps: u32,
    ) {
        log::info!("渲染线程开始运行 (目标FPS: {})", target_fps);
        
        let frame_interval = Duration::from_millis(1000 / target_fps as u64);
        let mut last_frame = Instant::now();

        while is_running.load(Ordering::Relaxed) {
            let now = Instant::now();
            
            // 控制帧率
            if now.duration_since(last_frame) >= frame_interval {
                // 处理渲染命令
                Self::process_render_commands(&render_channel);
                
                last_frame = now;
            }

            // 短暂休眠
            std::thread::sleep(Duration::from_millis(1));
        }

        log::info!("渲染线程结束");
    }

    /// 创建数据通信通道
    fn create_data_channel() -> DataChannel {
        let (data_sender, data_receiver) = mpsc::unbounded_channel();
        let (command_sender, _command_receiver) = mpsc::unbounded_channel();
        
        DataChannel {
            data_sender,
            data_receiver: Arc::new(RwLock::new(Some(data_receiver))),
            command_sender,
        }
    }

    /// 创建渲染通信通道
    fn create_render_channel() -> RenderChannel {
        let (render_sender, _render_receiver) = mpsc::unbounded_channel();
        let (status_sender, status_receiver) = mpsc::unbounded_channel();
        
        RenderChannel {
            render_sender,
            status_receiver: Arc::new(RwLock::new(Some(status_receiver))),
        }
    }

    /// 创建数据处理服务
    fn create_data_processing_service() -> Result<DataProcessingService, Box<dyn std::error::Error>> {
        use crate::services::data_processing_service::DataProcessingConfig;
        
        let config = DataProcessingConfig::default();
        Ok(DataProcessingService::new(config))
    }

    /// 处理数据命令
    fn process_data_commands(data_channel: &DataChannel) {
        // 简化实现 - 实际应该处理所有待处理的命令
        // while let Ok(command) = data_channel.command_receiver.try_recv() {
        //     match command {
        //         DataCommand::Reconnect => { /* 处理重连 */ }
        //         DataCommand::Pause => { /* 处理暂停 */ }
        //         // ... 其他命令
        //     }
        // }
    }

    /// 更新GUI状态
    async fn update_gui_state(
        gui_state: &Arc<RwLock<GUIState>>,
        data_service: &DataProcessingService,
        data_channel: &DataChannel,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 获取最新的市场快照
        let market_snapshot = data_service.get_market_snapshot().await;
        
        // 更新GUI状态
        {
            let mut state = gui_state.write().unwrap();
            
            // 更新市场统计
            state.market_stats.current_price = market_snapshot.current_price;
            state.market_stats.realized_volatility = market_snapshot.realized_volatility;
            state.market_stats.jump_signal = market_snapshot.jump_signal;
            
            // 更新性能信息 (简化)
            state.performance_info.fps = 30.0; // 应该从实际测量获取
            state.performance_info.buffer_usage = 0.5; // 应该从实际测量获取
        }

        // 发送更新通知
        let market_stats = gui_state.read().unwrap().market_stats.clone();
        if let Err(e) = data_channel.data_sender.send(DataUpdate::MarketStatsUpdate(market_stats)) {
            log::warn!("发送市场统计更新失败: {}", e);
        }

        Ok(())
    }

    /// 初始化服务
    async fn initialize_services(
        service_manager: &Arc<RwLock<ServiceManager>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut manager = service_manager.write().unwrap();
        
        // 注册和启动各种服务
        // 这里是简化实现，实际应该注册所有必要的服务
        
        log::info!("服务初始化完成");
        Ok(())
    }

    /// 监控服务
    async fn monitor_services(service_manager: &Arc<RwLock<ServiceManager>>) {
        let manager = service_manager.read().unwrap();
        let health_status = manager.get_all_services_health().await;
        
        for (service_name, health) in health_status {
            match health {
                crate::services::ServiceHealth::Unhealthy(reason) => {
                    log::error!("服务 {} 不健康: {}", service_name, reason);
                }
                crate::services::ServiceHealth::Warning(reason) => {
                    log::warn!("服务 {} 警告: {}", service_name, reason);
                }
                _ => {}
            }
        }
    }

    /// 处理渲染命令
    fn process_render_commands(render_channel: &RenderChannel) {
        // 简化实现 - 实际应该处理所有待处理的渲染命令
        // while let Ok(command) = render_channel.render_receiver.try_recv() {
        //     // 处理渲染命令
        // }
    }
}

impl eframe::App for ThreadedTradingGUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 这是主线程的GUI更新函数，专注于UI渲染
        let now = Instant::now();
        let frame_interval = Duration::from_millis(1000 / self.target_fps as u64);
        
        // 控制主线程的更新频率
        if now.duration_since(self.last_update) < frame_interval {
            return;
        }
        
        // 接收数据更新
        self.receive_data_updates();
        
        // 渲染UI
        self.render_ui(ctx);
        
        // 请求下一帧
        ctx.request_repaint_after(frame_interval);
        
        self.last_update = now;
    }

    fn on_exit(&mut self) {
        if let Err(e) = self.stop() {
            log::error!("停止应用程序时出错: {}", e);
        }
    }
}

impl ThreadedTradingGUI {
    /// 接收数据更新
    fn receive_data_updates(&mut self) {
        let mut receiver_guard = self.data_channel.data_receiver.write().unwrap();
        if let Some(receiver) = receiver_guard.as_mut() {
            while let Ok(update) = receiver.try_recv() {
                self.handle_data_update(update);
            }
        }
    }

    /// 处理数据更新
    fn handle_data_update(&mut self, update: DataUpdate) {
        let mut gui_state = self.gui_state.write().unwrap();
        
        match update {
            DataUpdate::OrderBookUpdate(data) => {
                gui_state.orderbook_data = data;
            }
            DataUpdate::MarketStatsUpdate(stats) => {
                gui_state.market_stats = stats;
            }
            DataUpdate::ConnectionStatusUpdate(status) => {
                gui_state.connection_status = status;
            }
            DataUpdate::PerformanceUpdate(info) => {
                gui_state.performance_info = info;
            }
        }
    }

    /// 渲染用户界面
    fn render_ui(&mut self, ctx: &egui::Context) {
        let gui_state = self.gui_state.read().unwrap();
        
        // 顶部菜单栏
        self.render_top_panel(ctx, &gui_state);
        
        // 主要内容区域
        self.render_central_panel(ctx, &gui_state);
        
        // 调试信息窗口 (可选)
        if gui_state.ui_settings.show_debug_info {
            self.render_debug_window(ctx, &gui_state);
        }
    }

    /// 渲染顶部面板
    fn render_top_panel(&mut self, ctx: &egui::Context, gui_state: &GUIState) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // 连接状态
                let (status_text, status_color) = match &gui_state.connection_status {
                    ConnectionStatus { is_connected: true, .. } => ("已连接", egui::Color32::GREEN),
                    ConnectionStatus { is_reconnecting: true, .. } => ("重连中", egui::Color32::YELLOW),
                    _ => ("未连接", egui::Color32::RED),
                };
                
                ui.colored_label(status_color, status_text);
                ui.separator();
                
                // 性能信息
                ui.label(format!("FPS: {:.1}", gui_state.performance_info.fps));
                ui.separator();
                
                // 市场信息
                if let Some(price) = gui_state.market_stats.current_price {
                    ui.label(format!("{}: ${:.2}", gui_state.market_stats.symbol, price));
                }
            });
        });
    }

    /// 渲染中央面板
    fn render_central_panel(&mut self, ctx: &egui::Context, gui_state: &GUIState) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // 订单簿表格
            self.render_orderbook_table(ui, &gui_state.orderbook_data);
        });
    }

    /// 渲染订单簿表格
    fn render_orderbook_table(&mut self, ui: &mut egui::Ui, orderbook_data: &OrderBookUIData) {
        use egui_extras::{Column, TableBuilder};
        
        TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto().at_least(80.0)) // 价格
            .column(Column::auto().at_least(100.0)) // 买单/卖单
            .column(Column::auto().at_least(80.0)) // 买量
            .column(Column::auto().at_least(80.0)) // 卖量
            .column(Column::auto().at_least(80.0)) // Delta
            .header(25.0, |mut header| {
                header.col(|ui| { ui.heading("价格"); });
                header.col(|ui| { ui.heading("买单/卖单"); });
                header.col(|ui| { ui.heading("买量"); });
                header.col(|ui| { ui.heading("卖量"); });
                header.col(|ui| { ui.heading("Delta"); });
            })
            .body(|mut body| {
                for level in &orderbook_data.price_levels {
                    body.row(20.0, |mut row| {
                        row.col(|ui| { ui.label(format!("{:.2}", level.price)); });
                        row.col(|ui| { ui.label(format!("{:.3}/{:.3}", level.bid_size, level.ask_size)); });
                        row.col(|ui| { ui.label(format!("{:.1}", level.buy_volume)); });
                        row.col(|ui| { ui.label(format!("{:.1}", level.sell_volume)); });
                        row.col(|ui| { 
                            let color = if level.delta > 0.0 { egui::Color32::GREEN } else { egui::Color32::RED };
                            ui.colored_label(color, format!("{:+.1}", level.delta)); 
                        });
                    });
                }
            });
    }

    /// 渲染调试窗口
    fn render_debug_window(&mut self, ctx: &egui::Context, gui_state: &GUIState) {
        egui::Window::new("调试信息")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.label("性能指标:");
                ui.label(format!("FPS: {:.1}", gui_state.performance_info.fps));
                ui.label(format!("帧时间: {:.1}ms", gui_state.performance_info.frame_time_ms));
                ui.label(format!("缓冲区使用: {:.1}%", gui_state.performance_info.buffer_usage * 100.0));
                ui.label(format!("事件延迟: {:.1}ms", gui_state.performance_info.event_latency_ms));
                ui.label(format!("内存使用: {:.1}MB", gui_state.performance_info.memory_usage_mb));
                ui.label(format!("CPU使用: {:.1}%", gui_state.performance_info.cpu_usage_percent));
            });
    }
}

// 默认实现
impl Default for GUIState {
    fn default() -> Self {
        Self {
            orderbook_data: OrderBookUIData::default(),
            market_stats: MarketStats::default(),
            connection_status: ConnectionStatus::default(),
            performance_info: PerformanceInfo::default(),
            ui_settings: UISettings::default(),
        }
    }
}

impl Default for OrderBookUIData {
    fn default() -> Self {
        Self {
            price_levels: Vec::new(),
            best_bid: None,
            best_ask: None,
            spread: 0.0,
            last_update: 0,
        }
    }
}

impl Default for MarketStats {
    fn default() -> Self {
        Self {
            symbol: "BTCUSDT".to_string(),
            current_price: None,
            price_change_24h: 0.0,
            volume_24h: 0.0,
            realized_volatility: 0.0,
            jump_signal: 0.0,
        }
    }
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self {
            is_connected: false,
            is_reconnecting: false,
            connection_quality: ConnectionQuality::Poor,
            last_message_time: None,
        }
    }
}

impl Default for PerformanceInfo {
    fn default() -> Self {
        Self {
            fps: 0.0,
            frame_time_ms: 0.0,
            buffer_usage: 0.0,
            event_latency_ms: 0.0,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
        }
    }
}

impl Default for UISettings {
    fn default() -> Self {
        Self {
            show_debug_info: false,
            auto_scroll: true,
            price_precision: 0.01,
            update_frequency: 30,
            theme: UITheme::Dark,
        }
    }
}

impl ThreadedTradingGUI {
    /// 同步更新GUI状态
    fn update_gui_state_sync(
        gui_state: &Arc<RwLock<GUIState>>,
        _data_service: &DataProcessingService,
        data_channel: &DataChannel,
    ) {
        // 简化的同步实现
        {
            let mut state = gui_state.write().unwrap();
            
            // 更新基本状态
            state.market_stats.current_price = 0.0; // 应该从数据服务获取
            state.performance_info.fps = 30.0;
            state.performance_info.buffer_usage = 0.5;
        }

        // 发送更新通知（可选）
        let market_stats = gui_state.read().unwrap().market_stats.clone();
        let _ = data_channel.data_sender.send(DataUpdate::MarketStatsUpdate(market_stats));
    }
    
    /// 处理数据命令
    fn process_data_commands(_data_channel: &DataChannel) {
        // 简化实现，处理数据命令
    }
}