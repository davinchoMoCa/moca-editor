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

/// Por encima de este tamaño (bytes) solo se resalta la sintaxis de una ventana
/// de líneas alrededor de lo visible: syntect sobre el archivo completo tarda
/// segundos y se repetiría con cada tecla.
const FULL_HIGHLIGHT_LIMIT: usize = 256 * 1024;
/// Granularidad de la ventana de resaltado, para que no cambie con cada pixel de scroll.
const CHUNK_LINES: usize = 400;
/// Color del texto fuera de la ventana resaltada.
const PLAIN_FG: egui::Color32 = egui::Color32::from_rgb(0xF8, 0xF8, 0xF2);

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
                let first_line_id = egui::Id::new("moca_first_visible_line");
                let gutter_width = ui
                    .painter()
                    .layout_no_wrap("0".repeat(digits), font_id.clone(), egui::Color32::WHITE)
                    .size()
                    .x
                    + 16.0
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
                    let text = buf.as_str();
                    let mut job = if text.len() <= FULL_HIGHLIGHT_LIMIT {
                        highlight(ui.ctx(), ui.style(), &theme, text, &lang)
                    } else {
                        // Ventana alrededor de la primera línea visible del frame
                        // anterior; `highlight` la memoiza mientras no cambie.
                        let first = ui
                            .ctx()
                            .data(|d| d.get_temp::<usize>(first_line_id))
                            .unwrap_or(0);
                        let starts = line_byte_starts(text);
                        let lines = highlight_window(first, starts.len());
                        let end = starts.get(lines.end).copied().unwrap_or(text.len());
                        let bytes = starts.get(lines.start).copied().unwrap_or(end)..end;
                        let sub =
                            highlight(ui.ctx(), ui.style(), &theme, &text[bytes.clone()], &lang);
                        let plain = egui::text::TextFormat::simple(font_id.clone(), PLAIN_FG);
                        compose_window_job(text, bytes, sub, plain)
                    };
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
                    .margin(egui::Margin {
                        left: 4,
                        right: 16,
                        top: 8,
                        bottom: 8,
                    })
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

                // Primera línea visible, para la ventana de resaltado y los números.
                let first_visible = ((ui.clip_rect().top() - output.galley_pos.y) / line_height)
                    .floor()
                    .max(0.0) as usize;
                ui.ctx()
                    .data_mut(|d| d.insert_temp(first_line_id, first_visible));

                // `doc.text` puede haber cambiado dentro de `show` (tecleo),
                // así que los números se calculan sobre el texto final.
                let visible_rows = (ui.clip_rect().height() / line_height).ceil() as usize + 2;
                paint_line_numbers(
                    ui,
                    line_count(&doc.text),
                    first_visible..first_visible + visible_rows,
                    &output,
                    font_id,
                    line_height,
                    gutter_rect.right(),
                );

                // Cuando cambia el match actual, lo selecciona y lo centra.
                reveal_current_match(ui, doc, matches, current_match, &output);
            });
        });
}

/// Cantidad de líneas del texto (una más que los '\n'; mínimo 1).
fn line_count(text: &str) -> usize {
    text.bytes().filter(|&b| b == b'\n').count() + 1
}

/// Byte donde empieza cada línea: 0 y la posición después de cada '\n'.
fn line_byte_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(
            text.bytes()
                .enumerate()
                .filter(|&(_, b)| b == b'\n')
                .map(|(i, _)| i + 1),
        )
        .collect()
}

/// Rango de líneas a resaltar alrededor de `first_visible_line`: el bloque de
/// `CHUNK_LINES` que la contiene más uno antes y uno después.
fn highlight_window(first_visible_line: usize, total_lines: usize) -> Range<usize> {
    let chunk = first_visible_line / CHUNK_LINES;
    let start = chunk.saturating_sub(1) * CHUNK_LINES;
    let end = ((chunk + 2) * CHUNK_LINES).min(total_lines);
    start.min(end)..end
}

/// Arma el job del texto completo: `sub` (el resaltado de `&full_text[window]`)
/// desplazado a su lugar, y texto plano antes y después de la ventana.
fn compose_window_job(
    full_text: &str,
    window: Range<usize>,
    sub: LayoutJob,
    plain: egui::text::TextFormat,
) -> LayoutJob {
    let mut job = LayoutJob {
        text: full_text.to_owned(),
        ..Default::default()
    };
    let plain_section = |range: Range<usize>| LayoutSection {
        leading_space: 0.0,
        byte_range: ByteIndex(range.start)..ByteIndex(range.end),
        format: plain.clone(),
    };
    if window.start > 0 {
        job.sections.push(plain_section(0..window.start));
    }
    for section in sub.sections {
        let start = usize::from(section.byte_range.start) + window.start;
        let end = usize::from(section.byte_range.end) + window.start;
        job.sections.push(LayoutSection {
            byte_range: ByteIndex(start)..ByteIndex(end),
            ..section
        });
    }
    if window.end < full_text.len() {
        job.sections
            .push(plain_section(window.end..full_text.len()));
    }
    job.wrap.max_width = f32::INFINITY;
    job
}

