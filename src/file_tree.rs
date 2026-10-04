use std::path::{Path, PathBuf};

/// Árbol de archivos de la carpeta abierta (barra lateral).
#[derive(Default)]
pub struct FileTree {
    root: Option<PathBuf>,
}

impl FileTree {
    pub fn set_root(&mut self, root: PathBuf) {
        self.root = Some(root);
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// Dibuja el árbol. Devuelve la ruta de un archivo si el usuario hizo clic en él.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        // TODO(ejecutor): árbol colapsable de carpetas/archivos.
        ui.label("Sin carpeta abierta");
        None
    }
}
