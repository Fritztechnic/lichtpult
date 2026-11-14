//! Fixture-Modell: Attribute, Typen und gepatchte Geräte.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Serialize, Deserialize)]
pub enum Attr {
    Dimmer,
    Red,
    Green,
    Blue,
    White,
    Pan,
    Tilt,
    Zoom,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Kind {
    Intensity,
    Color,
    Position,
    Beam,
}

impl Attr {
    pub fn name(self) -> &'static str {
        match self {
            Attr::Dimmer => "Dimmer",
            Attr::Red => "Rot",
            Attr::Green => "Grün",
            Attr::Blue => "Blau",
            Attr::White => "Weiß",
            Attr::Pan => "Pan",
            Attr::Tilt => "Tilt",
            Attr::Zoom => "Zoom",
        }
    }

    pub fn kind(self) -> Kind {
        match self {
            Attr::Dimmer => Kind::Intensity,
            Attr::Red | Attr::Green | Attr::Blue | Attr::White => Kind::Color,
            Attr::Pan | Attr::Tilt => Kind::Position,
            Attr::Zoom => Kind::Beam,
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

pub struct FixtureType {
    pub name: &'static str,
    pub channels: &'static [Attr],
}

use Attr::*;

/// Eingebaute Typen; OFL-/GDTF-Import ersetzt das später.
pub const TYPES: &[FixtureType] = &[
    FixtureType { name: "Dimmer", channels: &[Dimmer] },
    FixtureType { name: "RGB", channels: &[Red, Green, Blue] },
    FixtureType { name: "RGBW", channels: &[Red, Green, Blue, White] },
    FixtureType { name: "Dimmer + RGB", channels: &[Dimmer, Red, Green, Blue] },
    FixtureType {
        name: "Moving Head",
        channels: &[Pan, Tilt, Dimmer, Red, Green, Blue, White, Zoom],
    },
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fixture {
    pub id: u32,
    pub name: String,
    pub ty: usize,
    /// 1-basiert
    pub universe: u16,
    /// 1-basiert
    pub address: u16,
}

impl Fixture {
    pub fn channels(&self) -> &'static [Attr] {
        TYPES[self.ty].channels
    }

    pub fn range(&self) -> std::ops::Range<u16> {
        self.address..self.address + self.channels().len() as u16
    }
}
