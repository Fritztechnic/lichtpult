use eframe::egui::{self, Color32, CornerRadius, Rect, RichText, Stroke};

pub const BG: Color32 = Color32::from_rgb(22, 22, 22);
pub const PANEL: Color32 = Color32::from_rgb(33, 33, 33);
pub const RAISED: Color32 = Color32::from_rgb(48, 48, 48);
pub const LINE: Color32 = Color32::from_rgb(66, 66, 66);
pub const TEXT: Color32 = Color32::from_rgb(244, 244, 244);
pub const MUTED: Color32 = Color32::from_rgb(168, 168, 168);
pub const DANGER: Color32 = Color32::from_rgb(250, 77, 86);
pub const OK: Color32 = Color32::from_rgb(66, 190, 101);

pub const HOVER: Color32 = Color32::from_rgb(64, 64, 64);

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
    let r = CornerRadius::same(4);
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = Color32::from_rgb(14, 14, 14);
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
        (&mut v.widgets.hovered, HOVER),
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
        s.interaction.tooltip_delay = 0.15;
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
        info(ui, text, help);
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

/// Karte mit Titel (und optionalem Info-Text) für zusammengehörige Bedienelemente.
pub fn card<R>(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    card_info(ui, title, "", add)
}

pub fn card_info<R>(ui: &mut egui::Ui, title: &str, help: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(4)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !title.is_empty() {
                ui.horizontal(|ui| {
                    caption(ui, title);
                    if !help.is_empty() {
                        info(ui, title, help);
                    }
                });
                ui.add_space(4.0);
            }
            add(ui)
        })
        .inner
}

/// Lucide-Icons (ISC) als SVG, weiß gefärbt und per Tint eingefärbt.
fn icon_source(name: &str) -> egui::ImageSource<'static> {
    let bytes: &'static [u8] = match name {
        "undo" => include_bytes!("../assets/icons/undo-2.svg"),
        "redo" => include_bytes!("../assets/icons/redo-2.svg"),
        "settings" => include_bytes!("../assets/icons/settings.svg"),
        "x" => include_bytes!("../assets/icons/x.svg"),
        "play" => include_bytes!("../assets/icons/play.svg"),
        "stop" => include_bytes!("../assets/icons/square.svg"),
        "back" => include_bytes!("../assets/icons/skip-back.svg"),
        "plus" => include_bytes!("../assets/icons/plus.svg"),
        "info" => include_bytes!("../assets/icons/info.svg"),
        "trash" => include_bytes!("../assets/icons/trash.svg"),
        "reset" => include_bytes!("../assets/icons/rotate-ccw.svg"),
        "list" => include_bytes!("../assets/icons/list.svg"),
        "terminal" => include_bytes!("../assets/icons/terminal.svg"),
        "copy" => include_bytes!("../assets/icons/copy.svg"),
        "up" => include_bytes!("../assets/icons/arrow-up.svg"),
        "down" => include_bytes!("../assets/icons/arrow-down.svg"),
        "search" => include_bytes!("../assets/icons/search.svg"),
        "grip" => include_bytes!("../assets/icons/grip-vertical.svg"),
        "zoom-in" => include_bytes!("../assets/icons/zoom-in.svg"),
        "zoom-out" => include_bytes!("../assets/icons/zoom-out.svg"),
        "fit" => include_bytes!("../assets/icons/maximize.svg"),
        "pointer" => include_bytes!("../assets/icons/mouse-pointer-2.svg"),
        "hand" => include_bytes!("../assets/icons/hand.svg"),
        "dots" => include_bytes!("../assets/icons/grid-3x3.svg"),
        "snap" => include_bytes!("../assets/icons/magnet.svg"),
        "next" => include_bytes!("../assets/icons/skip-forward.svg"),
        "dist-h" => include_bytes!("../assets/icons/align-horizontal-distribute-center.svg"),
        "dist-v" => include_bytes!("../assets/icons/align-vertical-distribute-center.svg"),
        "rotate" => include_bytes!("../assets/icons/rotate-cw.svg"),
        "grid" => include_bytes!("../assets/icons/layout-grid.svg"),
        _ => include_bytes!("../assets/icons/layers.svg"),
    };
    egui::ImageSource::Bytes { uri: format!("bytes://{name}.svg").into(), bytes: bytes.into() }
}

