//! Show-Daten (speicherbar) und Engine (Laufzeitzustand + Rendering).

use crate::fixture::{Attr, Fixture, FixtureType, Kind, builtin_types};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const CHANNELS: usize = 512;
pub type Frame = [u8; CHANNELS];
/// Fixture-ID -> Attribut -> Wert
pub type Values = BTreeMap<u32, BTreeMap<Attr, u8>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub ids: Vec<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Palette {
    pub name: String,
    pub kind: Kind,
    pub values: BTreeMap<Attr, u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cue {
    pub name: String,
    pub fade: f32,
    pub values: Values,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Show {
    pub name: String,
    #[serde(default = "builtin_types")]
    pub types: Vec<FixtureType>,
    pub fixtures: Vec<Fixture>,
    pub groups: Vec<Group>,
    pub palettes: Vec<Palette>,
    pub cues: Vec<Cue>,
}

impl Default for Show {
    fn default() -> Self {
        Show {
            name: String::new(),
            types: builtin_types(),
            fixtures: vec![],
            groups: vec![],
            palettes: vec![],
            cues: vec![],
        }
    }
}

impl Show {
    pub fn channels(&self, f: &Fixture) -> &[Option<Attr>] {
        &self.types[f.ty].channels
    }

    pub fn has(&self, f: &Fixture, a: Attr) -> bool {
        self.channels(f).contains(&Some(a))
    }

    /// Belegte DMX-Adressen (1-basiert, Ende exklusiv).
    pub fn range(&self, f: &Fixture) -> std::ops::Range<u16> {
        f.address..f.address + self.channels(f).len() as u16
    }
}

/// Alles, was ein Bedienelement (UI, MIDI, Pult) tun kann.
/// Hardware-Controller werden später auf diese Aktionen abgebildet.
#[derive(Clone, Debug)]
pub enum Action {
    AddTypes(Vec<FixtureType>),
    Select(u32),
    Toggle(u32),
    SelectGroup(usize),
    SelectAll,
    ClearSelection,
    SetAttr(Attr, u8),
    ClearProgrammer,
    RecordGroup,
    RecordPalette(Kind),
    ApplyPalette(usize),
    RecordCue,
    Go,
    Back,
    GotoCue(usize),
    SetCueFade(usize, f32),
    DeleteCue(usize),
    Blackout(bool),
    Master(u8),
    AddFixtures { ty: usize, count: u16, universe: u16, address: u16 },
    RemoveFixture(u32),
}

pub struct Engine {
    pub show: Show,
    pub selection: BTreeSet<u32>,
    pub programmer: Values,
    pub master: u8,
    pub blackout: bool,
    pub cue: Option<usize>,
    pub fade_time: f32,
    fade_t: f32,
    from: Values,
    to: Values,
    pub frames: BTreeMap<u16, Frame>,
}

fn get(v: &Values, id: u32, a: Attr) -> u8 {
    v.get(&id).and_then(|m| m.get(&a)).copied().unwrap_or(0)
}

impl Engine {
    pub fn new(show: Show) -> Self {
        Engine {
            show,
            selection: BTreeSet::new(),
            programmer: Values::new(),
            master: 255,
            blackout: false,
            cue: None,
            fade_time: 0.0,
            fade_t: 0.0,
            from: Values::new(),
            to: Values::new(),
            frames: BTreeMap::new(),
        }
    }

    pub fn load_show(&mut self, show: Show) {
        *self = Engine::new(show);
    }

    /// Zustand der Cue-Liste inkl. laufendem Fade.
    pub fn playback_values(&self) -> Values {
        let k = if self.fade_time <= 0.0 { 1.0 } else { (self.fade_t / self.fade_time).min(1.0) };
        if k >= 1.0 {
            return self.to.clone();
        }
        let mut out = Values::new();
        for (&id, attrs) in self.from.iter().chain(self.to.iter()) {
            for &a in attrs.keys() {
                let (x, y) = (get(&self.from, id, a) as f32, get(&self.to, id, a) as f32);
                out.entry(id).or_default().insert(a, (x + (y - x) * k).round() as u8);
            }
        }
        out
    }

    /// Tracking: Cue-Zustand = alle Änderungen bis einschließlich Cue `i`.
    fn state_at(&self, i: usize) -> Values {
        let mut out = Values::new();
        for cue in &self.show.cues[..=i] {
            for (&id, attrs) in &cue.values {
                out.entry(id).or_default().extend(attrs);
            }
        }
        out
    }

    fn goto(&mut self, cue: Option<usize>) {
        self.from = self.playback_values();
        self.to = cue.map(|i| self.state_at(i)).unwrap_or_default();
        self.fade_time = cue.map_or(0.0, |i| self.show.cues[i].fade);
        self.fade_t = 0.0;
        self.cue = cue;
    }

    pub fn apply(&mut self, action: Action) {
        match action {
            Action::Select(id) => {
                self.selection = BTreeSet::from([id]);
            }
            Action::Toggle(id) => {
                if !self.selection.remove(&id) {
                    self.selection.insert(id);
                }
            }
            Action::SelectGroup(i) => {
                if let Some(g) = self.show.groups.get(i) {
                    self.selection = g.ids.iter().copied().collect();
                }
            }
            Action::SelectAll => self.selection = self.show.fixtures.iter().map(|f| f.id).collect(),
            Action::ClearSelection => self.selection.clear(),
            Action::SetAttr(a, v) => {
                for f in self.show.fixtures.iter().filter(|f| self.selection.contains(&f.id)) {
                    if self.show.has(f, a) {
                        self.programmer.entry(f.id).or_default().insert(a, v);
                    }
                }
            }
            Action::ClearProgrammer => self.programmer.clear(),
            Action::RecordGroup => {
                if !self.selection.is_empty() {
                    let name = format!("Gruppe {}", self.show.groups.len() + 1);
                    self.show.groups.push(Group { name, ids: self.selection.iter().copied().collect() });
                }
            }
            Action::RecordPalette(kind) => {
                let values: BTreeMap<Attr, u8> = self
                    .selection
                    .iter()
                    .filter_map(|id| self.programmer.get(id))
                    .flat_map(|m| m.iter())
                    .filter(|(a, _)| a.kind() == kind)
                    .map(|(a, v)| (*a, *v))
                    .collect();
                if !values.is_empty() {
                    let name = format!("{} {}", kind.name(), self.show.palettes.iter().filter(|p| p.kind == kind).count() + 1);
                    self.show.palettes.push(Palette { name, kind, values });
                }
            }
            Action::ApplyPalette(i) => {
                if let Some(p) = self.show.palettes.get(i).cloned() {
                    for (a, v) in p.values {
                        self.apply(Action::SetAttr(a, v));
                    }
                }
            }
            Action::RecordCue => {
                if !self.programmer.is_empty() {
                    let name = format!("Cue {}", self.show.cues.len() + 1);
                    self.show.cues.push(Cue { name, fade: 2.0, values: self.programmer.clone() });
                }
            }
            Action::Go => {
                let next = self.cue.map_or(0, |c| c + 1);
                if next < self.show.cues.len() {
                    self.goto(Some(next));
                }
            }
            Action::Back => match self.cue {
                Some(0) => self.goto(None),
                Some(c) => self.goto(Some(c - 1)),
                None => {}
            },
            Action::GotoCue(i) => {
                if i < self.show.cues.len() {
                    self.goto(Some(i));
                }
            }
            Action::SetCueFade(i, t) => {
                if let Some(c) = self.show.cues.get_mut(i) {
                    c.fade = t.max(0.0);
                }
            }
            Action::DeleteCue(i) => {
                if i < self.show.cues.len() {
                    self.show.cues.remove(i);
                    self.goto(None);
                }
            }
            Action::Blackout(b) => self.blackout = b,
            Action::Master(m) => self.master = m,
            Action::AddFixtures { ty, count, universe, address } => {
                let Some(t) = self.show.types.get(ty) else { return };
                let (width, name) = (t.channels.len() as u32, t.name.clone());
                let mut next_id = self.show.fixtures.iter().map(|f| f.id).max().unwrap_or(0) + 1;
                for n in 0..count as u32 {
                    let address = address as u32 + n * width;
                    if width == 0 || address + width - 1 > CHANNELS as u32 {
                        break;
                    }
                    self.show.fixtures.push(Fixture {
                        id: next_id,
                        name: format!("{name} {next_id}"),
                        ty,
                        universe,
                        address: address as u16,
                    });
                    next_id += 1;
                }
            }
            Action::AddTypes(types) => {
                for t in types {
                    if !self.show.types.iter().any(|x| x.name == t.name) {
                        self.show.types.push(t);
                    }
                }
            }
            Action::RemoveFixture(id) => {
                self.show.fixtures.retain(|f| f.id != id);
                self.selection.remove(&id);
                self.programmer.remove(&id);
                for g in &mut self.show.groups {
                    g.ids.retain(|&x| x != id);
                }
            }
        }
    }

    /// Fixture-IDs, deren Adressbereich sich mit einem anderen überschneidet.
    pub fn conflicts(&self) -> BTreeSet<u32> {
        let fx = &self.show.fixtures;
        let mut out = BTreeSet::new();
        for (i, a) in fx.iter().enumerate() {
            for b in &fx[i + 1..] {
                let (ra, rb) = (self.show.range(a), self.show.range(b));
                if a.universe == b.universe && ra.start < rb.end && rb.start < ra.end {
                    out.insert(a.id);
                    out.insert(b.id);
                }
            }
        }
        out
    }

    /// Schritt `dt` Sekunden weiter und DMX-Frames neu berechnen.
    pub fn tick(&mut self, dt: f32) {
        self.fade_t += dt;
        let play = self.playback_values();
        self.frames.clear();
        for f in &self.show.fixtures {
            let frame = self.frames.entry(f.universe).or_insert([0; CHANNELS]);
            for (i, slot) in self.show.channels(f).iter().enumerate() {
                let Some(a) = *slot else { continue };
                let v = self
                    .programmer
                    .get(&f.id)
                    .and_then(|m| m.get(&a))
                    .copied()
                    .unwrap_or_else(|| get(&play, f.id, a));
                let v = if self.blackout && a.is_intensity_like() {
                    0
                } else if a.is_intensity_like() {
                    (v as u16 * self.master as u16 / 255) as u8
                } else {
                    v
                };
                if let Some(out) = frame.get_mut(f.address as usize - 1 + i) {
                    *out = v;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        let mut e = Engine::new(Show::default());
        e.apply(Action::AddFixtures { ty: 3, count: 2, universe: 1, address: 1 });
        e
    }

    #[test]
    fn programmer_renders_to_dmx() {
        let mut e = engine();
        e.apply(Action::SelectAll);
        e.apply(Action::SetAttr(Attr::Dimmer, 200));
        e.tick(0.0);
        assert_eq!(e.frames[&1][0], 200);
        assert_eq!(e.frames[&1][4], 200);
    }

    #[test]
    fn master_and_blackout_scale_intensity() {
        let mut e = engine();
        e.apply(Action::SelectAll);
        e.apply(Action::SetAttr(Attr::Dimmer, 200));
        e.apply(Action::Master(128));
        e.tick(0.0);
        assert_eq!(e.frames[&1][0], 100);
        e.apply(Action::Blackout(true));
        e.tick(0.0);
        assert_eq!(e.frames[&1][0], 0);
    }

    #[test]
    fn cue_fades_and_tracks() {
        let mut e = engine();
        e.apply(Action::SelectAll);
        e.apply(Action::SetAttr(Attr::Dimmer, 100));
        e.apply(Action::RecordCue);
        e.apply(Action::SetAttr(Attr::Red, 50));
        e.apply(Action::RecordCue);
        e.apply(Action::ClearProgrammer);
        e.apply(Action::Go);
        e.tick(1.0);
        assert_eq!(e.frames[&1][0], 50); // halber Fade (2 s)
        e.tick(5.0);
        assert_eq!(e.frames[&1][0], 100);
        e.apply(Action::Go);
        e.tick(5.0);
        assert_eq!(e.frames[&1][0], 100); // Dimmer wird getrackt
        assert_eq!(e.frames[&1][1], 50);
    }

    #[test]
    fn detects_overlap() {
        let mut e = engine();
        e.apply(Action::AddFixtures { ty: 3, count: 1, universe: 1, address: 3 });
        assert_eq!(e.conflicts().len(), 3); // 3–6 überlappt 1–4 und 5–8
    }
}
