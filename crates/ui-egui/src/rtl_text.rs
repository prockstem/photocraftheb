//! Right-to-left text (Hebrew, Arabic) in the UI.
//!
//! egui does not apply the Unicode bidirectional algorithm. Its shaper (harfrust) draws each run
//! of right-to-left letters right to left, but runs are split by font, so the words, spaces,
//! digits and Latin text of a Hebrew sentence stay in left-to-right order and the sentence reads
//! backwards. This egui plugin fixes the final shapes of every frame: in each laid-out row that
//! contains right-to-left characters, every shaped right-to-left run is kept as a block and the
//! blocks and remaining glyphs are put into visual order with [`unicode_bidi`] (UAX #9), by
//! moving glyphs inside the already tessellated mesh. Rows are reordered after egui has wrapped
//! them, so wrapped paragraphs keep their lines in reading order; a right-to-left paragraph's
//! rows are also right-aligned when the text was laid out left-aligned. Mirrored pairs such as
//! `(` and `)` are flipped inside right-to-left text.
//!
//! Only drawing changes: layout sizes, hit-testing and text-edit cursors are egui's own, so a
//! caret inside right-to-left text being edited can be drawn away from its letter.
//! Text without right-to-left characters is untouched and costs one scan per galley.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Arc, Weak};

use egui::epaint::text::{Galley, PlacedRow};
use egui::{Align, Rect, Shape};
use unicode_bidi::{BidiClass, BidiInfo, Level, bidi_class};

/// Is `c` a strong right-to-left character (Hebrew, Arabic, Syriac, Thaana, N'Ko, …)?
fn is_rtl_char(c: char) -> bool {
    matches!(bidi_class(c), BidiClass::R | BidiClass::AL)
}

/// Does `text` contain any right-to-left character? (ASCII text never does.)
pub fn has_rtl(text: &str) -> bool {
    !text.is_ascii() && text.chars().any(is_rtl_char)
}

/// The direction of a paragraph from its first strong character (UAX #9 rules P2–P3): `true`
/// for right-to-left. A paragraph with no strong character is left-to-right.
fn paragraph_is_rtl(chars: impl IntoIterator<Item = char>) -> bool {
    for c in chars {
        match bidi_class(c) {
            BidiClass::R | BidiClass::AL => return true,
            BidiClass::L => return false,
            _ => {}
        }
    }
    false
}

/// Characters drawn mirrored inside right-to-left runs (Unicode `Bidi_Mirrored` pairs that occur
/// in UI text).
fn is_mirrored(c: char) -> bool {
    matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '«' | '»' | '‹' | '›' | '≤' | '≥')
}

/// The visual order of one line of a paragraph: indices into `chars` from left to right, each
/// with whether the glyph is drawn mirrored. Always a permutation of `0..chars.len()`.
pub fn visual_order(chars: &[char], rtl: bool) -> Vec<(usize, bool)> {
    let identity = || (0..chars.len()).map(|i| (i, false)).collect();
    let text: String = chars.iter().collect();
    let base = if rtl { Level::rtl() } else { Level::ltr() };
    let info = BidiInfo::new(&text, Some(base));
    let mut char_at_byte = HashMap::with_capacity(chars.len());
    for (ci, (bi, _)) in text.char_indices().enumerate() {
        char_at_byte.insert(bi, ci);
    }
    let mut out = Vec::with_capacity(chars.len());
    for para in &info.paragraphs {
        let (levels, runs) = info.visual_runs(para, para.range.clone());
        for run in runs {
            let level = levels.get(run.start).copied().unwrap_or(base);
            let Some(slice) = text.get(run.clone()) else { return identity() };
            let mut idx: Vec<usize> = slice.char_indices().filter_map(|(b, _)| char_at_byte.get(&(run.start + b)).copied()).collect();
            if level.is_rtl() {
                idx.reverse();
            }
            for i in idx {
                let mirror = level.is_rtl() && chars.get(i).is_some_and(|c| is_mirrored(*c));
                out.push((i, mirror));
            }
        }
    }
    // Defensive: anything but a permutation draws the line as laid out.
    let mut seen = vec![false; chars.len()];
    for &(i, _) in &out {
        match seen.get_mut(i) {
            Some(s) if !*s => *s = true,
            _ => return identity(),
        }
    }
    if out.len() != chars.len() {
        return identity();
    }
    out
}

