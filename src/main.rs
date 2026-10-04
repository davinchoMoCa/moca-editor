mod app;
mod document;
mod editor;
mod file_tree;
mod finder;
mod search;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Moca Editor")
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Moca Editor",
        options,
        Box::new(|_cc| Ok(Box::new(app::MocaEditor::default()))),
    )
}
