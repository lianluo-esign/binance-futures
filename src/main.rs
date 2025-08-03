//! FlowSight - 高性能币安期货交易分析系统
//! 
//! 特性:
//! - 线程分离架构 (GUI/数据/服务分离)
//! - 服务化模块设计 (高内聚低耦合)
//! - 自适应性能调优 (动态FPS/缓冲区调整)
//! - GPU硬件加速渲染
//! - 智能背压控制
//! 
//! 使用方法:
//! ```bash
//! # 启动GUI版本
//! cargo run --features gui
//! 
//! # 指定交易对
//! cargo run --features gui ETHUSDT
//! 
//! # 启用详细日志
//! RUST_LOG=debug cargo run --features gui
//! ```

#[cfg(feature = "gui")]
use flow_sight::{
    init_logging, 
    PerformanceConfig,
    services::{
        ServiceManager,
        data_processing_service::{DataProcessingService, DataProcessingConfig},
        rendering_service::{RenderingService, RenderingConfig},
        performance_service::{PerformanceService, PerformanceServiceConfig},
        event_service::EventService,
        configuration_service::ConfigurationService,
    }
};
#[cfg(feature = "gui")]
use flow_sight::gui::ThreadedTradingGUI;

#[cfg(feature = "gui")]
use std::env;

#[cfg(feature = "gui")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    init_logging();
    log::info!("启动FlowSight高性能交易分析系统 v{}", flow_sight::VERSION);

    // 获取交易对参数
    let symbol = env::args().nth(1).unwrap_or_else(|| "BTCUSDT".to_string());
    log::info!("交易对: {}", symbol);

    // 创建性能配置
    let performance_config = create_optimized_performance_config();
    log::info!("性能配置已加载: 自适应FPS={}, 缓冲区={}K", 
              performance_config.gui.adaptive_fps, 
              performance_config.buffer.event_buffer_size / 1024);

    // 创建并配置服务管理器
    let mut service_manager = ServiceManager::new();
    setup_services(&mut service_manager, &symbol)?;

    // 创建线程分离的GUI应用
    let mut app = ThreadedTradingGUI::new(performance_config.clone());
    
    // 启动应用程序
    log::info!("启动多线程架构...");
    app.start()?;

    // 创建高性能GPU渲染配置
    let options = create_gpu_render_options();

    // 运行GUI应用
    log::info!("启动GPU硬件加速渲染循环 (WGPU)");
    let result = eframe::run_native(
        "FlowSight - 高性能交易分析系统",
        options,
        Box::new(|cc| {
            // 配置egui上下文以获得最佳性能
            setup_egui_context(&cc.egui_ctx, &performance_config);
            
            // 显示启动信息
            log::info!("GPU渲染器: {:?}", cc.integration_info.web_info);
            log::info!("系统信息: CPU核心={}, 可用内存={}MB", 
                      num_cpus::get(),
                      get_available_memory_mb());
            
            Box::new(app)
        }),
    );

    // 处理退出结果
    match result {
        Ok(()) => {
            log::info!("FlowSight正常退出");
            Ok(())
        }
        Err(e) => {
            log::error!("FlowSight异常退出: {}", e);
            Err(e.into())
        }
    }
}

/// 创建优化的性能配置
#[cfg(feature = "gui")]
fn create_optimized_performance_config() -> PerformanceConfig {
    use flow_sight::core::{GUIPerformanceConfig, BufferConfig, DataProcessingConfig, MonitoringConfig};
    
    PerformanceConfig {
        gui: GUIPerformanceConfig {
            target_fps: 30,           // 平衡性能和流畅度
            min_fps: 15,              // 最低可接受帧率
            max_fps: 60,              // 最高帧率限制
            adaptive_fps: true,       // 启用自适应帧率
            render_batch_size: 200,   // 批量渲染命令
            vsync_enabled: false,     // 禁用垂直同步减少延迟
        },
        buffer: BufferConfig {
            event_buffer_size: 32768,     // 32K事件缓冲区
            auto_resize: true,            // 启用自动扩容
            max_buffer_size: 131072,      // 最大128K
            resize_threshold: 0.8,        // 80%使用率触发扩容
            backpressure_enabled: true,   // 启用背压控制
            backpressure_threshold: 0.9,  // 90%使用率触发背压
        },
        processing: DataProcessingConfig {
            max_events_per_cycle: 1000,      // 每周期最大处理事件数
            aggregation_batch_size: 2000,    // 聚合批处理大小
            price_precision: 0.01,           // 1分钱价格精度
            caching_enabled: true,           // 启用智能缓存
            cache_expiry_ms: 2000,           // 2秒缓存过期
        },
        monitoring: MonitoringConfig {
            metrics_interval_ms: 1000,       // 1秒收集指标
            log_level_filter: "info".to_string(),
            report_interval_ms: 10000,       // 10秒性能报告
            memory_monitoring: true,         // 启用内存监控
        },
    }
}

