use binance_futures::{init_logging, Config, TradingGUI};
use std::env;
use std::path::Path;

fn setup_custom_fonts(ctx: &egui::Context) {
    use egui::{FontDefinitions, FontData, FontFamily};

    let mut fonts = FontDefinitions::default();

    // 尝试加载系统中文字体
    let chinese_font_paths = vec![
        "C:/Windows/Fonts/msyh.ttc",      // 微软雅黑
        "C:/Windows/Fonts/simsun.ttc",   // 宋体
        "C:/Windows/Fonts/simhei.ttf",   // 黑体
        "C:/Windows/Fonts/msyh.ttf",     // 微软雅黑 TTF
    ];

    let mut font_loaded = false;
    for font_path in chinese_font_paths {
        if let Ok(font_data) = std::fs::read(font_path) {
            fonts.font_data.insert(
                "chinese".to_owned(),
                FontData::from_owned(font_data),
            );
            font_loaded = true;
            // 字体加载信息写入日志文件，不输出到控制台
            log::info!("已加载中文字体: {}", font_path);
            break;
        }
    }

    if !font_loaded {
        // 字体加载警告写入日志文件，不输出到控制台
        log::warn!("无法从系统加载中文字体");
        return; // 如果无法加载字体，就不设置字体
    }

    // 将中文字体添加到字体族的最前面，确保优先使用
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "chinese".to_owned());

    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "chinese".to_owned());

    // 设置字体
    ctx.set_fonts(fonts);
}

fn setup_dark_theme(ctx: &egui::Context) {
    use egui::{Color32, Rounding, Stroke, Visuals};

    let mut visuals = Visuals::dark();

    // 设置纯黑色背景
    visuals.panel_fill = Color32::BLACK;
    visuals.window_fill = Color32::BLACK;
    visuals.faint_bg_color = Color32::from_gray(10);
    visuals.extreme_bg_color = Color32::from_gray(5);

    // 设置文本颜色为白色，确保对比度
    visuals.override_text_color = Some(Color32::WHITE);
    visuals.warn_fg_color = Color32::from_rgb(255, 200, 100);
    visuals.error_fg_color = Color32::from_rgb(255, 100, 100);

    // 设置表格条纹颜色
    visuals.striped = true;
    visuals.code_bg_color = Color32::from_gray(15);

    // 设置边框和分隔符
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_gray(40));
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_gray(40));
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_gray(60));
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Color32::from_gray(80));

    // 设置按钮和控件背景
    visuals.widgets.noninteractive.bg_fill = Color32::from_gray(20);
    visuals.widgets.inactive.bg_fill = Color32::from_gray(25);
    visuals.widgets.hovered.bg_fill = Color32::from_gray(35);
    visuals.widgets.active.bg_fill = Color32::from_gray(45);

    // 设置圆角
    visuals.widgets.noninteractive.rounding = Rounding::same(4.0);
    visuals.widgets.inactive.rounding = Rounding::same(4.0);
    visuals.widgets.hovered.rounding = Rounding::same(4.0);
    visuals.widgets.active.rounding = Rounding::same(4.0);

    ctx.set_visuals(visuals);
}

/// 加载应用程序图标
fn load_icon() -> egui::IconData {
    let icon_path = "src/image/logo04.png";

    if Path::new(icon_path).exists() {
        match image::open(icon_path) {
            Ok(image) => {
                let image = image.to_rgba8();
                let (width, height) = image.dimensions();
                let rgba = image.into_raw();

                egui::IconData {
                    rgba,
                    width: width as u32,
                    height: height as u32,
                }
            }
            Err(e) => {
                log::warn!("无法加载图标文件 {}: {}", icon_path, e);
                create_default_icon()
            }
        }
    } else {
        log::info!("图标文件不存在: {}, 使用默认图标", icon_path);
        create_default_icon()
    }
}

/// 创建默认图标（简单的32x32像素图标）
fn create_default_icon() -> egui::IconData {
    let size = 32;
    let mut rgba = Vec::with_capacity(size * size * 4);

    for y in 0..size {
        for x in 0..size {
            // 创建一个简单的渐变图标
            let center_x = size as f32 / 2.0;
            let center_y = size as f32 / 2.0;
            let distance = ((x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2)).sqrt();
            let max_distance = center_x;

            if distance <= max_distance {
                // 内部：蓝色渐变
                let intensity = (1.0 - distance / max_distance) * 255.0;
                rgba.push(30);  // R
                rgba.push((60.0 + intensity * 0.4) as u8);  // G
                rgba.push((120.0 + intensity * 0.5) as u8); // B
                rgba.push(255); // A
            } else {
                // 外部：透明
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
            }
        }
    }

    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    init_logging();

    // 获取交易对参数
    let symbol = env::args().nth(1).unwrap_or_else(|| "BTCUSDT".to_string());

    // 创建配置
    let config = Config::new(symbol)
        .with_buffer_size(10000)
        .with_max_reconnects(5)
        .with_max_visible_rows(3000)    // 设置最大可见行数为3000
        .with_price_precision(0.01);    // 设置价格精度为0.01 USD (1分)

    // 创建egui应用
    let app = TradingGUI::new(config);

    // 加载应用程序图标
    let icon_data = load_icon();

    // 配置egui应用选项
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("FlowSight")
            .with_icon(icon_data),
        ..Default::default()
    };

    // 运行egui应用
    eframe::run_native(
        "FlowSight",
        options,
        Box::new(|cc| {
            // 配置中文字体
            setup_custom_fonts(&cc.egui_ctx);

            // 设置黑色主题
            setup_dark_theme(&cc.egui_ctx);

            Box::new(app)
        }),
    )?;

    Ok(())
}