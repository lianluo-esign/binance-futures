use eframe::egui;
use binance_futures::gui::UnifiedOrderBookWidget;
use binance_futures::app::ReactiveApp;
use binance_futures::Config;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("测试按钮功能"),
        ..Default::default()
    };

    eframe::run_native(
        "测试按钮功能",
        options,
        Box::new(|_cc| Box::new(TestApp::default())),
    )
}

struct TestApp {
    widget: UnifiedOrderBookWidget,
    app: ReactiveApp,
}

impl Default for TestApp {
    fn default() -> Self {
        let config = Config::default();
        Self {
            widget: UnifiedOrderBookWidget::new(),
            app: ReactiveApp::new(config),
        }
    }
}

impl eframe::App for TestApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.widget.show(ui, &self.app);
        });
    }
}
