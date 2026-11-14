//! Befehlszeile im Stil klassischer Lichtpulte, z. B. `1 thru 8 @ 50`, `group 2 red 100`, `go 2`.
//! Der Parser erzeugt nur `Action`s, die Engine bleibt unverändert.

use crate::engine::{Action, Engine};
use crate::fixture::{Attr, Kind};
use std::collections::BTreeSet;

fn tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let flush = |cur: &mut String, out: &mut Vec<String>| {
        if !cur.is_empty() {
            out.push(std::mem::take(cur));
        }
    };
    for c in line.to_lowercase().chars() {
        if c.is_alphanumeric() || c == '.' || c == ',' {
            cur.push(if c == ',' { '.' } else { c });
        } else {
            flush(&mut cur, &mut out);
            if matches!(c, '@' | '+' | '-' | '>') {
                out.push(c.to_string());
            }
        }
    }
    flush(&mut cur, &mut out);
    out
}

fn attr_of(word: &str) -> Option<Attr> {
    Some(match word {
        "dimmer" | "dim" | "intensity" | "int" => Attr::Dimmer,
        "red" | "rot" | "r" => Attr::Red,
        "green" | "gruen" | "grün" | "g" => Attr::Green,
        "blue" | "blau" | "b" => Attr::Blue,
        "white" | "weiss" | "weiß" | "w" => Attr::White,
        "amber" => Attr::Amber,
        "uv" => Attr::Uv,
        "pan" => Attr::Pan,
        "tilt" => Attr::Tilt,
        "zoom" => Attr::Zoom,
        "strobe" | "blitz" => Attr::Strobe,
        "gobo" => Attr::Gobo,
        "prism" | "prisma" => Attr::Prism,
        "iris" => Attr::Iris,
        "focus" | "fokus" => Attr::Focus,
        "frost" => Attr::Frost,
        "wheel" | "farbrad" => Attr::ColorWheel,
        _ => return None,
    })
}

fn kind_of(word: &str) -> Option<Kind> {
    Some(match word {
        "intensity" | "intensität" | "intensitaet" | "int" => Kind::Intensity,
        "color" | "colour" | "farbe" => Kind::Color,
        "position" | "pos" => Kind::Position,
        "beam" => Kind::Beam,
        _ => return None,
    })
}

struct Cursor<'a> {
    t: &'a [String],
    i: usize,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<&str> {
        self.t.get(self.i).map(String::as_str)
    }

    fn next(&mut self) -> Option<&str> {
        self.i += 1;
        self.t.get(self.i - 1).map(String::as_str)
    }

    fn number(&mut self) -> Option<f32> {
        let v = self.peek()?.parse().ok()?;
        self.i += 1;
        Some(v)
    }

    fn index(&mut self, what: &str) -> Result<usize, String> {
        match self.number() {
            Some(n) if n >= 1.0 => Ok(n as usize - 1),
            _ => Err(format!("{what}: Nummer ab 1 erwartet")),
        }
    }
}

/// Prozent (0–100) oder `full`/`out` -> DMX-Wert.
fn value(c: &mut Cursor) -> Result<u8, String> {
    if let Some(n) = c.number() {
        return Ok((n.clamp(0.0, 100.0) * 255.0 / 100.0).round() as u8);
    }
    match c.next() {
        Some("full" | "voll") => Ok(255),
        Some("out" | "aus") => Ok(0),
        _ => Err("Wert erwartet (0–100, full oder out)".into()),
    }
}

pub fn parse(line: &str, e: &Engine) -> Result<Vec<Action>, String> {
    let mut out = Vec::new();
    for stmt in line.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        statement(stmt, e, &mut out)?;
    }
    if out.is_empty() { Err("Leere Eingabe".into()) } else { Ok(out) }
}

fn statement(stmt: &str, e: &Engine, out: &mut Vec<Action>) -> Result<(), String> {
    let t = tokens(stmt);
    let mut c = Cursor { t: &t, i: 0 };
    let list = |c: &mut Cursor| c.number().map_or(e.active, |n| (n as usize).saturating_sub(1));
    match c.peek().unwrap_or("") {
        "go" => {
            c.next();
            out.push(Action::Go(list(&mut c)));
        }
        "back" | "zurück" | "zurueck" => {
            c.next();
            out.push(Action::Back(list(&mut c)));
        }
        "release" | "rel" => {
            c.next();
            out.push(Action::ReleaseList(list(&mut c)));
        }
        "goto" => {
            c.next();
            out.push(Action::GotoCue(c.index("goto")?));
        }
        "list" | "liste" => {
            c.next();
            out.push(Action::SelectList(c.index("list")?));
        }
        "level" => {
            c.next();
            let l = c.index("level")?;
            out.push(Action::SetLevel(l, value(&mut c)?));
        }
        "clear" | "leeren" => out.push(Action::ClearProgrammer),
        "undo" => out.push(Action::Undo),
        "redo" => out.push(Action::Redo),
        "blackout" | "bo" => {
            c.next();
            out.push(Action::Blackout(!matches!(c.next(), Some("off" | "aus"))));
        }
        "master" => {
            c.next();
            out.push(Action::Master(value(&mut c)?));
        }
        "fx" | "effect" | "effekt" => {
            c.next();
            if matches!(c.peek(), Some("stop" | "off" | "aus")) {
                c.next();
                out.push(Action::StopEffects);
            } else {
                out.push(Action::ToggleEffect(c.index("fx")?));
            }
        }
        "record" | "rec" => {
            c.next();
            match c.next() {
                Some("cue") => out.push(Action::RecordCue),
                Some("group" | "gruppe") => out.push(Action::RecordGroup),
                Some("palette" | "pal") => {
                    let k = c.next().and_then(kind_of).ok_or("record palette: intensity, color, position oder beam")?;
                    out.push(Action::RecordPalette(k));
                }
                Some(k) => out.push(Action::RecordPalette(kind_of(k).ok_or(format!("Unbekannt: record {k}"))?)),
                None => out.push(Action::RecordCue),
            }
        }
        _ => selection(&mut c, e, out)?,
    }
    match c.peek() {
        Some(x) => Err(format!("Unerwartet: „{x}“")),
        None => Ok(()),
    }
}

