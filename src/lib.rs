//! Kern: DMX-Universum, geteilt zwischen UI und Ausgabe-Thread.

use std::sync::{Arc, Mutex};

pub const CHANNELS: usize = 512;

pub type Universe = Arc<Mutex<[u8; CHANNELS]>>;

pub fn new_universe() -> Universe {
    Arc::new(Mutex::new([0; CHANNELS]))
}

/// Baut ein Art-Net-DMX-Paket (OpDmx) für ein Universum.
pub fn artnet_packet(seq: u8, universe: u16, data: &[u8; CHANNELS]) -> Vec<u8> {
    let mut p = Vec::with_capacity(18 + CHANNELS);
    p.extend_from_slice(b"Art-Net\0");
    p.extend_from_slice(&[0x00, 0x50, 0, 14, seq, 0]);
    p.extend_from_slice(&universe.to_le_bytes());
    p.extend_from_slice(&(CHANNELS as u16).to_be_bytes());
    p.extend_from_slice(data);
    p
}

/// Sendet das Universum mit 44 Hz per Art-Net (Universum 0).
pub fn spawn_output(universe: Universe, target: &'static str) {
    std::thread::spawn(move || {
        let sock = std::net::UdpSocket::bind("0.0.0.0:0").expect("UDP bind");
        sock.set_broadcast(true).ok();
        let mut seq = 1u8;
        loop {
            let data = *universe.lock().unwrap();
            let _ = sock.send_to(&artnet_packet(seq, 0, &data), target);
            seq = seq.wrapping_add(1).max(1);
            std::thread::sleep(std::time::Duration::from_micros(22_700));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artnet_header_and_length() {
        let mut d = [0u8; CHANNELS];
        d[0] = 255;
        let p = artnet_packet(1, 0, &d);
        assert_eq!(&p[..8], b"Art-Net\0");
        assert_eq!(p.len(), 18 + CHANNELS);
        assert_eq!(p[18], 255);
    }
}
