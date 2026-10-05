use crate::document::Document;
use egui::text::{ByteIndex, CCursor, CCursorRange, LayoutJob, LayoutSection};
use egui_extras::syntax_highlighting::{CodeTheme, highlight};
use std::ops::Range;

/// Tamaño de fuente del editor, compartido por el texto y los números de línea.
const FONT_SIZE: f32 = 14.0;

/// Fondo del área de código (Monokai): gutter y texto comparten el mismo fondo,
/// sin caja ni borde alrededor del texto.
pub const BG: egui::Color32 = egui::Color32::from_rgb(0x27, 0x28, 0x22);
/// Números de línea del gutter, líneas inactivas.
pub const GUTTER_FG: egui::Color32 = egui::Color32::from_rgb(0x75, 0x71, 0x5E);
/// Número de línea de la fila donde está el cursor.
pub const GUTTER_FG_ACTIVE: egui::Color32 = egui::Color32::from_rgb(0xF8, 0xF8, 0xF2);
/// Resaltado de la línea actual (se pinta debajo del texto).
pub const LINE_HL: egui::Color32 = egui::Color32::from_rgb(0x3E, 0x3D, 0x32);
/// Fondo de la barra de pestañas.
pub const TAB_BAR_BG: egui::Color32 = egui::Color32::from_rgb(0x1E, 0x1F, 0x1C);

/// Fondo de los matches de búsqueda (amarillo tenue).
const MATCH_BG: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 220, 0, 60);
/// Fondo del match actual (naranja).
const CURRENT_MATCH_BG: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(255, 140, 0, 120);

/// Dibuja el área de edición del documento activo: números de línea a la izquierda
/// y el texto con resaltado de sintaxis y de búsqueda. `matches` son rangos (en
/// bytes) a resaltar por la búsqueda; `current_match` es el seleccionado.
pub fn show(
    ui: &mut egui::Ui,
    doc: &mut Document,
    matches: &[Range<usize>],
    current_match: Option<usize>,
) {
    // Fondo Monokai para toda el área, sin borde ni caja alrededor del texto.
    ui.painter().rect_filled(ui.max_rect(), 0.0, BG);

    let lang = doc.extension();
    let theme = CodeTheme::dark(FONT_SIZE);
    let font_id = egui::FontId::monospace(FONT_SIZE);

    // Altura visible antes de entrar al ScrollArea: adentro el ui entrega todo
    // el espacio que le pedimos a los widgets, no el que cabe en pantalla.
    let line_height = ui.fonts_mut(|f| f.row_height(&font_id));
    let visible_rows = ((ui.available_height() / line_height).ceil() as usize).clamp(1, 100_000);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                // --- Números de línea ---
                // Solo reservamos el ancho (según la cantidad de dígitos); los
                // números se pintan después con `paint_line_numbers`, usando la
                // posición real del galley del TextEdit para que alineen
                // exactamente con las líneas del texto.
                let digits = line_count(&doc.text).to_string().len();
                let gutter_width = ui
                    .painter()
                    .layout_no_wrap("0".repeat(digits), font_id.clone(), egui::Color32::WHITE)
                    .size()
                    .x
                    + 12.0;
                let gutter_rect = ui
                    .allocate_exact_size(egui::vec2(gutter_width, 1.0), egui::Sense::hover())
                    .0;

                // Reservamos el shape de la línea actual ANTES de pintar el
                // gutter, para que el rect empiece en el borde izquierdo del
                // gutter y quede debajo del texto (y de los números).
                let hl_idx = ui.painter().add(egui::Shape::Noop);

                // --- TextEdit con resaltado de sintaxis y de búsqueda ---
                // El layouter usa `buf` (el texto que le pasa el TextEdit), no
                // `doc.text`, para no pelear el préstamo mutable del buffer.
                let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap_width: f32| {
                    let mut job = highlight(ui.ctx(), ui.style(), &theme, buf.as_str(), &lang);
                    apply_highlights(&mut job, matches, current_match);
                    // Sin wrap: las líneas largas ensanchan el contenido y
                    // activan el scroll horizontal. (El wrap_width que pasa el
                    // TextEdit en 0.36 es el ancho visible, aunque estemos
                    // dentro de un ScrollArea horizontal.)
                    job.wrap.max_width = f32::INFINITY;
                    ui.fonts_mut(|f| f.layout_job(job))
                };

                let output = egui::TextEdit::multiline(&mut doc.text)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(visible_rows)
                    .lock_focus(true)
                    .layouter(&mut layouter)
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::symmetric(8, 0))
                    .show(ui);

                // --- Línea actual resaltada (debajo del texto) ---
                if let Some(range) = output.cursor_range {
                    let row = output
                        .galley
                        .pos_from_cursor(range.primary)
                        .translate(output.galley_pos.to_vec2());
                    let full = egui::Rect::from_min_max(
                        egui::pos2(ui.max_rect().left(), row.top()),
                        egui::pos2(ui.max_rect().right() + 10_000.0, row.bottom()),
                    );
                    ui.painter()
                        .set(hl_idx, egui::Shape::rect_filled(full, 0.0, LINE_HL));
                }

                // `doc.text` puede haber cambiado dentro de `show` (tecleo),
                // así que los números se calculan sobre el texto final.
                paint_line_numbers(ui, &doc.text, &output, font_id, gutter_rect.right());

                // Cuando cambia el match actual, lo selecciona y lo centra.
                reveal_current_match(ui, doc, matches, current_match, &output);
            });
        });
}

