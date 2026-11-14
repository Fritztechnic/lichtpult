//! Pult: Auswahl (links), Programmer (Mitte), Paletten/Effekte (rechts).

use crate::theme::{self, Fader, card, fader, hint, section, tile};
use crate::{App, rgb_of, short};
use eframe::egui::{self, Color32, RichText};
use lichtpult::engine::{Action, Engine, Shape, Target};
use lichtpult::fixture::{Attr, Fixture, Kind};

/// Kachel für ein Fixture mit Live-Anzeige von Farbe und Intensität.
fn fixture_tile(ui: &mut egui::Ui, e: &Engine, f: &Fixture, selected: bool, bad: bool, accent: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(92.0, 54.0), egui::Sense::click());
    let p = ui.painter();
    let fill = if selected {
        accent.gamma_multiply(0.16)
    } else if resp.hovered() {
        Color32::from_rgb(38, 44, 58)
    } else {
        theme::RAISED
    };
    let line = if bad { theme::DANGER } else if selected { accent } else { theme::LINE };
    p.rect(rect, 7.0, fill, egui::Stroke::new(if selected || bad { 1.5 } else { 1.0 }, line), egui::StrokeKind::Inside);
    p.text(rect.left_top() + egui::vec2(8.0, 6.0), egui::Align2::LEFT_TOP, f.id.to_string(), egui::FontId::proportional(14.0), if selected { accent } else { theme::TEXT });
    p.text(rect.left_top() + egui::vec2(8.0, 25.0), egui::Align2::LEFT_TOP, short(&f.name, 13), egui::FontId::proportional(10.5), theme::MUTED);

    let out = |a| e.output_of(f, a);
    let has_color = [Attr::Red, Attr::Green, Attr::Blue, Attr::White].iter().any(|a| e.show.has(f, *a));
    let color = rgb_of(out);
    let mx = color.r().max(color.g()).max(color.b());
    let level = if e.show.has(f, Attr::Dimmer) { out(Attr::Dimmer) } else { mx };
    let hue = if has_color && mx > 0 {
        let k = 255.0 / mx as f32;
        Color32::from_rgb((color.r() as f32 * k) as u8, (color.g() as f32 * k) as u8, (color.b() as f32 * k) as u8)
    } else {
        Color32::from_rgb(235, 235, 240)
    };
    let bar = egui::Rect::from_min_size(rect.left_bottom() + egui::vec2(8.0, -9.0), egui::vec2(rect.width() - 16.0, 3.0));
    p.rect_filled(bar, 1.5, theme::LINE);
    let mut lit = bar;
    lit.set_width(bar.width() * level as f32 / 255.0);
    p.rect_filled(lit, 1.5, hue);
    if level > 0 {
        p.circle_filled(rect.right_top() + egui::vec2(-11.0, 11.0), 4.5, hue);
    }
    resp
}

/// Zweidimensionales Feld für Pan/Tilt.
fn xy_pad(ui: &mut egui::Ui, x: u8, y: u8, tint: Color32) -> Option<(u8, u8)> {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(150.0, 150.0), egui::Sense::click_and_drag());
    let p = ui.painter();
    p.rect(rect, 8.0, Color32::from_rgb(10, 12, 16), egui::Stroke::new(1.0, theme::LINE), egui::StrokeKind::Inside);
    p.line_segment([rect.center_top(), rect.center_bottom()], egui::Stroke::new(1.0, theme::LINE));
    p.line_segment([rect.left_center(), rect.right_center()], egui::Stroke::new(1.0, theme::LINE));
    let pos = egui::pos2(rect.left() + rect.width() * x as f32 / 255.0, rect.top() + rect.height() * y as f32 / 255.0);
    p.circle(pos, 7.0, tint.gamma_multiply(0.4), egui::Stroke::new(2.0, tint));
    if r.is_pointer_button_down_on() {
        let h = r.interact_pointer_pos()?;
        let k = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        return Some((k((h.x - rect.left()) / rect.width()), k((h.y - rect.top()) / rect.height())));
    }
    None
}

const QUICK: [(&str, [u8; 3]); 9] = [
    ("Rot", [255, 0, 0]),
    ("Orange", [255, 110, 0]),
    ("Gelb", [255, 220, 0]),
    ("Grün", [0, 255, 0]),
    ("Cyan", [0, 220, 255]),
    ("Blau", [0, 40, 255]),
    ("Magenta", [255, 0, 200]),
    ("Weiß", [255, 255, 255]),
    ("Aus", [0, 0, 0]),
];

