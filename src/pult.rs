//! Pult: Auswahl (links), Programmer (Mitte), Paletten/Effekte (rechts).

use crate::theme::{self, Fader, caption, fader, hint, icon_btn, section, tile};
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
        theme::HOVER
    } else {
        theme::RAISED
    };
    let line = if bad { theme::DANGER } else if selected { accent } else { theme::LINE };
    p.rect(rect, 4.0, fill, egui::Stroke::new(if selected || bad { 1.5 } else { 1.0 }, line), egui::StrokeKind::Inside);
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
    p.rect(rect, 4.0, Color32::from_rgb(14, 14, 14), egui::Stroke::new(1.0, theme::LINE), egui::StrokeKind::Inside);
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
                if theme::button(ui, "Keine").clicked() {
                    acts.push(Action::ClearSelection);
                }
                if theme::button(ui, "Alle").clicked() {
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
            if ui.add_enabled(has_sel, theme::icon_text("plus", "Aus Auswahl", theme::TEXT)).on_hover_text("Aktuelle Auswahl als Gruppe speichern").clicked() {
                acts.push(Action::RecordGroup);
            }
        });
        if e.show.groups.is_empty() {
            hint(ui, "Auswahl treffen, dann „Aus Auswahl“. Rechtsklick = umbenennen/löschen.");
        }
    }

    /// Mitte: Programmer (manuelle Werte der gewählten Fixtures).
    pub fn pult(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let sel: Vec<&Fixture> = e.show.fixtures.iter().filter(|f| e.selection.contains(&f.id)).collect();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Programmer").heading().strong());
            theme::info(ui, "Programmer", "Hier stellst du die gewählten Fixtures manuell ein. Helle Regler sind deine Werte, gedimmte kommen aus einer laufenden Szene. Mit dem Pfeil rechts neben einem Regler gibst du den Wert wieder frei. „Als Szene speichern“ hält den Look fest; abrufen kannst du ihn im Tab „Szenen“.");
            if sel.is_empty() {
                hint(ui, "keine Fixtures ausgewählt");
            } else {
                hint(ui, &format!("{} Fixture(s) ausgewählt", sel.len()));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let store = theme::icon_text("plus", "Als Szene speichern", Color32::BLACK).fill(accent).min_size(egui::vec2(170.0, 32.0));
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
                ui.add_space(10.0);
                if ui.button("Alle Fixtures auswählen").clicked() {
                    acts.push(Action::SelectAll);
                }
            });
            return;
        }
        let play = e.playback_values();
        let mut cards: Vec<(Kind, Vec<Attr>)> = Vec::new();
        for kind in Kind::ALL {
            let attrs: Vec<Attr> = Attr::ALL.into_iter().filter(|a| a.kind() == kind && sel.iter().any(|f| e.show.has(f, *a))).collect();
            if !attrs.is_empty() {
                cards.push((kind, attrs));
            }
        }
        // Karten zeilenweise packen, damit nichts überlappt.
        let avail = ui.available_width();
        let card_w = |attrs: &[Attr]| (attrs.len() as f32 * (theme::FADER_W + 8.0)).clamp(300.0, (avail - 34.0).max(300.0));
        let mut rows: Vec<Vec<usize>> = vec![Vec::new()];
        let mut used = 0.0;
        for i in 0..cards.len() {
            let w = card_w(&cards[i].1) + 34.0;
            if used + w > avail && used > 0.0 {
                rows.push(Vec::new());
                used = 0.0;
            }
            used += w;
            rows.last_mut().unwrap().push(i);
        }
        egui::ScrollArea::vertical().id_salt("programmer").show(ui, |ui| {
            for row in rows {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    for i in row {
                        let (kind, attrs) = &cards[i];
                        let kind = *kind;
                        egui::Frame::new().fill(theme::PANEL).stroke(egui::Stroke::new(1.0, theme::LINE)).corner_radius(4).inner_margin(16).show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.set_width(card_w(attrs));
                                caption(ui, kind.name());
                                let get = |a: Attr| -> (u8, bool) {
                                    let vals: Vec<(u8, bool)> = sel.iter().filter(|f| e.show.has(f, a)).map(|f| e.value_of(&play, f.id, a)).collect();
                                    (vals.first().map_or(0, |v| v.0), vals.iter().any(|v| v.1))
                                };
                                if kind == Kind::Color && attrs.iter().any(|a| matches!(a, Attr::Red | Attr::Green | Attr::Blue)) {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                                        for (name, c) in QUICK {
                                            let b = egui::Button::new("").fill(Color32::from_rgb(c[0], c[1], c[2])).min_size(egui::vec2(32.0, 32.0));
                                            if ui.add(b).on_hover_text(name).clicked() {
                                                for (a, v) in [(Attr::Red, c[0]), (Attr::Green, c[1]), (Attr::Blue, c[2])] {
                                                    acts.push(Action::SetAttr(a, v));
                                                }
                                            }
                                        }
                                    });
                                    let mut rgb = [get(Attr::Red).0, get(Attr::Green).0, get(Attr::Blue).0];
                                    ui.horizontal(|ui| {
                                        if ui.color_edit_button_srgb(&mut rgb).on_hover_text("Farbwähler").changed() {
                                            for (a, v) in [(Attr::Red, rgb[0]), (Attr::Green, rgb[1]), (Attr::Blue, rgb[2])] {
                                                acts.push(Action::SetAttr(a, v));
                                            }
                                        }
                                        ui.label(RichText::new("Eigene Farbe").color(theme::MUTED));
                                    });
                                }
                                ui.horizontal_top(|ui| {
                                    ui.spacing_mut().item_spacing.x = 8.0;
                                    for a in attrs {
                                        let (value, set) = get(*a);
                                        match fader(ui, &short(a.name(), 9), value, theme::attr_tint(*a, accent), set) {
                                            Fader::Set(v) => acts.push(Action::SetAttr(*a, v)),
                                            Fader::Reset => acts.push(Action::ReleaseAttr(*a)),
                                            Fader::None => {}
                                        }
                                    }
                                });
                                if kind == Kind::Position && attrs.contains(&Attr::Pan) && attrs.contains(&Attr::Tilt) {
                                    if let Some((x, y)) = xy_pad(ui, get(Attr::Pan).0, get(Attr::Tilt).0, accent) {
                                        acts.push(Action::SetAttr(Attr::Pan, x));
                                        acts.push(Action::SetAttr(Attr::Tilt, y));
                                    }
                                }
                            });
                        });
                    }
                });
                ui.add_space(8.0);
            }
        });
    }

    /// Rechte Seite: Paletten und Effekte.
    pub fn pult_library(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let has_sel = !e.selection.is_empty();
        egui::ScrollArea::vertical().id_salt("library").show(ui, |ui| {
            ui.set_width(ui.available_width());
            section(ui, "Paletten", "Eine Palette speichert eine Einstellung (z. B. die Farbe „Rot“ oder eine Position). Ein Klick wendet sie auf die aktuelle Auswahl an. So legst du eine an: Fixtures wählen, Wert einstellen, „Speichern“. Rechtsklick = umbenennen oder löschen.");
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
                if ui.add_enabled(can, theme::icon_text("plus", "Speichern", theme::TEXT)).on_hover_text("Die eingestellten Werte dieser Art als Palette sichern").clicked() {
                    acts.push(Action::RecordPalette(self.pal_kind));
                }
            });
            ui.add_space(10.0);
            ui.separator();

            section(ui, "Effekte", "Effekte bewegen Werte automatisch: pulsierende Helligkeit, Lauflicht, Regenbogen oder kreisende Bewegung. Sie laufen auf den gewählten Fixtures – wähle diese zuerst aus.");
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
            for (i, fx) in e.show.effects.iter().enumerate() {
                let mut n = fx.clone();
                let mut changed = false;
                ui.add_space(4.0);
                egui::Frame::new().fill(theme::RAISED).corner_radius(4).inner_margin(8).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if icon_btn(ui, if fx.running { "stop" } else { "play" }, "Starten/Stoppen").clicked() {
                            acts.push(Action::ToggleEffect(i));
                        }
                        ui.label(RichText::new(short(&fx.name, 16)).strong().color(if fx.running { accent } else { theme::TEXT }));
                        hint(ui, &format!("{} Fixtures", fx.ids.len()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if icon_btn(ui, "x", "Effekt löschen").clicked() {
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
