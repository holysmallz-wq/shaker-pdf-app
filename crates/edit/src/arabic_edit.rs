//! Editing existing Arabic text (SHAKER PDF).
//!
//! Arabic letters change shape with their neighbours, so the per-character Type 3 replacement
//! that Edit text uses for Hebrew and Japanese can't draw them. Instead, an edited paragraph that
//! holds Arabic is taken off the page (its glyphs are removed the same way the editor preview
//! removes them, so the rest of the page keeps its positions) and drawn again as an added text
//! item in the same place, size and colour. Added text shapes Arabic with HarfBuzz rules
//! (joining forms, lam-alef, marks), lays out mixed Arabic/Latin/digits with the Unicode
//! bidirectional algorithm, and keeps a ToUnicode map, so the result stays searchable and can be
//! edited again (as an added item).
//!
//! Reading: producers often write Arabic through presentation forms (U+FB50–U+FDFF,
//! U+FE70–U+FEFF); [`normalize_presentation_forms`] turns them back into the letters a person
//! types, so the editor shows plain Arabic text.

use pdfcraft_cos::Document;
use unicode_normalization::UnicodeNormalization;

use crate::EditError;
use crate::added::{AddedText, Align, Content};
use crate::text::{BlockStyle, LineEdit, TextBlock};

/// Arabic letters, marks, digits, punctuation and presentation forms.
pub(crate) fn is_arabic(c: char) -> bool {
    matches!(u32::from(c), 0x0600..=0x06FF | 0x0750..=0x077F | 0x0870..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFC)
}

fn is_presentation_form(c: char) -> bool {
    matches!(u32::from(c), 0xFB50..=0xFDFF | 0xFE70..=0xFEFC)
}

/// Whether a paragraph should take the Arabic path.
pub(crate) fn has_arabic(text: &str) -> bool {
    text.chars().any(is_arabic)
}

/// Presentation forms back to the letters they show (compatibility decomposition, only for
/// those blocks: other characters are left exactly as they were). Lines without them are
/// returned unchanged.
pub(crate) fn normalize_presentation_forms(text: &str) -> String {
    if !text.chars().any(is_presentation_form) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if is_presentation_form(c) {
            out.extend(c.to_string().nfkc());
        } else {
            out.push(c);
        }
    }
    out
}

/// The default face for a rewritten Arabic paragraph: a serif-looking source keeps a Naskh face,
/// everything else a clean sans. `None` when this build has neither.
fn default_face(base_font: &str) -> Option<String> {
    let lower = base_font.to_ascii_lowercase();
    // The source's own family when this build embeds it ("BAAAAA+Tajawal-Bold" → Tajawal).
    let squashed: String = lower.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if let Some(same) = pdfcraft_fonts::pickable_families().into_iter().find(|f| {
        let f: String = f.to_ascii_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        squashed.contains(&f)
    }) {
        return Some(same.to_string());
    }
    let serif =
        ["naskh", "amiri", "times", "serif", "traditional", "lotus", "badr", "scheherazade", "mitra", "nazanin"].iter().any(|k| lower.contains(k));
    let kufi = lower.contains("kufi");
    let wanted: &[&str] = if serif {
        &["Noto Naskh Arabic", "Amiri", "Scheherazade New"]
    } else if kufi {
        &["Noto Kufi Arabic", "Cairo", "IBM Plex Sans Arabic"]
    } else {
        &["IBM Plex Sans Arabic", "Tajawal", "Cairo", "Noto Kufi Arabic"]
    };
    let have = pdfcraft_fonts::pickable_families();
    wanted.iter().find(|w| have.contains(w)).map(|w| (*w).to_string())
}

/// A rectangle in user space as the page displays it (`view` maps display space to user space).
fn to_display(view: [f64; 6], r: [f64; 4]) -> Option<[f64; 4]> {
    let [a, b, c, d, e, f] = view;
    let det = a * d - b * c;
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = |x: f64, y: f64| {
        let (x, y) = (x - e, y - f);
        ((d * x - c * y) / det, (-b * x + a * y) / det)
    };
    let pts = [inv(r[0], r[1]), inv(r[2], r[1]), inv(r[0], r[3]), inv(r[2], r[3])];
    let x0 = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let y0 = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let x1 = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let y1 = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    [x0, y0, x1, y1].iter().all(|v| v.is_finite()).then_some([x0, y0, x1, y1])
}

