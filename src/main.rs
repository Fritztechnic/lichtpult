mod theme;

use eframe::egui::{self, Color32, RichText};
use lichtpult::engine::{Action, Engine, Show};
use lichtpult::fixture::{Attr, Fixture, FixtureType, Kind, builtin_types, import_ofl};
use lichtpult::{Shared, json_files, load_library, load_show, save_library, save_show, spawn_engine};
use std::sync::{Arc, Mutex};
use theme::{Fader, caption, card, fader, tile};

#[derive(PartialEq, Clone, Copy)]
enum View {
    Live,
    Stage,
    Patch,
    Output,
}

#[derive(PartialEq, Clone, Copy)]
enum Side {
    Cues,
    Effects,
}

struct App {
    engine: Shared,
    view: View,
    side: Side,
    cmd: String,
    cmd_error: bool,
    fx_pick: (lichtpult::engine::Target, lichtpult::engine::Shape),
    accent: [u8; 3],
    saved_accent: [u8; 3],
    status: String,
    patch: (usize, u16, u16, u16),
    library: Vec<FixtureType>,
    search: String,
    import_path: String,
    out_universe: u16,
    pal_kind: Kind,
    pending_load: Option<Show>,
    frames: u32,
}

fn accent_path() -> std::path::PathBuf {
    lichtpult::show_path().with_file_name("accent")
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.into() } else { format!("{}…", s.chars().take(n - 1).collect::<String>()) }
}

fn rgb_of(get: impl Fn(Attr) -> u8) -> Color32 {
    let (r, g, b, w) = (get(Attr::Red), get(Attr::Green), get(Attr::Blue), get(Attr::White));
    Color32::from_rgb(r.saturating_add(w), g.saturating_add(w), b.saturating_add(w))
}

/// Kachel für ein Fixture mit Live-Anzeige von Farbe und Intensität.
fn fixture_tile(ui: &mut egui::Ui, e: &Engine, f: &Fixture, selected: bool, bad: bool, accent: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(108.0, 58.0), egui::Sense::click());
    let p = ui.painter();
    let fill = if selected {
        accent.gamma_multiply(0.16)
    } else if resp.hovered() {
        Color32::from_rgb(32, 32, 37)
    } else {
        theme::RAISED
    };
    let line = if bad { theme::DANGER } else if selected { accent } else { theme::LINE };
    p.rect(rect, 6.0, fill, egui::Stroke::new(if selected || bad { 1.5 } else { 1.0 }, line), egui::StrokeKind::Inside);
    p.text(
        rect.left_top() + egui::vec2(8.0, 6.0),
        egui::Align2::LEFT_TOP,
        f.id.to_string(),
        egui::FontId::proportional(15.0),
        if selected { accent } else { theme::TEXT },
    );
    p.text(
        rect.left_top() + egui::vec2(8.0, 26.0),
        egui::Align2::LEFT_TOP,
        short(&f.name, 15),
        egui::FontId::proportional(11.0),
        theme::MUTED,
    );

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
    let bar = egui::Rect::from_min_size(rect.left_bottom() + egui::vec2(8.0, -11.0), egui::vec2(rect.width() - 16.0, 4.0));
    p.rect_filled(bar, 2.0, theme::LINE);
    let mut lit = bar;
    lit.set_width(bar.width() * level as f32 / 255.0);
    p.rect_filled(lit, 2.0, hue);
    resp
}

