//! Schmale JSON-Schnittstelle zwischen Kern und Oberfläche.
//! Desktop-App (Tauri) und der Browser-Server (`lichtpult-server`) rufen beide `Core::call` auf.

use crate::engine::{Action, Engine, Show};
use crate::fixture::{Attr, FixtureType, builtin_types};
use crate::midi::{self, Midi};
use crate::{OutConfig, Shared};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct Core {
    pub engine: Shared,
    pub out: Arc<Mutex<OutConfig>>,
    pub midi: Arc<Mutex<Midi>>,
    library: Mutex<Vec<FixtureType>>,
}

#[derive(Serialize)]
struct PlayerInfo {
    cue: Option<usize>,
    level: u8,
    progress: f32,
}

#[derive(Serialize)]
struct FixtureLive {
    id: u32,
    rgb: [u8; 3],
    level: f32,
}

fn live_of(e: &Engine, f: &crate::fixture::Fixture) -> FixtureLive {
    let out = |a| e.output_of(f, a);
    let (r, g, b, w) = (out(Attr::Red), out(Attr::Green), out(Attr::Blue), out(Attr::White));
    let c = [r.saturating_add(w), g.saturating_add(w), b.saturating_add(w)];
    let mx = c.into_iter().max().unwrap_or(0);
    let has_color = [Attr::Red, Attr::Green, Attr::Blue, Attr::White].iter().any(|a| e.show.has(f, *a));
    let level = if e.show.has(f, Attr::Dimmer) { out(Attr::Dimmer) } else { mx };
    let rgb = if has_color && mx > 0 {
        let k = 255.0 / mx as f32;
        c.map(|v| (v as f32 * k) as u8)
    } else if level > 0 {
        [235, 235, 240]
    } else {
        [74, 74, 77]
    };
    FixtureLive { id: f.id, rgb, level: level as f32 / 255.0 }
}

pub fn live_json(e: &Engine) -> Value {
    let players: Vec<PlayerInfo> = e.players.iter().enumerate().map(|(i, p)| PlayerInfo { cue: p.cue, level: p.level, progress: e.fade_progress(i) }).collect();
    let fixtures: Vec<FixtureLive> = e.show.fixtures.iter().map(|f| live_of(e, f)).collect();
    let frames: BTreeMap<u16, &[u8]> = e.frames.iter().map(|(u, f)| (*u, &f[..])).collect();
    json!({
        "rev": e.rev,
        "selection": e.selection,
        "last_selected": e.last_selected,
        "programmer": e.programmer,
        "parked": e.parked,
        "master": e.master,
        "blackout": e.blackout,
        "blind": e.blind,
        "highlight": e.highlight,
        "fade_speed": e.fade_speed,
        "active": e.active,
        "can_undo": e.can_undo(),
        "can_redo": e.can_redo(),
        "players": players,
        "fx_running": e.show.effects.iter().map(|f| f.running).collect::<Vec<_>>(),
        "conflicts": e.conflicts(),
        "fixtures": fixtures,
        "frames": frames,
    })
}

fn safe_name(s: &str) -> String {
    let n: String = s.chars().filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_')).collect();
    n.trim().to_string()
}

fn shows_dir() -> std::path::PathBuf {
    crate::data_dir().join("shows")
}

#[derive(Deserialize)]
struct ImportReq {
    name: String,
    bytes: Vec<u8>,
}

#[derive(Deserialize)]
struct PatchReq {
    ty: FixtureType,
    count: u16,
    universe: u16,
    address: u16,
}

impl Core {
    /// Lädt die letzte Show, startet Takt/Ausgabe, MIDI und Autosave.
    pub fn start() -> Arc<Core> {
        let show = crate::load_show().unwrap_or_else(|_| Show { name: "New Show".into(), ..Show::default() });
        let engine: Shared = Arc::new(Mutex::new(Engine::new(show)));
        let out = Arc::new(Mutex::new(crate::output::load()));
        crate::spawn_engine(engine.clone(), out.clone());
        let midi = midi::spawn(engine.clone());
        let core = Arc::new(Core { engine: engine.clone(), out, midi, library: Mutex::new(crate::load_library()) });
        std::thread::spawn(move || {
            let mut saved = 0;
            loop {
                std::thread::sleep(Duration::from_secs(3));
                let snapshot = {
                    let e = engine.lock().unwrap();
                    (e.rev != saved).then(|| (e.rev, e.show.clone()))
                };
                if let Some((rev, show)) = snapshot {
                    if crate::save_show(&show).is_ok() {
                        saved = rev;
                    }
                }
            }
        });
        core
    }

    fn library(&self) -> Vec<FixtureType> {
        [builtin_types(), self.library.lock().unwrap().clone()].concat()
    }

