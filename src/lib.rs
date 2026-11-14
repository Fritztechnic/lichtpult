//! Lichtpult: Kern ohne UI. UI und (später) Hardware-Pulte steuern die
//! `Engine` ausschließlich über `Action`s.

pub mod api;
pub mod cmd;
pub mod engine;
pub mod fixture;
pub mod gdtf;
pub mod midi;
pub mod output;

use engine::{Engine, Show};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub type Shared = Arc<Mutex<Engine>>;

const PERIOD: Duration = Duration::from_micros(22_700); // ~44 Hz

pub use output::{OutConfig, Protocol, artnet_packet};

/// Engine-Takt und Netzwerk-Ausgabe (Art-Net/sACN) in einem Thread.
pub fn spawn_engine(engine: Shared, cfg: Arc<Mutex<OutConfig>>) {
    std::thread::spawn(move || {
        let sock = std::net::UdpSocket::bind("0.0.0.0:0").expect("UDP bind");
        sock.set_broadcast(true).ok();
        let cid: [u8; 16] = std::array::from_fn(|i| (std::process::id() as u8).wrapping_add(i as u8 * 17));
        let (mut seq, mut last) = (1u8, Instant::now());
        loop {
            let frames = {
                let mut e = engine.lock().unwrap();
                let now = Instant::now();
                e.tick((now - last).as_secs_f32());
                last = now;
                e.frames.clone()
            };
            let c = cfg.lock().unwrap().clone();
            if c.enabled {
                let host = c.target.trim();
                for (u, data) in &frames {
                    if c.protocol != Protocol::Sacn {
                        let to = if host.is_empty() { "255.255.255.255:6454".into() } else { format!("{host}:6454") };
                        let _ = sock.send_to(&artnet_packet(seq, u.saturating_sub(1), data), to);
                    }
                    if c.protocol != Protocol::ArtNet {
                        let to = if host.is_empty() { output::sacn_multicast(*u) } else { format!("{host}:5568") };
                        let _ = sock.send_to(&output::sacn_packet(&cid, seq, *u, c.sacn_priority, data), to);
                    }
                }
            }
            seq = seq.wrapping_add(1).max(1);
            std::thread::sleep(PERIOD);
        }
    });
}

pub fn data_dir() -> std::path::PathBuf {
    let home = std::env::var_os("HOME").unwrap_or_default();
    std::path::Path::new(&home).join(".local/share/lichtpult")
}

pub fn show_path() -> std::path::PathBuf {
    data_dir().join("show.json")
}

/// Importierte Fixture-Typen (unabhängig von der Show).
pub fn load_library() -> Vec<fixture::FixtureType> {
    std::fs::read(data_dir().join("library.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save_library(lib: &[fixture::FixtureType]) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir())?;
    std::fs::write(data_dir().join("library.json"), serde_json::to_vec(lib)?)
}

/// Alle `.json`-/`.gdtf`-Dateien unter `path` (Datei oder Ordner, rekursiv).
pub fn json_files(path: &std::path::Path) -> Vec<std::path::PathBuf> {
    if path.is_dir() {
        let Ok(rd) = std::fs::read_dir(path) else { return vec![] };
        rd.flatten().flat_map(|e| json_files(&e.path())).collect()
    } else if path.extension().is_some_and(|e| e == "json" || e == "gdtf") {
        vec![path.to_path_buf()]
    } else {
        vec![]
    }
}

pub fn save_show(show: &Show) -> std::io::Result<()> {
    let path = show_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(path, serde_json::to_vec_pretty(show)?)
}

pub fn load_show() -> std::io::Result<Show> {
    Ok(serde_json::from_slice(&std::fs::read(show_path())?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artnet_header_and_length() {
        let mut d = [0u8; engine::CHANNELS];
        d[0] = 255;
        let p = artnet_packet(1, 0, &d);
        assert_eq!(&p[..8], b"Art-Net\0");
        assert_eq!(p.len(), 18 + engine::CHANNELS);
        assert_eq!(p[18], 255);
    }

    #[test]
    fn show_roundtrips_json() {
        let mut e = Engine::new(Show::default());
        e.apply(engine::Action::AddFixtures { ty: 1, count: 2, universe: 1, address: 1 });
        e.apply(engine::Action::SelectAll);
        e.apply(engine::Action::SetAttr(fixture::Attr::Red, 9));
        e.apply(engine::Action::RecordCue);
        let json = serde_json::to_string(&e.show).unwrap();
        let back: Show = serde_json::from_str(&json).unwrap();
        assert_eq!(back.lists[0].cues[0].values[&1][&fixture::Attr::Red], 9);

        // Altes Format (cues statt lists) wird übernommen.
        let mut old: serde_json::Value = serde_json::from_str(&json).unwrap();
        old["cues"] = old["lists"][0]["cues"].take();
        old.as_object_mut().unwrap().remove("lists");
        let migrated = Engine::new(serde_json::from_value(old).unwrap());
        assert_eq!(migrated.list().cues.len(), 1);
    }
}
