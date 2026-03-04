use eframe::egui;
use resist_gui::app;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("Resist — Circuit Simulator"),
        ..Default::default()
    };

    eframe::run_native(
        "Resist GUI",
        options,
        Box::new(|cc| Ok(Box::new(app::ResistApp::new(cc)))),
    )
}
