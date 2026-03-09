use eframe::egui;
use resist_gui::app;

fn main() -> eframe::Result {
    let icon = image::load_from_memory(include_bytes!("../../public/ohm.png"))
        .expect("Failed to load icon")
        .to_rgba8();
    let (icon_width, icon_height) = icon.dimensions();
    let icon_data = egui::IconData {
        rgba: icon.into_raw(),
        width: icon_width,
        height: icon_height,
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("Resist — Circuit Simulator")
            .with_icon(icon_data),
        ..Default::default()
    };

    eframe::run_native(
        "Resist GUI",
        options,
        Box::new(|cc| Ok(Box::new(app::ResistApp::new(cc)))),
    )
}