/// `1 thru 8 - 3 + 12`, `all`, `group 2`, gefolgt von `@ 50`, `red 100`, `pal 3`.
fn selection(c: &mut Cursor, e: &Engine, out: &mut Vec<Action>) -> Result<(), String> {
    let ids: Vec<u32> = e.show.fixtures.iter().map(|f| f.id).collect();
    let mut set: BTreeSet<u32> = if matches!(c.peek(), Some("+" | "-")) { e.selection.clone() } else { BTreeSet::new() };
    let mut touched = false;
    loop {
        let remove = match c.peek() {
            Some("+") => {
                c.next();
                false
            }
            Some("-") => {
                c.next();
                true
            }
            _ => false,
        };
        let part: Vec<u32> = match c.peek() {
            Some("all" | "alle") => {
                c.next();
                ids.clone()
            }
            Some("group" | "gruppe") => {
                c.next();
                let g = c.index("group")?;
                e.show.groups.get(g).ok_or("Gruppe existiert nicht")?.ids.clone()
            }
            Some(w) if w.parse::<u32>().is_ok() => {
                let a = c.number().unwrap() as u32;
                let b = if matches!(c.peek(), Some("thru" | "bis" | ">")) {
                    c.next();
                    c.number().ok_or("thru: Endnummer erwartet")? as u32
                } else {
                    a
                };
                ids.iter().copied().filter(|id| (a.min(b)..=a.max(b)).contains(id)).collect()
            }
            _ => break,
        };
        touched = true;
        for id in part {
            if remove {
                set.remove(&id);
            } else {
                set.insert(id);
            }
        }
    }
    if touched {
        out.push(Action::SelectIds(set.into_iter().collect()));
    }
    let mut did = touched;
    while let Some(w) = c.peek() {
        if w == "@" {
            c.next();
            out.push(Action::SetAttr(Attr::Dimmer, value(c)?));
        } else if matches!(w, "palette" | "pal") {
            c.next();
            out.push(Action::ApplyPalette(c.index("palette")?));
        } else if let Some(a) = attr_of(w) {
            c.next();
            out.push(Action::SetAttr(a, value(c)?));
        } else {
            return if did { Err(format!("Unerwartet: „{w}“")) } else { Err(format!("Unbekannter Befehl: „{w}“")) };
        }
        did = true;
    }
    if did { Ok(()) } else { Err("Leere Eingabe".into()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Show;

    fn engine() -> Engine {
        let mut e = Engine::new(Show::default());
        e.apply(Action::AddFixtures { ty: 3, count: 6, universe: 1, address: 1 });
        e
    }

    fn dbg(line: &str, e: &Engine) -> String {
        format!("{:?}", parse(line, e).unwrap())
    }

    #[test]
    fn selects_and_sets_levels() {
        let e = engine();
        assert_eq!(dbg("1 thru 3 @ 50", &e), "[SelectIds([1, 2, 3]), SetAttr(Dimmer, 128)]");
        assert_eq!(dbg("1 thru 6 - 3 + 9 red 100", &e), "[SelectIds([1, 2, 4, 5, 6]), SetAttr(Red, 255)]");
        assert_eq!(dbg("alle @ full", &e), "[SelectIds([1, 2, 3, 4, 5, 6]), SetAttr(Dimmer, 255)]");
    }

    #[test]
    fn playback_commands() {
        let e = engine();
        assert_eq!(dbg("go", &e), "[Go(0)]");
        assert_eq!(dbg("go 2; back 2", &e), "[Go(1), Back(1)]");
        assert_eq!(dbg("record cue", &e), "[RecordCue]");
        assert_eq!(dbg("rec color", &e), "[RecordPalette(Color)]");
        assert_eq!(dbg("fx stop", &e), "[StopEffects]");
    }

    #[test]
    fn rejects_garbage() {
        let e = engine();
        assert!(parse("blubb", &e).is_err());
        assert!(parse("1 @", &e).is_err());
        assert!(parse("group 9", &e).is_err());
        assert!(parse("", &e).is_err());
    }
}