impl App {
    fn new(engine: Shared) -> Self {
        let accent = std::fs::read(accent_path())
            .ok()
            .and_then(|b| <[u8; 3]>::try_from(b).ok())
            .unwrap_or(theme::PRESETS[0]);
        App {
            engine,
            view: match std::env::var("LICHTPULT_VIEW").as_deref() {
                Ok("patch") => View::Patch,
                Ok("output") => View::Output,
                Ok("stage") => View::Stage,
                _ => View::Live,
            },
            side: if std::env::var("LICHTPULT_SIDE").as_deref() == Ok("fx") { Side::Effects } else { Side::Cues },
            cmd: String::new(),
            cmd_error: false,
            fx_pick: (lichtpult::engine::Target::Intensity, lichtpult::engine::Shape::Sine),
            accent,
            saved_accent: accent,
            status: String::new(),
            patch: (1, 1, 1, 1),
            library: [builtin_types(), load_library()].concat(),
            search: String::new(),
            import_path: String::new(),
            out_universe: 1,
            pal_kind: Kind::Color,
            pending_load: None,
            frames: 0,
        }
    }

    fn accent(&self) -> Color32 {
        Color32::from_rgb(self.accent[0], self.accent[1], self.accent[2])
    }

    /// Entwicklungshilfe: `LICHTPULT_SHOT=datei.ppm [LICHTPULT_SELECT=1]` speichert
    /// den Fensterinhalt (unabhängig von anderen Fenstern) und beendet die App.
    fn dev_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = std::env::var_os("LICHTPULT_SHOT") else { return };
        self.frames += 1;
        if self.frames == 3 && std::env::var_os("LICHTPULT_SELECT").is_some() {
            let mut e = self.engine.lock().unwrap();
            e.apply(Action::Go(0));
            e.apply(Action::Select(3));
            e.apply(Action::Toggle(4));
            e.apply(Action::SetAttr(Attr::Green, 180));
            if std::env::var("LICHTPULT_SIDE").as_deref() == Ok("fx") {
                e.apply(Action::SelectAll);
                e.apply(Action::AddEffect(lichtpult::engine::Target::Color, lichtpult::engine::Shape::Sine));
                e.apply(Action::Select(3));
            }
        }
        if self.frames == 8 {
            let ctx2 = ctx.clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |img| {
                let mut out = format!("P6\n{} {}\n255\n", img.width(), img.height()).into_bytes();
                out.extend(img.pixels.iter().flat_map(|p| [p.r(), p.g(), p.b()]));
                let _ = std::fs::write(path, out);
                ctx2.send_viewport_cmd(egui::ViewportCommand::Close);
            })));
        }
    }

    /// Importiert OFL-Dateien oder Ordner in die Bibliothek.
    fn import(&mut self, paths: Vec<std::path::PathBuf>) {
        let (mut added, mut failed) = (0, 0);
        for file in paths.iter().flat_map(|p| json_files(p)) {
            match std::fs::read_to_string(&file).map_err(|e| e.to_string()).and_then(|s| import_ofl(&s)) {
                Ok(types) => {
                    for t in types {
                        if !self.library.iter().any(|x| x.name == t.name) {
                            self.library.push(t);
                            added += 1;
                        }
                    }
                }
                Err(_) => failed += 1,
            }
        }
        let user = &self.library[builtin_types().len()..];
        self.status = match save_library(user) {
            Ok(()) => format!("{added} Typen importiert, {failed} Dateien übersprungen"),
            Err(err) => format!("Bibliothek nicht gespeichert: {err}"),
        };
    }

    fn top_bar(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        ui.horizontal_centered(|ui| {
            ui.label(RichText::new("LICHTPULT").heading().strong().color(self.accent()));
            ui.add_space(16.0);
            for (v, name) in [(View::Live, "Live"), (View::Stage, "Stage"), (View::Patch, "Patch"), (View::Output, "Output")] {
                if tile(ui, name, self.view == v, self.accent()).clicked() {
                    self.view = v;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.settings(ui, e);
                let (mut m, mut black) = (e.master, e.blackout);
                if tile(ui, "Blackout", black, theme::DANGER).clicked() {
                    black = !black;
                    acts.push(Action::Blackout(black));
                }
                ui.spacing_mut().slider_width = 120.0;
                if ui.add(egui::Slider::new(&mut m, 0..=255).text("Master")).changed() {
                    acts.push(Action::Master(m));
                }
                if ui.add_enabled(e.can_redo(), egui::Button::new("↷")).on_hover_text("Wiederholen (Strg+Shift+Z)").clicked() {
                    acts.push(Action::Redo);
                }
                if ui.add_enabled(e.can_undo(), egui::Button::new("↶")).on_hover_text("Rückgängig (Strg+Z)").clicked() {
                    acts.push(Action::Undo);
                }
            });
        });
    }

    fn settings(&mut self, ui: &mut egui::Ui, e: &Engine) {
        ui.menu_button("⚙", |ui| {
            caption(ui, "Akzentfarbe");
            ui.horizontal(|ui| {
                for p in theme::PRESETS {
                    let c = Color32::from_rgb(p[0], p[1], p[2]);
                    if ui.add(egui::Button::new("").fill(c).min_size(egui::vec2(24.0, 24.0))).clicked() {
                        self.accent = p;
                    }
                }
            });
            ui.color_edit_button_srgb(&mut self.accent);
            ui.separator();
            if ui.button("Show speichern").clicked() {
                self.status = match save_show(&e.show) {
                    Ok(()) => "Show gespeichert".into(),
                    Err(err) => format!("Speichern fehlgeschlagen: {err}"),
                };
            }
            if ui.button("Show laden").clicked() {
                match load_show() {
                    Ok(s) => {
                        self.pending_load = Some(s);
                        self.status = "Show geladen".into();
                    }
                    Err(err) => self.status = format!("Laden fehlgeschlagen: {err}"),
                }
            }
        });
    }

    /// Linke Seite: Gruppen und Paletten.
    fn library_panel(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let has_sel = !e.selection.is_empty();
        caption(ui, "Gruppen");
        ui.horizontal_wrapped(|ui| {
            for (i, g) in e.show.groups.iter().enumerate() {
                let r = tile(ui, short(&g.name, 12), false, accent);
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
        });
        if ui.add_enabled(has_sel, egui::Button::new("+ Gruppe aus Auswahl")).clicked() {
            acts.push(Action::RecordGroup);
        }
        ui.add_space(8.0);
        ui.separator();

        caption(ui, "Paletten");
        ui.horizontal_wrapped(|ui| {
            for k in Kind::ALL {
                ui.selectable_value(&mut self.pal_kind, k, k.name());
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (i, p) in e.show.palettes.iter().enumerate().filter(|(_, p)| p.kind == self.pal_kind) {
                let r = tile(ui, short(&p.name, 12), false, accent);
                if p.kind == Kind::Color {
                    let c = rgb_of(|a| p.values.get(&a).copied().unwrap_or(0));
                    let bar = egui::Rect::from_min_size(r.rect.left_bottom() + egui::vec2(6.0, -7.0), egui::vec2(r.rect.width() - 12.0, 3.0));
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
        });
        if ui.add_enabled(has_sel, egui::Button::new("+ Palette aus Programmer")).clicked() {
            acts.push(Action::RecordPalette(self.pal_kind));
        }
        ui.add_space(8.0);
        ui.label(RichText::new("Rechtsklick auf Kachel: umbenennen, aktualisieren, löschen").small().color(theme::MUTED));
    }

    fn live(&self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        ui.horizontal(|ui| {
            caption(ui, "Fixtures");
            ui.label(RichText::new(format!("{} ausgewählt", e.selection.len())).color(theme::MUTED));
        });
        ui.horizontal(|ui| {
            if ui.button("Alle").clicked() {
                acts.push(Action::SelectAll);
            }
            if ui.button("Keine").clicked() {
                acts.push(Action::ClearSelection);
            }
            if ui.add_enabled(!e.programmer.is_empty(), egui::Button::new("Programmer leeren")).clicked() {
                acts.push(Action::ClearProgrammer);
            }
        });
        let conflicts = e.conflicts();
        egui::ScrollArea::vertical().id_salt("fixtures").max_height((ui.available_height() * 0.4).max(120.0)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for f in &e.show.fixtures {
                    let r = fixture_tile(ui, e, f, e.selection.contains(&f.id), conflicts.contains(&f.id), accent);
                    if r.clicked() {
                        acts.push(if ui.input(|i| i.modifiers.command || i.modifiers.shift) {
                            Action::Toggle(f.id)
                        } else {
                            Action::Select(f.id)
                        });
                    }
                }
            });
            if e.show.fixtures.is_empty() {
                ui.label(RichText::new("Noch keine Fixtures – im Patch-Tab hinzufügen.").color(theme::MUTED));
            }
        });
        ui.add_space(4.0);
        ui.separator();

        let sel: Vec<&Fixture> = e.show.fixtures.iter().filter(|f| e.selection.contains(&f.id)).collect();
        if sel.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Fixtures oder eine Gruppe auswählen").heading().color(theme::MUTED));
                ui.label(RichText::new("Strg/Shift + Klick für Mehrfachauswahl · Esc hebt die Auswahl auf").color(theme::MUTED));
            });
            return;
        }
        let play = e.playback_values();
        egui::ScrollArea::both().id_salt("encoders").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for kind in Kind::ALL {
                    let attrs: Vec<Attr> = Attr::ALL
                        .into_iter()
                        .filter(|a| a.kind() == kind && sel.iter().any(|f| e.show.has(f, *a)))
                        .collect();
                    if attrs.is_empty() {
                        continue;
                    }
                    card(ui, kind.name(), |ui| {
                        ui.horizontal_top(|ui| {
                            for a in &attrs {
                                let vals: Vec<(u8, bool)> =
                                    sel.iter().filter(|f| e.show.has(f, *a)).map(|f| e.value_of(&play, f.id, *a)).collect();
                                let value = vals[0].0;
                                let set = vals.iter().any(|v| v.1);
                                match fader(ui, a.name(), value, theme::attr_tint(*a, accent), set) {
                                    Fader::Set(v) => acts.push(Action::SetAttr(*a, v)),
                                    Fader::Reset => acts.push(Action::ReleaseAttr(*a)),
                                    Fader::None => {}
                                }
                            }
                            if kind == Kind::Color {
                                let first = sel[0];
                                let c = rgb_of(|a| if e.show.has(first, a) { e.value_of(&play, first.id, a).0 } else { 0 });
                                let (rect, _) = ui.allocate_exact_size(egui::vec2(58.0, 180.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 6.0, c);
                                ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(1.0, theme::LINE), egui::StrokeKind::Inside);
                            }
                        });
                    });
                }
            });
            ui.label(RichText::new("Fader ziehen zum Einstellen · Doppelklick gibt den Wert wieder frei").small().color(theme::MUTED));
        });
    }

    fn patch(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        card(ui, "Fixture-Bibliothek", |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Suchen…").desired_width(220.0));
                ui.add(
                    egui::TextEdit::singleline(&mut self.import_path)
                        .hint_text("OFL-Datei/Ordner (oder hierher ziehen)")
                        .desired_width(320.0),
                );
                if ui.button("Importieren").clicked() {
                    let p = std::path::PathBuf::from(self.import_path.trim());
                    self.import(vec![p]);
                }
            });
            let q = self.search.to_lowercase();
            let hits: Vec<usize> =
                (0..self.library.len()).filter(|&i| self.library[i].name.to_lowercase().contains(&q)).collect();
            egui::ScrollArea::vertical().id_salt("lib").max_height(200.0).show(ui, |ui| {
                for &i in hits.iter().take(200) {
                    let ch = self.library[i].channels.len();
                    ui.selectable_value(&mut self.patch.0, i, format!("{}  ·  {ch} Kanäle", self.library[i].name));
                }
                if hits.len() > 200 {
                    ui.label(RichText::new(format!("… {} weitere – Suche eingrenzen", hits.len() - 200)).color(theme::MUTED));
                }
            });
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut self.patch.1).range(1..=512).prefix("Anzahl "));
                ui.add(egui::DragValue::new(&mut self.patch.2).range(1..=32768).prefix("Universum "));
                ui.add(egui::DragValue::new(&mut self.patch.3).range(1..=512).prefix("Adresse "));
                if ui.button("Patchen").clicked() {
                    let (lib, count, universe, address) = self.patch;
                    let ty = self.library[lib].clone();
                    let index = e.show.types.iter().position(|t| t.name == ty.name).unwrap_or(e.show.types.len());
                    let width = ty.channels.len() as u32;
                    acts.push(Action::AddTypes(vec![ty]));
                    acts.push(Action::AddFixtures { ty: index, count, universe, address });
                    self.patch.3 = (address as u32 + count as u32 * width).min(512) as u16;
                }
            });
        });
        ui.add_space(8.0);
        caption(ui, "Gepatchte Fixtures");
        let conflicts = e.conflicts();
        egui::ScrollArea::vertical().id_salt("patched").show(ui, |ui| {
            egui::Grid::new("patch_grid").striped(true).spacing([16.0, 6.0]).show(ui, |ui| {
                for h in ["ID", "Name", "Typ", "Universum", "Adresse", "Bereich", ""] {
                    ui.label(RichText::new(h).small().color(theme::MUTED));
                }
                ui.end_row();
                for f in &e.show.fixtures {
                    let bad = conflicts.contains(&f.id);
                    ui.label(f.id.to_string());
                    let mut name = f.name.clone();
                    if ui.add(egui::TextEdit::singleline(&mut name).desired_width(170.0)).changed() {
                        acts.push(Action::RenameFixture(f.id, name));
                    }
                    ui.label(RichText::new(short(&e.show.types[f.ty].name, 30)).color(theme::MUTED));
                    let (mut uni, mut adr) = (f.universe, f.address);
                    let a = ui.add(egui::DragValue::new(&mut uni).range(1..=32768)).changed();
                    let b = ui.add(egui::DragValue::new(&mut adr).range(1..=512)).changed();
                    if a || b {
                        acts.push(Action::SetPatch { id: f.id, universe: uni, address: adr });
                    }
                    let span = format!("{}–{}", f.address, e.show.range(f).end.saturating_sub(1));
                    ui.label(if bad { RichText::new(format!("{span}  Konflikt")).color(theme::DANGER) } else { RichText::new(span) });
                    if ui.small_button("✕").on_hover_text("Fixture entfernen").clicked() {
                        acts.push(Action::RemoveFixture(f.id));
                    }
                    ui.end_row();
                }
            });
        });
    }

    fn output(&mut self, ui: &mut egui::Ui, e: &Engine) {
        ui.horizontal(|ui| {
            caption(ui, "DMX-Ausgabe");
            ui.add(egui::DragValue::new(&mut self.out_universe).range(1..=32768).prefix("Universum "));
        });
        let empty = [0u8; 512];
        let frame = e.frames.get(&self.out_universe).unwrap_or(&empty);
        let accent = self.accent();
        for row in frame.chunks(32) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for &v in row {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(34.0, 22.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 3.0, theme::RAISED);
                    ui.painter().rect_filled(rect, 3.0, accent.gamma_multiply(v as f32 / 255.0));
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        v.to_string(),
                        egui::FontId::monospace(11.0),
                        if v > 140 { Color32::BLACK } else { theme::MUTED },
                    );
                }
            });
        }
    }

}

