//! MIDI-Eingang mit Lernfunktion: Noten/Controller werden auf `Action`s gelegt.

use crate::Shared;
use crate::engine::{Action, Engine};
use midir::{MidiInput, MidiInputConnection};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Kanal (0-15) und Nummer.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Source {
    Note(u8, u8),
    Cc(u8, u8),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Target {
    Go(usize),
    Back(usize),
    Release(usize),
    /// Playback-Fader
    Level(usize),
    Master,
    Blackout,
    Effect(usize),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Bind {
    pub src: Source,
    pub target: Target,
}

#[derive(Default)]
pub struct Midi {
    pub binds: Vec<Bind>,
    pub learn: Option<Target>,
    pub ports: Vec<String>,
    pub last: Option<Source>,
}

pub fn target_name(t: Target) -> String {
    match t {
        Target::Go(l) => format!("GO Liste {}", l + 1),
        Target::Back(l) => format!("Back Liste {}", l + 1),
        Target::Release(l) => format!("Freigeben Liste {}", l + 1),
        Target::Level(l) => format!("Fader Liste {}", l + 1),
        Target::Master => "Master".into(),
        Target::Blackout => "Blackout".into(),
        Target::Effect(i) => format!("Effekt {}", i + 1),
    }
}

pub fn source_name(s: Source) -> String {
    match s {
        Source::Note(c, n) => format!("Note {n} · Kanal {}", c + 1),
        Source::Cc(c, n) => format!("CC {n} · Kanal {}", c + 1),
    }
}

/// Rohnachricht -> Quelle und Wert (0..127). Note-Off liefert `None`.
pub fn parse(msg: &[u8]) -> Option<(Source, u8)> {
    let [status, n, v, ..] = *msg else { return None };
    let ch = status & 15;
    match status & 0xF0 {
        0x90 if v > 0 => Some((Source::Note(ch, n), v)),
        0xB0 => Some((Source::Cc(ch, n), v)),
        _ => None,
    }
}

/// Wert einer Bindung in eine Aktion übersetzen. Tasten reagieren auf Druck (Wert >= 64 bei CC).
pub fn action(t: Target, src: Source, v: u8, e: &Engine) -> Option<Action> {
    let press = matches!(src, Source::Note(..)) || v >= 64;
    let scaled = ((v as u16 * 255 + 63) / 127) as u8;
    match t {
        Target::Go(l) => press.then_some(Action::Go(l)),
        Target::Back(l) => press.then_some(Action::Back(l)),
        Target::Release(l) => press.then_some(Action::ReleaseList(l)),
        Target::Effect(i) => press.then_some(Action::ToggleEffect(i)),
        Target::Blackout => press.then_some(Action::Blackout(!e.blackout)),
        Target::Level(l) => Some(Action::SetLevel(l, scaled)),
        Target::Master => Some(Action::Master(scaled)),
    }
}

fn path() -> std::path::PathBuf {
    crate::data_dir().join("midi.json")
}

fn save(binds: &[Bind]) {
    let _ = std::fs::create_dir_all(crate::data_dir());
    if let Ok(b) = serde_json::to_vec_pretty(binds) {
        let _ = std::fs::write(path(), b);
    }
}

pub fn save_state(m: &Midi) {
    save(&m.binds);
}

fn handle(state: &Arc<Mutex<Midi>>, engine: &Shared, msg: &[u8]) {
    let Some((src, v)) = parse(msg) else { return };
    let hits: Vec<Target> = {
        let mut m = state.lock().unwrap();
        m.last = Some(src);
        if let Some(t) = m.learn.take() {
            m.binds.retain(|b| b.src != src && b.target != t);
            m.binds.push(Bind { src, target: t });
            save(&m.binds);
            return;
        }
        m.binds.iter().filter(|b| b.src == src).map(|b| b.target).collect()
    };
    if hits.is_empty() {
        return;
    }
    let mut e = engine.lock().unwrap();
    for t in hits {
        if let Some(a) = action(t, src, v, &e) {
            e.apply(a);
        }
    }
}

/// Startet den Hintergrund-Thread, der neue MIDI-Eingänge automatisch verbindet.
pub fn spawn(engine: Shared) -> Arc<Mutex<Midi>> {
    let binds = std::fs::read(path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let state = Arc::new(Mutex::new(Midi { binds, ..Midi::default() }));
    let st = state.clone();
    std::thread::spawn(move || {
        let mut conns: Vec<(String, MidiInputConnection<()>)> = Vec::new();
        loop {
            if let Ok(probe) = MidiInput::new("lichtpult") {
                let ports = probe.ports();
                let names: Vec<String> = ports.iter().filter_map(|p| probe.port_name(p).ok()).collect();
                conns.retain(|(n, _)| names.contains(n));
                for (p, name) in ports.iter().zip(&names) {
                    if name.contains("Midi Through") || conns.iter().any(|(n, _)| n == name) {
                        continue;
                    }
                    let Ok(input) = MidiInput::new("lichtpult") else { continue };
                    let (s, e) = (st.clone(), engine.clone());
                    if let Ok(c) = input.connect(p, "lichtpult-in", move |_, msg, _| handle(&s, &e, msg), ()) {
                        conns.push((name.clone(), c));
                    }
                }
                st.lock().unwrap().ports = conns.iter().map(|(n, _)| n.clone()).collect();
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Show;

    #[test]
    fn parse_and_scale() {
        assert_eq!(parse(&[0x91, 60, 100]), Some((Source::Note(1, 60), 100)));
        assert_eq!(parse(&[0x90, 60, 0]), None);
        let e = Engine::new(Show::default());
        let Some(Action::Master(m)) = action(Target::Master, Source::Cc(0, 1), 127, &e) else { panic!() };
        assert_eq!(m, 255);
        assert!(action(Target::Go(0), Source::Cc(0, 1), 10, &e).is_none());
    }
}