/// `galley` with every row that needs it in visual order; `None` when nothing changes.
pub fn reorder_galley(galley: &Galley) -> Option<Galley> {
    if !has_rtl(&galley.job.text) {
        return None;
    }
    let mut g = galley.clone();
    let ppp = g.pixels_per_point.max(f32::EPSILON);
    let align_right = g.job.halign == Align::LEFT;
    let right_edge = g.rect.max.x;
    let mut changed = false;
    let mut start = 0;
    while start < g.rows.len() {
        // A paragraph: rows up to and including the one ending with a newline.
        let mut end = start;
        while end + 1 < g.rows.len() && !g.rows.get(end).is_some_and(|r| r.ends_with_newline) {
            end += 1;
        }
        let rows = g.rows.get_mut(start..=end).unwrap_or_default();
        let rtl = paragraph_is_rtl(rows.iter().flat_map(|r| r.row.glyphs.iter().map(|gl| gl.chr)));
        for placed in rows {
            changed |= reorder_row(placed, rtl, ppp);
            if rtl && align_right {
                let shift = right_edge - (placed.pos.x + placed.row.size.x);
                if shift > 0.5 {
                    placed.pos.x += shift;
                    changed = true;
                }
            }
        }
        start = end + 1;
    }
    if !changed {
        return None;
    }
    g.mesh_bounds = g.rows.iter().fold(Rect::NOTHING, |acc, r| acc.union(r.row.visuals.mesh_bounds.translate(r.pos.to_vec2())));
    Some(g)
}

/// Move one row's glyphs into visual order. Returns whether anything moved.
///
/// egui's shaper (harfrust) already lays each run of right-to-left letters out right to left,
/// but runs are split by font, so spaces, digits, punctuation and Latin text sit in separate
/// runs that stay in left-to-right order. Each run of right-to-left glyphs is therefore kept as
/// one block, and the blocks and the remaining glyphs are reordered with the bidi algorithm.
fn reorder_row(placed: &mut PlacedRow, rtl: bool, ppp: f32) -> bool {
    /// Stands in for a whole right-to-left block when computing the bidi order.
    const RTL_BLOCK: char = '\u{05D0}';
    let glyphs = &placed.row.glyphs;
    let mut tokens: Vec<(Range<usize>, char)> = Vec::new();
    let mut i = 0;
    while let Some(g) = glyphs.get(i) {
        if is_rtl_char(g.chr) {
            let start = i;
            while glyphs.get(i).is_some_and(|g| is_rtl_char(g.chr)) {
                i += 1;
            }
            tokens.push((start..i, RTL_BLOCK));
        } else {
            tokens.push((i..i + 1, g.chr));
            i += 1;
        }
    }
    if tokens.is_empty() || (!rtl && !tokens.iter().any(|t| t.1 == RTL_BLOCK)) {
        return false;
    }
    let chars: Vec<char> = tokens.iter().map(|t| t.1).collect();
    let order = visual_order(&chars, rtl);
    if order.iter().enumerate().all(|(at, &(i, m))| at == i && !m) {
        return false;
    }
    let row = Arc::make_mut(&mut placed.row);
    let vertex_end = row.visuals.glyph_vertex_range.end;
    let vertices_of = |glyphs: &[egui::epaint::text::Glyph], i: usize| -> Range<usize> {
        let s = glyphs.get(i).map_or(vertex_end, |g| g.first_vertex as usize);
        let e = glyphs.get(i + 1).map_or(vertex_end, |g| g.first_vertex as usize);
        s..e.max(s).min(vertex_end.max(s))
    };
    let ranges: Vec<Range<usize>> = (0..row.glyphs.len()).map(|i| vertices_of(&row.glyphs, i)).collect();
    let mut x = row.glyphs.iter().map(|g| g.pos.x).fold(f32::INFINITY, f32::min);
    if !x.is_finite() {
        return false;
    }
    let round = |v: f32| (v * ppp).round() / ppp;
    for (t, mirror) in order {
        let Some((range, _)) = tokens.get(t) else { continue };
        let block = row.glyphs.get(range.clone()).unwrap_or_default();
        let left = block.iter().map(|g| g.pos.x).fold(f32::INFINITY, f32::min);
        let right = block.iter().map(|g| g.pos.x + g.advance_width).fold(f32::NEG_INFINITY, f32::max);
        if !left.is_finite() || !right.is_finite() {
            continue;
        }
        let dx = round(x - left);
        let (new_left, new_right) = (left + dx, right + dx);
        for gi in range.clone() {
            if let Some(g) = row.glyphs.get_mut(gi) {
                g.pos.x += dx;
            }
            let verts = ranges.get(gi).cloned().unwrap_or(0..0);
            for v in row.visuals.mesh.vertices.get_mut(verts).unwrap_or_default() {
                v.pos.x += dx;
                if mirror {
                    v.pos.x = new_left + new_right - v.pos.x;
                }
            }
        }
        x += (right - left).max(0.0);
    }
    row.visuals.mesh_bounds = row.visuals.mesh.calc_bounds();
    true
}

