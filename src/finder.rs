use std::path::{Path, PathBuf};

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

/// Máximo de archivos indexados al abrir el buscador.
const MAX_FILES: usize = 20_000;
/// Máximo de resultados filtrados y mostrados.
const MAX_RESULTS: usize = 50;

/// Buscador rápido de archivos (Cmd+P) con coincidencia difusa.
#[derive(Default)]
pub struct Finder {
    pub open: bool,
    /// Carpeta raíz indexada. `None` = todavía no se abrió una carpeta.
    root: Option<PathBuf>,
    /// Rutas relativas a `root` de los archivos indexados.
    files: Vec<String>,
    query: String,
    selected: usize,
    /// El TextEdit debe pedir el foco al abrir.
    focus_pending: bool,
}

impl Finder {
    /// Abre/cierra el buscador. Al abrir, indexa los archivos de `root`.
    pub fn toggle(&mut self, root: Option<&Path>) {
        self.open = !self.open;
        if self.open {
            self.root = root.map(Path::to_path_buf);
            self.files = root.map_or_else(Vec::new, index_files);
            self.query.clear();
            self.selected = 0;
            self.focus_pending = true;
        }
    }

    /// Dibuja el buscador si está abierto. Devuelve el archivo elegido.
    pub fn ui(&mut self, ctx: &egui::Context) -> Option<PathBuf> {
        if !self.open {
            return None;
        }

        let mut chosen = None;
        let results = filter(&self.files, &self.query, MAX_RESULTS);

        // La selección no puede apuntar fuera de los resultados.
        if self.selected >= results.len() {
            self.selected = results.len().saturating_sub(1);
        }

        // Consumar las teclas ANTES de dibujar el TextEdit para que no las procese él.
        let mut scroll_to_selection = false;
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown))
            && self.selected + 1 < results.len()
        {
            self.selected += 1;
            scroll_to_selection = true;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp))
            && self.selected > 0
        {
            self.selected -= 1;
            scroll_to_selection = true;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
            if let Some(&(file_idx, _)) = results.get(self.selected) {
                if let Some(path) = self.path_of(file_idx) {
                    chosen = Some(path);
                    self.open = false;
                }
            }
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.open = false;
        }

        egui::Window::new("finder")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .movable(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 60.0])
            .default_width(500.0).min_width(500.0)
            .show(ctx, |ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Buscar archivo…")
                        .desired_width(f32::INFINITY),
                );
                if self.focus_pending {
                    resp.request_focus();
                    self.focus_pending = false;
                }
                if resp.changed() {
                    self.selected = 0;
                    scroll_to_selection = true;
                }

                if self.root.is_none() {
                    ui.label("Abre una carpeta primero (⇧⌘O)");
                } else if results.is_empty() {
                    ui.weak("Sin resultados");
                } else {
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        for (row, (file_idx, indices)) in results.iter().enumerate() {
                            let selected = row == self.selected;
                            let job = highlight(ui, &self.files[*file_idx], indices);
                            let row_resp = ui.selectable_label(selected, job);
                            if row_resp.clicked() && let Some(path) = self.path_of(*file_idx) {
                                chosen = Some(path);
                                self.open = false;
                            }
                            if selected && scroll_to_selection {
                                row_resp.scroll_to_me(None);
                            }
                        }
                    });
                }
            });

        chosen
    }

    /// Ruta absoluta del archivo indexado en la posición `idx` (`root` + relativa).
    fn path_of(&self, idx: usize) -> Option<PathBuf> {
        let rel = self.files.get(idx)?;
        self.root.as_ref().map(|root| root.join(rel))
    }
}

/// Indexa recursivamente los archivos de `root` (respeta .gitignore, ignora
/// .git/target/node_modules) hasta `MAX_FILES`. Devuelve rutas relativas.
fn index_files(root: &Path) -> Vec<String> {
    ignore::WalkBuilder::new(root)
        .hidden(false)
        .require_git(false)
        .filter_entry(|e| {
            !matches!(e.file_name().to_str(), Some(".git" | "target" | "node_modules"))
        })
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_some_and(|t| t.is_file()))
        .take(MAX_FILES)
        .filter_map(|entry| {
            entry
                .path()
                .strip_prefix(root)
                .ok()
                .map(|rel| rel.display().to_string())
        })
        .collect()
}

/// Filtra `files` con coincidencia difusa sobre la ruta relativa.
/// Devuelve (índice en `files`, índices de caracteres coincidentes) por
/// resultado, ordenados por puntaje descendente (empates por índice) y
/// hasta `limit`. Con consulta vacía devuelve los primeros `limit`.
fn filter(files: &[String], query: &str, limit: usize) -> Vec<(usize, Vec<usize>)> {
    if query.is_empty() {
        return files
            .iter()
            .enumerate()
            .take(limit)
            .map(|(i, _)| (i, Vec::new()))
            .collect();
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, usize, Vec<usize>)> = files
        .iter()
        .enumerate()
        .filter_map(|(i, path)| {
            matcher
                .fuzzy_indices(path, query)
                .map(|(score, indices)| (score, i, indices))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, i, indices)| (i, indices))
        .collect()
}

/// Construye la fila de la lista resaltando los caracteres coincidentes.
fn highlight(ui: &egui::Ui, text: &str, indices: &[usize]) -> egui::text::LayoutJob {
    let strong = ui.visuals().strong_text_color();
    let weak = ui.visuals().weak_text_color();
    let mut job = egui::text::LayoutJob::default();
    for (i, ch) in text.chars().enumerate() {
        let color = if indices.contains(&i) { strong } else { weak };
        job.append(
            &ch.to_string(),
            0.0,
            egui::TextFormat {
                color,
                ..Default::default()
            },
        );
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> Vec<String> {
        vec![
            "src/main.rs".to_string(),
            "src/app.rs".to_string(),
            "src/finder.rs".to_string(),
            "README.md".to_string(),
            "Cargo.toml".to_string(),
        ]
    }

    #[test]
    fn consulta_vacia_devuelve_los_primeros_archivos() {
        let res = filter(&files(), "", 3);
        assert_eq!(
            res,
            vec![(0, Vec::new()), (1, Vec::new()), (2, Vec::new())]
        );
    }

    #[test]
    fn coincidencia_difusa_encuentra_src_main_rs() {
        let res = filter(&files(), "mnrs", MAX_RESULTS);
        assert_eq!(res.len(), 1, "solo src/main.rs coincide con mnrs");
        let (idx, indices) = &res[0];
        assert_eq!(*idx, 0);
        // Los índices de caracteres deletrean "mnrs" en "src/main.rs".
        let text: Vec<char> = files()[0].chars().collect();
        let deletreado: String = indices.iter().map(|&i| text[i]).collect();
        assert_eq!(deletreado, "mnrs");
    }

    #[test]
    fn respeta_el_limite() {
        let todos = filter(&files(), "rs", 100);
        assert!(todos.len() > 2, "deben coincidir varios .rs");
        let limitados = filter(&files(), "rs", 2);
        assert_eq!(limitados.len(), 2);
        assert_eq!(&limitados, &todos[..2]);
    }

    #[test]
    fn ordena_por_puntaje_descendente() {
        let f = vec![
            "zmxain.rs".to_string(), // coincidencia dispersa: puntúa menos
            "main.rs".to_string(),   // coincidencia contigua al inicio: puntúa más
        ];
        let res = filter(&f, "main", 10);
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].0, 1, "main.rs debe ir primero (mayor puntaje)");
        assert_eq!(res[1].0, 0);
    }
}