impl App {
    /// Linke Seite: Fixtures wählen und Gruppen.
    pub fn pult_select(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let has_sel = !e.selection.is_empty();
        ui.horizontal(|ui| {
            section(ui, "Fixtures", "Wähle die Lampen aus, die du einstellen willst. Strg/Shift + Klick wählt mehrere.");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Keine").clicked() {
                    acts.push(Action::ClearSelection);
                }
                if ui.small_button("Alle").clicked() {
                    acts.push(Action::SelectAll);
                }
            });
        });
        let conflicts = e.conflicts();
        egui::ScrollArea::vertical().id_salt("fixtures").max_height((ui.available_height() - 190.0).max(140.0)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for f in &e.show.fixtures {
                    let r = fixture_tile(ui, e, f, e.selection.contains(&f.id), conflicts.contains(&f.id), accent);
                    if r.clicked() {
                        acts.push(if ui.input(|i| i.modifiers.command || i.modifiers.shift) { Action::Toggle(f.id) } else { Action::Select(f.id) });
                    }
                }
            });
            if e.show.fixtures.is_empty() {
                hint(ui, "Noch keine Fixtures – im Tab „Patch“ hinzufügen.");
            }
        });
        ui.add_space(8.0);
        ui.separator();
        section(ui, "Gruppen", "Eine Gruppe merkt sich eine Auswahl (z. B. „Bühne vorne“). Ein Klick wählt alle Mitglieder.");
        ui.horizontal_wrapped(|ui| {
            for (i, g) in e.show.groups.iter().enumerate() {
                let r = tile(ui, short(&g.name, 14), false, accent);
                if r.clicked() {
                    acts.push(Action::SelectGroup(i));
                }
                r.context_menu(|ui| {
                    let mut name = g.name.clone();
                    if ui.text_edit_singleline(&mut name).changed() {
                        acts.push(Action::RenameGroup(i, name));
                    }
                    if ui.add_enabled(has_sel, egui::Button::new("Mit Auswahl überschreiben")).clicked() {
                        acts.push(Action::UpdateGroup(i));
                        ui.close();
                    }
                    if ui.button("Löschen").clicked() {
                        acts.push(Action::DeleteGroup(i));
                        ui.close();
                    }
                });
            }
            if ui.add_enabled(has_sel, egui::Button::new("＋ Aus Auswahl")).on_hover_text("Aktuelle Auswahl als Gruppe speichern").clicked() {
                acts.push(Action::RecordGroup);
            }
        });
        if e.show.groups.is_empty() {
            hint(ui, "Auswahl treffen, dann „＋ Aus Auswahl“. Rechtsklick = umbenennen/löschen.");
        }
    }

    /// Mitte: Programmer (manuelle Werte der gewählten Fixtures).
    pub fn pult(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let sel: Vec<&Fixture> = e.show.fixtures.iter().filter(|f| e.selection.contains(&f.id)).collect();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Programmer").heading().strong());
            if sel.is_empty() {
                hint(ui, "keine Fixtures ausgewählt");
            } else {
                hint(ui, &format!("{} Fixture(s) ausgewählt", sel.len()));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let store = egui::Button::new(RichText::new("● Als Szene speichern").strong().color(Color32::BLACK)).fill(accent).min_size(egui::vec2(170.0, 32.0));
                if ui
                    .add_enabled(!e.show.fixtures.is_empty(), store)
                    .on_hover_text("Hält den aktuellen Look fest. Du findest ihn im Tab „Szenen“ und kannst ihn dort per Klick abrufen.")
                    .clicked()
                {
                    acts.push(Action::StoreScene);
                }
                if ui.add_enabled(!e.programmer.is_empty(), egui::Button::new("Leeren")).on_hover_text("Alle manuellen Werte verwerfen").clicked() {
                    acts.push(Action::ClearProgrammer);
                }
            });
        });
        ui.add_space(6.0);
        if sel.is_empty() {
            ui.add_space(60.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Fixtures links auswählen").size(20.0).color(theme::MUTED));
                ui.add_space(4.0);
                ui.label(RichText::new("Dann stellst du hier Helligkeit, Farbe und Bewegung ein.").color(theme::MUTED));
                ui.label(RichText::new("Mit „Als Szene speichern“ hältst du den Look fest und rufst ihn im Tab „Szenen“ wieder ab.").color(theme::MUTED));
                ui.add_space(10.0);
                if ui.button("Alle Fixtures auswählen").clicked() {
                    acts.push(Action::SelectAll);
                }
            });
            return;
        }
        let play = e.playback_values();
        egui::ScrollArea::vertical().id_salt("programmer").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
                for kind in Kind::ALL {
                    let attrs: Vec<Attr> = Attr::ALL.into_iter().filter(|a| a.kind() == kind && sel.iter().any(|f| e.show.has(f, *a))).collect();
                    if attrs.is_empty() {
                        continue;
                    }
                    ui.allocate_ui(egui::vec2(380.0, 0.0), |ui| {
                        card(ui, kind.name(), |ui| {
                            ui.set_width(356.0);
                            let get = |a: Attr| -> (u8, bool) {
                                let vals: Vec<(u8, bool)> = sel.iter().filter(|f| e.show.has(f, a)).map(|f| e.value_of(&play, f.id, a)).collect();
                                (vals.first().map_or(0, |v| v.0), vals.iter().any(|v| v.1))
                            };
                            if kind == Kind::Color && attrs.iter().any(|a| matches!(a, Attr::Red | Attr::Green | Attr::Blue)) {
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
                                    for (name, c) in QUICK {
                                        let b = egui::Button::new("").fill(Color32::from_rgb(c[0], c[1], c[2])).min_size(egui::vec2(34.0, 24.0));
                                        if ui.add(b).on_hover_text(name).clicked() {
                                            for (a, v) in [(Attr::Red, c[0]), (Attr::Green, c[1]), (Attr::Blue, c[2])] {
                                                acts.push(Action::SetAttr(a, v));
                                            }
                                        }
                                    }
                                    let mut rgb = [get(Attr::Red).0, get(Attr::Green).0, get(Attr::Blue).0];
                                    if ui.color_edit_button_srgb(&mut rgb).on_hover_text("Farbwähler").changed() {
                                        for (a, v) in [(Attr::Red, rgb[0]), (Attr::Green, rgb[1]), (Attr::Blue, rgb[2])] {
                                            acts.push(Action::SetAttr(a, v));
                                        }
                                    }
                                });
                                ui.add_space(4.0);
                            }
                            for a in &attrs {
                                let (value, set) = get(*a);
                                match fader(ui, a.name(), value, theme::attr_tint(*a, accent), set) {
                                    Fader::Set(v) => acts.push(Action::SetAttr(*a, v)),
                                    Fader::Reset => acts.push(Action::ReleaseAttr(*a)),
                                    Fader::None => {}
                                }
                            }
                            if kind == Kind::Position && attrs.contains(&Attr::Pan) && attrs.contains(&Attr::Tilt) {
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    if let Some((x, y)) = xy_pad(ui, get(Attr::Pan).0, get(Attr::Tilt).0, accent) {
                                        acts.push(Action::SetAttr(Attr::Pan, x));
                                        acts.push(Action::SetAttr(Attr::Tilt, y));
                                    }
                                    hint(ui, "Ziehen im Feld:\nwaagrecht = Pan\nsenkrecht = Tilt");
                                });
                            }
                        });
                    });
                }
            });
            ui.add_space(6.0);
            hint(ui, "Regler ziehen zum Einstellen · Doppelklick gibt einen Wert wieder frei");
        });
    }

    /// Rechte Seite: Paletten und Effekte.
    pub fn pult_library(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let has_sel = !e.selection.is_empty();
        egui::ScrollArea::vertical().id_salt("library").show(ui, |ui| {
            ui.set_width(ui.available_width());
            section(ui, "Paletten", "Eine Palette speichert eine Einstellung (z. B. die Farbe „Rot“ oder eine Position). Ein Klick wendet sie auf die aktuelle Auswahl an.");
            ui.horizontal_wrapped(|ui| {
                for k in Kind::ALL {
                    if tile(ui, k.name(), self.pal_kind == k, accent).clicked() {
                        self.pal_kind = k;
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                for (i, p) in e.show.palettes.iter().enumerate().filter(|(_, p)| p.kind == self.pal_kind) {
                    let r = tile(ui, short(&p.name, 12), false, accent);
                    if p.kind == Kind::Color {
                        let c = rgb_of(|a| p.values.get(&a).copied().unwrap_or(0));
                        let bar = egui::Rect::from_min_size(r.rect.left_bottom() + egui::vec2(6.0, -5.0), egui::vec2(r.rect.width() - 12.0, 3.0));
                        ui.painter().rect_filled(bar, 1.5, c);
                    }
                    if r.clicked() {
                        acts.push(Action::ApplyPalette(i));
                    }
                    r.context_menu(|ui| {
                        let mut name = p.name.clone();
                        if ui.text_edit_singleline(&mut name).changed() {
                            acts.push(Action::RenamePalette(i, name));
                        }
                        if ui.button("Mit Programmer überschreiben").clicked() {
                            acts.push(Action::UpdatePalette(i));
                            ui.close();
                        }
                        if ui.button("Löschen").clicked() {
                            acts.push(Action::DeletePalette(i));
                            ui.close();
                        }
                    });
                }
                let can = !e.programmer.is_empty();
                if ui.add_enabled(can, egui::Button::new("＋ Speichern")).on_hover_text("Die eingestellten Werte dieser Art als Palette sichern").clicked() {
                    acts.push(Action::RecordPalette(self.pal_kind));
                }
            });
            hint(ui, "So geht’s: Fixtures wählen → Wert einstellen → „＋ Speichern“. Rechtsklick = umbenennen/löschen.");
            ui.add_space(10.0);
            ui.separator();

            section(ui, "Effekte", "Effekte bewegen Werte automatisch: pulsierende Helligkeit, Lauflicht, Regenbogen oder kreisende Bewegung. Sie laufen auf den gewählten Fixtures.");
            ui.horizontal_wrapped(|ui| {
                for (name, t, s) in [
                    ("Pulsieren", Target::Intensity, Shape::Sine),
                    ("Lauflicht", Target::Intensity, Shape::Square),
                    ("Regenbogen", Target::Color, Shape::Sine),
                    ("Kreis", Target::Position, Shape::Sine),
                ] {
                    if ui.add_enabled(has_sel, egui::Button::new(name)).on_hover_text("Neuen Effekt für die Auswahl starten").clicked() {
                        acts.push(Action::AddEffect(t, s));
                    }
                }
            });
            if !has_sel {
                hint(ui, "Zuerst Fixtures auswählen, dann einen Effekt wählen.");
            }
            for (i, fx) in e.show.effects.iter().enumerate() {
                let mut n = fx.clone();
                let mut changed = false;
                ui.add_space(4.0);
                egui::Frame::new().fill(theme::RAISED).corner_radius(8).inner_margin(8).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let b = egui::Button::new(if fx.running { "■" } else { "▶" }).min_size(egui::vec2(30.0, 26.0));
                        if ui.add(if fx.running { b.fill(accent.gamma_multiply(0.3)) } else { b }).on_hover_text("Starten/Stoppen").clicked() {
                            acts.push(Action::ToggleEffect(i));
                        }
                        ui.label(RichText::new(short(&fx.name, 16)).strong().color(if fx.running { accent } else { theme::TEXT }));
                        hint(ui, &format!("{} Fixtures", fx.ids.len()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("✕").on_hover_text("Effekt löschen").clicked() {
                                acts.push(Action::DeleteEffect(i));
                            }
                        });
                    });
                    ui.horizontal(|ui| {
                        changed |= ui.add(egui::DragValue::new(&mut n.rate).speed(0.02).suffix(" Hz").range(0.01..=20.0)).on_hover_text("Geschwindigkeit").changed();
                        changed |= ui.add(egui::DragValue::new(&mut n.size).prefix("Stärke ")).changed();
                        changed |= ui.add(egui::DragValue::new(&mut n.spread).speed(0.01).prefix("Versatz ").range(0.0..=1.0)).on_hover_text("Phasenversatz zwischen den Fixtures (Lauflicht-Effekt)").changed();
                    });
                });
                if changed {
                    acts.push(Action::SetEffect(i, n));
                }
            }
            if e.show.effects.iter().any(|f| f.running) && ui.button("■ Alle Effekte stoppen").clicked() {
                acts.push(Action::StopEffects);
            }
        });
    }
}
