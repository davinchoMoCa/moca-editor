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
        ui.horizontal(|ui| {
            // --- Buscar ---
            let search_response = ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Buscar")
                    .desired_width(150.0),
            );
            // Singleline pierde el foco al pulsar Enter (patrón canónico de egui).
            let enter_next = ui.input(|i| i.key_pressed(egui::Key::Enter))
                && (search_response.has_focus() || search_response.lost_focus());

            // --- Reemplazar ---
            ui.add(
                egui::TextEdit::singleline(&mut self.replacement)
                    .hint_text("Reemplazar")
                    .desired_width(150.0),
            );

            // --- Aa ---
            ui.checkbox(&mut self.case_sensitive, "Aa");

            // Coincidencias con el estado ya actualizado de la fila.
            let mut matches = find_all(text, &self.query, self.case_sensitive);
            self.clamp_current(&matches);

            // --- Contador ---
            let counter = if let Some(i) = self.current {
                format!("{} de {}", i + 1, matches.len())
            } else {
                "sin resultados".to_owned()
            };
            ui.label(counter);

            // --- ◀ / ▶ ---
            let prev_clicked = ui
                .add_enabled(!matches.is_empty(), egui::Button::new("◀"))
                .clicked();
            let next_clicked = ui
                .add_enabled(!matches.is_empty(), egui::Button::new("▶"))
                .clicked();
            let next = enter_next || next_clicked;

            // --- Reemplazar / Todo ---
            let replace_one = ui
                .add_enabled(self.current.is_some(), egui::Button::new("Reemplazar"))
                .clicked();
            let replace_every = ui
                .add_enabled(!matches.is_empty(), egui::Button::new("Todo"))
                .clicked();

            // === Acciones ===
            if enter_next {
                // Mantener el foco en Buscar para poder encadenar Enter.
                search_response.request_focus();
            }
            if !matches.is_empty() {
                let n = matches.len();
                if next {
                    self.current = Some(match self.current {
                        Some(i) => (i + 1) % n,
                        None => 0,
                    });
                }
                if prev_clicked {
                    self.current = Some(match self.current {
                        Some(i) => (i + n - 1) % n,
                        None => n - 1,
                    });
                }
            }
            if replace_one {
                if let Some(i) = self.current {
                    if let Some(range) = matches.get(i).cloned() {
                        text.replace_range(range, &self.replacement);
                        matches = find_all(text, &self.query, self.case_sensitive);
                        self.clamp_current(&matches);
                    }
                }
            }
            if replace_every {
                replace_all(text, &self.query, &self.replacement, self.case_sensitive);
                matches = find_all(text, &self.query, self.case_sensitive);
                self.clamp_current(&matches);
            }

            matches
        })
        .inner
    }

    /// Mantiene `current` dentro de rango: `None` si no hay resultados.
    fn clamp_current(&mut self, matches: &[Range<usize>]) {
        let n = matches.len();
        self.current = match self.current {
            _ if n == 0 => None,
            Some(i) if i < n => Some(i),
            Some(_) => Some(n - 1),
            None => Some(0),
        };
    }
}

/// Todos los rangos (en bytes) donde aparece `query` en `text`.
pub fn find_all(text: &str, query: &str, case_sensitive: bool) -> Vec<Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }
    let query_chars: Vec<char> = query.chars().collect();
    let mut matches = Vec::new();
    let mut cursor = text.char_indices().peekable();

    'search: while let Some(&(start, _)) = cursor.peek() {
        // Empareja `query_chars` desde aquí. Si no coincide, `cursor` avanza un
        // carácter; si coincide, salta la coincidencia completa (sin solapamientos,
        // de izquierda a derecha). El rango se mide en bytes del texto original,
        // nunca sobre una versión lowercase (que puede cambiar de longitud).
        let mut candidate = cursor.clone();
        let mut end = start;
        for &q in &query_chars {
            match candidate.next() {
                Some((pos, c)) => {
                    if chars_match(c, q, case_sensitive) {
                        end = pos + c.len_utf8();
                    } else {
                        cursor.next();
                        continue 'search;
                    }
                }
                None => {
                    cursor.next();
                    continue 'search;
                }
            }
        }
        matches.push(start..end);
        cursor = candidate;
    }
    matches
}

/// Reemplaza todas las apariciones; devuelve cuántas reemplazó.
pub fn replace_all(
    text: &mut String,
    query: &str,
    replacement: &str,
    case_sensitive: bool,
) -> usize {
    let ranges = find_all(text, query, case_sensitive);
    if ranges.is_empty() {
        return 0;
    }
    let mut result = String::with_capacity(text.len() + ranges.len() * replacement.len());
    let mut last = 0;
    for range in &ranges {
        result.push_str(&text[last..range.start]);
        result.push_str(replacement);
        last = range.end;
    }
    result.push_str(&text[last..]);
    *text = result;
    ranges.len()
}