/// Whether more than half of `a` lies inside `b` (both `[x0, y0, x1, y1]`).
fn mostly_inside(a: [f64; 4], b: [f64; 4]) -> bool {
    let w = (a[2].min(b[2]) - a[0].max(b[0])).max(0.0);
    let h = (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
    let area = (a[2] - a[0]).max(0.0) * (a[3] - a[1]).max(0.0);
    area > 0.0 && w * h > area * 0.5
}

/// The added text that replaces paragraph `b` with `text` and `style`, in display space.
pub(crate) fn replacement(doc: &Document, page: usize, b: &TextBlock, text: &str, style: &BlockStyle, room: bool) -> Result<AddedText, EditError> {
    if b.rotation.abs() > 0.5 && (b.rotation - 360.0).abs() > 0.5 {
        return Err(EditError::Invalid("editing turned Arabic text isn't supported yet".into()));
    }
    let face = style.face.clone().or_else(|| default_face(&b.base_font));
    if face.is_none() && pdfcraft_fonts::document_arabic_font().is_none() {
        return Err(EditError::Invalid("this build has no Arabic font to draw the text with".into()));
    }
    let p = pdfcraft_model::pages(doc).into_iter().nth(page).ok_or_else(|| EditError::Invalid(format!("no page {}", page + 1)))?;
    let [dx, dy] = style.offset.unwrap_or([0.0, 0.0]);
    if !(dx.is_finite() && dy.is_finite()) {
        return Err(EditError::Invalid("the paragraph can't be moved that far".into()));
    }
    let mut user = [b.rect[0] + dx, b.rect[1] + dy, b.rect[2] + dx, b.rect[3] + dy];
    if let Some(w) = style.width.filter(|w| w.is_finite() && *w > 0.0) {
        // Arabic grows to the left: keep the right edge.
        user[0] = user[2] - w.min(14_400.0);
    }
    let rect = to_display(p.view_matrix(doc), user).ok_or_else(|| EditError::Invalid("the page's geometry is invalid".into()))?;
    let size = style.size.unwrap_or(b.size).clamp(1.0, 1_000.0);
    let rtl = crate::bidi::rtl_base(text);
    // A little slack, so a paragraph that fitted before still fits with a slightly wider face.
    // A single row (`room`) may grow up to the page margin on the side it grows towards
    // (left for right-to-left text), so a longer line stays one line.
    let slack = if style.width.is_some() { 0.0 } else { size * 0.6 };
    let (page_w, _) = p.display_size(doc);
    let margin = 18.0_f64.min(page_w / 10.0);
    let rect = match (rtl, room && style.width.is_none()) {
        (true, true) => [margin.min(rect[0] - slack), rect[1], rect[2], rect[3]],
        (false, true) => [rect[0], rect[1], (page_w - margin).max(rect[2] + slack), rect[3]],
        (true, false) => [rect[0] - slack, rect[1], rect[2], rect[3]],
        (false, false) => [rect[0], rect[1], rect[2] + slack, rect[3]],
    };
    let bold = style.bold.or(style.family.map(|f| f.1)).unwrap_or(b.bold);
    Ok(AddedText {
        rect,
        text: text.to_string(),
        family: style.family.map_or(crate::added::Family::Helvetica, |f| f.0),
        bold,
        italic: style.family.map_or(b.italic, |f| f.2),
        size,
        color: style.color.unwrap_or(b.color),
        align: style.align.unwrap_or(if rtl { Align::Right } else { Align::Left }),
        face,
    })
}

/// Rewrite Arabic paragraph `block` (an index into `text_blocks`) of `page`.
///
/// When the draft keeps the paragraph's rows (one line per row, as the editor shows them), each
/// row is redrawn on its own with its own size, weight and colour, anchored where it was (a
/// bold title stays a bold title). Otherwise the paragraph is redrawn as one wrapped block.
pub(crate) fn rewrite(
    doc: &mut Document,
    page: usize,
    block: usize,
    b: &TextBlock,
    text: Option<&str>,
    style: &BlockStyle,
) -> Result<LineEdit, EditError> {
    let text = text.unwrap_or(&b.editor_text).trim_end().to_string();
    let draft_rows: Vec<&str> = text.split('\n').collect();
    let mut items = Vec::new();
    let per_row = b.rows.len() > 1 && draft_rows.len() == b.rows.len() && style.offset.is_none() && style.width.is_none();
    if per_row {
        let lines = crate::text::text_lines(doc, page)?;
        for (row_text, row) in draft_rows.iter().zip(&b.rows) {
            let row_text = row_text.trim();
            if row_text.is_empty() {
                continue;
            }
            let members: Vec<_> = row.iter().filter_map(|i| lines.get(*i)).collect();
            let Some(first) = members.first() else { continue };
            let rect = members.iter().map(|l| l.rect).fold(first.rect, |a, r| [a[0].min(r[0]), a[1].min(r[1]), a[2].max(r[2]), a[3].max(r[3])]);
            let row_block = TextBlock {
                text: row_text.to_string(),
                rect,
                base_font: first.base_font.clone(),
                size: members.iter().map(|l| l.size).fold(first.size, f64::max),
                bold: members.iter().any(|l| l.bold),
                italic: first.italic,
                color: first.color,
                rotation: b.rotation,
                lines: row.clone(),
                rows: vec![row.clone()],
                editor_text: row_text.to_string(),
                spans: Vec::new(),
            };
            items.push(replacement(doc, page, &row_block, row_text, style, true)?);
        }
    } else if !text.trim().is_empty() {
        items.push(replacement(doc, page, b, &text, style, b.rows.len() <= 1)?);
    }
    // Work on a copy: a failure leaves the document as it was.
    let mut draft = doc.clone();
    crate::text::clear_block(&mut draft, page, block)?;
    // Text this paragraph was drawn with by an earlier edit (added items under it) goes too, so
    // editing the same paragraph again leaves one copy, not a blanked one that could reappear.
    if let Some(p) = pdfcraft_model::pages(&draft).into_iter().nth(page)
        && let Some(target) = to_display(p.view_matrix(&draft), b.rect)
    {
        let stale: Vec<usize> = crate::added::list_added(&draft)
            .into_iter()
            .filter(|a| a.page == page)
            .enumerate()
            .filter_map(|(i, a)| match &a.content {
                Content::Text(t) => mostly_inside(crate::added::text_rect(t), target).then_some(i),
                Content::Image(_) => None,
            })
            .collect();
        for i in stale.into_iter().rev() {
            crate::added::delete_content(&mut draft, page, i)?;
        }
    }
    for item in &items {
        crate::added::add_content(&mut draft, page, &Content::Text(item.clone()))?;
    }
    *doc = draft;
    Ok(LineEdit { substituted: items.into_iter().find_map(|i| i.face) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_forms_become_letters() {
        // ﺳﻼﻡ (seen, lam-alef, meem in presentation forms) reads as سلام.
        assert_eq!(normalize_presentation_forms("\u{FEB3}\u{FEFC}\u{FEE1}"), "سلام");
        assert_eq!(normalize_presentation_forms("abc 123"), "abc 123");
        assert_eq!(normalize_presentation_forms("سلام"), "سلام");
    }

    #[test]
    fn display_rect_round_trips_unrotated() {
        assert_eq!(to_display([1.0, 0.0, 0.0, 1.0, 0.0, 0.0], [10.0, 20.0, 30.0, 40.0]), Some([10.0, 20.0, 30.0, 40.0]));
        let r = to_display([0.0, 1.0, -1.0, 0.0, 600.0, 0.0], [10.0, 20.0, 30.0, 40.0]).unwrap();
        assert!(r.iter().all(|v| v.is_finite()));
        assert_eq!(to_display([0.0; 6], [0.0; 4]), None);
    }
}
