//! Pult: Pad-Raster für Fixtures/Gruppen, darunter Attribut-Reiter mit Reglern, Paletten und Effekten.

use crate::theme::{self, Fader, fader, hint, section};
use crate::{App, rgb_of, short};
use eframe::egui::{self, Color32, RichText};
use lichtpult::engine::{Action, Engine, Shape, Target};
use lichtpult::fixture::{Attr, Fixture, Kind};

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

/// Reihenfolge der Attribut-Reiter wie bei Onyx: A Intensität, B Position, C Farbe, D Beam.
const TABS: [(&str, Kind); 4] = [("A", Kind::Intensity), ("B", Kind::Position), ("C", Kind::Color), ("D", Kind::Beam)];
const PAD: egui::Vec2 = egui::vec2(112.0, 72.0);

/// Live-Farbe und -Pegel eines Fixtures für den Pad-Streifen.
fn live(e: &Engine, f: &Fixture) -> (Color32, f32) {
    let out = |a| e.output_of(f, a);
    let c = rgb_of(out);
    let mx = c.r().max(c.g()).max(c.b());
    let has_color = [Attr::Red, Attr::Green, Attr::Blue, Attr::White].iter().any(|a| e.show.has(f, *a));
    let level = if e.show.has(f, Attr::Dimmer) { out(Attr::Dimmer) } else { mx };
    let hue = if has_color && mx > 0 {
        let k = 255.0 / mx as f32;
        Color32::from_rgb((c.r() as f32 * k) as u8, (c.g() as f32 * k) as u8, (c.b() as f32 * k) as u8)
    } else if level > 0 {
        Color32::from_rgb(235, 235, 240)
    } else {
        theme::LINE
    };
    (hue, level as f32 / 255.0)
}

