use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Un archivo abierto en una pestaña.
pub struct Document {
    pub path: Option<PathBuf>,
    pub text: String,
    saved_text: String,
}

impl Document {
    pub fn new_empty() -> Self {
        Self {
            path: None,
            text: String::new(),
            saved_text: String::new(),
        }
    }

    pub fn open(path: &Path) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        Ok(Self {
            path: Some(path.to_path_buf()),
            saved_text: text.clone(),
            text,
        })
    }

    /// Guarda en `self.path`. Falla si el documento nunca se ha guardado.
    pub fn save(&mut self) -> io::Result<()> {
        let path = self
            .path
            .clone()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "documento sin ruta"))?;
        self.save_as(&path)
    }

    pub fn save_as(&mut self, path: &Path) -> io::Result<()> {
        fs::write(path, &self.text)?;
        self.path = Some(path.to_path_buf());
        self.saved_text = self.text.clone();
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.text != self.saved_text
    }

    pub fn title(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "sin título".to_owned())
    }

    /// Extensión usada para elegir el resaltado de sintaxis ("rs", "py", ...).
    pub fn extension(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.extension())
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_else(|| "txt".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_tracking_and_save() {
        let dir = std::env::temp_dir().join("moca-editor-doc-test");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.txt");
        fs::write(&path, "hola").unwrap();

        let mut doc = Document::open(&path).unwrap();
        assert!(!doc.is_dirty());
        doc.text.push_str(" mundo");
        assert!(doc.is_dirty());
        doc.save().unwrap();
        assert!(!doc.is_dirty());
        assert_eq!(fs::read_to_string(&path).unwrap(), "hola mundo");
        assert_eq!(doc.title(), "a.txt");
        assert_eq!(doc.extension(), "txt");
    }
}