/// Cantidad de líneas del texto: `lines()` + 1 si termina en '\n' (mínimo 1).
fn line_count(text: &str) -> usize {
    let mut count = text.lines().count();
    if text.ends_with('\n') {
        count += 1;
    }
    count.max(1)
}

/// Índice (en caracteres) del primer carácter de cada línea.
fn line_start_chars(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    let mut chars = 0;
    for ch in text.chars() {
        chars += 1;
        if ch == '\n' {
            starts.push(chars);
        }
    }
    starts
}

/// Índice en CARACTERES del byte `byte_index`, o `None` si no cae en un límite.
fn char_index(text: &str, byte_index: usize) -> Option<usize> {
    (byte_index <= text.len() && text.is_char_boundary(byte_index))
        .then(|| text[..byte_index].chars().count())
}

/// Pinta los números de línea, alineados con las filas del galley del TextEdit
/// (misma fuente, mismo alto de fila). El número de la línea donde está el
/// cursor se pinta claro; el resto, tenue. El gutter comparte el fondo del
/// editor (`BG`), sin línea separadora.
fn paint_line_numbers(
    ui: &egui::Ui,
    text: &str,
    output: &egui::widgets::text_edit::TextEditOutput,
    font_id: egui::FontId,
    right_x: f32,
) {
    let painter = ui.painter();
    // Fila (y, en coords del galley) del cursor primario.
    let cursor_row_y = output
        .cursor_range
        .map(|range| output.galley.pos_from_cursor(range.primary).min.y);

    for (i, char_start) in line_start_chars(text).into_iter().enumerate() {
        // Rect (de alto una fila) del inicio de la línea, en coordenadas del
        // galley; se pasa a pantalla con `galley_pos`.
        let row_rect = output.galley.pos_from_cursor(CCursor::new(char_start));
        let is_active = cursor_row_y.is_some_and(|y| (row_rect.min.y - y).abs() < 0.5);
        // 12 px de espacio a la derecha de los números.
        let pos = egui::pos2(right_x - 12.0, row_rect.min.y + output.galley_pos.y);
        painter.text(
            pos,
            egui::Align2::RIGHT_TOP,
            (i + 1).to_string(),
            font_id.clone(),
            if is_active {
                GUTTER_FG_ACTIVE
            } else {
                GUTTER_FG
            },
        );
    }
}

