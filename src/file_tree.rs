use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Una entrada de directorio: carpeta o archivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

const ALWAYS_IGNORED: [&str; 3] = [".git", "target", "node_modules"];

/// Lista el contenido inmediato de `dir` (solo un nivel).
///
/// Respeta lo que diga `.gitignore`, ignora siempre `.git`, `target` y
/// `node_modules`, y muestra archivos ocultos (`.env` sí aparece).
/// Devuelve las carpetas primero y luego los archivos, alfabético sin
/// distinguir mayúsculas.
pub fn list_dir(dir: &Path) -> Vec<Entry> {
    let mut builder = ignore::WalkBuilder::new(dir);
    builder
        .max_depth(Some(1))
        .hidden(false) // mostrar ocultos como .env
        .require_git(false) // respetar .gitignore aunque no haya repo de git
        .filter_entry(|e| {
            !ALWAYS_IGNORED
                .iter()
                .any(|name| e.file_name().to_str() == Some(*name))
        });

    let mut out = Vec::new();
    for entry in builder.build() {
        let Ok(entry) = entry else { continue };
        // La primera entrada es `dir` mismo (depth 0).
        if entry.depth() == 0 {
            continue;
        }
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        let Some(name) = entry.path().file_name() else {
            continue;
        };
        out.push(Entry {
            path: entry.path().to_path_buf(),
            name: name.to_string_lossy().into_owned(),
            is_dir: file_type.is_dir(),
        });
    }

    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    out
}

/// Árbol de archivos de la carpeta abierta (barra lateral).
#[derive(Default)]
pub struct FileTree {
    root: Option<PathBuf>,
    /// Caché de contenidos por carpeta, para no leer disco cada frame.
    cache: HashMap<PathBuf, Arc<Vec<Entry>>>,
    /// Última ruta clicada en el árbol.
    selected: Option<PathBuf>,
}

impl FileTree {
    pub fn set_root(&mut self, root: PathBuf) {
        self.root = Some(root);
        self.cache.clear();
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// Devuelve el contenido de `dir`, leyendo disco solo la primera vez.
    fn children(&mut self, dir: &Path) -> Arc<Vec<Entry>> {
        self.cache
            .entry(dir.to_path_buf())
            .or_insert_with(|| Arc::new(list_dir(dir)))
            .clone()
    }

    /// Dibuja el árbol. Devuelve la ruta de un archivo si el usuario hizo clic en él.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        let Some(root) = self.root.clone() else {
            ui.label("Sin carpeta abierta");
            ui.label("⇧⌘O para abrir una");
            return None;
        };

        // Encabezado con el nombre de la carpeta raíz y botón de refresco.
        ui.horizontal(|ui| {
            if ui
                .small_button("⟳")
                .on_hover_text("Recargar árbol de archivos")
                .clicked()
            {
                self.cache.clear();
            }
            ui.label(root.file_name().map_or_else(
                || root.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            ));
        });

        let mut clicked: Option<PathBuf> = None;
        self.show_dir(ui, &root, &mut clicked);

        if let Some(path) = &clicked {
            self.selected = Some(path.clone());
        }
        clicked
    }

    /// Dibuja el contenido (un nivel) de `dir`; recursivo para subcarpetas.
    fn show_dir(&mut self, ui: &mut egui::Ui, dir: &Path, clicked: &mut Option<PathBuf>) {
        let entries = self.children(dir);
        for entry in entries.iter() {
            if entry.is_dir {
                egui::CollapsingHeader::new(&entry.name)
                    .id_salt(entry.path.clone())
                    .default_open(false)
                    .show(ui, |ui| {
                        self.show_dir(ui, &entry.path, clicked);
                    });
            } else {
                let selected = self.selected.as_deref() == Some(entry.path.as_path());
                if ui.selectable_label(selected, &entry.name).clicked() {
                    *clicked = Some(entry.path.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process;

    #[test]
    fn list_dir_orden_e_ignores() {
        let tmp = std::env::temp_dir().join(format!(
            "moca-file-tree-test-{}-{}",
            process::id(),
            "orden_e_ignores"
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::create_dir_all(tmp.join("target")).unwrap();
        fs::create_dir_all(tmp.join("build")).unwrap();
        fs::create_dir_all(tmp.join("Zeta")).unwrap();
        fs::write(tmp.join("zeta.rs"), "").unwrap();
        fs::write(tmp.join("Alpha.txt"), "").unwrap();
        fs::write(tmp.join(".env"), "SECRET=1").unwrap();
        // `build` solo se ignora vía .gitignore; `target` siempre igual.
        fs::write(tmp.join(".gitignore"), "build/\n").unwrap();

        let entries = list_dir(&tmp);
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();

        // Carpetas primero (alfabético), luego archivos (alfabético sin mayús).
        assert_eq!(
            names,
            vec!["src", "Zeta", ".env", ".gitignore", "Alpha.txt", "zeta.rs"]
        );

        // `target` y lo del .gitignore (`build`) no aparecen.
        assert!(!names.contains(&"target"));
        assert!(!names.contains(&"build"));

        // Los flags de carpeta/archivo están bien puestos.
        assert!(entries.iter().filter(|e| e.is_dir).count() == 2);
        assert!(entries.iter().find(|e| e.name == ".env").unwrap().is_dir == false);

        fs::remove_dir_all(&tmp).unwrap();
    }
}
