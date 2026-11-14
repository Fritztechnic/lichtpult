//! Lichtpult: Kern ohne UI. UI und (später) Hardware-Pulte steuern die
//! `Engine` ausschließlich über `Action`s.

pub mod engine;
pub mod fixture;

use engine::{Engine, Show};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub type Shared = Arc<Mutex<Engine>>;

const PERIOD: Duration = Duration::from_micros(22_700); // ~44 Hz

/// Art-Net-DMX-Paket (OpDmx). `universe` ist 0-basiert.
pub fn artnet_packet(seq: u8, universe: u16, data: &engine::Frame) -> Vec<u8> {
    let mut p = Vec::with_capacity(18 + data.len());
    p.extend_from_slice(b"Art-Net\0");
    p.extend_from_slice(&[0x00, 0x50, 0, 14, seq, 0]);
    p.extend_from_slice(&universe.to_le_bytes());
    p.extend_from_slice(&(data.len() as u16).to_be_bytes());
    p.extend_from_slice(data);
    p
}

/// Engine-Takt und Art-Net-Ausgabe in einem Thread.
pub fn spawn_engine(engine: Shared, target: &'static str) {
    std::thread::spawn(move || {
        let sock = std::net::UdpSocket::bind("0.0.0.0:0").expect("UDP bind");
        sock.set_broadcast(true).ok();
        let (mut seq, mut last) = (1u8, Instant::now());
        loop {
            let frames = {
                let mut e = engine.lock().unwrap();
                let now = Instant::now();
                e.tick((now - last).as_secs_f32());
                last = now;
                e.frames.clone()
            };
            for (u, data) in &frames {
                let _ = sock.send_to(&artnet_packet(seq, u.saturating_sub(1), data), target);
            }
            seq = seq.wrapping_add(1).max(1);
            std::thread::sleep(PERIOD);
        }
    });
}

pub fn show_path() -> std::path::PathBuf {
    let home = std::env::var_os("HOME").unwrap_or_default();
    std::path::Path::new(&home).join(".local/share/lichtpult/show.json")
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
        assert_eq!(back.cues[0].values[&1][&fixture::Attr::Red], 9);
    }
}
