mod outview;
mod patch;
mod pult;
mod scenes;
mod stage;
mod theme;

use eframe::egui::{self, Color32, RichText};
use lichtpult::engine::{Action, Engine, Show};
use lichtpult::fixture::{Attr, FixtureType, Kind, builtin_types, import_ofl};
use lichtpult::{Shared, json_files, load_library, load_show, save_library, save_show, spawn_engine};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use theme::{caption, pill};

#[derive(PartialEq, Clone, Copy)]
enum View {
    Pult,
    Scenes,
    Stage,
    Patch,
    Output,
}

pub struct App {
    engine: Shared,
    view: View,
    cmd: String,
    cmd_error: bool,
    accent: [u8; 3],
    saved_accent: [u8; 3],
    status: String,
    /// Patch-Formular: (Bibliotheksindex, Anzahl, Universum, Adresse)
    patch: (usize, u16, u16, u16),
    library: Vec<FixtureType>,
    search: String,
    import_path: String,
    out_universe: u16,
    out: Arc<Mutex<lichtpult::OutConfig>>,
    midi: Arc<Mutex<lichtpult::midi::Midi>>,
    pal_kind: Kind,
    fx_tab: bool,
    cmd_open: bool,
    scene_sel: usize,
    scene_grid: bool,
    scene_drag: Option<usize>,
    prop_sel: Option<usize>,
    stage_zoom: f32,
    stage_pan: egui::Vec2,
    stage_snap: bool,
    pending_load: Option<Show>,
    frames: u32,
    saved_json: String,
    last_save: Instant,
}

fn accent_path() -> std::path::PathBuf {
    lichtpult::data_dir().join("accent")
}

pub fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n - 1).collect::<String>()) }
}

pub fn rgb_of(get: impl Fn(Attr) -> u8) -> Color32 {
    let (r, g, b, w) = (get(Attr::Red), get(Attr::Green), get(Attr::Blue), get(Attr::White));
    Color32::from_rgb(r.saturating_add(w), g.saturating_add(w), b.saturating_add(w))
}

/// Wiedergabe-Aktionen räumen vorher den Programmer auf, sonst überdeckt er die Szene.
pub fn play(acts: &mut Vec<Action>, e: &Engine, a: Action) {
    if !e.programmer.is_empty() {
        acts.push(Action::ClearProgrammer);
    }
    acts.push(a);
}

impl App {
    fn new(engine: Shared, out: Arc<Mutex<lichtpult::OutConfig>>, midi: Arc<Mutex<lichtpult::midi::Midi>>) -> Self {
        let accent = std::fs::read(accent_path())
            .ok()
            .and_then(|b| <[u8; 3]>::try_from(b).ok())
            .unwrap_or(theme::PRESETS[0]);
        let saved_json = serde_json::to_string(&engine.lock().unwrap().show).unwrap_or_default();
        App {
            engine,
            view: match std::env::var("LICHTPULT_VIEW").as_deref() {
                Ok("scenes") => View::Scenes,
                Ok("stage") => View::Stage,
                Ok("patch") => View::Patch,
                Ok("output") => View::Output,
                _ => View::Pult,
            },
            cmd: String::new(),
            cmd_error: false,
            accent,
            saved_accent: accent,
            status: String::new(),
            patch: (1, 1, 1, 1),
            library: [builtin_types(), load_library()].concat(),
            search: String::new(),
            import_path: String::new(),
            out_universe: 1,
            out,
            midi,
            pal_kind: Kind::Intensity,
            fx_tab: false,
            cmd_open: false,
            scene_sel: 0,
            scene_grid: false,
            scene_drag: None,
            prop_sel: None,
            stage_zoom: 1.0,
            stage_pan: egui::Vec2::ZERO,
            stage_snap: true,
            pending_load: None,
            frames: 0,
            saved_json,
            last_save: Instant::now(),
        }
    }

    pub fn accent(&self) -> Color32 {
        Color32::from_rgb(self.accent[0], self.accent[1], self.accent[2])
    }