/// 创建GPU渲染选项
#[cfg(feature = "gui")]
fn create_gpu_render_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 1000.0])  // 更大的默认窗口
            .with_title("FlowSight - 高性能币安期货分析系统")
            .with_min_inner_size([1200.0, 800.0])
            .with_icon(create_app_icon())
            .with_resizable(true)
            .with_maximized(false),
        
        // GPU硬件加速配置
        renderer: eframe::Renderer::Wgpu,
        hardware_acceleration: eframe::HardwareAcceleration::Required,
        
        // 性能优化设置
        vsync: false,                    // 禁用垂直同步
        multisampling: 0,                // 禁用抗锯齿以提高性能
        depth_buffer: 0,                 // 不需要深度缓冲区
        stencil_buffer: 0,               // 不需要模板缓冲区
        
        // 窗口设置
        transparent: false,              // 不透明以提高性能
        decorations: true,               // 保留窗口装饰
        fullsize_content: false,         // 标准内容大小
        titlebar_shown: true,            // 显示标题栏
        titlebar_buttons_shown: true,    // 显示窗口按钮
        always_on_top: false,            // 不总是置顶
        
        ..Default::default()
    }
}

/// 创建应用程序图标
#[cfg(feature = "gui")]
fn create_app_icon() -> egui::IconData {
    // 创建一个简单的32x32像素图标
    let size = 32;
    let mut rgba = Vec::with_capacity(size * size * 4);

    for y in 0..size {
        for x in 0..size {
            let center_x = size as f32 / 2.0;
            let center_y = size as f32 / 2.0;
            let distance = ((x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2)).sqrt();
            let max_distance = center_x;

            if distance <= max_distance {
                // 创建蓝色到金色的渐变效果
                let intensity = (1.0 - distance / max_distance);
                rgba.push((50.0 + intensity * 205.0) as u8);   // R: 蓝到金
                rgba.push((100.0 + intensity * 155.0) as u8);  // G: 渐变
                rgba.push((200.0 - intensity * 150.0) as u8);  // B: 蓝色基调
                rgba.push(255);                                 // A: 完全不透明
            } else {
                // 透明背景
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}

/// 设置服务
#[cfg(feature = "gui")]
fn setup_services(service_manager: &mut ServiceManager, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("初始化服务架构...");

    // 1. 配置管理服务 (基础服务，无依赖)
    let config_service = Box::new(ConfigurationService::new());
    service_manager.register_service(
        "configuration".to_string(),
        config_service,
        vec![],
    )?;
    log::debug!("✓ 配置管理服务已注册");

    // 2. 事件服务 (依赖配置服务)
    let event_service = Box::new(EventService::new(32768)); // 32K事件缓冲区
    service_manager.register_service(
        "event".to_string(),
        event_service,
        vec!["configuration".to_string()],
    )?;
    log::debug!("✓ 事件处理服务已注册");

    // 3. 数据处理服务 (核心服务)
    let data_config = DataProcessingConfig {
        worker_threads: (num_cpus::get().max(2)).min(8), // 2-8个工作线程
        batch_size: 2000,                                // 大批量处理
        processing_timeout: std::time::Duration::from_millis(100),
        price_precision: 0.01,                           // 1分钱精度
        enable_caching: true,                            // 启用缓存
        cache_expiry: std::time::Duration::from_secs(3), // 3秒缓存
        max_queue_size: 50000,                           // 50K队列大小
    };
    let data_service = Box::new(DataProcessingService::new(data_config));
    service_manager.register_service(
        "data_processing".to_string(),
        data_service,
        vec!["event".to_string()],
    )?;
    log::debug!("✓ 数据处理服务已注册 ({}线程)", num_cpus::get().max(2));

    // 4. 渲染服务 (GUI服务)
    let render_config = RenderingConfig {
        target_fps: 30,              // 30 FPS目标
        min_fps: 10,                 // 最低10 FPS
        max_fps: 60,                 // 最高60 FPS
        adaptive_fps: true,          // 自适应帧率
        vsync: false,                // 禁用垂直同步
        batch_size: 500,             // 大批量渲染
        gpu_acceleration: true,       // GPU加速
        render_threads: 2,           // 2个渲染线程
        max_queue_size: 5000,        // 5K渲染队列
    };
    let render_service = Box::new(RenderingService::new(render_config));
    service_manager.register_service(
        "rendering".to_string(),
        render_service,
        vec!["data_processing".to_string()],
    )?;
    log::debug!("✓ GPU渲染服务已注册");

    // 5. 性能监控服务 (监控所有服务)
    let perf_config = PerformanceServiceConfig {
        monitoring_interval: std::time::Duration::from_millis(2000), // 2秒监控间隔
        reporting_interval: std::time::Duration::from_secs(30),      // 30秒报告间隔
        auto_tuning_enabled: true,                                   // 启用自动调优
        memory_monitoring_enabled: true,                             // 内存监控
        cpu_monitoring_enabled: true,                                // CPU监控
        network_monitoring_enabled: false,                           // 网络监控(暂时禁用)
        metrics_retention: std::time::Duration::from_hours(2),       // 2小时指标保留
        performance_thresholds: Default::default(),
    };
    let performance_service = Box::new(PerformanceService::new(PerformanceConfig::default()));
    service_manager.register_service(
        "performance".to_string(),
        performance_service,
        vec!["configuration".to_string(), "event".to_string(), 
             "data_processing".to_string(), "rendering".to_string()],
    )?;
    log::debug!("✓ 性能监控服务已注册");

    log::info!("服务架构初始化完成 - 共5个服务已注册");
    Ok(())
}

/// 设置egui上下文
#[cfg(feature = "gui")]
fn setup_egui_context(ctx: &egui::Context, config: &PerformanceConfig) {
    // 高性能渲染选项
    ctx.options_mut(|o| {
        // 禁用辅助功能以提高性能
        o.screen_reader = false;
        o.preload_font_glyphs = false;
        
        // 优化交互响应
        o.input_options.scroll_zoom_speed = 0.3;
        o.input_options.zoom_speed = 0.8;
        o.input_options.zoom_with_mouse_wheel = true;
        
        // 设置合理的缩放
        o.zoom_factor = 1.0;
        
        // GPU渲染优化
        o.tessellation_options.feathering_size_in_pixels = 1.0;
        o.tessellation_options.coarse_tessellation_culling = true;
        o.tessellation_options.parallel_tessellation = true;
        o.tessellation_options.bezier_tolerance = 0.02;
        o.tessellation_options.epsilon = 0.01;
    });

    // 设置高性能视觉样式
    setup_high_performance_style(ctx);
    
    // 配置优化字体
    setup_optimized_fonts(ctx);
    
    log::info!("egui高性能上下文配置完成");
}

/// 设置高性能样式
#[cfg(feature = "gui")]
fn setup_high_performance_style(ctx: &egui::Context) {
    use egui::{Color32, Rounding, Stroke, Visuals};

    let mut visuals = Visuals::dark();

    // 纯色背景 - 避免渐变提高性能
    visuals.panel_fill = Color32::from_gray(18);
    visuals.window_fill = Color32::from_gray(22);
    visuals.faint_bg_color = Color32::from_gray(12);
    visuals.extreme_bg_color = Color32::from_gray(8);

    // 高对比度文本
    visuals.override_text_color = Some(Color32::WHITE);
    visuals.warn_fg_color = Color32::from_rgb(255, 200, 50);
    visuals.error_fg_color = Color32::from_rgb(255, 80, 80);
    visuals.hyperlink_color = Color32::from_rgb(100, 150, 255);

    // 简化控件样式
    visuals.widgets.noninteractive.bg_fill = Color32::from_gray(28);
    visuals.widgets.inactive.bg_fill = Color32::from_gray(32);
    visuals.widgets.hovered.bg_fill = Color32::from_gray(42);
    visuals.widgets.active.bg_fill = Color32::from_gray(52);

    // 最小圆角设置
    let minimal_rounding = Rounding::same(1.0);
    visuals.widgets.noninteractive.rounding = minimal_rounding;
    visuals.widgets.inactive.rounding = minimal_rounding;
    visuals.widgets.hovered.rounding = minimal_rounding;
    visuals.widgets.active.rounding = minimal_rounding;

    // 简化边框
    let thin_stroke = Stroke::new(0.5, Color32::from_gray(60));
    visuals.widgets.noninteractive.bg_stroke = thin_stroke;
    visuals.widgets.inactive.bg_stroke = thin_stroke;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_gray(80));
    visuals.widgets.active.bg_stroke = Stroke::new(1.5, Color32::from_gray(100));

    // 禁用装饰效果
    visuals.striped = false;
    visuals.indent_has_left_vline = false;
    visuals.button_frame = true;
    visuals.menu_rounding = Rounding::same(2.0);
    visuals.slider_trailing_fill = false;

    ctx.set_visuals(visuals);
}

/// 设置优化字体
#[cfg(feature = "gui")]
fn setup_optimized_fonts(ctx: &egui::Context) {
    use egui::{FontDefinitions, FontData, FontFamily};

    let mut fonts = FontDefinitions::default();

    // 系统字体路径 (Windows)
    let font_candidates = vec![
        ("Microsoft YaHei", "C:/Windows/Fonts/msyh.ttc"),     // 微软雅黑
        ("SimHei", "C:/Windows/Fonts/simhei.ttf"),            // 黑体
        ("Consolas", "C:/Windows/Fonts/consola.ttf"),         // 等宽字体
        ("Segoe UI", "C:/Windows/Fonts/segoeui.ttf"),         // 系统UI字体
    ];

    let mut loaded_fonts = 0;
    for (name, path) in font_candidates {
        if let Ok(font_data) = std::fs::read(path) {
            fonts.font_data.insert(
                name.to_lowercase(),
                FontData::from_owned(font_data),
            );
            
            // 优先级设置
            match name {
                "Consolas" => {
                    fonts.families
                        .entry(FontFamily::Monospace)
                        .or_default()
                        .insert(0, name.to_lowercase());
                }
                _ => {
                    fonts.families
                        .entry(FontFamily::Proportional)
                        .or_default()
                        .insert(loaded_fonts, name.to_lowercase());
                }
            }
            
            loaded_fonts += 1;
            log::debug!("已加载字体: {} ({})", name, path);
            
            if loaded_fonts >= 2 { // 限制字体数量以节省内存
                break;
            }
        }
    }

    if loaded_fonts > 0 {
        ctx.set_fonts(fonts);
        log::info!("字体系统初始化完成 - 加载了{}个字体", loaded_fonts);
    } else {
        log::warn!("未能加载系统字体，使用默认字体");
    }
}

/// 获取可用内存(MB)
#[cfg(feature = "gui")]
fn get_available_memory_mb() -> u64 {
    // 简化的内存获取，实际项目中可以使用系统API
    1024 // 返回1GB作为示例
}

/// 非GUI模式提示
#[cfg(not(feature = "gui"))]
fn main() {
    println!("FlowSight - 高性能币安期货交易分析系统");
    println!();
    println!("此版本需要GUI功能支持。请使用以下命令启动:");
    println!("  cargo run --features gui");
    println!();
    println!("可选参数:");
    println!("  cargo run --features gui ETHUSDT     # 指定交易对");
    println!("  RUST_LOG=debug cargo run --features gui  # 启用调试日志");
    println!();
    println!("系统要求:");
    println!("  - Windows 10+ 或 Linux");
    println!("  - 支持WGPU的GPU (DirectX 12/Vulkan/OpenGL)");
    println!("  - 最少4GB内存");
    
    std::process::exit(1);
}