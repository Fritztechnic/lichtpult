//! Fixture-Modell: Attribute, Typen und gepatchte Geräte.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Serialize, Deserialize)]
pub enum Attr {
    Dimmer,
    Red,
    Green,
    Blue,
    White,
    Amber,
    Uv,
    Pan,
    Tilt,
    Zoom,
    Strobe,
    /// Einzeln dimmbare Lichter eines Geräts (z. B. Lichtorgel).
    L1,
    L2,
    L3,
    L4,
    L5,
    L6,
    L7,
    L8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Kind {
    Intensity,
    Color,
    Position,
    Beam,
}

impl Attr {
    pub const ALL: [Attr; 19] = [
        Attr::Dimmer,
        Attr::Red,
        Attr::Green,
        Attr::Blue,
        Attr::White,
        Attr::Amber,
        Attr::Uv,
        Attr::Pan,
        Attr::Tilt,
        Attr::Zoom,
        Attr::Strobe,
        Attr::L1,
        Attr::L2,
        Attr::L3,
        Attr::L4,
        Attr::L5,
        Attr::L6,
        Attr::L7,
        Attr::L8,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Attr::Amber => "Amber",
            Attr::Uv => "UV",
            Attr::Dimmer => "Dimmer",
            Attr::Red => "Rot",
            Attr::Green => "Grün",
            Attr::Blue => "Blau",
            Attr::White => "Weiß",
            Attr::Pan => "Pan",
            Attr::Tilt => "Tilt",
            Attr::Zoom => "Zoom",
            Attr::Strobe => "Strobe",
            Attr::L1 => "Licht 1",
            Attr::L2 => "Licht 2",
            Attr::L3 => "Licht 3",
            Attr::L4 => "Licht 4",
            Attr::L5 => "Licht 5",
            Attr::L6 => "Licht 6",
            Attr::L7 => "Licht 7",
            Attr::L8 => "Licht 8",
        }
    }

    pub fn kind(self) -> Kind {
        match self {
            Attr::Dimmer | Attr::L1 | Attr::L2 | Attr::L3 | Attr::L4 | Attr::L5 | Attr::L6 | Attr::L7 | Attr::L8 => Kind::Intensity,
            Attr::Red | Attr::Green | Attr::Blue | Attr::White | Attr::Amber | Attr::Uv => Kind::Color,
            Attr::Pan | Attr::Tilt => Kind::Position,
            Attr::Zoom | Attr::Strobe => Kind::Beam,
        }
    }

    /// Attribute, die vom Grand Master skaliert werden.
    pub fn is_intensity_like(self) -> bool {
        matches!(self.kind(), Kind::Intensity | Kind::Color)
    }
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Intensity, Kind::Color, Kind::Position, Kind::Beam];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Intensity => "Intensität",
            Kind::Color => "Farbe",
            Kind::Position => "Position",
            Kind::Beam => "Beam",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FixtureType {
    pub name: String,
    /// `None` = belegter, aber (noch) nicht unterstützter Kanal.
    pub channels: Vec<Option<Attr>>,
}

pub fn builtin_types() -> Vec<FixtureType> {
    use Attr::*;
    let t = |name: &str, ch: &[Attr]| FixtureType {
        name: name.into(),
        channels: ch.iter().map(|a| Some(*a)).collect(),
    };
    vec![
        t("Dimmer", &[Dimmer]),
        t("RGB", &[Red, Green, Blue]),
        t("RGBW", &[Red, Green, Blue, White]),
        t("Dimmer + RGB", &[Dimmer, Red, Green, Blue]),
        t("Moving Head", &[Pan, Tilt, Dimmer, Red, Green, Blue, White, Zoom]),
        t("Uking Lichtorgel (12 Kanäle)", &[Pan, Tilt, Dimmer, Strobe, L1, L2, L3, L4, L5, L6, L7, L8]),
    ]
}

/// Importiert alle Modi einer Open-Fixture-Library-Datei als eigene Typen.
/// Fine-Kanäle und nicht unterstützte Funktionen bleiben als leere Kanäle erhalten.
pub fn import_ofl(json: &str) -> Result<Vec<FixtureType>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let name = v["name"].as_str().ok_or("kein OFL-Fixture (name fehlt)")?;
    let modes = v["modes"].as_array().ok_or("keine Modi gefunden")?;
    let attr_of = |channel: &str| -> Option<Attr> {
        let def = &v["availableChannels"][channel];
        let cap = if def["capability"].is_object() { &def["capability"] } else { &def["capabilities"][0] };
        match cap["type"].as_str()? {
            "Intensity" => Some(Attr::Dimmer),
            "ColorIntensity" => match cap["color"].as_str()? {
                "Red" => Some(Attr::Red),
                "Green" => Some(Attr::Green),
                "Blue" => Some(Attr::Blue),
                "White" => Some(Attr::White),
                "Amber" => Some(Attr::Amber),
                "UV" => Some(Attr::Uv),
                _ => None,
            },
            "Pan" | "PanContinuous" => Some(Attr::Pan),
            "Tilt" | "TiltContinuous" => Some(Attr::Tilt),
            "Zoom" => Some(Attr::Zoom),
            _ => None,
        }
    };
    let types: Vec<_> = modes
        .iter()
        .map(|m| {
            let mut used = Vec::new();
            let channels = m["channels"]
                .as_array()
                .map(|c| c.as_slice())
                .unwrap_or_default()
                .iter()
                .map(|c| {
                    let a = attr_of(c.as_str()?).filter(|a| !used.contains(a))?;
                    used.push(a);
                    Some(a)
                })
                .collect();
            let name = if modes.len() > 1 {
                format!("{name} ({})", m["name"].as_str().unwrap_or("?"))
            } else {
                name.to_string()
            };
            FixtureType { name, channels }
        })
        .collect();
    if types.is_empty() { Err("keine Modi gefunden".into()) } else { Ok(types) }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fixture {
    pub id: u32,
    pub name: String,
    /// Index in `Show::types`
    pub ty: usize,
    /// 1-basiert
    pub universe: u16,
    /// 1-basiert
    pub address: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_ofl_modes() {
        let json = r#"{"name":"RGB Fader","availableChannels":{
            "Red":{"capability":{"type":"ColorIntensity","color":"Red"}},
            "Green":{"capability":{"type":"ColorIntensity","color":"Green"}}},
          "modes":[{"name":"8 bit","channels":["Red","Green"]},
                   {"name":"16 bit","channels":["Red","Red fine","Green",null]}]}"#;
        let t = import_ofl(json).unwrap();
        assert_eq!(t[0].name, "RGB Fader (8 bit)");
        assert_eq!(t[1].channels, [Some(Attr::Red), None, Some(Attr::Green), None]);
    }
}