    /// Entwicklungshilfe: `LICHTPULT_SHOT=datei.ppm [LICHTPULT_SELECT=1]` speichert
    /// den Fensterinhalt (unabhängig von anderen Fenstern) und beendet die App.
    fn dev_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = std::env::var_os("LICHTPULT_SHOT") else { return };
        self.frames += 1;
        if self.frames == 3 && std::env::var_os("LICHTPULT_SELECT").is_some() {
            let mut e = self.engine.lock().unwrap();
            let ids: Vec<u32> = match std::env::var("LICHTPULT_SELECT").as_deref() {
                Ok("1") => vec![3, 4],
                Ok(v) => v.split(',').filter_map(|x| x.parse().ok()).collect(),
                _ => vec![],
            };
            e.apply(Action::Select(ids[0]));
            for id in &ids[1..] {
                e.apply(Action::Toggle(*id));
            }
            e.apply(Action::SetAttr(Attr::Green, 180));
            e.apply(Action::SetAttr(Attr::Dimmer, 220));
        }
        if self.frames == 4 {
            match std::env::var("LICHTPULT_TAB").as_deref() {
                Ok("pos") => self.pal_kind = Kind::Position,
                Ok("fx") => self.fx_tab = true,
                _ => {}
            }
            if std::env::var_os("LICHTPULT_PLAY").is_some() {
                let mut e = self.engine.lock().unwrap();
                e.apply(Action::ClearProgrammer);
                e.apply(Action::GotoCue(1));
            }
        }
        if self.frames == 12 {
            let ctx2 = ctx.clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |img| {
                let mut out = format!("P6\n{} {}\n255\n", img.width(), img.height()).into_bytes();
                out.extend(img.pixels.iter().flat_map(|p| [p.r(), p.g(), p.b()]));
                let _ = std::fs::write(path, out);
                ctx2.send_viewport_cmd(egui::ViewportCommand::Close);
            })));
        }
    }

    fn import_file(&mut self, file: &std::path::Path) -> Result<Vec<FixtureType>, String> {
        if file.extension().is_some_and(|x| x == "gdtf") {
            std::fs::read(file).map_err(|e| e.to_string()).and_then(|b| lichtpult::gdtf::import_gdtf(&b))
        } else {
            std::fs::read_to_string(file).map_err(|e| e.to_string()).and_then(|s| import_ofl(&s))
        }
    }

    fn import(&mut self, paths: Vec<std::path::PathBuf>) {
        let (mut added, mut failed) = (0, 0);
        for file in paths.iter().flat_map(|p| json_files(p)) {
            match self.import_file(&file) {
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

    /// Show regelmäßig und nur bei Änderungen sichern.
    fn autosave(&mut self, e: &Engine) {
        if self.last_save.elapsed() < Duration::from_secs(3) {
            return;
        }
        self.last_save = Instant::now();
        if let Ok(json) = serde_json::to_string(&e.show) {
            if json != self.saved_json {
                if save_show(&e.show).is_ok() {
                    self.saved_json = json;
                }
            }
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>, narrow: bool) {
        let accent = self.accent();
        ui.horizontal_centered(|ui| {
            ui.add_space(6.0);
            ui.label(RichText::new("●").color(accent).size(12.0));
            ui.label(RichText::new("LICHTPULT").strong().size(15.0).color(theme::TEXT));
            ui.add_space(14.0);
            ui.label(RichText::new(format!("GM: {}%", (e.master as u32 * 100 + 127) / 255)).size(12.5).monospace().color(theme::MUTED));
            ui.add_space(10.0);
            ui.label(RichText::new(short(&e.show.name, 24)).size(12.5).color(theme::MUTED));
            if !narrow {
                ui.add_space(10.0);
                let n = e.selection.len();
                ui.label(RichText::new(if n == 0 { "Keine Fixtures gewählt".to_string() } else { format!("{n} Fixture(s) gewählt") }).size(12.5).color(if n == 0 { theme::MUTED } else { theme::TEXT }));
            }
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                self.settings(ui, e);
                if theme::icon_btn(ui, "terminal", "Befehlszeile (Strg+L)").clicked() {
                    self.cmd_open = !self.cmd_open;
                }
                if !narrow {
                    self.output_status(ui);
                }
                let mut black = e.blackout;
                let b = egui::Button::new(RichText::new("Blackout").strong().color(if black { Color32::BLACK } else { theme::TEXT }))
                    .fill(if black { theme::DANGER } else { theme::RAISED })
                    .min_size(egui::vec2(88.0, 28.0));
                if ui.add(b).clicked() {
                    black = !black;
                    acts.push(Action::Blackout(black));
                }
                let mut m = e.master;
                ui.spacing_mut().slider_width = if narrow { 70.0 } else { 130.0 };
                if ui.add(egui::Slider::new(&mut m, 0..=255).show_value(false)).changed() {
                    acts.push(Action::Master(m));
                }
                ui.label(RichText::new("GM").size(12.5).color(theme::MUTED));
                ui.add_space(8.0);
                if ui.add_enabled(e.can_redo(), egui::Button::image(theme::icon("redo", theme::TEXT, 16.0)).min_size(egui::vec2(32.0, 32.0))).on_hover_text("Wiederholen (Strg+Shift+Z)").clicked() {
                    acts.push(Action::Redo);
                }
                if ui.add_enabled(e.can_undo(), egui::Button::image(theme::icon("undo", theme::TEXT, 16.0)).min_size(egui::vec2(32.0, 32.0))).on_hover_text("Rückgängig (Strg+Z)").clicked() {
                    acts.push(Action::Undo);
                }
            });
        });
    }

    fn settings(&mut self, ui: &mut egui::Ui, e: &Engine) {
        egui::containers::menu::MenuButton::from_button(egui::Button::image(theme::icon("settings", theme::TEXT, 16.0)).min_size(egui::vec2(32.0, 32.0))).ui(ui, |ui| {
            caption(ui, "Akzentfarbe");
            ui.horizontal(|ui| {
                for p in theme::PRESETS {
                    let c = Color32::from_rgb(p[0], p[1], p[2]);
                    if ui.add(egui::Button::new("").fill(c).min_size(egui::vec2(22.0, 22.0))).clicked() {
                        self.accent = p;
                    }
                }
            });
            ui.color_edit_button_srgb(&mut self.accent);
            ui.separator();
            ui.label(RichText::new("Die Show wird automatisch gespeichert.").small().color(theme::MUTED));
            if ui.button("Jetzt speichern").clicked() {
                self.status = match save_show(&e.show) {
                    Ok(()) => "Show gespeichert".into(),
                    Err(err) => format!("Speichern fehlgeschlagen: {err}"),
                };
            }
            if ui.button("Zuletzt gespeicherte Show laden").clicked() {
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

    /// Playback-Leiste: Transport links, Szenen als Pads, Pegel rechts.
    fn player(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let i = e.active;
        let list = e.list();
        let cur = e.cue();
        let full = ui.max_rect();
        ui.painter().line_segment([full.left_top(), full.right_top()], egui::Stroke::new(1.0, theme::LINE));
        let left = egui::Rect::from_min_size(full.min + egui::vec2(10.0, 8.0), egui::vec2(188.0, full.height() - 16.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
            ui.label(RichText::new(short(&list.name, 22)).size(12.5).strong());
            let now = cur.and_then(|c| list.cues.get(c)).map_or("— keine Szene —".to_string(), |c| short(&c.name, 22));
            ui.label(RichText::new(now).size(12.0).color(if cur.is_some() { accent } else { theme::MUTED }));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let sz = egui::vec2(52.0, 42.0);
                if ui.add(egui::Button::image(theme::icon("back", theme::TEXT, 18.0)).min_size(sz)).on_hover_text("Zurück (Backspace)").clicked() {
                    play(acts, e, Action::Back(i));
                }
                if ui.add(egui::Button::image(theme::icon("play", Color32::BLACK, 20.0)).fill(accent).min_size(egui::vec2(64.0, 42.0))).on_hover_text("GO – nächste Szene (Leertaste)").clicked() {
                    play(acts, e, Action::Go(i));
                }
                if ui.add(egui::Button::image(theme::icon("stop", theme::TEXT, 16.0)).min_size(sz)).on_hover_text("Liste ausschalten").clicked() {
                    acts.push(Action::ReleaseList(i));
                }
            });
        });
        let rw = 200.0;
        let right = egui::Rect::from_min_max(egui::pos2(full.right() - rw - 10.0, full.top() + 8.0), egui::pos2(full.right() - 10.0, full.bottom() - 8.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
            ui.label(RichText::new("LISTEN-PEGEL").size(10.5).color(theme::MUTED));
            let mut lvl = e.players.get(i).map_or(255, |p| p.level) as f32 / 2.55;
            ui.spacing_mut().slider_width = 110.0;
            if ui.add(egui::Slider::new(&mut lvl, 0.0..=100.0).fixed_decimals(0).suffix("%")).changed() {
                acts.push(Action::SetLevel(i, (lvl * 2.55).round() as u8));
            }
            if !e.programmer.is_empty() {
                ui.horizontal(|ui| {
                    pill(ui, "Programmer", theme::DANGER);
                    if theme::button(ui, "Leeren").on_hover_text("Manuelle Werte überdecken die Szenen – leeren, um die Szenen zu sehen").clicked() {
                        acts.push(Action::ClearProgrammer);
                    }
                });
            }
        });
        let mid = egui::Rect::from_min_max(egui::pos2(full.left() + 208.0, full.top() + 8.0), egui::pos2(full.right() - rw - 20.0, full.bottom() - 8.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(mid), |ui| {
            if list.cues.is_empty() {
                ui.label(RichText::new("Noch keine Szenen – im Pult einen Look einstellen und „Als Szene speichern“.").color(theme::MUTED));
                return;
            }
            egui::ScrollArea::horizontal().id_salt("bank").auto_shrink([false, false]).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    for (k, c) in list.cues.iter().enumerate() {
                        let live = cur == Some(k);
                        let col = scenes::preview(e, c).into_iter().next().unwrap_or(theme::LINE);
                        let prog = if live { Some(e.fade_progress(i)) } else { None };
                        let r = theme::pad(ui, egui::vec2(116.0, 84.0), &(k + 1).to_string(), &short(&c.name, 14), col, live, theme::OK, prog);
                        if r.clicked() {
                            play(acts, e, Action::GotoCue(k));
                        }
                        r.on_hover_text(format!("{} · {:.1} s", c.name, c.fade));
                    }
                });
            });
        });
    }

    /// Befehlszeile als Overlay (Strg+L).
    fn command_overlay(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        if !self.cmd_open {
            return;
        }
        let accent = self.accent();
        egui::Area::new(egui::Id::new("cmd_overlay")).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -100.0)).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).fill(theme::PANEL).inner_margin(12).show(ui, |ui| {
                ui.set_width(600.0);
                ui.horizontal(|ui| {
                    let ir = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover()).0;
                    theme::icon("terminal", accent, 16.0).paint_at(ui, ir);
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.cmd)
                            .id(egui::Id::new("cmdline"))
                            .desired_width(f32::INFINITY)
                            .hint_text("Befehl: 1 thru 8 @ 50 · red 255 · go · fx 1 · record scene"),
                    );
                    r.request_focus();
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.cmd_open = false;
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.cmd.trim().is_empty() {
                        match lichtpult::cmd::parse(&self.cmd, e) {
                            Ok(a) => {
                                acts.extend(a);
                                self.cmd.clear();
                                self.cmd_error = false;
                                self.status = "OK".into();
                                self.cmd_open = false;
                            }
                            Err(err) => {
                                self.cmd_error = true;
                                self.status = err;
                            }
                        }
                    }
                });
                if self.cmd_error {
                    ui.label(RichText::new(&self.status).size(12.5).color(theme::DANGER));
                } else {
                    ui.label(RichText::new("Enter = ausführen · Esc = schließen").size(12.0).color(theme::MUTED));
                }
            });
        });
    }

    fn output_status(&self, ui: &mut egui::Ui) {
        let c = self.out.lock().unwrap().clone();
        if c.enabled {
            let proto = match c.protocol {
                lichtpult::Protocol::ArtNet => "Art-Net",
                lichtpult::Protocol::Sacn => "sACN",
                lichtpult::Protocol::Both => "Art-Net + sACN",
            };
            let to = if c.target.trim().is_empty() { "Broadcast" } else { c.target.trim() };
            pill(ui, &format!("{proto} → {to}"), theme::OK);
        } else {
            pill(ui, "Ausgabe aus", theme::DANGER);
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
        ui.ctx().request_repaint_after(Duration::from_millis(33));
        self.dev_screenshot(ui.ctx());

        let dropped: Vec<_> = ui.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        if !dropped.is_empty() {
            self.import(dropped);
            self.view = View::Patch;
        }

        let engine = self.engine.clone();
        let mut e = engine.lock().unwrap();
        let mut acts = Vec::new();
        self.autosave(&e);

        if !ui.ctx().egui_wants_keyboard_input() {
            let active = e.active;
            let busy = !e.programmer.is_empty();
            ui.input(|i| {
                let mut p = |a: Action| {
                    if busy {
                        acts.push(Action::ClearProgrammer);
                    }
                    acts.push(a);
                };
                if i.key_pressed(egui::Key::Space) {
                    p(Action::Go(active));
                }
                if i.key_pressed(egui::Key::Backspace) {
                    p(Action::Back(active));
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
            self.cmd_open = !self.cmd_open;
        }

        let narrow = ui.available_width() < 1150.0;
        let bar = egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(8, 0));
        egui::Panel::top("top").exact_size(38.0).frame(bar).show(ui, |ui| self.top_bar(ui, &e, &mut acts, narrow));
        let player = egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(0, 0));
        egui::Panel::bottom("player").exact_size(108.0).frame(player).show(ui, |ui| self.player(ui, &e, &mut acts));
        let rail = egui::Frame::new().fill(theme::PANEL);
        egui::Panel::left("rail").exact_size(78.0).frame(rail).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let accent = self.accent();
            for (v, icon, name) in [(View::Pult, "sliders", "Pult"), (View::Scenes, "clapper", "Szenen"), (View::Stage, "stage", "Bühne"), (View::Patch, "plug", "Patch"), (View::Output, "cable", "Ausgabe")] {
                if theme::rail_btn(ui, icon, name, self.view == v, accent).clicked() {
                    self.view = v;
                }
            }
        });
        if self.view == View::Pult {
            let attrs = egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin { left: 12, right: 12, top: 0, bottom: 8 });
            egui::Panel::bottom("attrs").exact_size(if narrow { 330.0 } else { 340.0 }).frame(attrs).show(ui, |ui| self.pult_attrs(ui, &e, &mut acts));
        }
        let inspector = egui::Frame::new().fill(theme::PANEL).inner_margin(16);
        if self.view == View::Scenes {
            egui::Panel::right("scene_inspector").exact_size(if narrow { 260.0 } else { 320.0 }).frame(inspector).show(ui, |ui| self.scene_inspector(ui, &e, &mut acts));
        }
        if self.view == View::Stage {
            egui::Panel::right("stage_inspector").exact_size(if narrow { 260.0 } else { 300.0 }).frame(inspector).show(ui, |ui| self.stage_inspector(ui, &e, &mut acts));
        }
        if self.view == View::Patch {
            let side = egui::Frame::new().fill(theme::PANEL).inner_margin(16);
            egui::Panel::left("patch_side").exact_size(if narrow { 280.0 } else { 360.0 }).frame(side).show(ui, |ui| self.patch_side(ui, &e, &mut acts));
        }
        let center = egui::Frame::new().fill(theme::BG).inner_margin(16);
        egui::CentralPanel::default().frame(center).show(ui, |ui| match self.view {
            View::Pult => self.pult(ui, &e, &mut acts),
            View::Scenes => self.scenes(ui, &e, &mut acts),
            View::Stage => self.stage(ui, &e, &mut acts),
            View::Patch => self.patch(ui, &e, &mut acts),
            View::Output => self.output(ui, &e),
        });

        self.command_overlay(ui, &e, &mut acts);
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
    let out = Arc::new(Mutex::new(lichtpult::output::load()));
    spawn_engine(engine.clone(), out.clone());
    let midi = lichtpult::midi::spawn(engine.clone());
    eframe::run_native(
        "Lichtpult",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1400.0, 850.0])
                .with_min_inner_size([1100.0, 650.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(engine, out, midi)))
        }),
    )
}
