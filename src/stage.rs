//! Bühne: Fixtures und Bühnenobjekte frei anordnen, per Rahmen auswählen.

use crate::theme::{self, caption, hint};
use crate::{App, rgb_of};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke};
use lichtpult::engine::{Action, Engine, Prop, PropKind};
use lichtpult::fixture::{Attr, Fixture};

#[derive(Clone, Copy)]
enum Hit {
    Fixture(u32),
    Resize(usize),
    Prop(usize),
    Nothing,
}

#[derive(Clone, Copy)]
enum Drag {
    Fixtures,
    Prop(usize),
    Resize(usize),
    Band(Pos2),
}

fn prop_rect(p: &Prop, stage: Rect) -> Rect {
    let c = egui::pos2(stage.left() + p.x * stage.width(), stage.top() + p.y * stage.height());
    Rect::from_center_size(c, egui::vec2(p.w * stage.width(), p.h * stage.height()))
}

fn draw_prop(painter: &egui::Painter, p: &Prop, r: Rect, selected: bool, accent: Color32) {
    let line = Stroke::new(1.0, if selected { accent } else { theme::MUTED });
    let font = egui::FontId::proportional(12.0);
    match p.kind {
        PropKind::Podest => {
            painter.rect(r, 2.0, theme::RAISED, line, egui::StrokeKind::Inside);
        }
        PropKind::Truss => {
            painter.rect(r, 0.0, theme::PANEL, line, egui::StrokeKind::Inside);
            let n = (r.width() / r.height().max(8.0)).max(1.0) as i32;
            for k in 0..n {
                let (a, b) = (r.left() + r.width() * k as f32 / n as f32, r.left() + r.width() * (k + 1) as f32 / n as f32);
                let (ya, yb) = if k % 2 == 0 { (r.top(), r.bottom()) } else { (r.bottom(), r.top()) };
                painter.line_segment([egui::pos2(a, ya), egui::pos2(b, yb)], line);
            }
        }
        PropKind::Lautsprecher => {
            painter.rect(r, 2.0, theme::PANEL, line, egui::StrokeKind::Inside);
            let m = r.width().min(r.height());
            painter.circle_stroke(r.center() + egui::vec2(0.0, m * 0.18), m * 0.28, line);
            painter.circle_stroke(r.center() - egui::vec2(0.0, m * 0.3), m * 0.1, line);
        }
        PropKind::Pult => {
            painter.rect(r, 2.0, theme::PANEL, line, egui::StrokeKind::Inside);
            let n = ((r.width() / 12.0) as i32).max(2);
            for k in 0..n {
                let x = r.left() + r.width() * (k as f32 + 0.5) / n as f32;
                painter.line_segment([egui::pos2(x, r.top() + 6.0), egui::pos2(x, r.bottom() - 6.0)], Stroke::new(1.0, theme::LINE));
                painter.rect_filled(Rect::from_center_size(egui::pos2(x, r.center().y + (k % 3) as f32 * 3.0 - 3.0), egui::vec2(6.0, 3.0)), 1.0, line.color);
            }
        }
        PropKind::Publikum => {
            painter.rect(r, 2.0, Color32::TRANSPARENT, Stroke::new(1.0, if selected { accent } else { theme::LINE }), egui::StrokeKind::Inside);
            let (cols, rows) = (((r.width() / 22.0) as i32).max(1), ((r.height() / 22.0) as i32).max(1));
            for i in 0..cols {
                for j in 0..rows {
                    let c = egui::pos2(r.left() + r.width() * (i as f32 + 0.5) / cols as f32, r.top() + r.height() * (j as f32 + 0.5) / rows as f32);
                    painter.circle_filled(c, 2.5, theme::LINE);
                }
            }
        }
        PropKind::Text => {
            if selected {
                painter.rect_stroke(r, 2.0, Stroke::new(1.0, accent), egui::StrokeKind::Inside);
            }
        }
    }
    let label = if p.kind == PropKind::Text && p.label.is_empty() { "Text" } else { &p.label };
    if !label.is_empty() {
        let size = if p.kind == PropKind::Text { 16.0 } else { font.size };
        painter.text(r.center(), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(size), if p.kind == PropKind::Text { theme::TEXT } else { theme::MUTED });
    }
}