impl App {
    /// Mitte: Fixtures und Gruppen als Pad-Raster mit Werkzeugleiste.
    pub fn pult(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let has_sel = !e.selection.is_empty();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            let tb = |ui: &mut egui::Ui, icon: &str, text: &str, on: bool, tip: &str| ui.add_enabled(on, theme::icon_text(icon, text, theme::TEXT).min_size(egui::vec2(0.0, 32.0))).on_hover_text(tip).clicked();
            if tb(ui, "grid", "Alle", !e.show.fixtures.is_empty(), "Alle Fixtures auswählen") {
                acts.push(Action::SelectAll);
            }
            if tb(ui, "x", "Keine", has_sel, "Auswahl aufheben") {
                acts.push(Action::ClearSelection);
            }
            if tb(ui, "grid", "Invert", !e.show.fixtures.is_empty(), "Auswahl umkehren (Onyx: /)") {
                acts.push(Action::InvertSelection);
            }
            if tb(ui, "grid", "Jedes 2.", e.selection.len() > 1, "Nur jedes zweite Fixture der Auswahl behalten (Every 2)") {
                acts.push(Action::SelectEvery(2, 0));
            }
            if tb(ui, "grid", "Gegenreihe", e.selection.len() > 1, "Jedes zweite Fixture, versetzt (Every 2, Offset 1)") {
                acts.push(Action::SelectEvery(2, 1));
            }
            if tb(ui, "x", "◀ Last", !e.show.fixtures.is_empty(), "Vorheriges Fixture wählen (PgDn)") {
                acts.push(Action::SelectStep(-1));
            }
            if tb(ui, "x", "Next ▶", !e.show.fixtures.is_empty(), "Nächstes Fixture wählen (PgUp)") {
                acts.push(Action::SelectStep(1));
            }
            let blind = if e.blind { "Blind: AN" } else { "Blind" };
            if tb(ui, "x", blind, true, "Blind/Preview: Programmer wird nicht ausgegeben (Strg+P)") {
                acts.push(Action::SetBlind(!e.blind));
            }
            if tb(ui, "x", "Unpark", !e.parked.is_empty(), "Alle geparkten Werte freigeben") {
                acts.push(Action::Unpark);
            }
            ui.add_space(8.0);
            theme::info(ui, "Fixtures wählen", "Klick wählt ein Fixture, Strg/Shift + Klick mehrere. Mit den Reitern unten stellst du die Auswahl ein. Der Streifen eines Pads zeigt Farbe und Helligkeit live.");
            {
                let store = theme::icon_text("plus", "Als Szene speichern", Color32::BLACK).fill(accent).min_size(egui::vec2(170.0, 32.0));
                if ui.add_enabled(!e.show.fixtures.is_empty(), store).on_hover_text("Hält den aktuellen Look fest. Abrufen: Pad in der Playback-Leiste unten oder Tab „Szenen“.").clicked() {
                    acts.push(Action::StoreScene);
                }
                if ui.add_enabled(!e.programmer.is_empty(), egui::Button::new("Programmer leeren").min_size(egui::vec2(0.0, 32.0))).on_hover_text("Alle manuellen Werte verwerfen").clicked() {
                    acts.push(Action::ClearProgrammer);
                }
            }
        });
        ui.add_space(6.0);
        egui::ScrollArea::vertical().id_salt("pads").auto_shrink([false, false]).show(ui, |ui| {
            ui.set_width(ui.available_width());
            section(ui, "Fixtures", "Die Lampen deiner Show. Pads leuchten grün umrandet, wenn sie ausgewählt sind.");
            let conflicts = e.conflicts();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                for f in &e.show.fixtures {
                    let (hue, lvl) = live(e, f);
                    let sel = e.selection.contains(&f.id);
                    let r = theme::pad(ui, PAD, &f.id.to_string(), &short(&f.name, 13), if conflicts.contains(&f.id) { theme::DANGER } else { hue }, sel, if e.last_selected == Some(f.id) { theme::DANGER } else { accent }, Some(lvl));
                    if r.clicked() {
                        acts.push(if ui.input(|i| i.modifiers.command || i.modifiers.shift) { Action::Toggle(f.id) } else { Action::Select(f.id) });
                    }
                }
            });
            if e.show.fixtures.is_empty() {
                hint(ui, "Noch keine Fixtures – im Tab „Patch“ hinzufügen.");
            }
            ui.add_space(14.0);
            section(ui, "Gruppen", "Eine Gruppe merkt sich eine Auswahl (z. B. „Bühne vorne“). Klick wählt alle Mitglieder, Rechtsklick = umbenennen/überschreiben/löschen.");
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                for (i, g) in e.show.groups.iter().enumerate() {
                    let r = theme::pad(ui, PAD, &(i + 1).to_string(), &short(&g.name, 13), theme::kind_color(Kind::Position), false, accent, None);
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
                if theme::pad_add(ui, PAD, "Gruppe", has_sel).on_hover_text("Aktuelle Auswahl als Gruppe speichern").clicked() {
                    acts.push(Action::RecordGroup);
                }
            });
        });
    }

    /// Unteres Panel: Attribut-Reiter (A–D, E = Effekte) mit Reglern links und Paletten rechts.
    pub fn pult_attrs(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let sel: Vec<&Fixture> = e.show.fixtures.iter().filter(|f| e.selection.contains(&f.id)).collect();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (i, (letter, kind)) in TABS.iter().enumerate() {
                let on = !self.fx_tab && self.pal_kind == *kind;
                if theme::attr_tab(ui, letter, kind.name(), theme::kind_color(*kind), on).clicked() {
                    self.pal_kind = *kind;
                    self.fx_tab = false;
                }
                let _ = i;
            }
            if theme::attr_tab(ui, "E", "Effekte", Color32::from_rgb(240, 170, 40), self.fx_tab).clicked() {
                self.fx_tab = true;
            }
            ui.add_space(12.0);
            theme::info(ui, "Attribute", "Reiter wählen, was du an den ausgewählten Fixtures einstellst. Links die Regler (hell = dein Wert, gedimmt = aus einer Szene), rechts die gespeicherten Paletten dieser Art. Effekte (E) bewegen Werte automatisch.");
        });
        let top = ui.cursor().top();
        ui.painter().line_segment([egui::pos2(ui.max_rect().left(), top), egui::pos2(ui.max_rect().right(), top)], egui::Stroke::new(1.0, theme::LINE));
        ui.add_space(8.0);
        if self.fx_tab {
            self.effects_ui(ui, e, acts);
            return;
        }
        let attrs_of = |k: Kind| -> Vec<Attr> { Attr::ALL.into_iter().filter(|a| a.kind() == k && sel.iter().any(|f| e.show.has(f, *a))).collect() };
        if attrs_of(self.pal_kind).is_empty() {
            if let Some((_, k)) = TABS.iter().find(|(_, k)| !attrs_of(*k).is_empty()) {
                self.pal_kind = *k;
            }
        }
        let kind = self.pal_kind;
        let attrs = attrs_of(kind);
        let faders_w = (attrs.len() as f32 * (theme::FADER_W + 8.0)).max(150.0) + if kind == Kind::Position { 170.0 } else { 0.0 };
        let faders_w = faders_w.min(ui.available_width() * 0.55);
        ui.horizontal_top(|ui| {
            ui.allocate_ui(egui::vec2(faders_w, ui.available_height()), |ui| {
                if sel.is_empty() {
                    hint(ui, "Fixtures oben auswählen, um sie einzustellen.");
                } else if attrs.is_empty() {
                    hint(ui, "Die gewählten Fixtures haben keine Attribute dieser Art.");
                } else {
                    self.kind_faders(ui, e, &sel, kind, &attrs, acts);
                }
            });
            ui.separator();
            self.palette_pads(ui, e, acts);
        });
    }

    fn kind_faders(&self, ui: &mut egui::Ui, e: &Engine, sel: &[&Fixture], kind: Kind, attrs: &[Attr], acts: &mut Vec<Action>) {
        let accent = self.accent();
        let play = e.playback_values();
        let get = |a: Attr| -> (u8, bool) {
            let vals: Vec<(u8, bool)> = sel.iter().filter(|f| e.show.has(f, a)).map(|f| e.value_of(&play, f.id, a)).collect();
            (vals.first().map_or(0, |v| v.0), vals.iter().any(|v| v.1))
        };
        egui::ScrollArea::horizontal().id_salt("faders").show(ui, |ui| {
            ui.add_space(6.0);
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
                if kind == Kind::Position && attrs.contains(&Attr::Pan) && attrs.contains(&Attr::Tilt) {
                    ui.add_space(8.0);
                    if let Some((x, y)) = xy_pad(ui, get(Attr::Pan).0, get(Attr::Tilt).0, accent) {
                        acts.push(Action::SetAttr(Attr::Pan, x));
                        acts.push(Action::SetAttr(Attr::Tilt, y));
                    }
                }
            });
        });
    }

    /// Paletten der aktiven Art als Pads, bei Farbe mit Schnellfarben und Farbwähler.
    fn palette_pads(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let kind = self.pal_kind;
        egui::ScrollArea::vertical().id_salt("palettes").auto_shrink([false, false]).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                if kind == Kind::Color {
                    for (name, c) in QUICK {
                        let col = Color32::from_rgb(c[0], c[1], c[2]);
                        if theme::pad(ui, egui::vec2(64.0, 66.0), "", name, col, false, accent, None).clicked() {
                            for (a, v) in [(Attr::Red, c[0]), (Attr::Green, c[1]), (Attr::Blue, c[2])] {
                                acts.push(Action::SetAttr(a, v));
                            }
                        }
                    }
                    let play = e.playback_values();
                    let sel: Vec<&Fixture> = e.show.fixtures.iter().filter(|f| e.selection.contains(&f.id)).collect();
                    let cur = |a: Attr| sel.iter().find(|f| e.show.has(f, a)).map_or(0, |f| e.value_of(&play, f.id, a).0);
                    let mut rgb = [cur(Attr::Red), cur(Attr::Green), cur(Attr::Blue)];
                    ui.add_enabled_ui(!sel.is_empty(), |ui| {
                        ui.spacing_mut().interact_size = egui::vec2(64.0, 66.0);
                        if ui.color_edit_button_srgb(&mut rgb).on_hover_text("Eigene Farbe wählen").changed() {
                            for (a, v) in [(Attr::Red, rgb[0]), (Attr::Green, rgb[1]), (Attr::Blue, rgb[2])] {
                                acts.push(Action::SetAttr(a, v));
                            }
                        }
                    });
                    ui.add_space(8.0);
                }
                for (i, p) in e.show.palettes.iter().enumerate().filter(|(_, p)| p.kind == kind) {
                    let strip = if kind == Kind::Color { rgb_of(|a| p.values.get(&a).copied().unwrap_or(0)) } else { theme::kind_color(kind) };
                    let r = theme::pad(ui, PAD, &(i + 1).to_string(), &short(&p.name, 13), strip, false, accent, None);
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
                if theme::pad_add(ui, PAD, "Palette", !e.programmer.is_empty()).on_hover_text("Aktuelle Programmer-Werte dieser Art als Palette speichern. Rechtsklick auf eine Palette: umbenennen/löschen.").clicked() {
                    acts.push(Action::RecordPalette(kind));
                }
            });
        });
    }

    /// Reiter E: Effekte starten, einstellen, stoppen.
    fn effects_ui(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let has_sel = !e.selection.is_empty();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(300.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                    for (name, t, s) in [
                        ("Pulsieren", Target::Intensity, Shape::Sine),
                        ("Lauflicht", Target::Intensity, Shape::Square),
                        ("Regenbogen", Target::Color, Shape::Sine),
                        ("Kreis", Target::Position, Shape::Sine),
                        ("Rampe", Target::Intensity, Shape::RampDown),
                        ("Stufen", Target::Intensity, Shape::Step),
                        ("Bounce", Target::Intensity, Shape::Bounce),
                    ] {
                        let strip = match t {
                            Target::Intensity => theme::kind_color(Kind::Intensity),
                            Target::Color => theme::kind_color(Kind::Color),
                            _ => theme::kind_color(Kind::Position),
                        };
                        let r = theme::pad(ui, PAD, "", name, strip, false, accent, None).on_hover_text("Neuen Effekt für die Auswahl starten (Fixtures vorher wählen)");
                        if r.clicked() && has_sel {
                            acts.push(Action::AddEffect(t, s));
                        }
                    }
                });
                if e.show.effects.iter().any(|f| f.running) && theme::button(ui, "Alle Effekte stoppen").clicked() {
                    acts.push(Action::StopEffects);
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt("fxlist").auto_shrink([false, false]).show(ui, |ui| {
                if e.show.effects.is_empty() {
                    hint(ui, "Noch keine Effekte. Fixtures wählen, dann links einen Effekt antippen.");
                }
                for (i, fx) in e.show.effects.iter().enumerate() {
                    let mut n = fx.clone();
                    let mut changed = false;
                    egui::Frame::new().fill(theme::RAISED).inner_margin(6).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if theme::icon_btn(ui, if fx.running { "stop" } else { "play" }, "Starten/Stoppen").clicked() {
                                acts.push(Action::ToggleEffect(i));
                            }
                            ui.label(RichText::new(short(&fx.name, 16)).strong().color(if fx.running { accent } else { theme::TEXT }));
                            hint(ui, &format!("{} Fixtures", fx.ids.len()));
                            egui::ComboBox::from_id_salt(("fxshape", i)).selected_text(fx.shape.name()).width(90.0).show_ui(ui, |ui| {
                                for sh in Shape::ALL {
                                    changed |= ui.selectable_value(&mut n.shape, sh, sh.name()).changed();
                                }
                            });
                            let mut bpm = n.rate * 60.0;
                            if ui.add(egui::DragValue::new(&mut bpm).speed(0.5).suffix(" BPM").range(1.0..=1200.0)).on_hover_text("Geschwindigkeit (Zyklen pro Minute)").changed() {
                                n.rate = bpm / 60.0;
                                changed = true;
                            }
                            changed |= ui.add(egui::DragValue::new(&mut n.size).prefix("Stärke ")).changed();
                            changed |= ui.add(egui::DragValue::new(&mut n.spread).speed(0.01).prefix("Versatz ").range(0.0..=1.0)).on_hover_text("Phasenversatz zwischen den Fixtures").changed();
                            if theme::icon_btn(ui, "x", "Effekt löschen").clicked() {
                                acts.push(Action::DeleteEffect(i));
                            }
                        });
                    });
                    if changed {
                        acts.push(Action::SetEffect(i, n));
                    }
                }
            });
        });
    }
}
