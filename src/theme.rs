use eframe::egui::{self, Color32, CornerRadius, Stroke};

pub const BG: Color32 = Color32::from_rgb(12, 14, 18);
pub const PANEL: Color32 = Color32::from_rgb(19, 22, 29);
pub const RAISED: Color32 = Color32::from_rgb(28, 32, 42);
pub const LINE: Color32 = Color32::from_rgb(43, 49, 64);
pub const TEXT: Color32 = Color32::from_rgb(230, 233, 240);
pub const MUTED: Color32 = Color32::from_rgb(134, 142, 160);
pub const DANGER: Color32 = Color32::from_rgb(240, 84, 84);
pub const OK: Color32 = Color32::from_rgb(70, 200, 130);

pub const PRESETS: [[u8; 3]; 6] = [
    [0, 170, 255],
    [60, 220, 140],
    [255, 160, 30],
    [255, 70, 110],
    [170, 110, 255],
    [235, 235, 240],
];

pub fn apply(ctx: &egui::Context, accent: Color32) {
    let mut v = egui::Visuals::dark();
    let r = CornerRadius::same(6);
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = Color32::from_rgb(9, 11, 15);
    v.faint_bg_color = RAISED;
    v.window_stroke = Stroke::new(1.0, LINE);
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = accent.gamma_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0, accent);
    v.hyperlink_color = accent;
    v.slider_trailing_fill = true;
    v.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.5 };
    for (w, fill) in [
        (&mut v.widgets.noninteractive, PANEL),
        (&mut v.widgets.inactive, RAISED),
        (&mut v.widgets.hovered, Color32::from_rgb(38, 44, 58)),
        (&mut v.widgets.active, accent.gamma_multiply(0.5)),
        (&mut v.widgets.open, RAISED),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.corner_radius = r;
        w.bg_stroke = Stroke::new(1.0, LINE);
    }
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, accent.gamma_multiply(0.7));
    ctx.set_visuals(v);

    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 8.0);
        s.spacing.button_padding = egui::vec2(12.0, 5.0);
        s.spacing.interact_size.y = 30.0;
        s.spacing.slider_width = 320.0;
        s.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
        s.text_styles.insert(egui::TextStyle::Monospace, egui::FontId::monospace(13.0));
        s.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(18.0));
    });
}

/// Auswahl-Chip (Gruppen, Paletten, Listen, Umschalter).
pub fn tile(ui: &mut egui::Ui, text: impl Into<String>, on: bool, accent: Color32) -> egui::Response {
    let text: String = text.into();
    let mut b = egui::Button::new(egui::RichText::new(text).color(if on { accent } else { TEXT })).min_size(egui::vec2(0.0, 30.0));
    if on {
        b = b.fill(accent.gamma_multiply(0.16)).stroke(Stroke::new(1.0, accent));
    }
    ui.add(b)
}

/// Segmentierte Navigation (aktiv = Akzent-Unterstrich).
pub fn nav(ui: &mut egui::Ui, text: &str, on: bool, accent: Color32) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(14.5), if on { TEXT } else { MUTED });
    let (rect, r) = ui.allocate_exact_size(egui::vec2(galley.size().x + 28.0, 40.0), egui::Sense::click());
    let col = if on { TEXT } else if r.hovered() { TEXT.gamma_multiply(0.85) } else { MUTED };
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, col);
    if on {
        let y = rect.bottom() - 2.0;
        ui.painter().line_segment([egui::pos2(rect.left() + 8.0, y), egui::pos2(rect.right() - 8.0, y)], Stroke::new(2.0, accent));
    }
    r
}

/// Kleine Statusanzeige mit Punkt.
pub fn pill(ui: &mut egui::Ui, text: &str, color: Color32) {
    ui.label(egui::RichText::new(format!("● {text}")).size(12.5).color(color));
}

/// Dezente Abschnittsüberschrift.
pub fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text.to_uppercase()).size(11.5).color(MUTED).strong());
}

/// Abschnittsüberschrift mit Erklärung im Tooltip (ⓘ).
pub fn section(ui: &mut egui::Ui, text: &str, help: &str) {
    ui.horizontal(|ui| {
        caption(ui, text);
        ui.label(egui::RichText::new("ⓘ").size(12.5).color(MUTED)).on_hover_text(help);
    });
}

/// Erklärtext in Grau.
pub fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(12.5).color(MUTED));
}

pub fn attr_tint(a: lichtpult::fixture::Attr, accent: Color32) -> Color32 {
    use lichtpult::fixture::Attr::*;
    match a {
        Dimmer | L1 | L2 | L3 | L4 | L5 | L6 | L7 | L8 => Color32::from_rgb(235, 235, 240),
        Red => Color32::from_rgb(255, 70, 70),
        Green => Color32::from_rgb(60, 220, 110),
        Blue => Color32::from_rgb(70, 120, 255),
        White => Color32::from_rgb(245, 245, 245),
        Amber => Color32::from_rgb(255, 170, 40),
        Uv => Color32::from_rgb(150, 80, 255),
        _ => accent,
    }
}

pub enum Fader {
    None,
    Set(u8),
    Reset,
}

/// Horizontaler Regler mit Beschriftung und Prozentwert.
/// `set` = Wert kommt aus dem Programmer (hell), sonst aus der Wiedergabe (gedimmt).
/// Doppelklick gibt den Programmer-Wert frei.
pub fn fader(ui: &mut egui::Ui, label: &str, value: u8, tint: Color32, set: bool) -> Fader {
    let mut out = Fader::None;
    let (rect, r) = ui.allocate_exact_size(egui::vec2(ui.available_width().max(220.0), 26.0), egui::Sense::click_and_drag());
    let p = ui.painter();
    let track = egui::Rect::from_min_max(egui::pos2(rect.left() + 72.0, rect.center().y - 4.0), egui::pos2(rect.right() - 46.0, rect.center().y + 4.0));
    p.text(egui::pos2(rect.left(), rect.center().y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(13.0), if set { TEXT } else { MUTED });
    p.rect_filled(track, 4.0, Color32::from_rgb(10, 12, 16));
    let k = value as f32 / 255.0;
    let mut fill = track;
    fill.set_right(track.left() + track.width() * k);
    p.rect_filled(fill, 4.0, if set { tint } else { tint.gamma_multiply(0.35) });
    let x = track.left() + track.width() * k;
    p.circle(egui::pos2(x, rect.center().y), 7.0, if set { Color32::WHITE } else { Color32::from_rgb(150, 156, 170) }, egui::Stroke::NONE);
    p.text(
        egui::pos2(rect.right(), rect.center().y),
        egui::Align2::RIGHT_CENTER,
        format!("{}%", (value as u32 * 100 + 127) / 255),
        egui::FontId::monospace(12.0),
        if set { TEXT } else { MUTED },
    );
    if r.double_clicked() {
        out = Fader::Reset;
    } else if r.is_pointer_button_down_on() {
        if let Some(pos) = r.interact_pointer_pos() {
            out = Fader::Set((((pos.x - track.left()) / track.width()).clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    out
}

/// Karte mit Titel für zusammengehörige Bedienelemente.
pub fn card<R>(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(10)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !title.is_empty() {
                caption(ui, title);
                ui.add_space(4.0);
            }
            add(ui)
        })
        .inner
}