/// Índice en CARACTERES del byte `byte_index`, o `None` si no cae en un límite.
fn char_index(text: &str, byte_index: usize) -> Option<usize> {
    (byte_index <= text.len() && text.is_char_boundary(byte_index))
        .then(|| text[..byte_index].chars().count())
}

/// Pinta los números de las líneas `visible` (acotadas a `total_lines`). Sin wrap
/// hay una fila por línea, así que la posición sale de multiplicar por el alto de
/// fila: nada de recorrer el galley por cada línea. El número de la línea del
/// cursor se pinta claro; el resto, tenue.
fn paint_line_numbers(
    ui: &egui::Ui,
    total_lines: usize,
    visible: Range<usize>,
    output: &egui::widgets::text_edit::TextEditOutput,
    font_id: egui::FontId,
    line_height: f32,
    right_x: f32,
) {
    let painter = ui.painter();
    let active_line = output.cursor_range.map(|range| {
        (output.galley.pos_from_cursor(range.primary).min.y / line_height).round() as usize
    });

    for i in visible.start.min(total_lines)..visible.end.min(total_lines) {
        // 12 px de espacio a la derecha de los números.
        let pos = egui::pos2(right_x - 12.0, output.galley_pos.y + i as f32 * line_height);
        painter.text(
            pos,
            egui::Align2::RIGHT_TOP,
            (i + 1).to_string(),
            font_id.clone(),
            if active_line == Some(i) {
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
        let first = ranges.partition_point(|r| r.end <= start);
        for range in ranges[first..].iter().take_while(|r| r.start < end) {
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
    // Los rangos vienen ordenados y sin solapes: el único candidato es el
    // primero que termina después de `sub_start`.
    let i = ranges.partition_point(|r| r.end <= sub_start);
    ranges
        .get(i)
        .is_some_and(|r| sub_start >= r.start && sub_end <= r.end)
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

        assert_eq!(line_byte_starts(""), vec![0]);
        assert_eq!(line_byte_starts("a\nbc\n"), vec![0, 2, 5]);
        for t in ["", "hola", "hola\n", "a\n\n\nb"] {
            assert_eq!(line_byte_starts(t).len(), line_count(t));
        }
    }

    #[test]
    fn ventana_de_resaltado() {
        assert_eq!(highlight_window(0, 10_000), 0..800);
        assert_eq!(highlight_window(1000, 10_000), 400..1600);
        assert_eq!(highlight_window(9_900, 10_000).end, 10_000);
        assert_eq!(highlight_window(0, 0), 0..0);
        assert_eq!(highlight_window(0, 50), 0..50);
    }

    #[test]
    fn compose_window_job_cubre_todo_el_texto() {
        let text = "aaa\nbbb\nccc\n";
        let window = 4..8; // "bbb\n"
        let sub = make_job("bbb\n", vec![section(0, 2), section(2, 4)]);
        let plain = TextFormat::simple(egui::FontId::monospace(14.0), PLAIN_FG);
        let job = compose_window_job(text, window, sub, plain);

        assert_full_coverage(&job);
        let ranges: Vec<_> = bounds(&job).iter().map(|&(a, b, _)| (a, b)).collect();
        assert_eq!(ranges, vec![(0, 4), (4, 6), (6, 8), (8, 12)]);
        assert_eq!(job.sections[0].format.color, PLAIN_FG);
        assert_eq!(job.sections[3].format.color, PLAIN_FG);
    }

    #[test]
    fn apply_highlights_muchos_rangos_es_lineal() {
        // 10.000 secciones de 10 bytes y un match de 3 bytes en cada una.
        let text = "x".repeat(100_000);
        let sections = (0..10_000).map(|i| section(i * 10, i * 10 + 10)).collect();
        let ranges: Vec<_> = (0..10_000).map(|i| i * 10 + 2..i * 10 + 5).collect();
        let mut job = make_job(&text, sections);

        let t = std::time::Instant::now();
        apply_highlights(&mut job, &ranges, Some(5_000));
        assert!(
            t.elapsed().as_millis() < 500,
            "apply_highlights tardó demasiado"
        );

        assert_full_coverage(&job);
        assert_eq!(job.sections.len(), 30_000);
    }
}