/// Cuando `current_match` cambia (respecto al último frame), mueve el cursor
/// del TextEdit al rango de la coincidencia y desplaza el scroll para
/// centrarla en la vista.
fn reveal_current_match(
    ui: &egui::Ui,
    doc: &Document,
    matches: &[Range<usize>],
    current_match: Option<usize>,
    output: &egui::widgets::text_edit::TextEditOutput,
) {
    let id = output.response.id;
    let last_key = id.with("moca_current_match");

    let last = ui
        .ctx()
        .data(|d| d.get_temp::<Option<usize>>(last_key))
        .flatten();
    if last == current_match {
        return; // no cambió
    }
    ui.ctx()
        .data_mut(|d| d.insert_temp(last_key, current_match));

    let Some(range) = current_match.and_then(|i| matches.get(i)) else {
        return;
    };
    let (Some(start_char), Some(end_char)) = (
        char_index(&doc.text, range.start),
        char_index(&doc.text, range.end),
    ) else {
        return; // el texto cambió y el rango quedó desalineado; el próximo frame lo corrige
    };

    // Selecciona el match moviendo el cursor del TextEdit.
    if let Some(mut state) = egui::widgets::text_edit::TextEditState::load(ui.ctx(), id) {
        state.cursor.set_char_range(Some(CCursorRange::two(
            CCursor::new(start_char),
            CCursor::new(end_char),
        )));
        state.store(ui.ctx(), id);
    }

    // Centra el match en la vista.
    let start_rect = output
        .galley
        .pos_from_cursor(CCursor::new(start_char))
        .translate(output.galley_pos.to_vec2());
    let end_rect = output
        .galley
        .pos_from_cursor(CCursor::new(end_char))
        .translate(output.galley_pos.to_vec2());
    ui.scroll_to_rect(start_rect.union(end_rect), Some(egui::Align::Center));
}

/// Resalta los rangos de `ranges` (índices en bytes del texto del job) pintando
/// el fondo de las secciones que se cruzan con cada rango: amarillo tenue para
/// un match cualquiera y naranja para `ranges[current]`. Las secciones se
/// parten en los límites de los rangos, conservando el cubrimiento completo
/// del texto sin huecos ni solapes.
fn apply_highlights(job: &mut LayoutJob, ranges: &[Range<usize>], current: Option<usize>) {
    if ranges.is_empty() {
        return;
    }

    let old_sections = std::mem::take(&mut job.sections);
    let mut sections = Vec::with_capacity(old_sections.len() + 2 * ranges.len());

    for section in old_sections {
        let start = usize::from(section.byte_range.start);
        let end = usize::from(section.byte_range.end);

        if start == end {
            sections.push(section); // sección vacía: se conserva intacta
            continue;
        }

        let leading_space = section.leading_space;
        let format = section.format;

        // Cortes dentro de la sección: sus límites más los de cada rango que la cruza.
        let mut cuts = vec![start, end];
        for range in ranges {
            let cut_start = range.start.clamp(start, end);
            let cut_end = range.end.clamp(start, end);
            if cut_start < cut_end {
                cuts.push(cut_start);
                cuts.push(cut_end);
            }
        }
        cuts.sort_unstable();
        cuts.dedup();

        // Solo el primer pedazo conserva el leading_space (indentación) de la
        // sección; los demás empiezan a mitad de fila.
        let mut first = true;
        for pair in cuts.windows(2) {
            let (sub_start, sub_end) = (pair[0], pair[1]);
            let mut sub_format = format.clone();
            if let Some(bg) = background_for(sub_start, sub_end, ranges, current) {
                sub_format.background = bg;
            }
            sections.push(LayoutSection {
                leading_space: if first { leading_space } else { 0.0 },
                byte_range: ByteIndex(sub_start)..ByteIndex(sub_end),
                format: sub_format,
            });
            first = false;
        }
    }

    job.sections = sections;
}

