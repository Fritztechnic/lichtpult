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