impl App {
    pub fn stage(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Bühne").heading().strong());
            theme::info(ui, "Bühne", "Ordne Fixtures und Bühnenobjekte so an wie in deinem Raum. Klick = wählen (Shift = hinzufügen), ziehen = verschieben, Leerraum ziehen = Rahmen aufziehen. Ein gewähltes Objekt hat unten rechts einen Griff zum Vergrößern; Entf löscht es.");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::containers::menu::MenuButton::from_button(theme::icon_text("plus", "Objekt hinzufügen", theme::TEXT)).ui(ui, |ui| {
                    for k in PropKind::ALL {
                        if ui.button(k.name()).clicked() {
                            acts.push(Action::AddProp(k));
                            self.prop_sel = Some(e.show.props.len());
                            ui.close();
                        }
                    }
                });
            });
        });
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        let p = ui.painter_at(rect);
        p.rect(rect, 4.0, Color32::from_rgb(14, 14, 14), Stroke::new(1.0, theme::LINE), egui::StrokeKind::Inside);
        let step = rect.width() / 24.0;
        let mut x = rect.left() + step;
        while x < rect.right() {
            let mut y = rect.top() + step;
            while y < rect.bottom() {
                p.circle_filled(egui::pos2(x, y), 1.0, theme::RAISED);
                y += step;
            }
            x += step;
        }
        let to_screen = |x: f32, y: f32| egui::pos2(rect.left() + x * rect.width(), rect.top() + y * rect.height());
        let pos = |i: usize, f: &Fixture| {
            if f.x == 0.0 && f.y == 0.0 {
                let (c, r) = (i % 10, i / 10);
                (0.1 + 0.8 * c as f32 / 9.0, 0.15 + 0.12 * r as f32)
            } else {
                (f.x, f.y)
            }
        };
        let selected_prop = self.prop_sel.filter(|i| *i < e.show.props.len());
        for (i, pr) in e.show.props.iter().enumerate() {
            draw_prop(&p, pr, prop_rect(pr, rect), selected_prop == Some(i), accent);
        }
        if let Some(i) = selected_prop {
            let r = prop_rect(&e.show.props[i], rect);
            p.circle(r.right_bottom(), 5.0, accent, Stroke::new(1.0, theme::BG));
        }
        for (i, f) in e.show.fixtures.iter().enumerate() {
            let (x, y) = pos(i, f);
            let c = to_screen(x, y);
            let sel = e.selection.contains(&f.id);
            let col = rgb_of(|a| e.output_of(f, a));
            let level = if e.show.has(f, Attr::Dimmer) { e.output_of(f, Attr::Dimmer) } else { col.r().max(col.g()).max(col.b()) };
            let base = if [Attr::Red, Attr::Green, Attr::Blue, Attr::White].iter().any(|a| e.show.has(f, *a)) && level > 0 { col } else { Color32::from_rgb(235, 235, 240) };
            let lit = base.gamma_multiply((level as f32 / 255.0).max(0.1));
            p.circle(c, 14.0, lit, Stroke::new(if sel { 2.0 } else { 1.0 }, if sel { accent } else { theme::LINE }));
            p.text(c + egui::vec2(0.0, 24.0), egui::Align2::CENTER_CENTER, f.id.to_string(), egui::FontId::proportional(11.0), theme::MUTED);
        }

        let hit_at = |pp: Pos2| -> Hit {
            for (i, f) in e.show.fixtures.iter().enumerate() {
                let (x, y) = pos(i, f);
                if pp.distance(to_screen(x, y)) <= 16.0 {
                    return Hit::Fixture(f.id);
                }
            }
            if let Some(i) = selected_prop {
                if pp.distance(prop_rect(&e.show.props[i], rect).right_bottom()) <= 10.0 {
                    return Hit::Resize(i);
                }
            }
            for (i, pr) in e.show.props.iter().enumerate().rev() {
                if prop_rect(pr, rect).expand(4.0).contains(pp) {
                    return Hit::Prop(i);
                }
            }
            Hit::Nothing
        };
        let pointer = resp.interact_pointer_pos();
        let shift = ui.input(|i| i.modifiers.shift || i.modifiers.command);
        let select = |id: u32| if shift { Action::Toggle(id) } else { Action::Select(id) };
        let drag_id = egui::Id::new("stage_drag");

        if resp.drag_started() {
            let drag = match pointer.map_or(Hit::Nothing, hit_at) {
                Hit::Fixture(id) => {
                    if !e.selection.contains(&id) {
                        acts.push(select(id));
                    }
                    Some(Drag::Fixtures)
                }
                Hit::Resize(i) => Some(Drag::Resize(i)),
                Hit::Prop(i) => {
                    self.prop_sel = Some(i);
                    Some(Drag::Prop(i))
                }
                Hit::Nothing => pointer.map(Drag::Band),
            };
            ui.data_mut(|d| d.insert_temp(drag_id, drag));
        }
        let drag: Option<Drag> = ui.data(|d| d.get_temp::<Option<Drag>>(drag_id)).flatten();
        if resp.dragged() {
            let d = resp.drag_delta();
            match (drag, pointer) {
                (Some(Drag::Fixtures), _) => {
                    for (i, f) in e.show.fixtures.iter().enumerate() {
                        if e.selection.contains(&f.id) {
                            let (x, y) = pos(i, f);
                            acts.push(Action::SetPosition(f.id, x + d.x / rect.width(), y + d.y / rect.height()));
                        }
                    }
                }
                (Some(Drag::Prop(i)), _) => {
                    if let Some(pr) = e.show.props.get(i) {
                        acts.push(Action::SetProp(i, Prop { x: pr.x + d.x / rect.width(), y: pr.y + d.y / rect.height(), ..pr.clone() }));
                    }
                }
                (Some(Drag::Resize(i)), Some(pp)) => {
                    if let Some(pr) = e.show.props.get(i) {
                        let c = to_screen(pr.x, pr.y);
                        let w = (2.0 * (pp.x - c.x) / rect.width()).max(0.02);
                        let h = (2.0 * (pp.y - c.y) / rect.height()).max(0.02);
                        acts.push(Action::SetProp(i, Prop { w, h, ..pr.clone() }));
                    }
                }
                (Some(Drag::Band(a)), Some(b)) => {
                    p.rect(Rect::from_two_pos(a, b), 0.0, accent.gamma_multiply(0.12), Stroke::new(1.0, accent), egui::StrokeKind::Inside);
                }
                _ => {}
            }
        }
        if resp.drag_stopped() {
            if let (Some(Drag::Band(a)), Some(b)) = (drag, pointer) {
                let r = Rect::from_two_pos(a, b);
                let ids = e.show.fixtures.iter().enumerate().filter(|(i, f)| r.contains(to_screen(pos(*i, f).0, pos(*i, f).1))).map(|(_, f)| f.id).collect();
                acts.push(Action::SelectIds(ids));
            }
            ui.data_mut(|d| d.insert_temp::<Option<Drag>>(drag_id, None));
        }
        if resp.clicked() {
            match pointer.map_or(Hit::Nothing, hit_at) {
                Hit::Fixture(id) => acts.push(select(id)),
                Hit::Prop(i) | Hit::Resize(i) => self.prop_sel = Some(i),
                Hit::Nothing => {
                    acts.push(Action::ClearSelection);
                    self.prop_sel = None;
                }
            }
        }
        if let Some(i) = selected_prop {
            if !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                acts.push(Action::DeleteProp(i));
                self.prop_sel = None;
            }
        }
    }

    /// Rechte Leiste: Auswahl und Bühnenobjekte.
    pub fn stage_inspector(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        theme::section(ui, "Objekte", "Bühnenobjekte sind reine Orientierung (Podest, Traverse, Lautsprecher, Pult, Publikum, Beschriftung). Sie ändern kein Licht. Wähle eines, um Name und Größe zu ändern.");
        ui.add_space(6.0);
        if e.show.props.is_empty() {
            hint(ui, "Noch keine Objekte. Über „Objekt hinzufügen“ oben kannst du welche anlegen.");
        }
        for (i, pr) in e.show.props.iter().enumerate() {
            let on = self.prop_sel == Some(i);
            let name = if pr.label.is_empty() { pr.kind.name().to_string() } else { format!("{} · {}", pr.kind.name(), crate::short(&pr.label, 14)) };
            let b = egui::Button::new(RichText::new(name).color(if on { accent } else { theme::TEXT })).min_size(egui::vec2(ui.available_width(), 30.0)).selected(on);
            if ui.add(b).clicked() {
                self.prop_sel = Some(i);
            }
        }
        let Some(i) = self.prop_sel.filter(|i| *i < e.show.props.len()) else { return };
        let pr = &e.show.props[i];
        ui.add_space(12.0);
        caption(ui, "Eigenschaften");
        ui.add_space(4.0);
        let mut n = pr.clone();
        let mut changed = false;
        changed |= ui.add(egui::TextEdit::singleline(&mut n.label).hint_text("Beschriftung").desired_width(f32::INFINITY)).changed();
        egui::Grid::new("prop_form").num_columns(2).spacing([12.0, 10.0]).show(ui, |ui| {
            for (name, v) in [("Breite", &mut n.w), ("Höhe", &mut n.h)] {
                ui.label(name);
                let mut pct = *v * 100.0;
                if ui.add(egui::DragValue::new(&mut pct).speed(0.2).range(1.0..=100.0).suffix(" %")).changed() {
                    *v = pct / 100.0;
                    changed = true;
                }
                ui.end_row();
            }
        });
        if changed {
            acts.push(Action::SetProp(i, n));
        }
        ui.add_space(8.0);
        if ui.add(theme::icon_text("trash", "Objekt löschen", theme::DANGER).min_size(egui::vec2(ui.available_width(), 34.0))).clicked() {
            acts.push(Action::DeleteProp(i));
            self.prop_sel = None;
        }
    }
}