/// Fondo para el subrango `[sub_start, sub_end)` si cae dentro de un match.
fn background_for(
    sub_start: usize,
    sub_end: usize,
    ranges: &[Range<usize>],
    current: Option<usize>,
) -> Option<egui::Color32> {
    if let Some(r) = current.and_then(|i| ranges.get(i))
        && sub_start >= r.start
        && sub_end <= r.end
    {
        return Some(CURRENT_MATCH_BG);
    }
    ranges
        .iter()
        .any(|r| sub_start >= r.start && sub_end <= r.end)
        .then_some(MATCH_BG)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::text::TextFormat;

    fn section(start: usize, end: usize) -> LayoutSection {
        LayoutSection {
            leading_space: 0.0,
            byte_range: ByteIndex(start)..ByteIndex(end),
            format: TextFormat::default(),
        }
    }

    fn make_job(text: &str, sections: Vec<LayoutSection>) -> LayoutJob {
        let mut job = LayoutJob::default();
        job.text = text.to_owned();
        job.sections = sections;
        job
    }

    /// (inicio, fin, fondo) de cada sección resultante.
    fn bounds(job: &LayoutJob) -> Vec<(usize, usize, egui::Color32)> {
        job.sections
            .iter()
            .map(|s| {
                (
                    usize::from(s.byte_range.start),
                    usize::from(s.byte_range.end),
                    s.format.background,
                )
            })
            .collect()
    }

    /// Las secciones cubren todo el texto sin huecos ni solapes.
    fn assert_full_coverage(job: &LayoutJob) {
        let mut pos = 0;
        for section in &job.sections {
            assert_eq!(
                usize::from(section.byte_range.start),
                pos,
                "la sección no empieza donde terminó la anterior (hueco o solape)"
            );
            pos = usize::from(section.byte_range.end);
        }
        assert_eq!(pos, job.text.len(), "cubrimiento incompleto del texto");
    }

    #[test]
    fn apply_highlights_parte_secciones_y_conserva_cubrimiento() {
        // "hola mundo cruel" (16 bytes) en tres secciones de sintaxis.
        let mut job = make_job(
            "hola mundo cruel",
            vec![section(0, 5), section(5, 11), section(11, 16)],
        );
        // "mundo" (match 0) y "cruel" (match 1, el actual).
        let ranges = vec![5..10, 11..16];

        apply_highlights(&mut job, &ranges, Some(1));

        assert_full_coverage(&job);
        assert_eq!(
            bounds(&job),
            vec![
                (0, 5, egui::Color32::TRANSPARENT),
                (5, 10, MATCH_BG), // match dentro de la sección 5..11
                (10, 11, egui::Color32::TRANSPARENT), // resto de esa sección
                (11, 16, CURRENT_MATCH_BG), // match actual, sección completa
            ]
        );
    }

    #[test]
    fn apply_highlights_parte_un_rango_que_cruza_secciones() {
        let mut job = make_job(
            "0123456789AB",
            vec![section(0, 4), section(4, 8), section(8, 12)],
        );
        apply_highlights(&mut job, &[2..10], None);

        assert_full_coverage(&job);
        assert_eq!(
            bounds(&job),
            vec![
                (0, 2, egui::Color32::TRANSPARENT),
                (2, 4, MATCH_BG),
                (4, 8, MATCH_BG),
                (8, 10, MATCH_BG),
                (10, 12, egui::Color32::TRANSPARENT),
            ]
        );
    }

    #[test]
    fn apply_highlights_casos_de_borde() {
        // Match pegado al límite de una sección y sección vacía conservada.
        let mut job = make_job(
            "abc\ndef",
            vec![section(0, 0), section(0, 4), section(4, 7)],
        );
        apply_highlights(&mut job, &[4..7], Some(0));

        assert_full_coverage(&job);
        assert_eq!(
            bounds(&job),
            vec![
                (0, 0, egui::Color32::TRANSPARENT), // sección vacía intacta
                (0, 4, egui::Color32::TRANSPARENT),
                (4, 7, CURRENT_MATCH_BG),
            ]
        );

        // Rango fuera del texto: no cambia nada.
        let mut job2 = make_job("abc", vec![section(0, 3)]);
        apply_highlights(&mut job2, &[10..20], None);
        assert_eq!(bounds(&job2), vec![(0, 3, egui::Color32::TRANSPARENT)]);

        // Sin rangos: intacto.
        let mut job3 = make_job("abc", vec![section(0, 3)]);
        apply_highlights(&mut job3, &[], None);
        assert_eq!(bounds(&job3), vec![(0, 3, egui::Color32::TRANSPARENT)]);
    }

    #[test]
    fn conteo_de_lineas() {
        assert_eq!(line_count(""), 1);
        assert_eq!(line_count("hola"), 1);
        assert_eq!(line_count("hola\n"), 2);
        assert_eq!(line_count("hola\nmundo"), 2);
        assert_eq!(line_count("a\n\n\nb"), 4);

        // line_start_chars siempre coincide con line_count.
        assert_eq!(line_start_chars(""), vec![0]);
        assert_eq!(line_start_chars("hola"), vec![0]);
        assert_eq!(line_start_chars("hola\n"), vec![0, 5]);
        assert_eq!(line_start_chars("hola\nmundo"), vec![0, 5]);
        assert_eq!(line_start_chars("a\n\n\nb"), vec![0, 2, 3, 4]);
        assert_eq!(line_start_chars("a\nb").len(), line_count("a\nb"));
        assert_eq!(line_start_chars("hola\n").len(), line_count("hola\n"));
    }
}
