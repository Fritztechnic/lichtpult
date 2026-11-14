//! Show-Daten (speicherbar) und Engine (Laufzeitzustand + Rendering).

use crate::fixture::{Attr, Fixture, FixtureType, Kind, builtin_types};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

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
    /// Auto-Follow: Sekunden nach Ende des Fades bis zum nächsten Cue.
    #[serde(default)]
    pub follow: Option<f32>,
    pub values: Values,
    /// Verzögerung vor Beginn der Überblendung (Sekunden).
    #[serde(default)]
    pub delay: f32,
    #[serde(default)]
    pub comment: String,
    /// Szene: enthält den vollständigen Look und erbt nichts von vorherigen Cues.
    #[serde(default)]
    pub block: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cuelist {
    pub name: String,
    pub cues: Vec<Cue>,
    /// Nach dem letzten Cue wieder bei Cue 1 beginnen (Chase).
    #[serde(default)]
    pub looped: bool,
    /// Chase: Cues laufen automatisch im Takt `bpm`.
    #[serde(default)]
    pub chase: bool,
    #[serde(default = "default_bpm")]
    pub bpm: f32,
}

fn default_bpm() -> f32 {
    80.0
}

impl Cuelist {
    pub fn new(name: impl Into<String>) -> Self {
        Cuelist { name: name.into(), cues: vec![], looped: false, chase: false, bpm: 80.0 }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Shape {
    Sine,
    Triangle,
    Saw,
    Square,
    Cosine,
    RampDown,
    Step,
    Bounce,
}

impl Shape {
    pub const ALL: [Shape; 8] =
        [Shape::Sine, Shape::Cosine, Shape::Triangle, Shape::Saw, Shape::RampDown, Shape::Square, Shape::Step, Shape::Bounce];

    pub fn name(self) -> &'static str {
        match self {
            Shape::Sine => "Sinus",
            Shape::Triangle => "Dreieck",
            Shape::Saw => "Sägezahn",
            Shape::Square => "Rechteck",
            Shape::Cosine => "Cosinus",
            Shape::RampDown => "Rampe ab",
            Shape::Step => "Stufen",
            Shape::Bounce => "Bounce",
        }
    }

    /// Phase 0..1 -> 0..1
    fn wave(self, p: f32) -> f32 {
        let p = p.rem_euclid(1.0);
        match self {
            Shape::Sine => 0.5 - 0.5 * (p * std::f32::consts::TAU).cos(),
            Shape::Triangle => 1.0 - (2.0 * p - 1.0).abs(),
            Shape::Saw => p,
            Shape::Square => (p < 0.5) as u8 as f32,
            Shape::Cosine => 0.5 + 0.5 * (p * std::f32::consts::TAU).cos(),
            Shape::RampDown => 1.0 - p,
            Shape::Step => (p * 4.0).floor() / 3.0,
            Shape::Bounce => 1.0 - (1.0 - (2.0 * p - 1.0).abs()).powi(2),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Target {
    /// Dimmer (oder Farbkanäle ohne Dimmer) pulsieren.
    Intensity,
    /// Regenbogen über die RGB-Kanäle.
    Color,
    /// Kreisbewegung um die Grundposition.
    Position,
}

impl Target {
    pub const ALL: [Target; 3] = [Target::Intensity, Target::Color, Target::Position];

    pub fn name(self) -> &'static str {
        match self {
            Target::Intensity => "Intensität",
            Target::Color => "Regenbogen",
            Target::Position => "Kreis",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Effect {
    pub name: String,
    pub target: Target,
    pub shape: Shape,
    /// Zyklen pro Sekunde
    pub rate: f32,
    /// Stärke (Tiefe bzw. Radius), 0..255
    pub size: u8,
    /// Phasenversatz über die Fixtures, 0..1 (1 = ein voller Zyklus verteilt)
    pub spread: f32,
    pub ids: Vec<u32>,
    #[serde(skip)]
    pub running: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropKind {
    Podest,
    Truss,
    Lautsprecher,
    Pult,
    Publikum,
    Text,
    Vorhang,
    Mikrofon,
    Treppe,
    Bildschirm,
    Rechteck,
}

impl PropKind {
    pub const ALL: [PropKind; 11] = [
        PropKind::Podest,
        PropKind::Truss,
        PropKind::Lautsprecher,
        PropKind::Pult,
        PropKind::Publikum,
        PropKind::Text,
        PropKind::Vorhang,
        PropKind::Mikrofon,
        PropKind::Treppe,
        PropKind::Bildschirm,
        PropKind::Rechteck,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PropKind::Podest => "Podest",
            PropKind::Truss => "Traverse",
            PropKind::Lautsprecher => "Lautsprecher",
            PropKind::Pult => "Pult",
            PropKind::Publikum => "Publikum",
            PropKind::Text => "Beschriftung",
            PropKind::Vorhang => "Vorhang",
            PropKind::Mikrofon => "Mikrofon",
            PropKind::Treppe => "Treppe",
            PropKind::Bildschirm => "Bildschirm",
            PropKind::Rechteck => "Rechteck",
        }
    }

    /// Startgröße als Anteil der Bühne.
    fn size(self) -> (f32, f32) {
        match self {
            PropKind::Podest => (0.2, 0.14),
            PropKind::Truss => (0.5, 0.025),
            PropKind::Lautsprecher => (0.05, 0.08),
            PropKind::Pult => (0.1, 0.07),
            PropKind::Publikum => (0.6, 0.12),
            PropKind::Text => (0.14, 0.05),
            PropKind::Vorhang => (0.5, 0.03),
            PropKind::Mikrofon => (0.03, 0.05),
            PropKind::Treppe => (0.1, 0.1),
            PropKind::Bildschirm => (0.25, 0.03),
            PropKind::Rechteck => (0.15, 0.1),
        }
    }
}

/// Dekoration auf der Bühnenansicht (Mittelpunkt und Größe als Anteil der Bühne).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prop {
    pub kind: PropKind,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub label: String,
    #[serde(default)]
    pub color: Option<[u8; 3]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Show {
    pub name: String,
    #[serde(default = "builtin_types")]
    pub types: Vec<FixtureType>,
    pub fixtures: Vec<Fixture>,
    pub groups: Vec<Group>,
    pub palettes: Vec<Palette>,
    /// Altes Format (eine Cuelist); wird beim Laden nach `lists` übernommen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cues: Vec<Cue>,
    #[serde(default)]
    pub lists: Vec<Cuelist>,
    #[serde(default)]
    pub effects: Vec<Effect>,
    #[serde(default)]
    pub props: Vec<Prop>,
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
            lists: vec![Cuelist::new("Cuelist 1")],
            effects: vec![],
            props: vec![],
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

    /// Farb-Fixtures ohne eigenen Dimmer bekommen einen virtuellen Dimmer, der die Farbkanäle skaliert.
    pub fn virtual_dim(&self, f: &Fixture) -> bool {
        !self.has(f, Attr::Dimmer) && [Attr::Red, Attr::Green, Attr::Blue, Attr::White].iter().any(|&c| self.has(f, c))
    }

    /// Belegte DMX-Adressen (1-basiert, Ende exklusiv).
    pub fn range(&self, f: &Fixture) -> std::ops::Range<u16> {
        f.address..f.address + self.channels(f).len() as u16
    }

    /// Erste Adresse im Universum, ab der `width` Kanäle frei sind.
    pub fn free_address(&self, universe: u16, width: u16) -> Option<u16> {
        let mut used = [false; 513];
        for f in self.fixtures.iter().filter(|f| f.universe == universe) {
            for a in self.range(f) {
                if let Some(u) = used.get_mut(a as usize) {
                    *u = true;
                }
            }
        }
        (1..=(513 - width.min(512))).find(|&a| (a..a + width).all(|c| !used[c as usize]))
    }

    /// Altes Format übernehmen und mindestens eine Cuelist sicherstellen.
    fn normalize(&mut self) {
        if self.lists.is_empty() {
            self.lists.push(Cuelist::new("Cuelist 1"));
        }
        if !self.cues.is_empty() {
            self.lists[0].cues.append(&mut self.cues);
        }
    }
}

/// Alles, was ein Bedienelement (UI, MIDI, Pult) tun kann.
/// Hardware-Controller werden auf diese Aktionen abgebildet.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    AddTypes(Vec<FixtureType>),
    /// Fixture-Typ ersetzen (Name muss frei bleiben); gepatchte Fixtures behalten den Index.
    UpdateType(usize, FixtureType),
    /// Ungenutzten Fixture-Typ löschen; Indizes der übrigen Typen rücken nach.
    RemoveType(usize),
    Select(u32),
    Toggle(u32),
    /// Auswahl um alle Fixtures mit ID von..=bis erweitern.
    SelectRange(u32, u32),
    /// Fixture-IDs ersetzen die Auswahl.
    SelectIds(Vec<u32>),
    Deselect(u32),
    SelectGroup(usize),
    SelectAll,
    ClearSelection,
    InvertSelection,
    /// Jedes n-te Fixture der aktuellen Auswahl behalten (ab Offset).
    SelectEvery(usize, usize),
    /// Auswahl um ein Fixture weiter (+1) oder zurück (-1).
    SelectStep(i32),
    /// Werte der Auswahl linear von `from` nach `to` auffächern.
    Fan(Attr, u8, u8, bool),
    SetBlind(bool),
    Highlight(bool),
    Park(Attr),
    Unpark,
    /// Nächsten Cue sofort ohne Überblendung.
    Snap(usize),
    SetCueDelay(usize, f32),
    SetCueComment(usize, String),
    SetChase(usize, bool),
    SetBpm(usize, f32),
    /// Globale Überblendgeschwindigkeit (0.1..10).
    FadeSpeed(f32),
    SetAttr(Attr, u8),
    ClearProgrammer,
    RecordGroup,
    RecordPalette(Kind),
    ApplyPalette(usize),
    RecordCue,
    /// Aktuellen Look als eigenständige Szene speichern, Programmer leeren und abspielen.
    StoreScene,
    /// Nächsten Cue der Cuelist starten.
    Go(usize),
    Back(usize),
    /// Cuelist ausschalten.
    ReleaseList(usize),
    /// Cuelist-Fader (0..255), skaliert Intensitäten.
    SetLevel(usize, u8),
    /// Cue der aktiven Cuelist anspringen.
    GotoCue(usize),
    SetCueFade(usize, f32),
    SetCueFollow(usize, Option<f32>),
    DeleteCue(usize),
    /// Szene von Position `from` nach `to` verschieben.
    MoveCue(usize, usize),
    DuplicateCue(usize),
    RenameCue(usize, String),
    /// Programmer-Werte in den Cue übernehmen.
    UpdateCue(usize),
    SelectList(usize),
    AddList,
    DeleteList(usize),
    RenameList(usize, String),
    SetLooped(usize, bool),
    RenameGroup(usize, String),
    DeleteGroup(usize),
    /// Gruppe mit der aktuellen Auswahl überschreiben.
    UpdateGroup(usize),
    RenamePalette(usize, String),
    DeletePalette(usize),
    /// Palette mit den aktuellen Programmer-Werten überschreiben.
    UpdatePalette(usize),
    /// Attribut der ausgewählten Fixtures aus dem Programmer entfernen.
    ReleaseAttr(Attr),
    RenameFixture(u32, String),
    SetPatch { id: u32, universe: u16, address: u16 },
    SetPosition(u32, f32, f32),
    AddProp(PropKind),
    SetProp(usize, Prop),
    DeleteProp(usize),
    DuplicateProp(usize),
    /// Reihenfolge ändern (höherer Index = weiter vorn).
    MoveProp(usize, usize),
    /// Neuen Effekt für die aktuelle Auswahl anlegen.
    AddEffect(Target, Shape),
    SetEffect(usize, Effect),
    DeleteEffect(usize),
    ToggleEffect(usize),
    StopEffects,
    Blackout(bool),
    Master(u8),
    AddFixtures { ty: usize, count: u16, universe: u16, address: u16 },
    RemoveFixture(u32),
    /// Fixtures eines Universums lückenlos ab Adresse 1 neu adressieren (ID-Reihenfolge).
    AutoAddress(u16),
    Undo,
    Redo,
}

impl Action {
    /// Verändert die gespeicherte Show (rückgängig-machbar).
    fn edits_show(&self) -> bool {
        use Action::*;
        matches!(
            self,
            AddTypes(_)
                | RecordGroup
                | RecordPalette(_)
                | RecordCue
                | StoreScene
                | SetCueFade(..)
                | SetCueFollow(..)
                | SetCueDelay(..)
                | SetCueComment(..)
                | SetChase(..)
                | SetBpm(..)
                | DeleteCue(_)
                | MoveCue(..)
                | DuplicateCue(_)
                | RenameCue(..)
                | UpdateCue(_)
                | AddList
                | DeleteList(_)
                | RenameList(..)
                | SetLooped(..)
                | RenameGroup(..)
                | DeleteGroup(_)
                | UpdateGroup(_)
                | RenamePalette(..)
                | DeletePalette(_)
                | UpdatePalette(_)
                | RenameFixture(..)
                | SetPatch { .. }
                | SetPosition(..)
                | AddProp(_)
                | DuplicateProp(_)
                | MoveProp(..)
                | SetProp(..)
                | DeleteProp(_)
                | AddEffect(..)
                | SetEffect(..)
                | DeleteEffect(_)
                | AddFixtures { .. }
                | UpdateType(..)
                | RemoveType(_)
                | RemoveFixture(_)
                | AutoAddress(_)
        )
    }
}

/// Laufzeitzustand einer Cuelist.
#[derive(Clone)]
pub struct Player {
    pub cue: Option<usize>,
    pub level: u8,
    from: Values,
    to: Values,
    fade_time: f32,
    fade_t: f32,
    delay: f32,
    order: u64,
}

impl Player {
    fn new() -> Self {
        Player { cue: None, level: 255, from: Values::new(), to: Values::new(), fade_time: 0.0, fade_t: 0.0, delay: 0.0, order: 0 }
    }

    fn values(&self) -> Values {
        let k = if self.fade_time <= 0.0 { f32::from(self.fade_t >= self.delay) } else { ((self.fade_t - self.delay) / self.fade_time).clamp(0.0, 1.0) };
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
}

pub struct Engine {
    pub show: Show,
    pub selection: BTreeSet<u32>,
    pub programmer: Values,
    pub master: u8,
    pub blackout: bool,
    /// Blind: Programmer wird nicht ausgegeben.
    pub blind: bool,
    pub highlight: bool,
    /// Zähler, der bei jeder Änderung der gespeicherten Show wächst.
    pub rev: u64,
    /// Geparkte Werte überschreiben alles andere.
    pub parked: Values,
    pub fade_speed: f32,
    /// Zuletzt gewähltes Fixture (rot im Pult).
    pub last_selected: Option<u32>,
    chase_t: Vec<f32>,
    /// Aktive (bearbeitete) Cuelist.
    pub active: usize,
    pub players: Vec<Player>,
    fx_t: f32,
    go_counter: u64,
    undo: Vec<Show>,
    redo: Vec<Show>,
    last_edit: Option<(std::mem::Discriminant<Action>, Instant)>,
    pub frames: BTreeMap<u16, Frame>,
}

fn get(v: &Values, id: u32, a: Attr) -> u8 {
    v.get(&id).and_then(|m| m.get(&a)).copied().unwrap_or(0)
}

fn hue_rgb(h: f32) -> [f32; 3] {
    let h = h.rem_euclid(1.0) * 6.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    match h as u32 {
        0 => [1.0, x, 0.0],
        1 => [x, 1.0, 0.0],
        2 => [0.0, 1.0, x],
        3 => [0.0, x, 1.0],
        4 => [x, 0.0, 1.0],
        _ => [1.0, 0.0, x],
    }
}

impl Engine {
    pub fn new(mut show: Show) -> Self {
        show.normalize();
        let n = show.lists.len();
        Engine {
            show,
            selection: BTreeSet::new(),
            programmer: Values::new(),
            master: 255,
            blackout: false,
            blind: false,
            highlight: false,
            rev: 1,
            parked: Values::new(),
            fade_speed: 1.0,
            last_selected: None,
            chase_t: vec![0.0; n],
            active: 0,
            players: vec![Player::new(); n],
            fx_t: 0.0,
            go_counter: 0,
            undo: vec![],
            redo: vec![],
            last_edit: None,
            frames: BTreeMap::new(),
        }
    }

    pub fn load_show(&mut self, show: Show) {
        let rev = self.rev + 1;
        *self = Engine::new(show);
        self.rev = rev;
    }

    /// Aktive Cuelist.
    pub fn list(&self) -> &Cuelist {
        &self.show.lists[self.active]
    }

    /// Aktueller Cue der aktiven Cuelist.
    pub fn cue(&self) -> Option<usize> {
        self.players[self.active].cue
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Zusammengeführter Zustand aller Cuelists: Intensitäten HTP, alles andere LTP.
    pub fn playback_values(&self) -> Values {
        let mut order: Vec<&Player> = self.players.iter().collect();
        order.sort_by_key(|p| p.order);
        let mut out = Values::new();
        for p in order {
            for (id, attrs) in p.values() {
                for (a, v) in attrs {
                    let slot = out.entry(id).or_default().entry(a).or_insert(0);
                    if a.is_intensity_like() {
                        *slot = (*slot).max((v as u16 * p.level as u16 / 255) as u8);
                    } else {
                        *slot = v;
                    }
                }
            }
        }
        out
    }

    /// Tracking: Cue-Zustand = alle Änderungen bis einschließlich Cue `i`.
    fn state_at(&self, l: usize, i: usize) -> Values {
        let mut out = Values::new();
        let start = self.show.lists[l].cues[..=i].iter().rposition(|c| c.block).unwrap_or(0);
        for cue in &self.show.lists[l].cues[start..=i] {
            for (&id, attrs) in &cue.values {
                out.entry(id).or_default().extend(attrs);
            }
        }
        out
    }

    fn goto(&mut self, l: usize, cue: Option<usize>) {
        let from = self.players[l].values();
        let to = cue.map(|i| self.state_at(l, i)).unwrap_or_default();
        let fade = cue.map_or(0.0, |i| self.show.lists[l].cues[i].fade);
        let delay = cue.map_or(0.0, |i| self.show.lists[l].cues[i].delay);
        self.go_counter += 1;
        self.chase_t[l] = 0.0;
        let p = &mut self.players[l];
        (p.from, p.to, p.fade_time, p.fade_t, p.delay, p.cue, p.order) = (from, to, fade, 0.0, delay, cue, self.go_counter);
    }

    fn go_next(&mut self, l: usize) {
        let len = self.show.lists[l].cues.len();
        let next = self.players[l].cue.map_or(0, |c| c + 1);
        if next < len {
            self.goto(l, Some(next));
        } else if self.show.lists[l].looped && len > 0 {
            self.goto(l, Some(0));
        }
    }

    /// Cue-Zustand ohne Fade neu berechnen (nach Änderungen an Cues).
    fn refresh_playback(&mut self, l: usize) {
        if let Some(c) = self.players[l].cue {
            let to = self.state_at(l, c);
            let p = &mut self.players[l];
            p.to = to;
            p.fade_time = 0.0;
        }
    }

    /// Spieler an die Anzahl Cuelists/Cues anpassen (nach Undo, Laden, Löschen).
    fn sync_players(&mut self) {
        self.players.resize(self.show.lists.len(), Player::new());
        self.chase_t.resize(self.players.len(), 0.0);
        self.active = self.active.min(self.show.lists.len() - 1);
        for l in 0..self.players.len() {
            match self.players[l].cue {
                Some(c) if c >= self.show.lists[l].cues.len() => self.goto(l, None),
                Some(_) => self.refresh_playback(l),
                None => {}
            }
        }
    }

    fn programmer_values(&self, kind: Kind) -> BTreeMap<Attr, u8> {
        self.selection
            .iter()
            .filter_map(|id| self.programmer.get(id))
            .flat_map(|m| m.iter())
            .filter(|(a, _)| a.kind() == kind)
            .map(|(a, v)| (*a, *v))
            .collect()
    }

    /// Anzeigewert eines Attributs (Programmer vor Cue) und ob er aus dem Programmer stammt.
    pub fn value_of(&self, play: &Values, id: u32, a: Attr) -> (u8, bool) {
        match self.programmer.get(&id).and_then(|m| m.get(&a)) {
            Some(&v) => (v, true),
            None => (get(play, id, a), false),
        }
    }

    /// Fortschritt der laufenden Überblendung (0..1) einer Cueliste.
    pub fn fade_progress(&self, l: usize) -> f32 {
        self.players.get(l).map_or(1.0, |p| if p.fade_time <= 0.0 { 1.0 } else { ((p.fade_t - p.delay) / p.fade_time).clamp(0.0, 1.0) })
    }

    /// Tatsächlich ausgegebener DMX-Wert (inkl. Master/Blackout).
    pub fn output_of(&self, f: &Fixture, a: Attr) -> u8 {
        let Some(i) = self.show.channels(f).iter().position(|c| *c == Some(a)) else { return 0 };
        self.frames.get(&f.universe).and_then(|fr| fr.get(f.address as usize - 1 + i)).copied().unwrap_or(0)
    }

    fn snapshot(&mut self, action: &Action) {
        let now = Instant::now();
        let kind = std::mem::discriminant(action);
        // Gleiche Aktion in schneller Folge (z. B. Ziehen eines Werts) = ein Undo-Schritt.
        let merge = matches!(self.last_edit, Some((k, t)) if k == kind && now - t < Duration::from_millis(800));
        self.last_edit = Some((kind, now));
        if !merge {
            self.undo.push(self.show.clone());
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
    }

    fn restore(&mut self, mut show: Show) {
        for (e, old) in show.effects.iter_mut().zip(&self.show.effects) {
            e.running = old.running;
        }
        self.show = show;
        self.sync_players();
    }

    pub fn apply(&mut self, action: Action) {
        if action.edits_show() || matches!(action, Action::Undo | Action::Redo) {
            self.rev += 1;
        }
        if action.edits_show() {
            self.snapshot(&action);
        }
        let l = self.active;
        match action {
            Action::Undo => {
                if let Some(prev) = self.undo.pop() {
                    self.redo.push(self.show.clone());
                    self.restore(prev);
                    self.last_edit = None;
                }
            }
            Action::Redo => {
                if let Some(next) = self.redo.pop() {
                    self.undo.push(self.show.clone());
                    self.restore(next);
                    self.last_edit = None;
                }
            }
            Action::Select(id) => {
                self.selection = BTreeSet::from([id]);
                self.last_selected = Some(id);
            }
            Action::Toggle(id) => {
                if !self.selection.remove(&id) {
                    self.selection.insert(id);
                    self.last_selected = Some(id);
                }
            }
            Action::Deselect(id) => {
                self.selection.remove(&id);
            }
            Action::SelectRange(a, b) => {
                let (a, b) = (a.min(b), a.max(b));
                self.selection.extend(self.show.fixtures.iter().map(|f| f.id).filter(|id| (a..=b).contains(id)));
            }
            Action::SelectIds(ids) => {
                self.selection = self.show.fixtures.iter().map(|f| f.id).filter(|id| ids.contains(id)).collect();
            }
            Action::SelectGroup(i) => {
                if let Some(g) = self.show.groups.get(i) {
                    self.selection = g.ids.iter().copied().collect();
                }
            }
            Action::SelectAll => self.selection = self.show.fixtures.iter().map(|f| f.id).collect(),
            Action::ClearSelection => self.selection.clear(),
            Action::InvertSelection => {
                self.selection = self.show.fixtures.iter().map(|f| f.id).filter(|id| !self.selection.contains(id)).collect();
            }
            Action::SelectEvery(n, offset) => {
                let n = n.max(1);
                self.selection = self.selection.iter().copied().enumerate().filter(|(i, _)| i % n == offset % n).map(|(_, id)| id).collect();
            }
            Action::SelectStep(d) => {
                let ids: Vec<u32> = self.show.fixtures.iter().map(|f| f.id).collect();
                if !ids.is_empty() {
                    let cur = self.selection.iter().next().and_then(|id| ids.iter().position(|x| x == id));
                    let i = cur.map_or(0, |i| (i as i32 + d).rem_euclid(ids.len() as i32) as usize);
                    self.selection = BTreeSet::from([ids[i]]);
                    self.last_selected = Some(ids[i]);
                }
            }
            Action::Fan(attr, from, to, reverse) => {
                let mut ids: Vec<u32> = self.show.fixtures.iter().filter(|f| self.selection.contains(&f.id) && self.show.has(f, attr)).map(|f| f.id).collect();
                if reverse {
                    ids.reverse();
                }
                let n = ids.len().max(2) as f32 - 1.0;
                for (i, id) in ids.into_iter().enumerate() {
                    let v = from as f32 + (to as f32 - from as f32) * i as f32 / n;
                    self.programmer.entry(id).or_default().insert(attr, v.round() as u8);
                }
            }
            Action::SetBlind(b) => self.blind = b,
            Action::Highlight(b) => self.highlight = b,
            Action::Park(attr) => {
                for f in self.show.fixtures.iter().filter(|f| self.selection.contains(&f.id)) {
                    if self.show.has(f, attr) {
                        let v = self.output_of(f, attr);
                        self.parked.entry(f.id).or_default().insert(attr, v);
                    }
                }
            }
            Action::Unpark => self.parked.clear(),
            Action::Snap(i) => {
                if i < self.players.len() {
                    self.go_next(i);
                    self.players[i].fade_time = 0.0;
                    self.players[i].delay = 0.0;
                }
            }
            Action::SetCueDelay(i, t) => {
                if let Some(c) = self.show.lists[l].cues.get_mut(i) {
                    c.delay = t.max(0.0);
                }
            }
            Action::SetCueComment(i, t) => {
                if let Some(c) = self.show.lists[l].cues.get_mut(i) {
                    c.comment = t.chars().take(21).collect();
                }
            }
            Action::SetChase(i, b) => {
                if let Some(x) = self.show.lists.get_mut(i) {
                    x.chase = b;
                }
            }
            Action::SetBpm(i, b) => {
                if let Some(x) = self.show.lists.get_mut(i) {
                    x.bpm = b.clamp(1.0, 600.0);
                }
            }
            Action::FadeSpeed(v) => self.fade_speed = v.clamp(0.1, 10.0),
            Action::SetAttr(a, v) => {
                for f in self.show.fixtures.iter().filter(|f| self.selection.contains(&f.id)) {
                    if self.show.has(f, a) || (a == Attr::Dimmer && self.show.virtual_dim(f)) {
                        self.programmer.entry(f.id).or_default().insert(a, v);
                    }
                }
            }
            Action::ClearProgrammer => self.programmer.clear(),
            Action::RecordGroup => {
                if !self.selection.is_empty() {
                    let name = format!("Group {}", self.show.groups.len() + 1);
                    self.show.groups.push(Group { name, ids: self.selection.iter().copied().collect() });
                }
            }
            Action::RecordPalette(kind) => {
                let values = self.programmer_values(kind);
                if !values.is_empty() {
                    let name = format!("{} {}", kind.name(), self.show.palettes.iter().filter(|p| p.kind == kind).count() + 1);
                    self.show.palettes.push(Palette { name, kind, values });
                }
            }
            Action::UpdatePalette(i) => {
                if let Some(kind) = self.show.palettes.get(i).map(|p| p.kind) {
                    let values = self.programmer_values(kind);
                    if !values.is_empty() {
                        self.show.palettes[i].values = values;
                    }
                }
            }
            Action::RenamePalette(i, n) => {
                if let Some(p) = self.show.palettes.get_mut(i) {
                    p.name = n;
                }
            }
            Action::DeletePalette(i) => {
                if i < self.show.palettes.len() {
                    self.show.palettes.remove(i);
                }
            }
            Action::RenameGroup(i, n) => {
                if let Some(g) = self.show.groups.get_mut(i) {
                    g.name = n;
                }
            }
            Action::DeleteGroup(i) => {
                if i < self.show.groups.len() {
                    self.show.groups.remove(i);
                }
            }
            Action::UpdateGroup(i) => {
                if let (Some(g), false) = (self.show.groups.get_mut(i), self.selection.is_empty()) {
                    g.ids = self.selection.iter().copied().collect();
                }
            }
            Action::ReleaseAttr(a) => {
                for id in &self.selection {
                    if let Some(m) = self.programmer.get_mut(id) {
                        m.remove(&a);
                    }
                }
                self.programmer.retain(|_, m| !m.is_empty());
            }
            Action::RenameFixture(id, n) => {
                if let Some(f) = self.show.fixtures.iter_mut().find(|f| f.id == id) {
                    f.name = n;
                }
            }
            Action::SetPatch { id, universe, address } => {
                if let Some(i) = self.show.fixtures.iter().position(|f| f.id == id) {
                    let width = self.show.channels(&self.show.fixtures[i]).len() as u16;
                    let f = &mut self.show.fixtures[i];
                    f.universe = universe.max(1);
                    f.address = address.clamp(1, (CHANNELS as u16 + 1).saturating_sub(width).max(1));
                }
            }
            Action::SetPosition(id, x, y) => {
                if let Some(f) = self.show.fixtures.iter_mut().find(|f| f.id == id) {
                    (f.x, f.y) = (x.clamp(0.02, 0.98), y.clamp(0.02, 0.98));
                }
            }
            Action::AddProp(kind) => {
                let (w, h) = kind.size();
                let label = if kind == PropKind::Text { "Text".to_string() } else { String::new() };
                self.show.props.push(Prop { kind, x: 0.5, y: 0.5, w, h, label, color: None });
            }
            Action::SetProp(i, mut p) => {
                if let Some(slot) = self.show.props.get_mut(i) {
                    (p.w, p.h) = (p.w.clamp(0.01, 1.0), p.h.clamp(0.01, 1.0));
                    (p.x, p.y) = (p.x.clamp(0.0, 1.0), p.y.clamp(0.0, 1.0));
                    *slot = p;
                }
            }
            Action::DeleteProp(i) => {
                if i < self.show.props.len() {
                    self.show.props.remove(i);
                }
            }
            Action::DuplicateProp(i) => {
                if let Some(mut p) = self.show.props.get(i).cloned() {
                    (p.x, p.y) = ((p.x + 0.03).min(1.0), (p.y + 0.03).min(1.0));
                    self.show.props.push(p);
                }
            }
            Action::MoveProp(from, to) => {
                if from < self.show.props.len() && to < self.show.props.len() {
                    let p = self.show.props.remove(from);
                    self.show.props.insert(to, p);
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
                    let name = format!("Cue {}", self.show.lists[l].cues.len() + 1);
                    let values = self.programmer.clone();
                    self.show.lists[l].cues.push(Cue { name, fade: 2.0, follow: None, values, delay: 0.0, comment: String::new(), block: false });
                }
            }
            Action::StoreScene => {
                let play = self.playback_values();
                let mut values = Values::new();
                for f in &self.show.fixtures {
                    for a in self.show.channels(f).iter().flatten().filter(|a| Attr::ALL.contains(a)) {
                        values.entry(f.id).or_default().insert(*a, self.value_of(&play, f.id, *a).0);
                    }
                    if self.show.virtual_dim(f) {
                        let d = self.programmer.get(&f.id).and_then(|m| m.get(&Attr::Dimmer)).or_else(|| play.get(&f.id).and_then(|m| m.get(&Attr::Dimmer))).copied().unwrap_or(255);
                        values.entry(f.id).or_default().insert(Attr::Dimmer, d);
                    }
                }
                if !values.is_empty() {
                    let name = format!("Scene {}", self.show.lists[l].cues.len() + 1);
                    self.show.lists[l].cues.push(Cue { name, fade: 1.0, follow: None, values, delay: 0.0, comment: String::new(), block: true });
                    self.programmer.clear();
                    let last = self.show.lists[l].cues.len() - 1;
                    self.goto(l, Some(last));
                }
            }
            Action::Go(i) => {
                if i < self.players.len() {
                    self.go_next(i);
                }
            }
            Action::Back(i) => {
                if i < self.players.len() {
                    match self.players[i].cue {
                        Some(0) => self.goto(i, None),
                        Some(c) => self.goto(i, Some(c - 1)),
                        None => {}
                    }
                }
            }
            Action::ReleaseList(i) => {
                if i < self.players.len() {
                    self.goto(i, None);
                }
            }
            Action::SetLevel(i, v) => {
                if let Some(p) = self.players.get_mut(i) {
                    p.level = v;
                }
            }
            Action::GotoCue(i) => {
                if i < self.show.lists[l].cues.len() {
                    self.goto(l, Some(i));
                }
            }
            Action::SetCueFade(i, t) => {
                if let Some(c) = self.show.lists[l].cues.get_mut(i) {
                    c.fade = t.max(0.0);
                }
            }
            Action::SetCueFollow(i, t) => {
                if let Some(c) = self.show.lists[l].cues.get_mut(i) {
                    c.follow = t.map(|t| t.max(0.0));
                }
            }
            Action::DeleteCue(i) => {
                if i < self.show.lists[l].cues.len() {
                    self.show.lists[l].cues.remove(i);
                    match self.players[l].cue {
                        Some(c) if c == i => self.goto(l, None),
                        Some(c) if c > i => {
                            self.players[l].cue = Some(c - 1);
                            self.refresh_playback(l);
                        }
                        _ => {}
                    }
                }
            }
            Action::MoveCue(from, to) => {
                let n = self.show.lists[l].cues.len();
                if from < n && to < n && from != to {
                    let c = self.show.lists[l].cues.remove(from);
                    self.show.lists[l].cues.insert(to, c);
                    if let Some(cur) = self.players[l].cue {
                        self.players[l].cue = Some(if cur == from {
                            to
                        } else if from < cur && cur <= to {
                            cur - 1
                        } else if to <= cur && cur < from {
                            cur + 1
                        } else {
                            cur
                        });
                    }
                }
            }
            Action::DuplicateCue(i) => {
                if let Some(mut c) = self.show.lists[l].cues.get(i).cloned() {
                    c.name = format!("{} Kopie", c.name);
                    self.show.lists[l].cues.insert(i + 1, c);
                    if let Some(cur) = self.players[l].cue.filter(|&c| c > i) {
                        self.players[l].cue = Some(cur + 1);
                    }
                }
            }
            Action::RenameCue(i, n) => {
                if let Some(c) = self.show.lists[l].cues.get_mut(i) {
                    c.name = n;
                }
            }
            Action::UpdateCue(i) => {
                if let Some(c) = self.show.lists[l].cues.get_mut(i) {
                    for (&id, attrs) in &self.programmer {
                        c.values.entry(id).or_default().extend(attrs);
                    }
                    self.refresh_playback(l);
                }
            }
            Action::SelectList(i) => {
                if i < self.show.lists.len() {
                    self.active = i;
                }
            }
            Action::AddList => {
                let name = format!("Cuelist {}", self.show.lists.len() + 1);
                self.show.lists.push(Cuelist::new(name));
                self.sync_players();
                self.active = self.show.lists.len() - 1;
            }
            Action::DeleteList(i) => {
                if self.show.lists.len() > 1 && i < self.show.lists.len() {
                    self.show.lists.remove(i);
                    self.players.remove(i);
                    self.active = self.active.min(self.show.lists.len() - 1);
                }
            }
            Action::RenameList(i, n) => {
                if let Some(c) = self.show.lists.get_mut(i) {
                    c.name = n;
                }
            }
            Action::SetLooped(i, b) => {
                if let Some(c) = self.show.lists.get_mut(i) {
                    c.looped = b;
                }
            }
            Action::AddEffect(target, shape) => {
                if !self.selection.is_empty() {
                    let n = self.show.effects.len() + 1;
                    self.show.effects.push(Effect {
                        name: format!("{} {n}", target.name()),
                        target,
                        shape,
                        rate: 0.5,
                        size: if target == Target::Color { 255 } else { 128 },
                        spread: 1.0,
                        ids: self.selection.iter().copied().collect(),
                        running: true,
                    });
                }
            }
            Action::SetEffect(i, e) => {
                if let Some(slot) = self.show.effects.get_mut(i) {
                    let running = slot.running;
                    *slot = Effect { running, ..e };
                }
            }
            Action::DeleteEffect(i) => {
                if i < self.show.effects.len() {
                    self.show.effects.remove(i);
                }
            }
            Action::ToggleEffect(i) => {
                if let Some(e) = self.show.effects.get_mut(i) {
                    e.running = !e.running;
                }
            }
            Action::StopEffects => self.show.effects.iter_mut().for_each(|e| e.running = false),
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
                    let idx = self.show.fixtures.len();
                    self.show.fixtures.push(Fixture {
                        id: next_id,
                        name: format!("{} {next_id}", name.split(" (").next().unwrap_or(&name)),
                        ty,
                        universe,
                        address: address as u16,
                        x: 0.1 + 0.8 * ((idx % 10) as f32 / 9.0),
                        y: 0.2 + 0.15 * (idx / 10) as f32,
                    });
                    next_id += 1;
                }
            }
            Action::AutoAddress(u) => {
                let mut next = 1u16;
                for i in 0..self.show.fixtures.len() {
                    if self.show.fixtures[i].universe != u {
                        continue;
                    }
                    let w = self.show.channels(&self.show.fixtures[i]).len() as u16;
                    if next + w > CHANNELS as u16 + 1 {
                        break;
                    }
                    self.show.fixtures[i].address = next;
                    next += w;
                }
            }
            Action::AddTypes(types) => {
                for t in types {
                    if !self.show.types.iter().any(|x| x.name == t.name) {
                        self.show.types.push(t);
                    }
                }
            }
            Action::UpdateType(i, t) => {
                let taken = self.show.types.iter().enumerate().any(|(j, x)| j != i && x.name == t.name);
                if i < self.show.types.len() && !taken {
                    self.show.types[i] = t;
                }
            }
            Action::RemoveType(i) => {
                if i < self.show.types.len() && !self.show.fixtures.iter().any(|f| f.ty == i) {
                    self.show.types.remove(i);
                    for f in &mut self.show.fixtures {
                        if f.ty > i {
                            f.ty -= 1;
                        }
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
                for e in &mut self.show.effects {
                    e.ids.retain(|&x| x != id);
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

    fn follow_cues(&mut self, dt: f32) {
        for l in 0..self.players.len() {
            let list = &self.show.lists[l];
            if list.chase && self.players[l].cue.is_some() && list.bpm > 0.0 {
                self.chase_t[l] += dt;
                if self.chase_t[l] >= 60.0 / list.bpm {
                    self.go_next(l);
                }
            }
        }
        for l in 0..self.players.len() {
            let p = &self.players[l];
            let Some(c) = p.cue else { continue };
            if let Some(f) = self.show.lists[l].cues[c].follow {
                if p.fade_t >= p.delay + p.fade_time + f {
                    self.go_next(l);
                }
            }
        }
    }

    /// Wellenwerte (Phase 0..1) der laufenden Effekte für ein Fixture.
    fn effects_for(&self, id: u32) -> Vec<(&Effect, f32)> {
        self.show
            .effects
            .iter()
            .filter(|e| e.running)
            .filter_map(|e| {
                let n = e.ids.iter().position(|&x| x == id)?;
                let phase = self.fx_t * e.rate + e.spread * n as f32 / e.ids.len().max(1) as f32;
                Some((e, phase))
            })
            .collect()
    }

    /// Wert eines Attributs nach Programmer/Playback und Effekten (0..255, mit Nachkommaanteil).
    fn eval(&self, f: &Fixture, a: Attr, play: &Values, fx: &[(&Effect, f32)]) -> f32 {
        let prog = if self.blind { None } else { self.programmer.get(&f.id).and_then(|m| m.get(&a)).copied() };
        let parked = self.parked.get(&f.id).and_then(|m| m.get(&a)).copied();
        let mut v = parked.or(prog).unwrap_or_else(|| get(play, f.id, a)) as f32;
        if a != Attr::Dimmer && a.kind() == Kind::Color && a.is_intensity_like() && self.show.virtual_dim(f) {
            let d = self.parked.get(&f.id).and_then(|m| m.get(&Attr::Dimmer)).copied().or(if self.blind { None } else { self.programmer.get(&f.id).and_then(|m| m.get(&Attr::Dimmer)).copied() }).or_else(|| play.get(&f.id).and_then(|m| m.get(&Attr::Dimmer)).copied()).unwrap_or(255);
            v *= d as f32 / 255.0;
        }
        for &(e, phase) in fx {
            let depth = e.size as f32 / 255.0;
            match e.target {
                Target::Intensity => {
                    let dim = if self.show.has(f, Attr::Dimmer) { a == Attr::Dimmer } else { a.kind() == Kind::Color && a.is_intensity_like() };
                    if dim {
                        v *= 1.0 - depth * (1.0 - e.shape.wave(phase));
                    }
                }
                Target::Color => {
                    let c = hue_rgb(phase);
                    let sat = depth;
                    let idx = match a {
                        Attr::Red => Some(0),
                        Attr::Green => Some(1),
                        Attr::Blue => Some(2),
                        _ => None,
                    };
                    if let Some(i) = idx {
                        v = 255.0 * (1.0 - sat + sat * c[i]);
                    }
                }
                Target::Position => {
                    let r = depth * 127.0;
                    let ang = phase * std::f32::consts::TAU;
                    match a {
                        Attr::Pan => v += r * ang.cos(),
                        Attr::Tilt => v += r * ang.sin(),
                        _ => {}
                    }
                }
            }
        }
        if self.highlight && a.is_intensity_like() && self.selection.contains(&f.id) {
            return 255.0;
        }
        v.clamp(0.0, 255.0)
    }

    /// Schritt `dt` Sekunden weiter und DMX-Frames neu berechnen.
    pub fn tick(&mut self, dt: f32) {
        for p in &mut self.players {
            p.fade_t += dt * self.fade_speed;
        }
        self.fx_t += dt;
        self.follow_cues(dt);
        let play = self.playback_values();
        let mut frames: BTreeMap<u16, Frame> = BTreeMap::new();
        for f in &self.show.fixtures {
            let fx = self.effects_for(f.id);
            let frame = frames.entry(f.universe).or_insert([0; CHANNELS]);
            for (i, slot) in self.show.channels(f).iter().enumerate() {
                let Some(a) = *slot else { continue };
                let v = match a {
                    Attr::PanFine => self.eval(f, Attr::Pan, &play, &fx).fract() * 255.0,
                    Attr::TiltFine => self.eval(f, Attr::Tilt, &play, &fx).fract() * 255.0,
                    Attr::Pan | Attr::Tilt => self.eval(f, a, &play, &fx).floor(),
                    _ => self.eval(f, a, &play, &fx).round(),
                };
                let v = if self.blackout && a.is_intensity_like() {
                    0.0
                } else if a.is_intensity_like() {
                    (v * self.master as f32 / 255.0).round()
                } else {
                    v
                };
                if let Some(out) = frame.get_mut(f.address as usize - 1 + i) {
                    *out = v as u8;
                }
            }
        }
        self.frames = frames;
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
    fn move_and_duplicate_cue_keep_playback_index() {
        let mut e = Engine::new(Show { types: builtin_types(), ..Show::default() });
        e.apply(Action::AddFixtures { ty: 0, count: 1, universe: 1, address: 1 });
        e.apply(Action::SelectAll);
        for _ in 0..3 {
            e.apply(Action::StoreScene);
        }
        e.apply(Action::GotoCue(0));
        e.apply(Action::MoveCue(0, 2));
        assert_eq!(e.cue(), Some(2));
        e.apply(Action::DuplicateCue(0));
        assert_eq!(e.list().cues.len(), 4);
        assert_eq!(e.cue(), Some(3));
    }

    #[test]
    fn remove_type_only_when_unused_and_reindexes() {
        let mut e = Engine::new(Show { types: builtin_types(), ..Show::default() });
        e.apply(Action::AddFixtures { ty: 3, count: 1, universe: 1, address: 1 });
        let n = e.show.types.len();
        e.apply(Action::RemoveType(3));
        assert_eq!(e.show.types.len(), n);
        e.apply(Action::RemoveType(0));
        assert_eq!(e.show.types.len(), n - 1);
        assert_eq!(e.show.fixtures[0].ty, 2);
    }

    #[test]
    fn update_type_rejects_duplicate_name() {
        let mut e = Engine::new(Show { types: builtin_types(), ..Show::default() });
        let mut t = e.show.types[1].clone();
        t.name = e.show.types[0].name.clone();
        e.apply(Action::UpdateType(1, t));
        assert_ne!(e.show.types[1].name, e.show.types[0].name);
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
        e.apply(Action::Go(0));
        e.tick(1.0);
        assert_eq!(e.frames[&1][0], 50); // halber Fade (2 s)
        e.tick(5.0);
        assert_eq!(e.frames[&1][0], 100);
        e.apply(Action::Go(0));
        e.tick(5.0);
        assert_eq!(e.frames[&1][0], 100); // Dimmer wird getrackt
        assert_eq!(e.frames[&1][1], 50);
    }

    #[test]
    fn manage_palettes_cues_and_release() {
        let mut e = engine();
        e.apply(Action::SelectAll);
        e.apply(Action::SetAttr(Attr::Red, 10));
        e.apply(Action::RecordPalette(Kind::Color));
        e.apply(Action::RenamePalette(0, "Rot".into()));
        e.apply(Action::SetAttr(Attr::Red, 99));
        e.apply(Action::UpdatePalette(0));
        assert_eq!(e.show.palettes[0].values[&Attr::Red], 99);
        e.apply(Action::DeletePalette(0));
        assert!(e.show.palettes.is_empty());

        e.apply(Action::RecordCue);
        e.apply(Action::ReleaseAttr(Attr::Red));
        assert!(e.programmer.is_empty());
        e.apply(Action::Go(0));
        e.apply(Action::SetAttr(Attr::Red, 7));
        e.apply(Action::UpdateCue(0));
        e.tick(0.0);
        assert_eq!(e.frames[&1][1], 7);
        e.apply(Action::DeleteCue(0));
        assert_eq!(e.cue(), None);
    }

    #[test]
    fn set_patch_clamps_to_universe() {
        let mut e = engine();
        e.apply(Action::SetPatch { id: 1, universe: 0, address: 600 });
        assert_eq!((e.show.fixtures[0].universe, e.show.fixtures[0].address), (1, 509));
    }

    #[test]
    fn detects_overlap() {
        let mut e = engine();
        e.apply(Action::AddFixtures { ty: 3, count: 1, universe: 1, address: 3 });
        assert_eq!(e.conflicts().len(), 3); // 3–6 überlappt 1–4 und 5–8
    }

    #[test]
    fn lists_merge_htp_and_levels() {
        let mut e = engine();
        e.apply(Action::SelectAll);
        e.apply(Action::SetAttr(Attr::Dimmer, 200));
        e.apply(Action::RecordCue);
        e.apply(Action::SetCueFade(0, 0.0));
        e.apply(Action::AddList);
        e.apply(Action::SetAttr(Attr::Dimmer, 100));
        e.apply(Action::RecordCue);
        e.apply(Action::SetCueFade(0, 0.0));
        e.apply(Action::ClearProgrammer);
        e.apply(Action::Go(0));
        e.apply(Action::Go(1));
        e.tick(0.0);
        assert_eq!(e.frames[&1][0], 200); // HTP
        e.apply(Action::SetLevel(0, 51));
        e.tick(0.0);
        assert_eq!(e.frames[&1][0], 100);
        e.apply(Action::ReleaseList(1));
        e.tick(0.0);
        assert_eq!(e.frames[&1][0], 40); // 200 * 51/255
    }

    #[test]
    fn auto_follow_loops() {
        let mut e = engine();
        e.apply(Action::SelectAll);
        for v in [10, 20] {
            e.apply(Action::SetAttr(Attr::Dimmer, v));
            e.apply(Action::RecordCue);
        }
        e.apply(Action::ClearProgrammer);
        for i in 0..2 {
            e.apply(Action::SetCueFade(i, 0.0));
            e.apply(Action::SetCueFollow(i, Some(1.0)));
        }
        e.apply(Action::SetLooped(0, true));
        e.apply(Action::Go(0));
        e.tick(0.0);
        assert_eq!(e.frames[&1][0], 10);
        e.tick(1.1);
        assert_eq!(e.frames[&1][0], 20);
        e.tick(1.1);
        assert_eq!(e.frames[&1][0], 10); // Loop
    }

    #[test]
    fn effects_modulate_and_undo_restores() {
        let mut e = engine();
        e.apply(Action::SelectAll);
        e.apply(Action::SetAttr(Attr::Dimmer, 200));
        e.apply(Action::AddEffect(Target::Intensity, Shape::Square));
        e.show.effects[0].spread = 0.0;
        e.show.effects[0].size = 255;
        e.show.effects[0].rate = 1.0;
        e.tick(0.0);
        assert_eq!(e.frames[&1][0], 200); // Rechteck: erste Hälfte an
        e.tick(0.6);
        assert_eq!(e.frames[&1][0], 0);
        e.apply(Action::StopEffects);
        e.tick(0.0);
        assert_eq!(e.frames[&1][0], 200);

        e.apply(Action::DeleteEffect(0));
        assert!(e.show.effects.is_empty());
        e.apply(Action::Undo);
        assert_eq!(e.show.effects.len(), 1);
        e.apply(Action::Redo);
        assert!(e.show.effects.is_empty());
    }

    #[test]
    fn pan_fine_channel_gets_fraction() {
        let mut e = Engine::new(Show::default());
        e.show.types.push(FixtureType {
            name: "Head16".into(),
            channels: vec![Some(Attr::Pan), Some(Attr::PanFine)],
        });
        e.apply(Action::AddFixtures { ty: e.show.types.len() - 1, count: 1, universe: 1, address: 1 });
        e.apply(Action::SelectAll);
        e.apply(Action::SetAttr(Attr::Pan, 100));
        e.apply(Action::AddEffect(Target::Position, Shape::Sine));
        e.show.effects[0].size = 10;
        e.show.effects[0].rate = 1.0;
        e.tick(0.1);
        assert_ne!(e.frames[&1][1], 0);
    }

    #[test]
    fn range_selection() {
        let mut e = engine();
        e.apply(Action::SelectRange(2, 1));
        assert_eq!(e.selection.len(), 2);
        e.apply(Action::Deselect(1));
        assert_eq!(e.selection.len(), 1);
    }

    #[test]
    fn auto_address_packs_and_finds_free_slot() {
        let mut e = engine();
        e.apply(Action::SetPatch { id: 2, universe: 1, address: 100 });
        e.apply(Action::AutoAddress(1));
        assert!(e.conflicts().is_empty());
        let a = e.show.fixtures[0].address;
        assert_eq!(a, 1);
        let w = e.show.channels(&e.show.fixtures[0]).len() as u16;
        assert_eq!(e.show.fixtures[1].address, 1 + w);
        let n = e.show.fixtures.len() as u16;
        assert_eq!(e.show.free_address(1, w), Some(1 + n * w));
    }

    #[test]
    fn scenes_are_independent_and_clear_programmer() {
        let mut e = engine();
        e.apply(Action::Select(1));
        e.apply(Action::SetAttr(Attr::Dimmer, 200));
        e.apply(Action::StoreScene);
        assert!(e.programmer.is_empty());
        e.tick(5.0);
        e.apply(Action::ClearSelection);
        e.apply(Action::Select(2));
        e.apply(Action::SetAttr(Attr::Dimmer, 100));
        e.apply(Action::StoreScene);
        e.tick(5.0);
        assert_eq!(e.frames[&1][0], 200); // Szene 2 sieht Szene 1 (bisheriger Look)
        e.apply(Action::GotoCue(0));
        e.tick(5.0);
        assert_eq!(e.frames[&1][4], 0); // Szene 1 kennt Fixture 2 nicht als an
        e.apply(Action::GotoCue(1));
        e.tick(5.0);
        assert_eq!(e.frames[&1][4], 100);
    }

    #[test]
    fn virtual_dimmer_scales_rgb_and_highlight_forces_full() {
        let mut e = Engine::new(Show { types: builtin_types(), ..Show::default() });
        e.apply(Action::AddFixtures { ty: 1, count: 1, universe: 0, address: 1 });
        e.apply(Action::SelectAll);
        e.apply(Action::SetAttr(Attr::Red, 200));
        e.apply(Action::SetAttr(Attr::Dimmer, 128));
        e.tick(0.0);
        assert_eq!(e.frames[&0][0], 100);
        e.apply(Action::Highlight(true));
        e.tick(0.0);
        assert_eq!(e.frames[&0][0], 255);
    }

    #[test]
    fn selection_tools_fan_and_blind() {
        let mut e = Engine::new(Show::default());
        e.apply(Action::AddFixtures { ty: 0, count: 4, universe: 1, address: 1 });
        e.apply(Action::SelectAll);
        e.apply(Action::SelectEvery(2, 1));
        assert_eq!(e.selection.iter().copied().collect::<Vec<_>>(), vec![2, 4]);
        e.apply(Action::InvertSelection);
        assert_eq!(e.selection.iter().copied().collect::<Vec<_>>(), vec![1, 3]);
        e.apply(Action::SelectAll);
        e.apply(Action::Fan(Attr::Dimmer, 0, 90, false));
        assert_eq!(e.programmer[&3][&Attr::Dimmer], 60);
        e.tick(0.01);
        assert_eq!(e.frames[&1][3], 90);
        e.apply(Action::SetBlind(true));
        e.tick(0.01);
        assert_eq!(e.frames[&1][3], 0);
    }

    #[test]
    fn cue_delay_holds_fade_back() {
        let mut e = Engine::new(Show::default());
        e.apply(Action::AddFixtures { ty: 0, count: 1, universe: 1, address: 1 });
        e.apply(Action::SelectAll);
        e.apply(Action::SetAttr(Attr::Dimmer, 200));
        e.apply(Action::StoreScene);
        e.apply(Action::SetCueFade(0, 0.0));
        e.apply(Action::SetCueDelay(0, 1.0));
        e.apply(Action::ReleaseList(0));
        e.apply(Action::GotoCue(0));
        e.tick(0.5);
        assert_eq!(e.frames[&1][0], 0);
        e.tick(0.6);
        assert_eq!(e.frames[&1][0], 200);
    }
}
