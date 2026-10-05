mod app;
mod document;
mod editor;
mod file_tree;
mod finder;
#[allow(dead_code)]
mod icons;
mod search;

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let candidates = [
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Apple Symbols.ttf",
    ];

    fonts.font_data.insert(
        "material_symbols".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/MaterialSymbolsOutlined-subset.ttf"
        ))),
    );

    if let Some(bytes) = candidates.iter().find_map(|p| std::fs::read(p).ok()) {
        fonts.font_data.insert(
            "symbols".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
    }

    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        if let Some(list) = fonts.families.get_mut(&family) {
            // Material primero: sus íconos viven en la zona de uso privado de Unicode,
            // que otras fuentes (SF, emoji de egui) también ocupan.
            if fonts.font_data.contains_key("symbols") {
                list.insert(1.min(list.len()), "symbols".to_owned());
            }
            list.insert(1.min(list.len()), "material_symbols".to_owned());
        } else {
            fonts
                .families
                .entry(family.clone())
                .or_default()
                .push("material_symbols".to_owned());
            if fonts.font_data.contains_key("symbols") {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .push("symbols".to_owned());
            }
        }
    }
    ctx.set_fonts(fonts);
}

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
        Box::new(|cc| {
            install_fonts(&cc.egui_ctx);
            Ok(Box::new(app::MocaEditor::from_args(
                std::env::args().skip(1),
            )))
        }),
    )
}
