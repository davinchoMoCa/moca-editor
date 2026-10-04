use std::path::{Path, PathBuf};

/// Buscador rápido de archivos (Cmd+P) con coincidencia difusa.
#[derive(Default)]
pub struct Finder {
    pub open: bool,
}

impl Finder {
    /// Abre/cierra el buscador. Al abrir, indexa los archivos de `root`.
    pub fn toggle(&mut self, root: Option<&Path>) {
        let _ = root;
        self.open = !self.open;
    }

    /// Dibuja el buscador si está abierto. Devuelve el archivo elegido.
    pub fn ui(&mut self, ctx: &egui::Context) -> Option<PathBuf> {
        // TODO(ejecutor): ventana flotante con input + lista filtrada.
        let _ = ctx;
        None
    }
}
