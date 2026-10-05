use crate::document::Document;
use crate::editor;
use crate::file_tree::FileTree;
use crate::finder::Finder;
use crate::search::SearchBar;
use egui::{Key, KeyboardShortcut, Modifiers};
use std::path::{Path, PathBuf};

const OPEN_FILE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
const OPEN_FOLDER: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::O);
const SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
const NEW: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::N);
const CLOSE_TAB: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::W);
const QUICK_OPEN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::P);
const FIND: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::F);
const OPEN_TERMINAL: KeyboardShortcut = KeyboardShortcut::new(Modifiers::CTRL, Key::Backtick);

pub struct MocaEditor {
    docs: Vec<Document>,
    active: usize,
    file_tree: FileTree,
    finder: Finder,
    search: SearchBar,
    status: String,
}

impl Default for MocaEditor {
    fn default() -> Self {
        Self {
            docs: vec![Document::new_empty()],
            active: 0,
            file_tree: FileTree::default(),
            finder: Finder::default(),
            search: SearchBar::default(),
            status: String::new(),
        }
    }
}

impl MocaEditor {
    /// `moca-editor [carpeta|archivo]...`: las carpetas se abren en el árbol, los archivos en pestañas.
    pub fn from_args(args: impl Iterator<Item = String>) -> Self {
        let mut app = Self::default();
        for arg in args {
            let path = PathBuf::from(arg);
            let path = path.canonicalize().unwrap_or(path);
            if path.is_dir() {
                app.file_tree.set_root(path);
            } else {
                app.open_path(&path);
            }
        }
        app
    }

    fn open_path(&mut self, path: &Path) {
        if let Some(i) = self.docs.iter().position(|d| d.path.as_deref() == Some(path)) {
            self.active = i;
            return;
        }
        match Document::open(path) {
            Ok(doc) => {
                // Reemplaza la pestaña inicial vacía en vez de acumularla.
                let active = &self.docs[self.active];
                if active.path.is_none() && !active.is_dirty() {
                    self.docs[self.active] = doc;
                } else {
                    self.docs.push(doc);
                    self.active = self.docs.len() - 1;
                }
                self.status.clear();
            }
            Err(e) => self.status = format!("No se pudo abrir {}: {e}", path.display()),
        }
    }

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.open_path(&path);
        }
    }

    fn open_folder_dialog(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            self.file_tree.set_root(dir);
        }
    }

    /// Abre Terminal.app en la raíz del proyecto; si no hay carpeta abierta,
    /// en la carpeta del archivo activo.
    fn open_terminal(&mut self) {
        let dir: Option<PathBuf> = self
            .file_tree
            .root()
            .map(Path::to_path_buf)
            .or_else(|| self.docs[self.active].path.as_ref().and_then(|p| p.parent().map(Path::to_path_buf)));
        let Some(dir) = dir else {
            self.status = "Abre una carpeta primero (⇧⌘O)".to_string();
            return;
        };
        match std::process::Command::new("open")
            .args(["-a", "Terminal"])
            .arg(&dir)
            .spawn()
        {
            Ok(_) => self.status = format!("Terminal abierta en {}", dir.display()),
            Err(e) => self.status = format!("No se pudo abrir Terminal: {e}"),
        }
    }

    fn save_active(&mut self) {
        let doc = &mut self.docs[self.active];
        let result = if doc.path.is_some() {
            doc.save()
        } else if let Some(path) = rfd::FileDialog::new().save_file() {
            doc.save_as(&path)
        } else {
            return;
        };
        self.status = match result {
            Ok(()) => format!("Guardado {}", doc.title()),
            Err(e) => format!("Error al guardar: {e}"),
        };
    }

    fn close_tab(&mut self, i: usize) {
        self.docs.remove(i);
        if self.docs.is_empty() {
            self.docs.push(Document::new_empty());
        }
        if self.active >= self.docs.len() {
            self.active = self.docs.len() - 1;
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // Los combos con Shift van primero para que Cmd+O no se coma Cmd+Shift+O.
        if ctx.input_mut(|i| i.consume_shortcut(&OPEN_FOLDER)) {
            self.open_folder_dialog();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&OPEN_FILE)) {
            self.open_file_dialog();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&SAVE)) {
            self.save_active();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&NEW)) {
            self.docs.push(Document::new_empty());
            self.active = self.docs.len() - 1;
        }
        if ctx.input_mut(|i| i.consume_shortcut(&CLOSE_TAB)) {
            self.close_tab(self.active);
        }
        if ctx.input_mut(|i| i.consume_shortcut(&QUICK_OPEN)) {
            let root: Option<PathBuf> = self.file_tree.root().map(Path::to_path_buf);
            self.finder.toggle(root.as_deref());
        }
        if ctx.input_mut(|i| i.consume_shortcut(&FIND)) {
            self.search.open = !self.search.open;
        }
        if ctx.input_mut(|i| i.consume_shortcut(&OPEN_TERMINAL)) {
            self.open_terminal();
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("Archivo", |ui| {
                if ui.button("Nuevo  ⌘N").clicked() {
                    self.docs.push(Document::new_empty());
                    self.active = self.docs.len() - 1;
                }
                if ui.button("Abrir archivo…  ⌘O").clicked() {
                    self.open_file_dialog();
                }
                if ui.button("Abrir carpeta…  ⇧⌘O").clicked() {
                    self.open_folder_dialog();
                }
                if ui.button("Guardar  ⌘S").clicked() {
                    self.save_active();
                }
            });
            ui.menu_button("Ir", |ui| {
                if ui.button("Abrir rápido…  ⌘P").clicked() {
                    let root: Option<PathBuf> = self.file_tree.root().map(Path::to_path_buf);
                    self.finder.toggle(root.as_deref());
                }
                if ui.button("Buscar  ⌘F").clicked() {
                    self.search.open = !self.search.open;
                }
                if ui.button("Abrir terminal  ⌃`").clicked() {
                    self.open_terminal();
                }
            });
        });
    }

    fn tab_bar(&mut self, ui: &mut egui::Ui) {
        let mut to_close = None;
        ui.horizontal(|ui| {
            for (i, doc) in self.docs.iter().enumerate() {
                let dot = if doc.is_dirty() { " ●" } else { "" };
                if ui
                    .selectable_label(i == self.active, format!("{}{dot}", doc.title()))
                    .clicked()
                {
                    self.active = i;
                }
                if ui.small_button("×").clicked() {
                    to_close = Some(i);
                }
                ui.separator();
            }
        });
        if let Some(i) = to_close {
            self.close_tab(i);
        }
    }
}

impl eframe::App for MocaEditor {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_shortcuts(&ctx);

        egui::Panel::top("menu").show(ui, |ui| self.menu_bar(ui));

        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                let doc = &self.docs[self.active];
                ui.label(doc.path.as_ref().map_or("sin título".into(), |p| p.display().to_string()));
                ui.separator();
                ui.label(doc.extension());
                if !self.status.is_empty() {
                    ui.separator();
                    ui.label(&self.status);
                }
            });
        });

        egui::Panel::left("files")
            .resizable(true)
            .default_size(220.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Some(path) = self.file_tree.ui(ui) {
                        self.open_path(&path);
                    }
                });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            self.tab_bar(ui);
            ui.separator();

            let doc = &mut self.docs[self.active];
            let matches = if self.search.open {
                self.search.ui(ui, &mut doc.text)
            } else {
                Vec::new()
            };
            editor::show(ui, doc, &matches, self.search.current);
        });

        if let Some(path) = self.finder.ui(&ctx) {
            self.open_path(&path);
        }
    }
}
