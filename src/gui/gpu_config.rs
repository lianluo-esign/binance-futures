use eframe::egui;

/// GPU渲染配置
pub struct GpuConfig {
    pub power_preference: eframe::wgpu::PowerPreference,
    pub vsync: bool,
    pub multisample_count: u32,
    pub antialiasing: bool,
    pub texture_format: eframe::wgpu::TextureFormat,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            power_preference: eframe::wgpu::PowerPreference::HighPerformance,
            vsync: true,
            multisample_count: 4,
            antialiasing: true,
            texture_format: eframe::wgpu::TextureFormat::Bgra8UnormSrgb,
        }
    }
}

impl GpuConfig {
    /// 创建高性能GPU配置
    pub fn high_performance() -> Self {
        Self {
            power_preference: eframe::wgpu::PowerPreference::HighPerformance,
            vsync: false, // 关闭垂直同步以获得更高帧率
            multisample_count: 1, // 减少多重采样以提高性能
            antialiasing: false, // 关闭抗锯齿以提高性能
            texture_format: eframe::wgpu::TextureFormat::Bgra8UnormSrgb,
        }
    }

    /// 创建高质量GPU配置
    pub fn high_quality() -> Self {
        Self {
            power_preference: eframe::wgpu::PowerPreference::HighPerformance,
            vsync: true,
            multisample_count: 4,
            antialiasing: true,
            texture_format: eframe::wgpu::TextureFormat::Bgra8UnormSrgb,
        }
    }

    /// 获取高性能GPU配置
    pub fn get_power_preference(&self) -> eframe::wgpu::PowerPreference {
        self.power_preference
    }
}

/// GPU性能监控
pub struct GpuPerformanceMonitor {
    frame_times: Vec<f32>,
    max_samples: usize,
    last_frame_time: std::time::Instant,
}

impl Default for GpuPerformanceMonitor {
    fn default() -> Self {
        Self {
            frame_times: Vec::new(),
            max_samples: 60, // 保存最近60帧的数据
            last_frame_time: std::time::Instant::now(),
        }
    }
}

impl GpuPerformanceMonitor {
    pub fn new(max_samples: usize) -> Self {
        Self {
            frame_times: Vec::new(),
            max_samples,
            last_frame_time: std::time::Instant::now(),
        }
    }

    /// 记录帧时间
    pub fn record_frame(&mut self) {
        let now = std::time::Instant::now();
        let frame_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        self.frame_times.push(frame_time);
        if self.frame_times.len() > self.max_samples {
            self.frame_times.remove(0);
        }
    }

    /// 获取平均帧率
    pub fn get_fps(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        let avg_frame_time = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        if avg_frame_time > 0.0 {
            1.0 / avg_frame_time
        } else {
            0.0
        }
    }

    /// 获取最小帧时间（最高帧率）
    pub fn get_min_frame_time(&self) -> f32 {
        self.frame_times.iter().fold(f32::INFINITY, |a, &b| a.min(b))
    }

    /// 获取最大帧时间（最低帧率）
    pub fn get_max_frame_time(&self) -> f32 {
        self.frame_times.iter().fold(0.0, |a, &b| a.max(b))
    }

    /// 渲染性能信息到UI
    pub fn render_performance_info(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("GPU性能监控");
            
            let fps = self.get_fps();
            let min_frame_time = self.get_min_frame_time();
            let max_frame_time = self.get_max_frame_time();
            
            ui.label(format!("平均帧率: {:.1} FPS", fps));
            ui.label(format!("最高帧率: {:.1} FPS", 1.0 / min_frame_time));
            ui.label(format!("最低帧率: {:.1} FPS", 1.0 / max_frame_time));
            ui.label(format!("平均帧时间: {:.2} ms", self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32 * 1000.0));
            
            // 帧率颜色指示
            let fps_color = if fps >= 60.0 {
                egui::Color32::GREEN
            } else if fps >= 30.0 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::RED
            };
            
            ui.colored_label(fps_color, format!("性能状态: {}", 
                if fps >= 60.0 { "优秀" } 
                else if fps >= 30.0 { "良好" } 
                else { "需要优化" }
            ));
        });
    }
}

/// GPU渲染优化设置
pub struct GpuRenderSettings {
    pub enable_gpu_acceleration: bool,
    pub enable_vsync: bool,
    pub enable_antialiasing: bool,
    pub target_fps: f32,
    pub adaptive_quality: bool,
}

impl Default for GpuRenderSettings {
    fn default() -> Self {
        Self {
            enable_gpu_acceleration: true,
            enable_vsync: true,
            enable_antialiasing: true,
            target_fps: 60.0,
            adaptive_quality: true,
        }
    }
}

impl GpuRenderSettings {
    /// 渲染设置UI
    pub fn render_settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("GPU渲染设置");
            
            ui.checkbox(&mut self.enable_gpu_acceleration, "启用GPU加速");
            ui.checkbox(&mut self.enable_vsync, "垂直同步");
            ui.checkbox(&mut self.enable_antialiasing, "抗锯齿");
            ui.checkbox(&mut self.adaptive_quality, "自适应质量");
            
            ui.add(egui::Slider::new(&mut self.target_fps, 30.0..=144.0).text("目标帧率"));
            
            ui.separator();
            
            ui.label("说明:");
            ui.label("• GPU加速: 使用GPU进行渲染，提高性能");
            ui.label("• 垂直同步: 防止画面撕裂，但可能降低帧率");
            ui.label("• 抗锯齿: 平滑边缘，但消耗更多GPU资源");
            ui.label("• 自适应质量: 根据性能自动调整渲染质量");
        });
    }

    /// 根据设置创建GPU配置
    pub fn create_gpu_config(&self) -> GpuConfig {
        if self.enable_gpu_acceleration {
            GpuConfig {
                power_preference: eframe::wgpu::PowerPreference::HighPerformance,
                vsync: self.enable_vsync,
                multisample_count: if self.enable_antialiasing { 4 } else { 1 },
                antialiasing: self.enable_antialiasing,
                texture_format: eframe::wgpu::TextureFormat::Bgra8UnormSrgb,
            }
        } else {
            // 回退到CPU渲染
            GpuConfig::default()
        }
    }
} 