use eframe::egui::{self, Color32, CornerRadius, Stroke};

pub const BG: Color32 = Color32::from_rgb(0, 0, 0);
pub const PANEL: Color32 = Color32::from_rgb(10, 10, 12);
pub const RAISED: Color32 = Color32::from_rgb(20, 20, 23);
pub const LINE: Color32 = Color32::from_rgb(38, 38, 43);
pub const TEXT: Color32 = Color32::from_rgb(228, 228, 232);
pub const MUTED: Color32 = Color32::from_rgb(120, 120, 128);
pub const DANGER: Color32 = Color32::from_rgb(230, 70, 70);

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
    let r = CornerRadius::same(5);
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = BG;
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
        (&mut v.widgets.inactive, Color32::from_rgb(30, 30, 35)),
        (&mut v.widgets.hovered, Color32::from_rgb(32, 32, 37)),
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
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
        s.spacing.slider_width = 320.0;
        s.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(18.0));
    });
}

/// Auswahl-Kachel (Fixtures, Gruppen, Paletten, Cues).
pub fn tile(ui: &mut egui::Ui, text: impl Into<String>, on: bool, accent: Color32) -> egui::Response {
    let text: String = text.into();
    let mut b = egui::Button::new(egui::RichText::new(text).color(if on { accent } else { TEXT }))
        .min_size(egui::vec2(104.0, 44.0));
    if on {
        b = b.fill(accent.gamma_multiply(0.18)).stroke(Stroke::new(1.5, accent));
    }
    ui.add(b)
}

/// Dezente Abschnittsüberschrift.
pub fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text.to_uppercase()).small().color(MUTED).strong());
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

/// Vertikaler Fader. `set` = Wert kommt aus dem Programmer (hell), sonst aus dem Cue (gedimmt).
/// Doppelklick gibt den Programmer-Wert frei.
pub fn fader(ui: &mut egui::Ui, label: &str, value: u8, tint: Color32, set: bool) -> Fader {
    let mut out = Fader::None;
    ui.vertical(|ui| {
        ui.set_width(58.0);
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new(format!("{}%", (value as u32 * 100 + 127) / 255)).monospace().color(if set { TEXT } else { MUTED }));
        });
        let (rect, r) = ui.allocate_exact_size(egui::vec2(58.0, 180.0), egui::Sense::click_and_drag());
        let p = ui.painter();
        p.rect_filled(rect, 6.0, Color32::from_rgb(22, 22, 26));
        let h = rect.height() * value as f32 / 255.0;
        let fill = egui::Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - h), rect.right_bottom());
        p.rect_filled(fill, 6.0, if set { tint } else { tint.gamma_multiply(0.3) });
        let border = if r.hovered() || r.dragged() { tint } else { LINE };
        p.rect_stroke(rect, 6.0, Stroke::new(1.0, border), egui::StrokeKind::Inside);
        if r.double_clicked() {
            out = Fader::Reset;
        } else if r.is_pointer_button_down_on() {
            if let Some(pos) = r.interact_pointer_pos() {
                let k = ((rect.bottom() - pos.y) / rect.height()).clamp(0.0, 1.0);
                out = Fader::Set((k * 255.0).round() as u8);
            }
        }
        ui.vertical_centered(|ui| ui.label(egui::RichText::new(label).small().color(MUTED)));
    });
    out
}

/// Karte mit Titel für zusammengehörige Bedienelemente.
pub fn card<R>(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(Color32::from_rgb(14, 14, 17))
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(8)
        .inner_margin(10)
        .show(ui, |ui| {
            caption(ui, title);
            add(ui)
        })
        .inner
}
