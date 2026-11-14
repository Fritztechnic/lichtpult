//! Netzwerk-Ausgabe: Art-Net und sACN (E1.31), Einstellungen.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Protocol {
    ArtNet,
    Sacn,
    Both,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutConfig {
    pub protocol: Protocol,
    /// Leer = Broadcast (Art-Net) bzw. Multicast (sACN).
    pub target: String,
    pub sacn_priority: u8,
    pub enabled: bool,
}

impl Default for OutConfig {
    fn default() -> Self {
        OutConfig { protocol: Protocol::ArtNet, target: String::new(), sacn_priority: 100, enabled: true }
    }
}

/// Art-Net-DMX-Paket (OpDmx). `universe` ist 0-basiert.
pub fn artnet_packet(seq: u8, universe: u16, data: &[u8]) -> Vec<u8> {
    let mut p = Vec::with_capacity(18 + data.len());
    p.extend_from_slice(b"Art-Net\0");
    p.extend_from_slice(&[0x00, 0x50, 0, 14, seq, 0]);
    p.extend_from_slice(&universe.to_le_bytes());
    p.extend_from_slice(&(data.len() as u16).to_be_bytes());
    p.extend_from_slice(data);
    p
}

/// sACN-/E1.31-Datenpaket für 512 Slots. `universe` ist 1-basiert.
pub fn sacn_packet(cid: &[u8; 16], seq: u8, universe: u16, priority: u8, data: &[u8]) -> Vec<u8> {
    let n = data.len().min(512);
    let total = 126 + n;
    let flags = |len: usize| (0x7000 | len as u16).to_be_bytes();
    let mut p = Vec::with_capacity(total);
    p.extend_from_slice(&[0, 0x10, 0, 0]);
    p.extend_from_slice(b"ASC-E1.17\0\0\0");
    p.extend_from_slice(&flags(total - 16));
    p.extend_from_slice(&4u32.to_be_bytes());
    p.extend_from_slice(cid);
    p.extend_from_slice(&flags(total - 38));
    p.extend_from_slice(&2u32.to_be_bytes());
    let mut name = [0u8; 64];
    name[..9].copy_from_slice(b"Lichtpult");
    p.extend_from_slice(&name);
    p.extend_from_slice(&[priority, 0, 0, seq, 0]);
    p.extend_from_slice(&universe.to_be_bytes());
    p.extend_from_slice(&flags(total - 115));
    p.extend_from_slice(&[2, 0xa1, 0, 0, 0, 1]);
    p.extend_from_slice(&((n + 1) as u16).to_be_bytes());
    p.push(0);
    p.extend_from_slice(&data[..n]);
    p
}

pub fn sacn_multicast(universe: u16) -> String {
    format!("239.255.{}.{}:5568", universe >> 8, universe & 255)
}

fn path() -> std::path::PathBuf {
    crate::data_dir().join("output.json")
}

pub fn load() -> OutConfig {
    std::fs::read(path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

pub fn save(c: &OutConfig) {
    let _ = std::fs::create_dir_all(crate::data_dir());
    if let Ok(b) = serde_json::to_vec_pretty(c) {
        let _ = std::fs::write(path(), b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sacn_layout() {
        let mut d = [0u8; 512];
        d[0] = 7;
        let p = sacn_packet(&[1; 16], 5, 0x0102, 100, &d);
        assert_eq!(p.len(), 638);
        assert_eq!(&p[4..16], b"ASC-E1.17\0\0\0");
        assert_eq!(&p[16..18], &[0x72, 0x6e]); // 0x7000 | 622
        assert_eq!(&p[38..40], &[0x72, 0x58]); // 0x7000 | 600
        assert_eq!(&p[113..115], &[0x01, 0x02]);
        assert_eq!(&p[115..117], &[0x72, 0x0b]); // 0x7000 | 523
        assert_eq!(&p[123..125], &[0x02, 0x01]);
        assert_eq!(p[125], 0);
        assert_eq!(p[126], 7);
        assert_eq!(sacn_multicast(258), "239.255.1.2:5568");
    }
}
