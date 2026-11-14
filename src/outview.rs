//! Ausgabe: Netzwerk, MIDI-Zuordnung und DMX-Monitor.

use crate::App;
use crate::theme::{self, card, hint, pill, tile};
use eframe::egui::{self, Color32, RichText};
use lichtpult::engine::Engine;
use lichtpult::midi::{Target, source_name, target_name};

impl App {
    pub fn output(&mut self, ui: &mut egui::Ui, e: &Engine) {
        egui::ScrollArea::vertical().id_salt("output").show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("Ausgabe").heading().strong());
            ui.add_space(6.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                let w = ((ui.available_width() - 14.0) / 2.0).max(380.0);
                ui.vertical(|ui| {
                    ui.set_width(w);
                    self.network_card(ui);
                });
                ui.vertical(|ui| {
                    ui.set_width(w);
                    self.midi_card(ui, e);
                });
            });
            ui.add_space(10.0);
            self.dmx_monitor(ui, e);
        });
    }

    fn dmx_monitor(&mut self, ui: &mut egui::Ui, e: &Engine) {
        let accent = self.accent();
        card(ui, "DMX-Monitor", |ui| {
            ui.horizontal_wrapped(|ui| {
                let mut unis: Vec<u16> = e.frames.keys().copied().collect();
                unis.sort_unstable();
                if unis.is_empty() {
                    unis.push(self.out_universe);
                }
                for u in unis {
                    if tile(ui, format!("Universum {u}"), u == self.out_universe, accent).clicked() {
                        self.out_universe = u;
                    }
                }
            });
            ui.add_space(4.0);
            let empty = [0u8; 512];
            let frame = e.frames.get(&self.out_universe).unwrap_or(&empty);
            let cell = ((ui.available_width() - 40.0) / 32.0).clamp(22.0, 40.0);
            for (r, row) in frame.chunks(32).enumerate() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let (lr, _) = ui.allocate_exact_size(egui::vec2(34.0, 20.0), egui::Sense::hover());
                    ui.painter().text(lr.right_center(), egui::Align2::RIGHT_CENTER, (r * 32 + 1).to_string(), egui::FontId::proportional(10.0), theme::MUTED);
                    for &v in row {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(cell - 2.0, 20.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 3.0, theme::RAISED);
                        ui.painter().rect_filled(rect, 3.0, accent.gamma_multiply(v as f32 / 255.0));
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            v.to_string(),
                            egui::FontId::monospace(10.0),
                            if v > 140 { Color32::BLACK } else { theme::MUTED },
                        );
                    }
                });
            }
        });
    }

    fn network_card(&mut self, ui: &mut egui::Ui) {
        use lichtpult::Protocol;
        let accent = self.accent();
        let out = self.out.clone();
        let mut c = out.lock().unwrap().clone();
        let before = c.clone();
        card(ui, "Netzwerk", |ui| {
            ui.set_min_height(170.0);
            ui.horizontal(|ui| {
                if c.enabled {
                    pill(ui, "sendet", theme::OK);
                } else {
                    pill(ui, "aus", theme::DANGER);
                }
                ui.checkbox(&mut c.enabled, "Aktiv");
            });
            ui.horizontal(|ui| {
                for (p, n) in [(Protocol::ArtNet, "Art-Net"), (Protocol::Sacn, "sACN"), (Protocol::Both, "Beide")] {
                    if tile(ui, n, c.protocol == p, accent).clicked() {
                        c.protocol = p;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Ziel-IP");
                ui.add(egui::TextEdit::singleline(&mut c.target).hint_text("leer = Broadcast / Multicast").desired_width(200.0));
            });
            if c.protocol != Protocol::ArtNet {
                ui.add(egui::DragValue::new(&mut c.sacn_priority).range(0..=200).prefix("sACN-Priorität "));
            }
            hint(ui, "Art-Net: UDP 6454 · sACN: UDP 5568. Mit Ziel-IP wird gezielt gesendet, z. B. an den ESP32-Node.");
        });
        if c.target != before.target || c.protocol != before.protocol || c.enabled != before.enabled || c.sacn_priority != before.sacn_priority {
            lichtpult::output::save(&c);
            *out.lock().unwrap() = c;
        }
    }

    /// Zuordnungstabelle: pro Funktion eine Zeile mit „Lernen“-Knopf.
    fn midi_card(&mut self, ui: &mut egui::Ui, e: &Engine) {
        let accent = self.accent();
        let midi = self.midi.clone();
        let mut m = midi.lock().unwrap();
        card(ui, "MIDI-Controller", |ui| {
            ui.horizontal(|ui| {
                if m.ports.is_empty() {
                    pill(ui, "kein Gerät", theme::MUTED);
                } else {
                    pill(ui, &m.ports.join(", "), theme::OK);
                }
                if let Some(s) = m.last {
                    hint(ui, &format!("zuletzt: {}", source_name(s)));
                }
            });
            hint(ui, "„Lernen“ klicken, dann Taste oder Regler am Controller betätigen.");
            let mut rows: Vec<Target> = Vec::new();
            for (i, _) in e.show.lists.iter().enumerate() {
                rows.extend([Target::Go(i), Target::Back(i), Target::Level(i)]);
            }
            rows.extend([Target::Master, Target::Blackout]);
            let mut del = None;
            egui::ScrollArea::vertical().id_salt("midi").max_height(220.0).show(ui, |ui| {
                egui::Grid::new("midi_grid").num_columns(3).spacing([12.0, 6.0]).show(ui, |ui| {
                    for t in rows {
                        ui.label(target_name(t));
                        let bound = m.binds.iter().position(|b| b.target == t);
                        match bound {
                            Some(i) => {
                                ui.label(RichText::new(source_name(m.binds[i].src)).color(accent));
                            }
                            None => {
                                ui.label(RichText::new("—").color(theme::MUTED));
                            }
                        }
                        ui.horizontal(|ui| {
                            if m.learn == Some(t) {
                                if ui.button("Warte…").clicked() {
                                    m.learn = None;
                                }
                            } else if ui.small_button("Lernen").clicked() {
                                m.learn = Some(t);
                            }
                            if let Some(i) = bound {
                                if theme::icon_btn(ui, "x", "Zuordnung entfernen").clicked() {
                                    del = Some(i);
                                }
                            }
                        });
                        ui.end_row();
                    }
                });
            });
            if let Some(i) = del {
                m.binds.remove(i);
                lichtpult::midi::save_state(&m);
            }
        });
    }
}
