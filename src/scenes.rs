//! Szenen: Cuelisten als Raster aus abrufbaren Szenen-Pads.

use crate::theme::{self, card, fader, hint, pill, tile, Fader};
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

impl App {
    pub fn scenes(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let active = e.active;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Szenen").heading().strong());
            hint(ui, "Eine Szene ist ein gespeicherter Look. Klick auf ein Pad = Szene abrufen.");
        });
        ui.add_space(6.0);

        // Listen
        ui.horizontal_wrapped(|ui| {
            for (i, l) in e.show.lists.iter().enumerate() {
                let on = e.players.get(i).is_some_and(|p| p.cue.is_some());
                let r = tile(ui, format!("{}{}", if on { "● " } else { "" }, short(&l.name, 16)), i == active, accent);
                if r.clicked() {
                    acts.push(Action::SelectList(i));
                }
            }
            if ui.button("＋ Liste").on_hover_text("Neue Cueliste").clicked() {
                acts.push(Action::AddList);
            }
        });
        ui.add_space(6.0);

        // Transport
        let list = e.list();
        let cur = e.cue();
        card(ui, "", |ui| {
            ui.horizontal(|ui| {
                let big = egui::vec2(52.0, 44.0);
                if ui.add(egui::Button::new(RichText::new("◀").size(18.0)).min_size(big)).on_hover_text("Zurück (Backspace)").clicked() {
                    play(acts, e, Action::Back(active));
                }
                let go = egui::Button::new(RichText::new("GO").size(20.0).strong().color(Color32::BLACK)).fill(accent).min_size(egui::vec2(110.0, 44.0));
                if ui.add(go).on_hover_text("Nächste Szene (Leertaste)").clicked() {
                    play(acts, e, Action::Go(active));
                }
                if ui.add(egui::Button::new(RichText::new("■").size(18.0)).min_size(big)).on_hover_text("Liste ausschalten").clicked() {
                    acts.push(Action::ReleaseList(active));
                }
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    let now = cur.and_then(|c| list.cues.get(c)).map_or("—".to_string(), |c| c.name.clone());
                    let next = list.cues.get(cur.map_or(0, |c| c + 1)).map_or("—".to_string(), |c| c.name.clone());
                    ui.label(RichText::new(format!("Aktuell: {now}")).strong());
                    ui.label(RichText::new(format!("Danach: {next}")).color(theme::MUTED));
                });
                ui.add_space(12.0);
                let lvl = e.players.get(active).map_or(255, |p| p.level);
                ui.vertical(|ui| {
                    ui.set_width(260.0);
                    if let Fader::Set(v) = fader(ui, "Helligkeit", lvl, accent, false) {
                        acts.push(Action::SetLevel(active, v));
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut name = list.name.clone();
                    if ui.add(egui::TextEdit::singleline(&mut name).desired_width(140.0)).changed() {
                        acts.push(Action::RenameList(active, name));
                    }
                    let mut looped = list.looped;
                    if ui.checkbox(&mut looped, "Schleife").on_hover_text("Nach der letzten Szene wieder bei der ersten beginnen").changed() {
                        acts.push(Action::SetLooped(active, looped));
                    }
                    if e.show.lists.len() > 1 && ui.button("Liste löschen").clicked() {
                        acts.push(Action::DeleteList(active));
                    }
                });
            });
            if !e.programmer.is_empty() {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    pill(ui, "Programmer aktiv – überdeckt Szenen", theme::DANGER);
                    if ui.button("Programmer leeren").clicked() {
                        acts.push(Action::ClearProgrammer);
                    }
                });
            }
        });
        ui.add_space(8.0);

        egui::ScrollArea::vertical().id_salt("pads").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
                for (i, c) in list.cues.iter().enumerate() {
                    let live = cur == Some(i);
                    let (rect, r) = ui.allocate_exact_size(egui::vec2(176.0, 96.0), egui::Sense::click());
                    let p = ui.painter();
                    let fill = if r.hovered() { Color32::from_rgb(34, 40, 52) } else { theme::PANEL };
                    p.rect(rect, 10.0, fill, egui::Stroke::new(if live { 2.0 } else { 1.0 }, if live { accent } else { theme::LINE }), egui::StrokeKind::Inside);
                    p.text(rect.left_top() + egui::vec2(12.0, 10.0), egui::Align2::LEFT_TOP, (i + 1).to_string(), egui::FontId::proportional(18.0), if live { accent } else { theme::TEXT });
                    if live {
                        p.text(rect.right_top() + egui::vec2(-12.0, 13.0), egui::Align2::RIGHT_TOP, "LIVE", egui::FontId::proportional(11.0), accent);
                    }
                    p.text(rect.left_top() + egui::vec2(12.0, 36.0), egui::Align2::LEFT_TOP, short(&c.name, 20), egui::FontId::proportional(13.5), theme::TEXT);
                    let mut info = format!("{:.1} s", c.fade);
                    if let Some(f) = c.follow {
                        info.push_str(&format!(" · Follow {f:.1} s"));
                    }
                    p.text(rect.left_top() + egui::vec2(12.0, 56.0), egui::Align2::LEFT_TOP, info, egui::FontId::proportional(11.0), theme::MUTED);
                    let cols = preview(e, c);
                    let strip = egui::Rect::from_min_size(rect.left_bottom() + egui::vec2(12.0, -16.0), egui::vec2(rect.width() - 24.0, 6.0));
                    p.rect_filled(strip, 3.0, theme::LINE);
                    if !cols.is_empty() {
                        let w = strip.width() / cols.len() as f32;
                        for (k, col) in cols.iter().enumerate() {
                            let seg = egui::Rect::from_min_size(strip.min + egui::vec2(k as f32 * w, 0.0), egui::vec2(w.max(1.0), strip.height()));
                            p.rect_filled(seg, 0.0, *col);
                        }
                    }
                    if r.clicked() {
                        play(acts, e, Action::GotoCue(i));
                    }
                    r.context_menu(|ui| {
                        let mut name = c.name.clone();
                        if ui.text_edit_singleline(&mut name).changed() {
                            acts.push(Action::RenameCue(i, name));
                        }
                        if ui.add_enabled(!e.programmer.is_empty(), egui::Button::new("Mit Programmer überschreiben")).clicked() {
                            acts.push(Action::UpdateCue(i));
                            ui.close();
                        }
                        if ui.button("Löschen").clicked() {
                            acts.push(Action::DeleteCue(i));
                            ui.close();
                        }
                    });
                }
                let (rect, r) = ui.allocate_exact_size(egui::vec2(176.0, 96.0), egui::Sense::click());
                let col = if r.hovered() { accent } else { theme::MUTED };
                ui.painter().rect(rect, 10.0, Color32::TRANSPARENT, egui::Stroke::new(1.0, col), egui::StrokeKind::Inside);
                ui.painter().text(rect.center() - egui::vec2(0.0, 8.0), egui::Align2::CENTER_CENTER, "＋", egui::FontId::proportional(24.0), col);
                ui.painter().text(rect.center() + egui::vec2(0.0, 16.0), egui::Align2::CENTER_CENTER, "Szene speichern", egui::FontId::proportional(12.0), col);
                if r.on_hover_text("Speichert den aktuellen Look aller Fixtures als neue Szene").clicked() {
                    acts.push(Action::StoreScene);
                }
            });
            if list.cues.is_empty() {
                ui.add_space(8.0);
                hint(ui, "Noch keine Szenen. Stelle im Tab „Pult“ einen Look ein und speichere ihn hier – oder klicke auf „＋ Szene speichern“.");
            }
            if let Some(i) = cur.filter(|i| *i < list.cues.len()) {
                ui.add_space(12.0);
                let c = &list.cues[i];
                card(ui, &format!("Szene {} bearbeiten", i + 1), |ui| {
                    ui.horizontal(|ui| {
                        let mut name = c.name.clone();
                        if ui.add(egui::TextEdit::singleline(&mut name).desired_width(200.0)).changed() {
                            acts.push(Action::RenameCue(i, name));
                        }
                        let mut fade = c.fade;
                        if ui.add(egui::DragValue::new(&mut fade).speed(0.05).range(0.0..=300.0).suffix(" s").prefix("Überblendzeit ")).changed() {
                            acts.push(Action::SetCueFade(i, fade));
                        }
                        let mut auto = c.follow.is_some();
                        if ui.checkbox(&mut auto, "Automatisch weiter").on_hover_text("Nach dem Überblenden und der Wartezeit startet die nächste Szene von selbst").changed() {
                            acts.push(Action::SetCueFollow(i, auto.then_some(2.0)));
                        }
                        if let Some(mut f) = c.follow {
                            if ui.add(egui::DragValue::new(&mut f).speed(0.05).range(0.0..=300.0).suffix(" s").prefix("nach ")).changed() {
                                acts.push(Action::SetCueFollow(i, Some(f)));
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!e.programmer.is_empty(), egui::Button::new("Mit Programmer überschreiben")).clicked() {
                            acts.push(Action::UpdateCue(i));
                        }
                        if ui.button("Löschen").clicked() {
                            acts.push(Action::DeleteCue(i));
                        }
                    });
                });
            }
        });
    }
}