impl App {
    fn command_bar(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        ui.horizontal_centered(|ui| {
            ui.label(RichText::new("›").color(self.accent()).strong());
            let w = (ui.available_width() - 280.0).max(200.0);
            let r = ui.add_sized(
                [w, 26.0],
                egui::TextEdit::singleline(&mut self.cmd)
                    .id(egui::Id::new("cmdline"))
                    .hint_text("Befehl: 1 thru 8 @ 50 · red 255 · go · fx 1 · record cue   (Strg+L)"),
            );
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.cmd.trim().is_empty() {
                match lichtpult::cmd::parse(&self.cmd, e) {
                    Ok(a) => {
                        acts.extend(a);
                        self.cmd.clear();
                        self.cmd_error = false;
                        self.status = "OK".into();
                    }
                    Err(err) => {
                        self.cmd_error = true;
                        self.status = err;
                    }
                }
                r.request_focus();
            }
            let color = if self.cmd_error { theme::DANGER } else { theme::MUTED };
            ui.label(RichText::new(&self.status).color(color));
        });
    }

    fn playbacks(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                for (i, l) in e.show.lists.iter().enumerate() {
                    let p = &e.players[i];
                    let active = i == e.active;
                    ui.vertical(|ui| {
                        ui.set_width(150.0);
                        let name = RichText::new(short(&l.name, 16)).color(if active { accent } else { theme::TEXT });
                        if ui.add(egui::Label::new(name).sense(egui::Sense::click())).clicked() {
                            acts.push(Action::SelectList(i));
                        }
                        let cur = p.cue.and_then(|c| l.cues.get(c)).map_or("—", |c| c.name.as_str());
                        ui.label(RichText::new(short(cur, 20)).small().color(theme::MUTED));
                        ui.horizontal(|ui| {
                            if ui.button("◀").clicked() {
                                acts.push(Action::Back(i));
                            }
                            let go = egui::Button::new(RichText::new("GO").strong().color(Color32::BLACK)).fill(accent);
                            if ui.add_sized([48.0, 22.0], go).clicked() {
                                acts.push(Action::Go(i));
                            }
                            if ui.button("■").on_hover_text("Freigeben").clicked() {
                                acts.push(Action::ReleaseList(i));
                            }
                        });
                        let mut lv = p.level;
                        ui.spacing_mut().slider_width = 120.0;
                        if ui.add(egui::Slider::new(&mut lv, 0..=255).show_value(false)).changed() {
                            acts.push(Action::SetLevel(i, lv));
                        }
                    });
                    ui.separator();
                }
            });
        });
    }

    fn side_panel(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        ui.horizontal(|ui| {
            for (s, n) in [(Side::Cues, "Cues"), (Side::Effects, "Effekte")] {
                if tile(ui, n, self.side == s, self.accent()).clicked() {
                    self.side = s;
                }
            }
        });
        ui.separator();
        match self.side {
            Side::Cues => self.cues(ui, e, acts),
            Side::Effects => self.effects(ui, e, acts),
        }
    }

    fn cues(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        ui.horizontal_wrapped(|ui| {
            for (i, l) in e.show.lists.iter().enumerate() {
                if tile(ui, short(&l.name, 12), i == e.active, accent).clicked() {
                    acts.push(Action::SelectList(i));
                }
            }
            if ui.button("+").on_hover_text("Neue Cueliste").clicked() {
                acts.push(Action::AddList);
            }
        });
        let list = e.list();
        ui.horizontal(|ui| {
            let mut name = list.name.clone();
            if ui.add(egui::TextEdit::singleline(&mut name).desired_width(110.0)).changed() {
                acts.push(Action::RenameList(e.active, name));
            }
            let mut lo = list.looped;
            if ui.checkbox(&mut lo, "Loop").changed() {
                acts.push(Action::SetLooped(e.active, lo));
            }
            if e.show.lists.len() > 1 && ui.button("🗑").on_hover_text("Liste löschen").clicked() {
                acts.push(Action::DeleteList(e.active));
            }
        });
        ui.horizontal(|ui| {
            let cur = e.cue();
            if ui.add_enabled(!e.programmer.is_empty(), egui::Button::new("● Aufnehmen")).clicked() {
                acts.push(Action::RecordCue);
            }
            if ui.add_enabled(cur.is_some() && !e.programmer.is_empty(), egui::Button::new("Aktualisieren")).clicked() {
                acts.push(Action::UpdateCue(cur.unwrap_or(0)));
            }
        });
        ui.add_space(4.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, c) in list.cues.iter().enumerate() {
                let current = e.cue() == Some(i);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(if current { "▶" } else { " " }).color(accent));
                    let r = ui.add_sized(
                        [110.0, 28.0],
                        egui::Button::new(RichText::new(short(&c.name, 14)).color(if current { accent } else { theme::TEXT }))
                            .fill(if current { accent.gamma_multiply(0.16) } else { theme::RAISED })
                            .stroke(egui::Stroke::new(1.0, if current { accent } else { theme::LINE })),
                    );
                    if r.clicked() {
                        acts.push(Action::GotoCue(i));
                    }
                    r.context_menu(|ui| {
                        let mut name = c.name.clone();
                        if ui.text_edit_singleline(&mut name).changed() {
                            acts.push(Action::RenameCue(i, name));
                        }
                        if ui.add_enabled(!e.programmer.is_empty(), egui::Button::new("Mit Programmer aktualisieren")).clicked() {
                            acts.push(Action::UpdateCue(i));
                            ui.close();
                        }
                        if ui.button("Löschen").clicked() {
                            acts.push(Action::DeleteCue(i));
                            ui.close();
                        }
                    });
                    let mut fade = c.fade;
                    if ui.add(egui::DragValue::new(&mut fade).speed(0.1).suffix(" s").range(0.0..=600.0)).on_hover_text("Fade-Zeit").changed() {
                        acts.push(Action::SetCueFade(i, fade));
                    }
                    let mut follow = c.follow.is_some();
                    if ui.checkbox(&mut follow, "").on_hover_text("Automatisch weiter (Follow)").changed() {
                        acts.push(Action::SetCueFollow(i, follow.then_some(2.0)));
                    }
                    if let Some(mut f) = c.follow {
                        if ui.add(egui::DragValue::new(&mut f).speed(0.1).suffix(" s").range(0.0..=600.0)).changed() {
                            acts.push(Action::SetCueFollow(i, Some(f)));
                        }
                    }
                });
            }
            if list.cues.is_empty() {
                ui.label(RichText::new("Noch keine Cues. Werte einstellen und „Aufnehmen“ drücken.").color(theme::MUTED));
            }
        });
    }

    fn effects(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        use lichtpult::engine::{Shape, Target};
        let accent = self.accent();
        caption(ui, "Neuer Effekt aus Auswahl");
        ui.horizontal_wrapped(|ui| {
            for (t, n) in [(Target::Intensity, "Intensität"), (Target::Color, "Farbe"), (Target::Position, "Position")] {
                if tile(ui, n, self.fx_pick.0 == t, accent).clicked() {
                    self.fx_pick.0 = t;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (s, n) in [(Shape::Sine, "Sinus"), (Shape::Triangle, "Dreieck"), (Shape::Saw, "Säge"), (Shape::Square, "Rechteck")] {
                if tile(ui, n, self.fx_pick.1 == s, accent).clicked() {
                    self.fx_pick.1 = s;
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.add_enabled(!e.selection.is_empty(), egui::Button::new("＋ Effekt")).clicked() {
                acts.push(Action::AddEffect(self.fx_pick.0, self.fx_pick.1));
            }
            if ui.button("■ Alle stoppen").clicked() {
                acts.push(Action::StopEffects);
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, fx) in e.show.effects.iter().enumerate() {
                let mut n = fx.clone();
                let mut changed = false;
                ui.horizontal(|ui| {
                    if tile(ui, if fx.running { "■" } else { "▶" }, fx.running, accent).clicked() {
                        acts.push(Action::ToggleEffect(i));
                    }
                    ui.label(RichText::new(short(&fx.name, 16)).color(if fx.running { accent } else { theme::TEXT }));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🗑").clicked() {
                            acts.push(Action::DeleteEffect(i));
                        }
                    });
                });
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut n.rate).speed(0.02).suffix(" Hz").range(0.01..=20.0)).changed();
                    changed |= ui.add(egui::DragValue::new(&mut n.size).suffix(" Stärke")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut n.spread).speed(0.01).prefix("Versatz ").range(0.0..=1.0)).changed();
                });
                if changed {
                    acts.push(Action::SetEffect(i, n));
                }
                ui.add_space(6.0);
            }
            if e.show.effects.is_empty() {
                ui.label(RichText::new("Fixtures wählen, Typ und Form wählen, „Effekt“ drücken.").color(theme::MUTED));
            }
        });
    }

    fn stage(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
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

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        theme::apply(ui.ctx(), self.accent());
        if self.accent != self.saved_accent {
            self.saved_accent = self.accent;
            let _ = std::fs::write(accent_path(), self.accent);
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        self.dev_screenshot(ui.ctx());

        let dropped: Vec<_> = ui.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        if !dropped.is_empty() {
            self.import(dropped);
            self.view = View::Patch;
        }

        let engine = self.engine.clone();
        let mut e = engine.lock().unwrap();
        let mut acts = Vec::new();

        if !ui.ctx().egui_wants_keyboard_input() {
            let active = e.active;
            ui.input(|i| {
                if i.key_pressed(egui::Key::Space) {
                    acts.push(Action::Go(active));
                }
                if i.key_pressed(egui::Key::Backspace) {
                    acts.push(Action::Back(active));
                }
                if i.key_pressed(egui::Key::Escape) {
                    acts.push(Action::ClearSelection);
                }
                if i.key_pressed(egui::Key::A) && i.modifiers.command {
                    acts.push(Action::SelectAll);
                }
                if i.key_pressed(egui::Key::Z) && i.modifiers.command {
                    acts.push(if i.modifiers.shift { Action::Redo } else { Action::Undo });
                }
                if i.key_pressed(egui::Key::Y) && i.modifiers.command {
                    acts.push(Action::Redo);
                }
            });
        }
        if ui.input(|i| i.key_pressed(egui::Key::L) && i.modifiers.command) {
            ui.memory_mut(|m| m.request_focus(egui::Id::new("cmdline")));
        }

        egui::Panel::top("top").exact_size(56.0).show(ui, |ui| self.top_bar(ui, &e, &mut acts));
        egui::Panel::bottom("cmdline_bar").exact_size(40.0).show(ui, |ui| self.command_bar(ui, &e, &mut acts));
        egui::Panel::bottom("playback").exact_size(124.0).show(ui, |ui| self.playbacks(ui, &e, &mut acts));
        if self.view == View::Live {
            egui::Panel::left("library").default_size(210.0).size_range(170.0..=360.0).show(ui, |ui| {
                self.library_panel(ui, &e, &mut acts);
            });
            egui::Panel::right("side").default_size(260.0).size_range(220.0..=460.0).show(ui, |ui| {
                self.side_panel(ui, &e, &mut acts);
            });
        }
        egui::CentralPanel::default().show(ui, |ui| match self.view {
            View::Live => self.live(ui, &e, &mut acts),
            View::Stage => self.stage(ui, &e, &mut acts),
            View::Patch => self.patch(ui, &e, &mut acts),
            View::Output => self.output(ui, &e),
        });

        for a in acts {
            e.apply(a);
        }
        if let Some(show) = self.pending_load.take() {
            e.load_show(show);
        }
    }
}

fn main() -> eframe::Result {
    let show = load_show().unwrap_or_else(|_| Show { name: "Neue Show".into(), ..Show::default() });
    let engine = Arc::new(Mutex::new(Engine::new(show)));
    spawn_engine(engine.clone(), "255.255.255.255:6454");
    eframe::run_native(
        "Lichtpult",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1400.0, 850.0])
                .with_min_inner_size([1000.0, 620.0]),
            ..Default::default()
        },
        Box::new(|_| Ok(Box::new(App::new(engine)))),
    )
}