/// The egui plugin that puts right-to-left text into visual order before each frame is drawn.
#[derive(Default)]
pub struct RtlTextPlugin {
    /// Reordered galleys by the address of the original (valid while the original is alive).
    cache: HashMap<usize, (Weak<Galley>, Arc<Galley>)>,
}

impl RtlTextPlugin {
    fn fix(&mut self, shape: &mut Shape) {
        match shape {
            Shape::Text(t) if has_rtl(&t.galley.job.text) => t.galley = self.reordered(&t.galley),
            Shape::Vec(v) => v.iter_mut().for_each(|s| self.fix(s)),
            _ => {}
        }
    }

    fn reordered(&mut self, galley: &Arc<Galley>) -> Arc<Galley> {
        let key = Arc::as_ptr(galley) as usize;
        if let Some((orig, done)) = self.cache.get(&key)
            && orig.upgrade().is_some_and(|o| Arc::ptr_eq(&o, galley))
        {
            return done.clone();
        }
        let done = reorder_galley(galley).map_or_else(|| galley.clone(), Arc::new);
        self.cache.insert(key, (Arc::downgrade(galley), done.clone()));
        done
    }
}

impl egui::Plugin for RtlTextPlugin {
    fn debug_name(&self) -> &'static str {
        "photocraft-rtl-text"
    }

    fn output_hook(&mut self, _ctx: &egui::Context, output: &mut egui::FullOutput) {
        self.cache.retain(|_, (orig, _)| orig.strong_count() > 0);
        for clipped in &mut output.shapes {
            self.fix(&mut clipped.shape);
        }
    }
}

