//! Szenen: Cuelisten als Raster aus abrufbaren Szenen-Pads.

use crate::theme::{self, hint, tile};
use crate::{App, play, short};
use eframe::egui::{self, Color32, RichText};
use lichtpult::engine::{Action, Cue, Engine};
use lichtpult::fixture::Attr;

/// Farbstreifen der Fixtures, die der Cue anfasst.
fn preview(e: &Engine, cue: &Cue) -> Vec<Color32> {
    cue.values
        .iter()
        .filter_map(|(id, vals)| {
            let f = e.show.fixtures.iter().find(|f| f.id == *id)?;
            let g = |a: Attr| vals.get(&a).copied().unwrap_or(0);
            let color = crate::rgb_of(g);
            let mx = color.r().max(color.g()).max(color.b());
            let level = vals.get(&Attr::Dimmer).copied().unwrap_or(mx.max(if mx == 0 { 0 } else { 255 }));
            let has_color = [Attr::Red, Attr::Green, Attr::Blue, Attr::White].iter().any(|a| e.show.has(f, *a));
            let base = if has_color && mx > 0 { color } else { Color32::from_rgb(235, 235, 240) };
            Some(base.gamma_multiply((level as f32 / 255.0).max(0.12)))
        })
        .collect()
}

const ROW_H: f32 = 44.0;