/// Igualdad de dos caracteres, opcionalmente ignorando mayúsculas.
/// Compara las expansiones de `to_lowercase` carácter a carácter: así "Á"
/// coincide con "á" sin reescribir el texto (que podría cambiar su longitud
/// en bytes, p. ej. "İ" baja a dos caracteres).
fn chars_match(a: char, b: char, case_sensitive: bool) -> bool {
    if case_sensitive {
        a == b
    } else {
        a == b || a.to_lowercase().eq(b.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- find_all ---

    #[test]
    fn find_all_basico() {
        let text = "hola mundo hola";
        assert_eq!(find_all(text, "hola", true), vec![0..4, 11..15]);
    }

    #[test]
    fn find_all_texto_vacio() {
        assert!(find_all("", "a", true).is_empty());
        assert!(find_all("", "", false).is_empty());
    }

    #[test]
    fn find_all_query_vacia() {
        assert!(find_all("hola", "", true).is_empty());
        assert!(find_all("hola", "", false).is_empty());
    }

    #[test]
    fn find_all_sin_coincidencias() {
        assert!(find_all("hola", "xyz", true).is_empty());
    }

    #[test]
    fn find_all_query_mas_larga_que_texto() {
        assert!(find_all("ab", "abc", true).is_empty());
    }

    #[test]
    fn find_all_case_sensitive() {
        let text = "Rust rust RUST";
        assert_eq!(find_all(text, "rust", true), vec![5..9]);
        assert_eq!(find_all(text, "RUST", true), vec![10..14]);
    }

    #[test]
    fn find_all_case_insensitive() {
        let text = "Rust rust RUST";
        assert_eq!(find_all(text, "rust", false), vec![0..4, 5..9, 10..14]);
    }

    #[test]
    fn find_all_acentos() {
        // "Á" (2 bytes) coincide con "á" y los rangos son válidos en el original.
        let text = "código CÓDIGO";
        assert_eq!(find_all(text, "código", false), vec![0..7, 8..15]);
        assert_eq!(&text[0..7], "código");
        assert_eq!(&text[8..15], "CÓDIGO");
    }

    #[test]
    fn find_all_acentos_case_sensitive() {
        assert_eq!(find_all("código CÓDIGO", "código", true), vec![0..7]);
    }

    #[test]
    fn find_all_acento_no_confunde_con_letra_plana() {
        // "á" no debe matchear "a" ni "A": solo se ignora la caja, no los acentos.
        assert_eq!(find_all("Á A a", "á", false), vec![0..2]);
    }

    #[test]
    fn find_all_rangos_validos_cuando_lowercase_cambia_longitud() {
        // İ (U+0130, 2 bytes) baja a dos caracteres; el rango sigue en bytes
        // del texto original.
        let text = "İstanbul";
        assert_eq!(find_all(text, "İ", false), vec![0..2]);
        assert_eq!(&text[0..2], "İ");
        // Con comparación carácter a carácter, "İ" tampoco matchea "i".
        assert!(find_all(text, "i", false).is_empty());
    }

    #[test]
    fn find_all_sin_solapamiento() {
        assert_eq!(find_all("aaa", "aa", true), vec![0..2]);
        assert_eq!(find_all("aaaa", "aa", true), vec![0..2, 2..4]);
    }

    #[test]
    fn find_all_izquierda_a_derecha() {
        assert_eq!(find_all("x x x", "x", true), vec![0..1, 2..3, 4..5]);
    }

    // --- replace_all ---

    #[test]
    fn replace_all_basico() {
        let mut text = String::from("hola hola hola");
        assert_eq!(replace_all(&mut text, "hola", "mundo", true), 3);
        assert_eq!(text, "mundo mundo mundo");
    }

    #[test]
    fn replace_all_query_vacia() {
        let mut text = String::from("hola");
        assert_eq!(replace_all(&mut text, "", "x", true), 0);
        assert_eq!(text, "hola");
    }

    #[test]
    fn replace_all_sin_coincidencias() {
        let mut text = String::from("hola");
        assert_eq!(replace_all(&mut text, "xyz", "abc", true), 0);
        assert_eq!(text, "hola");
    }

    #[test]
    fn replace_all_case_insensitive() {
        let mut text = String::from("Rust rust RUST");
        assert_eq!(replace_all(&mut text, "rust", "egui", false), 3);
        assert_eq!(text, "egui egui egui");
    }

    #[test]
    fn replace_all_acentos() {
        let mut text = String::from("cámara Cámara CAMARA");
        assert_eq!(replace_all(&mut text, "cámara", "casa", false), 2);
        assert_eq!(text, "casa casa CAMARA");
    }

    #[test]
    fn replace_all_multibyte() {
        let mut text = String::from("áéí óéí");
        assert_eq!(replace_all(&mut text, "é", "e", false), 2);
        assert_eq!(text, "áeí óeí");
    }

    #[test]
    fn replace_all_no_reemplaza_dentro_del_reemplazo() {
        let mut text = String::from("aa");
        assert_eq!(replace_all(&mut text, "a", "ba", true), 2);
        assert_eq!(text, "baba");
    }

    #[test]
    fn replace_all_borrado() {
        let mut text = String::from("a-b-c");
        assert_eq!(replace_all(&mut text, "-", "", true), 2);
        assert_eq!(text, "abc");
    }
}
