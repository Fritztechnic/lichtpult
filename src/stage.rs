//! Bühne: Fixtures frei anordnen und per Rahmen auswählen.

use crate::theme::{self, caption};
use crate::{App, rgb_of};
use eframe::egui::{self, Color32, RichText};
use lichtpult::engine::{Action, Engine};
use lichtpult::fixture::{Attr, Fixture};

impl App {
    pub fn stage(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        ui.horizontal(|ui| {
            caption(ui, "Bühne");
            ui.label(RichText::new("Klick = wählen (Shift = hinzufügen) · ziehen = verschieben · Leerraum ziehen = Rahmen").color(theme::MUTED));
        });
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        let p = ui.painter_at(rect);
        p.rect(rect, 6.0, Color32::from_rgb(8, 8, 10), egui::Stroke::new(1.0, theme::LINE), egui::StrokeKind::Inside);
        let to_screen = |x: f32, y: f32| egui::pos2(rect.left() + x * rect.width(), rect.top() + y * rect.height());
        let pos = |i: usize, f: &Fixture| {
            if f.x == 0.0 && f.y == 0.0 {
                let (c, r) = (i % 10, i / 10);
                (0.1 + 0.8 * c as f32 / 9.0, 0.15 + 0.12 * r as f32)
            } else {
                (f.x, f.y)
            }
        };
        let pointer = resp.interact_pointer_pos();
        let shift = ui.input(|i| i.modifiers.shift || i.modifiers.command);
        let mut hit: Option<u32> = None;
        for (i, f) in e.show.fixtures.iter().enumerate() {
            let (x, y) = pos(i, f);
            let c = to_screen(x, y);
            let sel = e.selection.contains(&f.id);
            let col = rgb_of(|a| e.output_of(f, a));
            let level = if e.show.has(f, Attr::Dimmer) { e.output_of(f, Attr::Dimmer) } else { col.r().max(col.g()).max(col.b()) };
            let base = if [Attr::Red, Attr::Green, Attr::Blue, Attr::White].iter().any(|a| e.show.has(f, *a)) && level > 0 {
                col
            } else {
                Color32::from_rgb(235, 235, 240)
            };
            let lit = base.gamma_multiply((level as f32 / 255.0).max(0.1));
            p.circle(c, 14.0, lit, egui::Stroke::new(if sel { 2.0 } else { 1.0 }, if sel { accent } else { theme::LINE }));
            p.text(c + egui::vec2(0.0, 24.0), egui::Align2::CENTER_CENTER, f.id.to_string(), egui::FontId::proportional(11.0), theme::MUTED);
            if let Some(pp) = pointer {
                if pp.distance(c) <= 16.0 && hit.is_none() && (resp.drag_started() || resp.clicked()) {
                    hit = Some(f.id);
                }
            }
        }
        let band_id = egui::Id::new("stage_drag");
        if resp.drag_started() {
            let target = hit;
            ui.data_mut(|d| d.insert_temp(band_id, (target, pointer)));
            if let Some(id) = target {
                if !e.selection.contains(&id) {
                    acts.push(if shift { Action::Toggle(id) } else { Action::Select(id) });
                }
            }
        }
        let state: Option<(Option<u32>, Option<egui::Pos2>)> = ui.data(|d| d.get_temp(band_id));
        if resp.dragged() {
            if let Some((Some(_), _)) = state {
                let d = resp.drag_delta();
                for (i, f) in e.show.fixtures.iter().enumerate() {
                    if e.selection.contains(&f.id) {
                        let (x, y) = pos(i, f);
                        acts.push(Action::SetPosition(f.id, x + d.x / rect.width(), y + d.y / rect.height()));
                    }
                }
            } else if let (Some((None, Some(a))), Some(b)) = (state, pointer) {
                let r = egui::Rect::from_two_pos(a, b);
                p.rect(r, 0.0, accent.gamma_multiply(0.12), egui::Stroke::new(1.0, accent), egui::StrokeKind::Inside);
            }
        }
        if resp.drag_stopped() {
            if let (Some((None, Some(a))), Some(b)) = (state, pointer) {
                let r = egui::Rect::from_two_pos(a, b);
                let ids = e
                    .show
                    .fixtures
                    .iter()
                    .enumerate()
                    .filter(|(i, f)| {
                        let (x, y) = pos(*i, f);
                        r.contains(to_screen(x, y))
                    })
                    .map(|(_, f)| f.id)
                    .collect();
                acts.push(Action::SelectIds(ids));
            }
        }
        if resp.clicked() {
            match hit {
                Some(id) => acts.push(if shift { Action::Toggle(id) } else { Action::Select(id) }),
                None => acts.push(Action::ClearSelection),
            }
        }
    }
}

