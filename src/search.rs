use std::ops::Range;

/// Barra de buscar/reemplazar (Cmd+F) del documento activo.
#[derive(Default)]
pub struct SearchBar {
    pub open: bool,
    pub query: String,
    pub replacement: String,
    pub case_sensitive: bool,
    pub current: Option<usize>,
}

impl SearchBar {
    /// Dibuja la barra y aplica reemplazos sobre `text`. Devuelve los rangos que coinciden.
    pub fn ui(&mut self, ui: &mut egui::Ui, text: &mut String) -> Vec<Range<usize>> {
        // TODO(ejecutor): inputs, siguiente/anterior, reemplazar, reemplazar todo.
        let _ = ui;
        find_all(text, &self.query, self.case_sensitive)
    }
}

/// Todos los rangos (en bytes) donde aparece `query` en `text`.
pub fn find_all(text: &str, query: &str, case_sensitive: bool) -> Vec<Range<usize>> {
    // TODO(ejecutor)
    let _ = (text, query, case_sensitive);
    Vec::new()
}

/// Reemplaza todas las apariciones; devuelve cuántas reemplazó.
pub fn replace_all(text: &mut String, query: &str, replacement: &str, case_sensitive: bool) -> usize {
    // TODO(ejecutor)
    let _ = (text, query, replacement, case_sensitive);
    0
}
