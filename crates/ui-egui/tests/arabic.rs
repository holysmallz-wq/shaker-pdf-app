//! SHAKER PDF: editing Arabic text in the real shell (kittest), with optional screenshots.
//!
//! `SHAKER_SHOTS=dir cargo test -p pdfcraft-ui-egui --test arabic` also writes PNGs of each step.
//! The fixture is a LibreOffice export (Tajawal, right to left, numbers and English mixed in).

use egui_kittest::Harness;
use pdfcraft_ui_egui::PdfCraftApp;

const QUOTE: &[u8] = include_bytes!("data/arabic-quote.pdf");

fn open(lang: &'static str) -> Harness<'static, PdfCraftApp> {
    Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(move |_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", lang).unwrap();
        app.open_bytes("quote.pdf", None, QUOTE.to_vec()).expect("the quote opens");
        app
    })
}

fn shot(h: &mut Harness<'static, PdfCraftApp>, name: &str) {
    if let Ok(dir) = std::env::var("SHAKER_SHOTS")
        && let Ok(image) = h.render()
    {
        let _ = image.save(std::path::Path::new(&dir).join(format!("{name}.png")));
    }
}

/// A point on page 1 (A4, 595 × 842 pt) from the top-left, in screen points.
fn at(h: &Harness<'static, PdfCraftApp>, x: f32, y_from_top: f32) -> egui::Pos2 {
    let page = h.state().views[0].page_screen_rect(0).expect("page 1 on screen");
    egui::pos2(page.left() + x / 595.0 * page.width(), page.top() + y_from_top / 842.0 * page.height())
}

/// Run frames until background page renders have arrived (at most a few seconds).
fn settle(h: &mut Harness<'static, PdfCraftApp>) {
    let start = std::time::Instant::now();
    while h.state().render_pending() && start.elapsed() < std::time::Duration::from_secs(10) {
        h.run_steps(2);
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    h.run_steps(3);
}

fn click(h: &mut Harness<'static, PdfCraftApp>, p: egui::Pos2) {
    h.hover_at(p);
    h.run_steps(1);
    h.drag_at(p);
    h.run_steps(1);
    h.drop_at(p);
    h.run_steps(4);
}

#[test]
fn arabic_paragraph_edits_in_place() {
    let mut h = open("ar");
    h.run_steps(8);
    settle(&mut h);
    shot(&mut h, "0-open");
    assert!(h.state_mut().execute("edit.edit_text"));
    h.run_steps(3);
    let p = at(&h, 200.0, 88.0);
    click(&mut h, p);
    let ed = h.state().views[0].line_editor.clone().expect("the Arabic paragraph opens for editing");
    assert!(ed.text.contains("للمقاولات"), "reads in typing order with lam-alef intact: {:?}", ed.text);
    assert!(ed.text.contains("60×60 سم بسعر 185 ريال"), "numbers stay inside the sentence: {:?}", ed.text);
    shot(&mut h, "1-editor");

    let edited = ed.text.replace("185", "210").replace("النخبة", "الرواد");
    h.state_mut().views[0].line_editor.as_mut().unwrap().text = edited;
    h.run_steps(3);
    shot(&mut h, "2-typed");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    h.run_steps(8);
    settle(&mut h);
    shot(&mut h, "3-applied");

    let s = h.state();
    let doc = s.session.get(s.views[0].id).unwrap();
    let text: String = doc.text_lines(0).iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    assert!(text.contains("210 ريال"), "{text}");
    assert!(text.contains("الرواد للمقاولات"), "{text}");
    assert!(!text.contains("185"), "{text}");
}

#[test]
fn interface_is_mirrored_in_arabic() {
    let mut h = open("ar");
    h.run_steps(8);
    assert!(h.state().docking.tools_on_right(), "Arabic puts Tools on the right");
    h.state_mut().set_option("language", "en").unwrap();
    h.run_steps(4);
    assert!(!h.state().docking.tools_on_right(), "English puts Tools back on the left");
}

#[test]
fn typing_arabic_in_a_new_text_box() {
    let mut h = open("ar");
    h.run_steps(8);
    settle(&mut h);
    h.state_mut().text_style.face = Some("Cairo".into());
    h.state_mut().text_style.size = 18.0;
    assert!(h.state_mut().execute("edit.text"));
    h.run_steps(3);
    let p = at(&h, 330.0, 300.0);
    click(&mut h, p);
    h.event(egui::Event::Text("شركة آسيا ستون — عرض رقم 2026/15".into()));
    h.run_steps(4);
    shot(&mut h, "4-add-typing");
    let s = h.state();
    let draft = s.views[0].content.draft.as_ref().expect("a new text box is open");
    assert_eq!(draft.text, "شركة آسيا ستون — عرض رقم 2026/15");
    assert_eq!(draft.style.face.as_deref(), Some("Cairo"));
}