    pub fn call(&self, method: &str, body: &str) -> Result<String, String> {
        let err = |e: serde_json::Error| e.to_string();
        let v = match method {
            "live" => live_json(&self.engine.lock().unwrap()),
            "show" => {
                let e = self.engine.lock().unwrap();
                json!({ "rev": e.rev, "show": e.show, "library": self.library() })
            }
            "dispatch" => {
                let acts: Vec<Action> = match serde_json::from_str::<Vec<Action>>(body) {
                    Ok(a) => a,
                    Err(_) => vec![serde_json::from_str(body).map_err(err)?],
                };
                let mut e = self.engine.lock().unwrap();
                for a in acts {
                    e.apply(a);
                }
                Value::Null
            }
            "command" => {
                let line: String = serde_json::from_str(body).map_err(err)?;
                let mut e = self.engine.lock().unwrap();
                let acts = crate::cmd::parse(&line, &e)?;
                for a in acts {
                    e.apply(a);
                }
                Value::Null
            }
            "patch_add" => {
                let r: PatchReq = serde_json::from_str(body).map_err(err)?;
                let mut e = self.engine.lock().unwrap();
                e.apply(Action::AddTypes(vec![r.ty.clone()]));
                let ty = e.show.types.iter().position(|t| t.name == r.ty.name).ok_or("type missing")?;
                e.apply(Action::AddFixtures { ty, count: r.count, universe: r.universe, address: r.address });
                Value::Null
            }
            "import" => {
                let r: ImportReq = serde_json::from_str(body).map_err(err)?;
                let types = if r.name.to_lowercase().ends_with(".gdtf") {
                    crate::gdtf::import_gdtf(&r.bytes)?
                } else {
                    crate::fixture::import_ofl(&String::from_utf8_lossy(&r.bytes))?
                };
                let n = types.len();
                let mut lib = self.library.lock().unwrap();
                for t in types {
                    lib.retain(|x| x.name != t.name);
                    lib.push(t);
                }
                crate::save_library(&lib).map_err(|e| e.to_string())?;
                json!(n)
            }
            "output_get" => json!(*self.out.lock().unwrap()),
            "output_set" => {
                let c: OutConfig = serde_json::from_str(body).map_err(err)?;
                crate::output::save(&c);
                *self.out.lock().unwrap() = c;
                Value::Null
            }
            "midi_get" => {
                let m = self.midi.lock().unwrap();
                json!({
                    "ports": m.ports,
                    "learn": m.learn.map(midi::target_name),
                    "last": m.last.map(|(s, v)| format!("{} = {v}", midi::source_name(s))),
                    "binds": m.binds.iter().map(|b| json!({"src": midi::source_name(b.src), "target": midi::target_name(b.target)})).collect::<Vec<_>>(),
                })
            }
            "midi_learn" => {
                let t: Option<midi::Target> = serde_json::from_str(body).map_err(err)?;
                self.midi.lock().unwrap().learn = t;
                Value::Null
            }
            "midi_inject" => {
                let msg: Vec<u8> = serde_json::from_str(body).map_err(err)?;
                midi::handle(&self.midi, &self.engine, &msg);
                Value::Null
            }
            "midi_unbind" => {
                let i: usize = serde_json::from_str(body).map_err(err)?;
                let mut m = self.midi.lock().unwrap();
                if i < m.binds.len() {
                    m.binds.remove(i);
                    midi::save_state(&m);
                }
                Value::Null
            }
            "show_list" => {
                let mut names: Vec<String> = std::fs::read_dir(shows_dir())
                    .map(|rd| rd.flatten().filter_map(|f| f.path().file_stem().map(|s| s.to_string_lossy().into_owned())).collect())
                    .unwrap_or_default();
                names.sort();
                json!(names)
            }
            "show_save_as" => {
                let name = safe_name(&serde_json::from_str::<String>(body).map_err(err)?);
                if name.is_empty() {
                    return Err("Name required".into());
                }
                let mut e = self.engine.lock().unwrap();
                e.show.name = name.clone();
                e.rev += 1;
                std::fs::create_dir_all(shows_dir()).map_err(|x| x.to_string())?;
                let json = serde_json::to_vec_pretty(&e.show).map_err(err)?;
                std::fs::write(shows_dir().join(format!("{name}.json")), json).map_err(|x| x.to_string())?;
                Value::Null
            }
            "show_load" => {
                let name = safe_name(&serde_json::from_str::<String>(body).map_err(err)?);
                let bytes = std::fs::read(shows_dir().join(format!("{name}.json"))).map_err(|x| x.to_string())?;
                let show: Show = serde_json::from_slice(&bytes).map_err(err)?;
                self.engine.lock().unwrap().load_show(show);
                Value::Null
            }
            "show_new" => {
                let name = serde_json::from_str::<String>(body).map_err(err)?;
                self.engine.lock().unwrap().load_show(Show { name, ..Show::default() });
                Value::Null
            }
            _ => return Err(format!("unknown method {method}")),
        };
        Ok(v.to_string())
    }
}
