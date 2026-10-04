use crate::document::Document;
use std::ops::Range;

/// Dibuja el área de edición del documento activo: números de línea a la izquierda
/// y el texto con resaltado de sintaxis. `matches` son rangos (en bytes) a resaltar
/// por la búsqueda; `current_match` es el seleccionado.
pub fn show(ui: &mut egui::Ui, doc: &mut Document, matches: &[Range<usize>], current_match: Option<usize>) {
    // TODO(ejecutor): números de línea + resaltado de sintaxis + resaltar matches.
    let _ = (matches, current_match);
    ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut doc.text).code_editor());
}