impl App {
    pub fn scenes(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let active = e.active;
        let list = e.list();
        self.scene_sel = self.scene_sel.min(list.cues.len().saturating_sub(1));

        ui.horizontal(|ui| {
            ui.label(RichText::new("Szenen").heading().strong());
            theme::info(ui, "Szenen", "Eine Szene ist ein gespeicherter Look. Mehrere Szenen bilden eine Cueliste, die du mit GO nacheinander abspielst. Ein Doppelklick oder das Play-Symbol ruft eine Szene direkt ab. Am Griff links ziehst du eine Szene an eine andere Stelle; Überblendzeit und Auto-Weiter stellst du direkt in der Zeile ein, Name und mehr rechts.");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let save = theme::icon_text("plus", "Szene speichern", Color32::BLACK).fill(accent).min_size(egui::vec2(150.0, 32.0));
                if ui.add_enabled(!e.show.fixtures.is_empty(), save).on_hover_text("Speichert den aktuellen Look aller Fixtures als neue Szene").clicked() {
                    acts.push(Action::StoreScene);
                    self.scene_sel = list.cues.len();
                }
                for (grid, icon, tip) in [(true, "grid", "Raster"), (false, "list", "Liste")] {
                    let on = self.scene_grid == grid;
                    let b = egui::Button::image(theme::icon(icon, if on { accent } else { theme::TEXT }, 16.0)).min_size(egui::vec2(32.0, 32.0)).selected(on);
                    if ui.add(b).on_hover_text(tip).clicked() {
                        self.scene_grid = grid;
                    }
                }
            });
        });

        ui.horizontal_wrapped(|ui| {
            for (i, l) in e.show.lists.iter().enumerate() {
                let on = e.players.get(i).is_some_and(|p| p.cue.is_some());
                let r = tile(ui, format!("{}{}", if on { "● " } else { "" }, short(&l.name, 16)), i == active, accent);
                if r.clicked() {
                    acts.push(Action::SelectList(i));
                }
            }
            if ui.add(theme::icon_text("plus", "Liste", theme::TEXT)).on_hover_text("Neue Cueliste").clicked() {
                acts.push(Action::AddList);
            }
            ui.add_space(12.0);
            let mut name = list.name.clone();
            if ui.add(egui::TextEdit::singleline(&mut name).desired_width(130.0)).on_hover_text("Name der Liste").changed() {
                acts.push(Action::RenameList(active, name));
            }
            let mut looped = list.looped;
            if ui.checkbox(&mut looped, "Schleife").on_hover_text("Nach der letzten Szene wieder bei der ersten beginnen").changed() {
                acts.push(Action::SetLooped(active, looped));
            }
            if e.show.lists.len() > 1 && theme::icon_btn(ui, "trash", "Liste löschen").clicked() {
                acts.push(Action::DeleteList(active));
            }
        });

        if list.cues.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Noch keine Szenen").size(20.0).color(theme::MUTED));
                ui.label(RichText::new("Stelle im Tab „Pult“ einen Look ein und speichere ihn mit „Szene speichern“.").color(theme::MUTED));
            });
            return;
        }
        egui::ScrollArea::vertical().id_salt("pads").auto_shrink([false, false]).show(ui, |ui| {
            if self.scene_grid {
                self.scene_pads(ui, e, acts);
            } else {
                self.scene_table(ui, e, acts);
            }
        });
    }

    fn scene_table(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let list = e.list();
        let cur = e.cue();
        let w = ui.available_width();
        let wide = w > 640.0;
        let (fade_x, follow_x, strip_w) = (if wide { w - 380.0 } else { w - 270.0 }, if wide { w - 270.0 } else { w - 160.0 }, if wide { 100.0 } else { 0.0 });
        let (hr, _) = ui.allocate_exact_size(egui::vec2(w, 26.0), egui::Sense::hover());
        let heads = [(76.0, "SZENE", ""), (fade_x, "ÜBERBLENDEN", "Wie lange die Szene braucht, um von der vorherigen auf diese zu wechseln."), (follow_x, "AUTO-WEITER", "Startet nach dieser Wartezeit selbst die nächste Szene. Aus = du drückst GO.")];
        for (x, t, _) in heads {
            ui.painter().text(egui::pos2(hr.left() + x, hr.center().y), egui::Align2::LEFT_CENTER, t, egui::FontId::proportional(11.0), theme::MUTED);
        }
        if wide {
            ui.painter().text(egui::pos2(hr.right() - strip_w - 12.0, hr.center().y), egui::Align2::LEFT_CENTER, "FARBEN", egui::FontId::proportional(11.0), theme::MUTED);
        }
        ui.spacing_mut().item_spacing.y = 4.0;
        let gap = 4.0;
        let top = ui.cursor().top();
        let pointer_y = ui.input(|i| i.pointer.interact_pos()).map(|p| p.y);
        let target = self.scene_drag.and_then(|_| pointer_y).map(|y| (((y - top) / (ROW_H + gap)).floor().max(0.0) as usize).min(list.cues.len().saturating_sub(1)));
        for (i, c) in list.cues.iter().enumerate() {
            let live = cur == Some(i);
            let picked = self.scene_sel == i;
            let (rect, r) = ui.allocate_exact_size(egui::vec2(w, ROW_H), egui::Sense::click());
            let dragging = self.scene_drag == Some(i);
            let fill = if picked { theme::RAISED } else if r.hovered() { theme::HOVER.gamma_multiply(0.5) } else { theme::PANEL };
            let p = ui.painter();
            p.rect(rect, 4.0, if dragging { fill.gamma_multiply(0.5) } else { fill }, egui::Stroke::new(1.0, if live { accent } else { theme::LINE }), egui::StrokeKind::Inside);
            let mid = rect.center().y;
            if r.double_clicked() {
                play(acts, e, Action::GotoCue(i));
            } else if r.clicked() {
                self.scene_sel = i;
            }
            let grip = egui::Rect::from_center_size(egui::pos2(rect.left() + 18.0, mid), egui::vec2(24.0, 32.0));
            let gr = ui.interact(grip, ui.id().with(("grip", i)), egui::Sense::drag());
            theme::icon("grip", if gr.hovered() || dragging { theme::TEXT } else { theme::MUTED }, 16.0).paint_at(ui, grip.shrink2(egui::vec2(4.0, 8.0)));
            if gr.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
            if gr.drag_started() {
                self.scene_drag = Some(i);
                self.scene_sel = i;
            }
            if gr.drag_stopped() {
                if let Some(t) = target.filter(|t| *t != i) {
                    acts.push(Action::MoveCue(i, t));
                    self.scene_sel = t;
                }
                self.scene_drag = None;
            }
            let play_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 52.0, mid), egui::vec2(28.0, 28.0));
            let pr = ui.interact(play_rect, ui.id().with(("play", i)), egui::Sense::click());
            if pr.hovered() {
                ui.painter().rect_filled(play_rect, 4.0, theme::HOVER);
            }
            theme::icon(if live { "stop" } else { "play" }, if live { accent } else { theme::MUTED }, 14.0).paint_at(ui, play_rect.shrink(7.0));
            if pr.on_hover_text(if live { "Liste ausschalten" } else { "Szene abrufen" }).clicked() {
                if live {
                    acts.push(Action::ReleaseList(e.active));
                } else {
                    play(acts, e, Action::GotoCue(i));
                }
            }
            ui.painter().text(egui::pos2(rect.left() + 76.0, mid), egui::Align2::LEFT_CENTER, format!("{}  {}", i + 1, short(&c.name, if wide { 30 } else { 16 })), egui::FontId::proportional(14.0), if live { accent } else { theme::TEXT });
            let cell = |x: f32, width: f32| egui::Rect::from_min_size(egui::pos2(rect.left() + x, mid - 14.0), egui::vec2(width, 28.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(cell(fade_x, 100.0)), |ui| {
                let mut fade = c.fade;
                if ui.add(egui::DragValue::new(&mut fade).speed(0.05).range(0.0..=300.0).suffix(" s").min_decimals(1)).on_hover_text("Ziehen oder doppelklicken zum Ändern").changed() {
                    acts.push(Action::SetCueFade(i, fade));
                }
            });
            ui.scope_builder(egui::UiBuilder::new().max_rect(cell(follow_x, 140.0)), |ui| {
                ui.horizontal(|ui| {
                    let mut auto = c.follow.is_some();
                    if ui.checkbox(&mut auto, "").on_hover_text("Automatisch zur nächsten Szene weiter").changed() {
                        acts.push(Action::SetCueFollow(i, auto.then_some(2.0)));
                    }
                    if let Some(mut f) = c.follow {
                        if ui.add(egui::DragValue::new(&mut f).speed(0.05).range(0.0..=300.0).suffix(" s").min_decimals(1)).on_hover_text("Wartezeit nach dem Überblenden").changed() {
                            acts.push(Action::SetCueFollow(i, Some(f)));
                        }
                    } else {
                        ui.label(RichText::new("Aus").color(theme::MUTED));
                    }
                });
            });
            if strip_w > 0.0 {
                let strip = egui::Rect::from_min_size(egui::pos2(rect.right() - strip_w - 12.0, mid - 6.0), egui::vec2(strip_w, 12.0));
                draw_strip(ui.painter(), strip, &preview(e, c));
            }
            if live {
                ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height())), 2.0, accent);
            }
            if let (Some(from), Some(t)) = (self.scene_drag, target) {
                if t == i && from != i {
                    let y = if t > from { rect.bottom() + gap / 2.0 } else { rect.top() - gap / 2.0 };
                    ui.painter().line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], egui::Stroke::new(2.0, accent));
                }
            }
        }
        if self.scene_drag.is_some() && !ui.input(|i| i.pointer.any_down()) {
            self.scene_drag = None;
        }
    }

    fn scene_pads(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let list = e.list();
        let cur = e.cue();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            for (i, c) in list.cues.iter().enumerate() {
                let live = cur == Some(i);
                let picked = self.scene_sel == i;
                let (rect, r) = ui.allocate_exact_size(egui::vec2(168.0, 92.0), egui::Sense::click());
                let p = ui.painter();
                let fill = if r.hovered() { theme::HOVER } else if picked { theme::RAISED } else { theme::PANEL };
                let line = if live { accent } else if picked { theme::MUTED } else { theme::LINE };
                p.rect(rect, 4.0, fill, egui::Stroke::new(if live { 2.0 } else { 1.0 }, line), egui::StrokeKind::Inside);
                p.text(rect.left_top() + egui::vec2(12.0, 10.0), egui::Align2::LEFT_TOP, (i + 1).to_string(), egui::FontId::proportional(18.0), if live { accent } else { theme::TEXT });
                if live {
                    p.text(rect.right_top() + egui::vec2(-12.0, 13.0), egui::Align2::RIGHT_TOP, "LIVE", egui::FontId::proportional(11.0), accent);
                }
                p.text(rect.left_top() + egui::vec2(12.0, 36.0), egui::Align2::LEFT_TOP, short(&c.name, 19), egui::FontId::proportional(13.5), theme::TEXT);
                p.text(rect.left_top() + egui::vec2(12.0, 55.0), egui::Align2::LEFT_TOP, format!("{:.1} s", c.fade), egui::FontId::proportional(11.5), theme::MUTED);
                let strip = egui::Rect::from_min_size(rect.left_bottom() + egui::vec2(12.0, -16.0), egui::vec2(rect.width() - 24.0, 6.0));
                draw_strip(p, strip, &preview(e, c));
                if r.clicked() {
                    self.scene_sel = i;
                    play(acts, e, Action::GotoCue(i));
                }
            }
        });
    }

    /// Rechte Leiste: ausgewählte Szene bearbeiten.
    pub fn scene_inspector(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let list = e.list();
        let i = self.scene_sel;
        let Some(c) = list.cues.get(i) else {
            theme::caption(ui, "Szene bearbeiten");
            ui.add_space(8.0);
            hint(ui, "Keine Szene ausgewählt.");
            return;
        };
        theme::section(ui, &format!("Szene {}", i + 1), "Hier änderst du Name, Überblendzeit und automatischen Ablauf der ausgewählten Szene. „Look aktualisieren“ übernimmt die aktuellen Programmer-Werte.");
        ui.add_space(8.0);
        let mut name = c.name.clone();
        if ui.add(egui::TextEdit::singleline(&mut name).desired_width(f32::INFINITY)).changed() {
            acts.push(Action::RenameCue(i, name));
        }
        ui.add_space(4.0);
        egui::Grid::new("scene_form").num_columns(2).spacing([12.0, 10.0]).show(ui, |ui| {
            ui.label("Überblenden");
            let mut fade = c.fade;
            if ui.add(egui::DragValue::new(&mut fade).speed(0.05).range(0.0..=300.0).suffix(" s")).changed() {
                acts.push(Action::SetCueFade(i, fade));
            }
            ui.end_row();
            ui.label("Automatisch weiter");
            let mut auto = c.follow.is_some();
            if ui.checkbox(&mut auto, "").on_hover_text("Nach dem Überblenden und der Wartezeit startet die nächste Szene von selbst").changed() {
                acts.push(Action::SetCueFollow(i, auto.then_some(2.0)));
            }
            ui.end_row();
            if let Some(mut f) = c.follow {
                ui.label("Wartezeit");
                if ui.add(egui::DragValue::new(&mut f).speed(0.05).range(0.0..=300.0).suffix(" s")).changed() {
                    acts.push(Action::SetCueFollow(i, Some(f)));
                }
                ui.end_row();
            }
        });
        ui.add_space(12.0);
        let full = egui::vec2(ui.available_width(), 34.0);
        if ui.add(theme::icon_text("play", "Abrufen", theme::TEXT).min_size(full)).clicked() {
            play(acts, e, Action::GotoCue(i));
        }
        if ui.add_enabled(!e.programmer.is_empty(), theme::icon_text("reset", "Look aktualisieren", theme::TEXT).min_size(full)).on_hover_text("Ersetzt den Look dieser Szene durch die aktuellen Programmer-Werte").clicked() {
            acts.push(Action::UpdateCue(i));
        }
        let n = list.cues.len();
        ui.horizontal(|ui| {
            if ui.add_enabled(i > 0, theme::icon_text("up", "Hoch", theme::TEXT).min_size(egui::vec2(88.0, 32.0))).clicked() {
                acts.push(Action::MoveCue(i, i - 1));
                self.scene_sel = i - 1;
            }
            if ui.add_enabled(i + 1 < n, theme::icon_text("down", "Runter", theme::TEXT).min_size(egui::vec2(88.0, 32.0))).clicked() {
                acts.push(Action::MoveCue(i, i + 1));
                self.scene_sel = i + 1;
            }
        });
        if ui.add(theme::icon_text("copy", "Duplizieren", theme::TEXT).min_size(full)).clicked() {
            acts.push(Action::DuplicateCue(i));
        }
        if ui.add(theme::icon_text("trash", "Szene löschen", theme::DANGER).min_size(full)).clicked() {
            acts.push(Action::DeleteCue(i));
        }
    }
}

fn draw_strip(p: &egui::Painter, strip: egui::Rect, cols: &[Color32]) {
    p.rect_filled(strip, 2.0, theme::LINE);
    if !cols.is_empty() {
        let w = strip.width() / cols.len() as f32;
        for (k, col) in cols.iter().enumerate() {
            let seg = egui::Rect::from_min_size(strip.min + egui::vec2(k as f32 * w, 0.0), egui::vec2(w.max(1.0), strip.height()));
            p.rect_filled(seg, 0.0, *col);
        }
    }
}
