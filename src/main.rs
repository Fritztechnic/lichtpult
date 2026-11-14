mod theme;

use eframe::egui::{self, Color32, RichText};
use lichtpult::engine::{Action, Engine, Show};
use lichtpult::fixture::{Attr, FixtureType, Kind, builtin_types, import_ofl};
use lichtpult::{Shared, json_files, load_library, load_show, save_library, save_show, spawn_engine};
use std::sync::{Arc, Mutex};
use theme::{caption, tile};

#[derive(PartialEq, Clone, Copy)]
enum View {
    Programmer,
    Cues,
    Patch,
    Output,
}

struct App {
    engine: Shared,
    view: View,
    accent: [u8; 3],
    saved_accent: [u8; 3],
    status: String,
    patch: (usize, u16, u16, u16),
    library: Vec<FixtureType>,
    search: String,
    import_path: String,
    frames: u32,
    out_universe: u16,
    pending_load: Option<Show>,
}

fn accent_path() -> std::path::PathBuf {
    lichtpult::show_path().with_file_name("accent")
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.into() } else { format!("{}…", s.chars().take(n - 1).collect::<String>()) }
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
                Ok("cues") => View::Cues,
                Ok("patch") => View::Patch,
                Ok("output") => View::Output,
                _ => View::Programmer,
            },
            frames: 0,
            accent,
            saved_accent: accent,
            status: String::new(),
            patch: (1, 1, 1, 1),
            library: [builtin_types(), load_library()].concat(),
            search: String::new(),
            import_path: String::new(),
            out_universe: 1,
            pending_load: None,
        }
    }

    /// Entwicklungshilfe: `LICHTPULT_SHOT=datei.ppm [LICHTPULT_SELECT=1]` speichert
    /// den Fensterinhalt (unabhängig von anderen Fenstern) und beendet die App.
    fn dev_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = std::env::var_os("LICHTPULT_SHOT") else { return };
        self.frames += 1;
        if self.frames == 3 && std::env::var_os("LICHTPULT_SELECT").is_some() {
            self.engine.lock().unwrap().apply(Action::SelectAll);
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

    fn accent(&self) -> Color32 {
        Color32::from_rgb(self.accent[0], self.accent[1], self.accent[2])
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
            for (v, name) in [
                (View::Programmer, "Programmer"),
                (View::Cues, "Cues"),
                (View::Patch, "Patch"),
                (View::Output, "Output"),
            ] {
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

    fn programmer(&self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        caption(ui, "Fixtures");
        ui.horizontal_wrapped(|ui| {
            for f in &e.show.fixtures {
                let on = e.selection.contains(&f.id);
                let r = tile(ui, format!("{}\n{}", f.id, short(&e.show.types[f.ty].name, 11)), on, accent);
                if r.clicked() {
                    acts.push(if ui.input(|i| i.modifiers.command || i.modifiers.shift) {
                        Action::Toggle(f.id)
                    } else {
                        Action::Select(f.id)
                    });
                }
            }
            if e.show.fixtures.is_empty() {
                ui.label(RichText::new("Noch keine Fixtures – im Patch anlegen.").color(theme::MUTED));
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Alle").clicked() {
                acts.push(Action::SelectAll);
            }
            if ui.button("Auswahl löschen").clicked() {
                acts.push(Action::ClearSelection);
            }
            if ui.button("Programmer leeren").clicked() {
                acts.push(Action::ClearProgrammer);
            }
        });
        ui.separator();

        caption(ui, "Gruppen");
        ui.horizontal_wrapped(|ui| {
            for (i, g) in e.show.groups.iter().enumerate() {
                if tile(ui, &g.name, false, accent).clicked() {
                    acts.push(Action::SelectGroup(i));
                }
            }
            if ui.button("+ Gruppe aus Auswahl").clicked() {
                acts.push(Action::RecordGroup);
            }
        });
        ui.separator();

        if e.selection.is_empty() {
            ui.label(RichText::new("Fixtures auswählen, um Werte zu setzen.").color(theme::MUTED));
            return;
        }
        let sel: Vec<_> = e.show.fixtures.iter().filter(|f| e.selection.contains(&f.id)).collect();
        for kind in Kind::ALL {
            let attrs: Vec<_> = Attr::ALL
                .into_iter()
                .filter(|a| a.kind() == kind && sel.iter().any(|f| e.show.has(f, *a)))
                .collect();
            if attrs.is_empty() {
                continue;
            }
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    caption(ui, kind.name());
                    for a in &attrs {
                        let first = sel.iter().find(|f| e.show.has(f, *a)).unwrap();
                        let mut v = e.programmer.get(&first.id).and_then(|m| m.get(a)).copied().unwrap_or(0);
                        if ui.add(egui::Slider::new(&mut v, 0..=255).text(a.name())).changed() {
                            acts.push(Action::SetAttr(*a, v));
                        }
                    }
                });
                ui.vertical(|ui| {
                    caption(ui, "Paletten");
                    ui.horizontal_wrapped(|ui| {
                        for (i, p) in e.show.palettes.iter().enumerate().filter(|(_, p)| p.kind == kind) {
                            if tile(ui, &p.name, false, accent).clicked() {
                                acts.push(Action::ApplyPalette(i));
                            }
                        }
                        if ui.button("+ Palette").clicked() {
                            acts.push(Action::RecordPalette(kind));
                        }
                    });
                });
            });
            ui.separator();
        }
    }

    fn cues(&self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        ui.horizontal(|ui| {
            caption(ui, "Cue-Liste");
            if ui.button("Cue aufnehmen").clicked() {
                acts.push(Action::RecordCue);
            }
        });
        for (i, c) in e.show.cues.iter().enumerate() {
            ui.horizontal(|ui| {
                if tile(ui, &c.name, e.cue == Some(i), accent).clicked() {
                    acts.push(Action::GotoCue(i));
                }
                let mut fade = c.fade;
                if ui.add(egui::DragValue::new(&mut fade).speed(0.1).suffix(" s").range(0.0..=600.0)).changed() {
                    acts.push(Action::SetCueFade(i, fade));
                }
                if ui.button("Löschen").clicked() {
                    acts.push(Action::DeleteCue(i));
                }
            });
        }
        if e.show.cues.is_empty() {
            ui.label(RichText::new("Werte im Programmer setzen, dann „Cue aufnehmen“.").color(theme::MUTED));
        }
    }

    fn patch(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        caption(ui, "Fixture-Bibliothek");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Suchen…").desired_width(220.0));
            ui.add(egui::TextEdit::singleline(&mut self.import_path).hint_text("OFL-Datei/Ordner (oder hierher ziehen)").desired_width(320.0));
            if ui.button("Importieren").clicked() {
                let p = std::path::PathBuf::from(self.import_path.trim());
                self.import(vec![p]);
            }
        });
        let q = self.search.to_lowercase();
        let hits: Vec<usize> = (0..self.library.len())
            .filter(|&i| self.library[i].name.to_lowercase().contains(&q))
            .collect();
        egui::ScrollArea::vertical().id_salt("lib").max_height(240.0).show(ui, |ui| {
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
            ui.add(egui::DragValue::new(&mut self.patch.2).range(1..=32768).prefix("Uni "));
            ui.add(egui::DragValue::new(&mut self.patch.3).range(1..=512).prefix("Adr "));
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
        ui.separator();
        caption(ui, "Gepatchte Fixtures");
        let conflicts = e.conflicts();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for f in &e.show.fixtures {
                ui.horizontal(|ui| {
                    let bad = conflicts.contains(&f.id);
                    let span = format!("{}–{}", f.address, e.show.range(f).end.saturating_sub(1));
                    let text = format!(
                        "{:>3}   {:<28} Uni {:<3} {span:<9}",
                        f.id,
                        short(&e.show.types[f.ty].name, 28),
                        f.universe,
                    );
                    let rt = RichText::new(text).monospace();
                    ui.label(if bad { rt.color(theme::DANGER) } else { rt });
                    if bad {
                        ui.label(RichText::new("Adresskonflikt").color(theme::DANGER).small());
                    }
                    if ui.small_button("✕").clicked() {
                        acts.push(Action::RemoveFixture(f.id));
                    }
                });
            }
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

    fn playback_bar(&self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        ui.horizontal_centered(|ui| {
            if ui.button("◀ Back").clicked() {
                acts.push(Action::Back);
            }
            let go = egui::Button::new(RichText::new("GO").strong().color(Color32::BLACK)).fill(self.accent());
            if ui.add_sized([96.0, 32.0], go).clicked() {
                acts.push(Action::Go);
            }
            let cur = e.cue.and_then(|i| e.show.cues.get(i)).map_or("—", |c| c.name.as_str());
            ui.label(format!("Aktuell: {cur}"));
            if let Some(next) = e.show.cues.get(e.cue.map_or(0, |c| c + 1)) {
                ui.label(RichText::new(format!("Nächste: {}", next.name)).color(theme::MUTED));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(&self.status).color(theme::MUTED));
            });
        });
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
            ui.input(|i| {
                if i.key_pressed(egui::Key::Space) {
                    acts.push(Action::Go);
                }
                if i.key_pressed(egui::Key::Backspace) {
                    acts.push(Action::Back);
                }
                if i.key_pressed(egui::Key::Escape) {
                    acts.push(Action::ClearSelection);
                }
            });
        }

        egui::Panel::top("top").exact_size(56.0).show(ui, |ui| self.top_bar(ui, &e, &mut acts));
        egui::Panel::bottom("playback").exact_size(52.0).show(ui, |ui| self.playback_bar(ui, &e, &mut acts));
        egui::CentralPanel::default().show(ui, |ui| match self.view {
            View::Programmer => {
                egui::ScrollArea::vertical().show(ui, |ui| self.programmer(ui, &e, &mut acts));
            }
            View::Cues => self.cues(ui, &e, &mut acts),
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
                .with_inner_size([1280.0, 800.0])
                .with_min_inner_size([960.0, 600.0]),
            ..Default::default()
        },
        Box::new(|_| Ok(Box::new(App::new(engine)))),
    )
}