pub fn icon(name: &str, color: Color32, size: f32) -> egui::Image<'static> {
    egui::Image::new(icon_source(name)).tint(color).fit_to_exact_size(egui::vec2(size, size))
}

/// Quadratischer Icon-Knopf (32 px).
pub fn icon_btn(ui: &mut egui::Ui, name: &str, tip: &str) -> egui::Response {
    ui.add(egui::Button::image(icon(name, TEXT, 16.0)).min_size(egui::vec2(32.0, 32.0))).on_hover_text(tip)
}

/// Knopf mit Icon und Text.
pub fn icon_text(name: &str, text: &str, color: Color32) -> egui::Button<'static> {
    egui::Button::image_and_text(icon(name, color, 16.0), egui::RichText::new(text.to_string()).color(color))
}

/// Breiter senkrechter Fader-Block: Wert oben, Füllung von unten in der Attributfarbe, Name darunter.
/// `set` = Wert kommt aus dem Programmer (hell), sonst aus der Wiedergabe (gedimmt).
/// Doppelklick oder Rechtsklick gibt den Programmer-Wert frei.
pub fn fader(ui: &mut egui::Ui, label: &str, value: u8, tint: Color32, set: bool) -> Fader {
    let mut out = Fader::None;
    ui.vertical(|ui| {
        ui.set_width(FADER_W);
        ui.spacing_mut().item_spacing.y = 6.0;
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(format!("{}%", (value as u32 * 100 + 127) / 255)).monospace().color(if set { TEXT } else { MUTED }));
        });
        let (rect, r) = ui.allocate_exact_size(egui::vec2(FADER_W, 200.0), egui::Sense::click_and_drag());
        let p = ui.painter();
        p.rect_filled(rect, 4.0, Color32::from_rgb(14, 14, 14));
        let h = rect.height() * value as f32 / 255.0;
        let fill = Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - h), rect.right_bottom());
        p.rect_filled(fill, 4.0, if set { tint } else { tint.gamma_multiply(0.35) });
        let border = if r.hovered() || r.dragged() { tint } else { LINE };
        p.rect_stroke(rect, 4.0, Stroke::new(1.0, border), egui::StrokeKind::Inside);
        if r.double_clicked() || r.secondary_clicked() {
            out = Fader::Reset;
        } else if r.is_pointer_button_down_on() {
            if let Some(pos) = r.interact_pointer_pos() {
                out = Fader::Set((((rect.bottom() - pos.y) / rect.height()).clamp(0.0, 1.0) * 255.0).round() as u8);
            }
        }
        r.on_hover_text("Ziehen = einstellen · Doppelklick oder Rechtsklick = freigeben");
        ui.vertical_centered(|ui| ui.label(RichText::new(label).size(12.5).color(MUTED)));
    });
    out
}

pub const FADER_W: f32 = 68.0;

/// Info-Symbol: erklärt beim Darüberfahren, wofür der Bereich da ist.
pub fn info(ui: &mut egui::Ui, title: &str, text: &str) {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
    icon("info", if r.hovered() { TEXT } else { MUTED }, 15.0).paint_at(ui, Rect::from_center_size(rect.center(), egui::vec2(15.0, 15.0)));
    r.on_hover_ui(|ui| {
        ui.set_max_width(320.0);
        ui.label(RichText::new(title).strong());
        ui.add_space(2.0);
        ui.label(RichText::new(text).color(MUTED));
    });
}

/// Runder Knopf mit Icon (Player-Leiste).
pub fn round_btn(ui: &mut egui::Ui, name: &str, diameter: f32, fill: Color32, icon_color: Color32, tip: &str) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(diameter, diameter), egui::Sense::click());
    let fill = if r.hovered() { fill.gamma_multiply(1.15) } else { fill };
    ui.painter().circle_filled(rect.center(), diameter / 2.0, fill);
    icon(name, icon_color, diameter * 0.42).paint_at(ui, Rect::from_center_size(rect.center(), egui::vec2(diameter * 0.42, diameter * 0.42)));
    r.on_hover_text(tip)
}

/// Normaler Knopf mit Mindestgröße, damit nichts gequetscht wirkt.
pub fn button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(egui::Button::new(text).min_size(egui::vec2(64.0, 30.0)))
}