/// Installs the right-to-left text plugin.
pub fn install(ctx: &egui::Context) {
    ctx.add_plugin(RtlTextPlugin::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Color32, FontId};

    fn order(s: &str, rtl: bool) -> String {
        let chars: Vec<char> = s.chars().collect();
        visual_order(&chars, rtl).into_iter().map(|(i, m)| if m { '*' } else { chars[i] }).collect()
    }

    #[test]
    fn detects_right_to_left_text() {
        assert!(has_rtl("שכבה"));
        assert!(has_rtl("Layer שכבה"));
        assert!(!has_rtl("Layer"));
        assert!(!has_rtl("レイヤー"));
        assert!(paragraph_is_rtl("12 שכבה abc".chars()));
        assert!(!paragraph_is_rtl("abc שכבה".chars()));
        assert!(!paragraph_is_rtl("123 …".chars()));
    }

    #[test]
    fn hebrew_runs_are_reversed_and_latin_and_numbers_kept() {
        assert_eq!(order("שלום", true), "םולש");
        // Latin and numbers inside Hebrew keep their own order; the runs go right to left.
        assert_eq!(order("שמירה בתור PSD", true), "PSD רותב הרימש");
        assert_eq!(order("גודל 100%", true), "100% לדוג");
        // A Hebrew word inside an English sentence.
        assert_eq!(order("Layer שכבה 1", false), "Layer 1 הבכש");
        // Brackets inside right-to-left text are mirrored.
        assert_eq!(order("שכבה (חדשה)", true), "*השדח* הבכש");
        assert_eq!(order("", true), "");
    }

    fn layout(text: &str, wrap: f32) -> Arc<Galley> {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
        out.textures_delta.clear();
        ctx.fonts_mut(|f| f.layout(text.to_owned(), FontId::proportional(14.0), Color32::WHITE, wrap))
    }

    /// x of the glyph for the `n`-th character in each row, by character.
    fn glyph_x(g: &Galley, c: char) -> Option<(usize, f32)> {
        g.rows.iter().enumerate().find_map(|(r, row)| row.row.glyphs.iter().find(|gl| gl.chr == c).map(|gl| (r, row.pos.x + gl.pos.x)))
    }

    #[test]
    fn a_hebrew_label_reads_right_to_left() {
        // egui's shaper already draws a single Hebrew word right to left: nothing to do.
        let word = layout("אבג", f32::INFINITY);
        let x = |g: &Galley, c| glyph_x(g, c).map(|(_, x)| x).unwrap_or(f32::NAN);
        assert!(x(&word, 'א') > x(&word, 'ג'));
        assert!(reorder_galley(&word).is_none());
        // Two words: egui keeps them in left-to-right order; they must swap.
        let g = layout("אבג דהו", f32::INFINITY);
        assert!(x(&g, 'א') < x(&g, 'ד'), "egui alone puts the first word on the left");
        let r = reorder_galley(&g).expect("reordered");
        assert!(x(&r, 'א') > x(&r, 'ד'), "the first word is drawn rightmost");
        assert!(x(&r, 'א') > x(&r, 'ג'), "and still reads right to left inside");
        // Vertices moved with their glyphs, so the mesh still spans the same width.
        assert!((r.mesh_bounds.width() - g.mesh_bounds.width()).abs() < 1.0);
        assert!(reorder_galley(&layout("Layer", f32::INFINITY)).is_none(), "English is left alone");
    }

    #[test]
    fn wrapped_paragraphs_keep_their_lines_in_reading_order() {
        let g = layout("אבגד הוזח טיכל מנסע", 60.0);
        assert!(g.rows.len() >= 2, "wraps: {} rows", g.rows.len());
        let r = reorder_galley(&g).expect("reordered");
        // The first word is still on the first row, and drawn at its right end.
        let (row_a, x_a) = glyph_x(&r, 'א').expect("alef");
        let (row_d, _) = glyph_x(&r, 'ע').expect("ayin");
        assert_eq!(row_a, 0);
        assert!(row_d > 0);
        let first = r.rows.first().expect("row");
        let right = first.pos.x + first.row.size.x;
        assert!(right >= r.rect.max.x - 1.0, "right-aligned");
        assert!(x_a > first.pos.x + first.row.size.x / 2.0, "alef, the first letter, is on the right half");
    }

    #[test]
    fn the_bundled_font_covers_hebrew_and_a_frame_draws_it_in_visual_order() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.label("אבג דהו");
        });
        out.textures_delta.clear();
        assert!(ctx.fonts_mut(|f| f.has_glyphs(&FontId::proportional(12.0), "שכבה חדשה")));
        // The installed plugin already reordered the label in the frame's output.
        let mut found = false;
        let mut stack: Vec<&Shape> = out.shapes.iter().map(|c| &c.shape).collect();
        while let Some(shape) = stack.pop() {
            match shape {
                Shape::Vec(v) => stack.extend(v.iter()),
                Shape::Text(t) if t.galley.job.text == "אבג דהו" => {
                    let x = |c| glyph_x(&t.galley, c).map(|(_, x)| x).unwrap_or(f32::NAN);
                    assert!(x('א') > x('ד'), "drawn right to left");
                    found = true;
                }
                _ => {}
            }
        }
        assert!(found, "the label was drawn");
    }

    #[test]
    fn the_plugin_rewrites_text_shapes_and_caches_them() {
        let g = layout("שמירה בשם", f32::INFINITY);
        let mut plugin = RtlTextPlugin::default();
        let mut shape = Shape::Vec(vec![Shape::galley(egui::pos2(0.0, 0.0), g.clone(), Color32::WHITE)]);
        plugin.fix(&mut shape);
        let Shape::Vec(v) = &shape else { panic!("vec") };
        let Some(Shape::Text(t)) = v.first() else { panic!("text") };
        assert!(!Arc::ptr_eq(&t.galley, &g));
        let again = plugin.reordered(&g);
        assert!(Arc::ptr_eq(&again, &t.galley), "cached");
    }
}
