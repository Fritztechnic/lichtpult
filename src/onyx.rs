//! Import von Onyx-Shows (`.OnyxShow`).
//!
//! Eine `.OnyxShow`-Datei ist ein ZIP mit einem SQL-Server-Backup (`Show`) der Onyx-Datenbank.
//! Statt SQL Server zu starten, lesen wir die Datenseiten (8 KiB) direkt: Katalogtabellen
//! (`sysschobjs`, `syscolpars`) liefern Tabellen- und Spaltennamen, danach lassen sich alle
//! Nutzertabellen als Zeilen lesen. Unterstützt werden nur die Typen, die Onyx tatsächlich nutzt.

use crate::engine::{Cue, Cuelist, Group, Show, Values};
use crate::fixture::{Attr, Fixture, FixtureType};
use std::collections::HashMap;
use std::io::Read;

const PAGE: usize = 8192;

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Null,
    Int(i64),
    Float(f64),
    Str(String),
    Bytes(Vec<u8>),
}

impl Val {
    pub fn int(&self) -> Option<i64> {
        if let Val::Int(i) = self { Some(*i) } else { None }
    }
    pub fn float(&self) -> Option<f64> {
        match self {
            Val::Float(f) => Some(*f),
            Val::Int(i) => Some(*i as f64),
            _ => None,
        }
    }
    pub fn str(&self) -> Option<&str> {
        if let Val::Str(s) = self { Some(s) } else { None }
    }
    pub fn bytes(&self) -> Option<&[u8]> {
        if let Val::Bytes(b) = self { Some(b) } else { None }
    }
}

#[derive(Clone, Debug)]
struct Col {
    name: String,
    xtype: u8,
    /// Länge in Bytes (-1 = max)
    len: i16,
    colid: i32,
}

impl Col {
    fn new(name: &str, xtype: u8, len: i16, colid: i32) -> Col {
        Col { name: name.into(), xtype, len, colid }
    }

    /// Feste Länge in Bytes, `None` = variable Länge.
    fn fixed(&self) -> Option<usize> {
        Some(match self.xtype {
            48 | 104 => 1,
            52 => 2,
            56 | 59 | 58 | 122 => 4,
            127 | 62 | 61 | 60 => 8,
            36 => 16,
            175 | 173 => self.len as usize,
            239 => self.len as usize,
            40 => 3,
            _ => return None,
        })
    }
}

/// Physische Position einer Spalte im Datensatz.
#[derive(Clone, Debug)]
struct Layout {
    col: Col,
    /// >= 0: fester Offset im Datensatz; < 0: variable Spalte Nr. -off (1-basiert)
    off: i32,
    /// 1-basierter Index im Null-Bitmap
    nullbit: usize,
    bitpos: u8,
}

/// Layout für Systemtabellen, deren physische Reihenfolge der Spaltenreihenfolge entspricht.
fn sequential(cols: &[Col]) -> Vec<Layout> {
    let (mut cur, mut var, mut bits) = (4i32, 0i32, 0u8);
    cols.iter()
        .enumerate()
        .map(|(i, c)| {
            let (off, bitpos) = match c.fixed() {
                Some(_) if c.xtype == 104 => {
                    if bits % 8 == 0 {
                        cur += 1;
                    }
                    bits += 1;
                    (cur - 1, (bits - 1) % 8)
                }
                Some(n) => {
                    cur += n as i32;
                    (cur - n as i32, 0)
                }
                None => {
                    var += 1;
                    (-var, 0)
                }
            };
            Layout { col: c.clone(), off, nullbit: i + 1, bitpos }
        })
        .collect()
}

/// Alle Seiten des Backups, nach Seitennummer.
pub struct Db {
    pages: HashMap<u32, Vec<u8>>,
    tables: HashMap<String, (i32, Vec<i64>, Vec<Layout>)>,
}

pub struct Table {
    names: Vec<String>,
    pub rows: Vec<Vec<Val>>,
}

impl Table {
    pub fn col(&self, name: &str) -> usize {
        self.names.iter().position(|n| n.eq_ignore_ascii_case(name)).unwrap_or_else(|| panic!("Spalte {name} fehlt"))
    }
}

/// Lesezugriffe ohne Panik: außerhalb des Puffers (beschädigte Datei) kommt 0 / leer zurück.
fn u16_at(b: &[u8], o: usize) -> usize {
    b.get(o..o + 2).map_or(0, |s| u16::from_le_bytes([s[0], s[1]]) as usize)
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    b.get(o..o + 4).map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn sl(b: &[u8], a: usize, e: usize) -> &[u8] {
    b.get(a..e).unwrap_or(&[])
}

/// Liefert den Rohinhalt der Backup-Datei (ZIP mit einem Eintrag oder direkt das Backup).
fn unwrap_backup(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.starts_with(b"PK") {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("ZIP: {e}"))?;
        let mut f = zip.by_index(0).map_err(|e| format!("ZIP: {e}"))?;
        let mut out = Vec::new();
        f.read_to_end(&mut out).map_err(|e| e.to_string())?;
        Ok(out)
    } else {
        Ok(bytes.to_vec())
    }
}

/// Torn-Page-Schutz: SQL Server ersetzt die untersten 2 Bit des letzten Bytes jedes 512-Byte-Sektors
/// (außer dem ersten) durch ein Muster und legt die Originalbits im Seitenkopf (`m_tornBits`) ab.
fn restore_torn_bits(page: &mut [u8]) {
    let torn = u32_at(page, 60);
    for k in 1..16 {
        let b = &mut page[512 * k + 511];
        *b = (*b & !3) | ((torn >> (2 * k)) & 3) as u8;
    }
}

impl Db {
    pub fn open(bytes: &[u8]) -> Result<Db, String> {
        let raw = unwrap_backup(bytes)?;
        if !raw.starts_with(b"TAPE") {
            return Err("Keine Onyx-Show (SQL-Server-Backup erwartet)".into());
        }
        let mut pages = HashMap::new();
        for chunk in raw.chunks_exact(PAGE) {
            // Seitenkopf: Version 1, Seiten-ID = Datei 1 : Nummer
            if chunk[0] == 1 && u16_at(chunk, 36) == 1 && (1..=20).contains(&chunk[1]) {
                let mut page = chunk.to_vec();
                restore_torn_bits(&mut page);
                pages.insert(u32_at(chunk, 32), page);
            }
        }
        if pages.is_empty() {
            return Err("Keine Datenseiten im Backup gefunden".into());
        }
        let mut db = Db { pages, tables: HashMap::new() };
        db.load_catalog()?;
        Ok(db)
    }

    /// Alle Datensätze eines Objekts (Heap oder gruppierter Index).
    fn records(&self, obj: i32) -> Vec<&[u8]> {
        self.records_where(|idx, o| o == obj && idx <= 1)
    }

    /// Datensätze einer Tabelle: Seiten mit Objekt-ID der Tabelle oder mit passender Allocation-Unit-ID
    /// (nach Index-Umbauten tragen die Seiten stattdessen eine eigene ID).
    fn records_table(&self, obj: i32, auids: &[i64]) -> Vec<&[u8]> {
        self.records_where(|idx, o| {
            (o == obj && idx <= 1) || auids.contains(&(((idx as i64) << 48) | ((o as u32 as i64) << 16)))
        })
    }

    fn records_where(&self, keep: impl Fn(usize, i32) -> bool) -> Vec<&[u8]> {
        let mut ids: Vec<&u32> = self.pages.keys().collect();
        ids.sort();
        let mut out = vec![];
        for id in ids {
            let p = &self.pages[id];
            let (ty, idx, slots, o) = (p[1], u16_at(p, 6), u16_at(p, 22), u32_at(p, 24) as i32);
            if ty != 1 || !keep(idx, o) {
                continue;
            }
            for s in 0..slots {
                let off = u16_at(p, PAGE - 2 * (s + 1));
                if off < 96 || off >= PAGE {
                    continue;
                }
                let rtype = (p[off] >> 1) & 7;
                // 0 = Datensatz, 1 = verschobener Datensatz; Ghost- und Verweis-Sätze überspringen
                if rtype <= 1 {
                    out.push(&p[off..]);
                }
            }
        }
        out
    }

    /// Datensatz anhand der physischen Spaltenpositionen lesen.
    fn parse(&self, rec: &[u8], cols: &[Layout]) -> Vec<Val> {
        let fixed_end = u16_at(rec, 2);
        let ncols = u16_at(rec, fixed_end);
        let bm = (ncols + 7) / 8;
        let nullmap = sl(rec, fixed_end + 2, fixed_end + 2 + bm);
        let p = fixed_end + 2 + bm;
        let has_var = rec[0] & 0x20 != 0;
        let nvar = if has_var { u16_at(rec, p) } else { 0 };
        let var_offs: Vec<usize> = (0..nvar).map(|i| u16_at(rec, p + 2 + 2 * i)).collect();
        let var_start = if has_var { p + 2 + 2 * nvar } else { p };
        let mut out = vec![Val::Null; cols.len()];
        for (i, l) in cols.iter().enumerate() {
            let c = &l.col;
            let nb = l.nullbit.wrapping_sub(1);
            if nb < ncols && nullmap.get(nb / 8).is_some_and(|b| b >> (nb % 8) & 1 == 1) {
                continue;
            }
            if l.off >= 0 {
                let o = l.off as usize;
                let n = c.fixed().unwrap_or(0);
                if o + n.max(1) > fixed_end.min(rec.len()) {
                    continue;
                }
                let b = &rec[o..o + n.max(1)];
                out[i] = match c.xtype {
                    104 => Val::Int((b[0] >> l.bitpos & 1) as i64),
                    48 => Val::Int(b[0] as i64),
                    52 => Val::Int(i16::from_le_bytes([b[0], b[1]]) as i64),
                    56 => Val::Int(i32::from_le_bytes(b.try_into().unwrap()) as i64),
                    127 => Val::Int(i64::from_le_bytes(b.try_into().unwrap())),
                    62 => Val::Float(f64::from_le_bytes(b.try_into().unwrap())),
                    59 => Val::Float(f32::from_le_bytes(b.try_into().unwrap()) as f64),
                    36 => Val::Str(guid(b)),
                    175 => Val::Str(String::from_utf8_lossy(b).trim_end().to_string()),
                    239 => Val::Str(utf16(b)),
                    _ => Val::Bytes(b.to_vec()),
                };
            } else {
                let vi = (-l.off - 1) as usize;
                if vi >= nvar {
                    continue;
                }
                let end_raw = var_offs[vi];
                let end = end_raw & 0x7FFF;
                let start = if vi == 0 { var_start } else { var_offs[vi - 1] & 0x7FFF };
                let data = sl(rec, start.min(end), end);
                out[i] = if end_raw & 0x8000 != 0 { self.lob(data) } else { decode_var(c, data) };
            }
        }
        out
    }

    /// Komplexe Spalte (LOB): folgt dem Zeiger auf die LOB-Seiten und liefert den Inhalt.
    fn lob(&self, data: &[u8]) -> Val {
        if data.len() == 16 && u16_at(data, 0) == 0 {
            let mut out = Vec::new();
            self.lob_walk(u32_at(data, 8), u16_at(data, 14), &mut out, 0);
            return Val::Bytes(out);
        }
        Val::Null
    }

    /// Roher Datensatz eines Slots auf einer Seite.
    fn lob_record(&self, page: u32, slot: usize) -> &[u8] {
        let Some(p) = self.pages.get(&page) else { return &[] };
        if slot >= u16_at(p, 22) {
            return &[];
        }
        let off = u16_at(p, PAGE - 2 * (slot + 1));
        p.get(off..).unwrap_or(&[])
    }

    /// Liest einen LOB-Baum: 0 = kleine Wurzel, 3 = Daten, 5/2 = Wurzel/Knoten mit Links.
    fn lob_walk(&self, page: u32, slot: usize, out: &mut Vec<u8>, depth: u8) {
        let r = self.lob_record(page, slot);
        if r.len() < 20 || depth > 8 {
            return;
        }
        match u16_at(r, 12) {
            0 => {
                let n = u32_at(r, 14) as usize;
                out.extend_from_slice(r.get(20..20 + n).unwrap_or(&[]));
            }
            3 => {
                let n = u32_at(r, 2) as usize;
                out.extend_from_slice(r.get(14..n.min(r.len())).unwrap_or(&[]));
            }
            t @ (5 | 2) => {
                let first = if t == 5 { 24 } else { 20 };
                for i in 0..u16_at(r, 16) {
                    let o = first + 12 * i;
                    if o + 12 > r.len() {
                        break;
                    }
                    self.lob_walk(u32_at(r, o + 4), u16_at(r, o + 10), out, depth + 1);
                }
            }
            _ => {}
        }
    }

    fn load_catalog(&mut self) -> Result<(), String> {
        // sysschobjs (Objekt-ID 34) und syscolpars (41) haben feste Spalten.
        let objs = [
            Col::new("id", 56, 4, 1),
            Col::new("name", 231, 256, 2),
            Col::new("nsid", 56, 4, 3),
            Col::new("nsclass", 48, 1, 4),
            Col::new("status", 56, 4, 5),
            Col::new("type", 175, 2, 6),
            Col::new("pid", 56, 4, 7),
            Col::new("pclass", 48, 1, 8),
            Col::new("intprop", 56, 4, 9),
            Col::new("created", 61, 8, 10),
            Col::new("modified", 61, 8, 11),
            Col::new("status2", 56, 4, 12),
        ];
        let colp = [
            Col::new("id", 56, 4, 1),
            Col::new("number", 52, 2, 2),
            Col::new("colid", 56, 4, 3),
            Col::new("name", 231, 256, 4),
            Col::new("xtype", 48, 1, 5),
            Col::new("utype", 56, 4, 6),
            Col::new("length", 52, 2, 7),
            Col::new("prec", 48, 1, 8),
            Col::new("scale", 48, 1, 9),
            Col::new("collationid", 56, 4, 10),
            Col::new("status", 56, 4, 11),
            Col::new("maxinrow", 52, 2, 12),
            Col::new("xmlns", 56, 4, 13),
            Col::new("dflt", 56, 4, 14),
            Col::new("chk", 56, 4, 15),
            Col::new("idtval", 165, 64, 16),
        ];
        let (seq_objs, seq_colp) = (sequential(&objs), sequential(&colp));
        let mut tables: HashMap<i32, String> = HashMap::new();
        for r in self.records(34) {
            let v = self.parse(r, &seq_objs);
            // type 'U ' = Nutzertabelle
            if v[5].str() == Some("U") {
                if let (Some(id), Some(n)) = (v[0].int(), v[1].str()) {
                    tables.insert(id as i32, n.to_string());
                }
            }
        }
        let mut cols: HashMap<i32, Vec<Col>> = HashMap::new();
        for r in self.records(41) {
            let v = self.parse(r, &seq_colp);
            let Some(id) = v[0].int().map(|i| i as i32) else { continue };
            if !tables.contains_key(&id) {
                continue;
            }
            // Gelöschte Spalten haben keinen Namen, belegen aber weiter Platz im Datensatz.
            let (Some(x), Some(l), Some(ci)) = (v[4].int(), v[6].int(), v[2].int()) else { continue };
            cols.entry(id).or_default().push(Col::new(v[3].str().unwrap_or(""), x as u8, l as i16, ci as i32));
        }
        // sysrowsets (5): Rowset-ID je Tabelle; sysrscols (3): Offset und Null-Bit je Spalte.
        let rowsets = sequential(&[
            Col::new("rowsetid", 127, 8, 1),
            Col::new("ownertype", 48, 1, 2),
            Col::new("idmajor", 56, 4, 3),
            Col::new("idminor", 56, 4, 4),
        ]);
        let mut rs_of: HashMap<i32, i64> = HashMap::new();
        for r in self.records(5) {
            let v = self.parse(r, &rowsets);
            if let (Some(rs), Some(o), Some(m)) = (v[0].int(), v[2].int(), v[3].int()) {
                if m <= 1 {
                    rs_of.insert(o as i32, rs);
                }
            }
        }
        let rscols = sequential(&[
            Col::new("rsid", 127, 8, 1),
            Col::new("rscolid", 56, 4, 2),
            Col::new("hbcolid", 56, 4, 3),
            Col::new("rcmodified", 127, 8, 4),
            Col::new("ti", 56, 4, 5),
            Col::new("cid", 56, 4, 6),
            Col::new("ordkey", 52, 2, 7),
            Col::new("maxinrowlen", 52, 2, 8),
            Col::new("status", 56, 4, 9),
            Col::new("offset", 56, 4, 10),
            Col::new("nullbit", 56, 4, 11),
            Col::new("bitpos", 52, 2, 12),
        ]);
        // (rowset, spalten-id) -> (offset, nullbit, bitpos)
        let mut phys: HashMap<(i64, i32), (i32, usize, u8)> = HashMap::new();
        for r in self.records(3) {
            let v = self.parse(r, &rscols);
            if let (Some(rs), Some(cid), Some(off), Some(nb), Some(bp)) = (v[0].int(), v[1].int(), v[9].int(), v[10].int(), v[11].int()) {
                phys.insert((rs, cid as i32), (off as u16 as i16 as i32, nb as usize, bp as u8));
            }
        }
        // sysallocunits (7): Rowset → Allocation Units mit Zeilendaten (type 1)
        let allocs = sequential(&[Col::new("auid", 127, 8, 1), Col::new("type", 48, 1, 2), Col::new("ownerid", 127, 8, 3)]);
        let mut au_of: HashMap<i64, Vec<i64>> = HashMap::new();
        for r in self.records(7) {
            let v = self.parse(r, &allocs);
            if let (Some(a), Some(1), Some(o)) = (v[0].int(), v[1].int(), v[2].int()) {
                au_of.entry(o).or_default().push(a);
            }
        }
        for (id, name) in tables {
            let (Some(cs), Some(rs)) = (cols.remove(&id), rs_of.get(&id)) else { continue };
            let mut cs = cs;
            cs.sort_by_key(|c| c.colid);
            let layouts: Vec<Layout> = cs
                .into_iter()
                .filter_map(|col| {
                    let (off, nullbit, bitpos) = *phys.get(&(*rs, col.colid))?;
                    Some(Layout { col, off, nullbit, bitpos })
                })
                .collect();
            self.tables.insert(name, (id, au_of.get(rs).cloned().unwrap_or_default(), layouts));
        }
        if self.tables.is_empty() {
            return Err("Katalog nicht lesbar (unbekanntes Backup-Format)".into());
        }
        Ok(())
    }

    pub fn table_names(&self) -> Vec<&String> {
        let mut v: Vec<&String> = self.tables.keys().collect();
        v.sort();
        v
    }

    pub fn table(&self, name: &str) -> Result<Table, String> {
        let (id, auids, layouts) = self.tables.get(name).ok_or_else(|| format!("Tabelle {name} fehlt"))?;
        let rows = self.records_table(*id, auids).into_iter().map(|r| self.parse(r, layouts)).collect();
        Ok(Table { names: layouts.iter().map(|l| l.col.name.clone()).collect(), rows })
    }
}

fn guid(b: &[u8]) -> String {
    format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        u32::from_le_bytes(b[0..4].try_into().unwrap()),
        u16::from_le_bytes(b[4..6].try_into().unwrap()),
        u16::from_le_bytes(b[6..8].try_into().unwrap()),
        b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

fn utf16(b: &[u8]) -> String {
    let u: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16_lossy(&u)
}

fn decode_var(c: &Col, d: &[u8]) -> Val {
    match c.xtype {
        231 => Val::Str(utf16(d)),
        167 => Val::Str(String::from_utf8_lossy(d).into_owned()),
        _ => Val::Bytes(d.to_vec()),
    }
}

/// Onyx-Kanalname → Attribut. Nicht unterstützte Kanäle (auch Fein-Kanäle von Farben) bleiben `None`.
fn attr_for(name: &str) -> Option<Attr> {
    Some(match name {
        "Intensity" | "Dimmer" => Attr::Dimmer,
        "Red" => Attr::Red,
        "Green" => Attr::Green,
        "Blue" => Attr::Blue,
        "White" => Attr::White,
        "Amber" => Attr::Amber,
        "UV" => Attr::Uv,
        "Pan" => Attr::Pan,
        "Tilt" => Attr::Tilt,
        "Pan Fine" => Attr::PanFine,
        "Tilt Fine" => Attr::TiltFine,
        "Zoom" => Attr::Zoom,
        "Strobe" => Attr::Strobe,
        "Color" => Attr::ColorWheel,
        "Gobo" => Attr::Gobo,
        "Prism" => Attr::Prism,
        "Iris" => Attr::Iris,
        "Focus" => Attr::Focus,
        "Frost" => Attr::Frost,
        _ => return None,
    })
}

/// Eine Onyx-Show (`.OnyxShow`) in ein [`Show`] umsetzen. Liefert zusätzlich Hinweise
/// zu allem, was nicht übernommen werden konnte.
pub fn import(bytes: &[u8]) -> Result<(Show, Vec<String>), String> {
    std::panic::catch_unwind(|| import_inner(bytes)).unwrap_or_else(|_| Err("Unbekanntes Onyx-Format".into()))
}

fn import_inner(bytes: &[u8]) -> Result<(Show, Vec<String>), String> {
    let db = Db::open(bytes)?;
    let mut warn = Vec::new();
    let s = |v: &Val| v.str().unwrap_or("").to_string();

    // DMX-Adressen je logischem Kanal
    let t = db.table("DMXChannel")?;
    let (lc, un, ad) = (t.col("logicalAddress"), t.col("universe"), t.col("address"));
    let dmx: HashMap<i64, (i64, i64)> = t.rows.iter().filter_map(|r| Some((r[lc].int()?, (r[un].int()?, r[ad].int()?)))).collect();

    // Kanalnamen
    let t = db.table("FixtureTypeDMXChannel")?;
    let (ci, cn) = (t.col("idFixtureTypeChannel"), t.col("name"));
    let chan_name: HashMap<String, String> = t.rows.iter().map(|r| (s(&r[ci]), s(&r[cn]))).collect();

    // Gerätetypen: Kanalgruppe → Typ
    let t = db.table("FixtureType")?;
    let (ti, tm, tmo, tmd) = (t.col("id"), t.col("manufacturer"), t.col("model"), t.col("mode"));
    let type_label: HashMap<String, String> = t
        .rows
        .iter()
        .map(|r| {
            let l = [s(&r[tm]), s(&r[tmo]), s(&r[tmd])].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" ");
            (s(&r[ti]), l)
        })
        .collect();
    let t = db.table("FixtureTypeDMXChannelGroup")?;
    let (gi, gt) = (t.col("idFixtureTypeChannelGroup"), t.col("idFixtureType"));
    let group_type: HashMap<String, String> = t.rows.iter().map(|r| (s(&r[gi]), s(&r[gt]))).collect();

    // logische Kanäle je Teilgerät
    let t = db.table("LogicalChannel")?;
    let (la, lf, lt, lr) = (t.col("address"), t.col("idFixture"), t.col("idFixtureTypeChannel"), t.col("d_resolution"));
    struct Lc {
        addr: i64,
        name: String,
        res: i64,
    }
    let mut by_part: HashMap<String, Vec<Lc>> = HashMap::new();
    for r in &t.rows {
        by_part.entry(s(&r[lf])).or_default().push(Lc {
            addr: r[la].int().unwrap_or(-1),
            name: chan_name.get(&s(&r[lt])).cloned().unwrap_or_default(),
            res: r[lr].int().unwrap_or(8),
        });
    }

    // Geräte: Teilgeräte gehören zum Hauptgerät (`idSuperPart`)
    let t = db.table("Fixture")?;
    let (fi, fs, fnr, fg, fname) =
        (t.col("id"), t.col("idSuperPart"), t.col("fixtureNr"), t.col("idFixtureTypeChannelGroup"), t.col("name"));
    let mut show = Show { name: "Onyx-Import".into(), types: vec![], fixtures: vec![], groups: vec![], palettes: vec![], cues: vec![], lists: vec![], effects: vec![], props: vec![] };
    let mut type_ix: HashMap<String, usize> = HashMap::new();
    let mut part_fixture: HashMap<String, u32> = HashMap::new();
    // logische Adresse → (Gerät, Attribut)
    let mut logical: HashMap<i64, (u32, Option<Attr>, i64)> = HashMap::new();
    let mut skipped = 0;
    for sup in t.rows.iter().filter(|r| r[fi].str() == r[fs].str()) {
        let (id, nr) = (s(&sup[fi]), sup[fnr].int().unwrap_or(0) as u32);
        let parts: Vec<&Vec<Val>> = t.rows.iter().filter(|r| r[fs].str() == Some(&id)).collect();
        let mut chans: Vec<(i64, i64, &Lc)> = vec![];
        for p in &parts {
            for l in by_part.get(&s(&p[fi])).into_iter().flatten() {
                if let Some(&(u, a)) = dmx.get(&l.addr) {
                    chans.push((u, a, l));
                }
            }
            part_fixture.insert(s(&p[fi]), nr);
        }
        let Some(&(uni, start, _)) = chans.iter().min_by_key(|c| (c.0, c.1)) else {
            skipped += 1; // Onyx-interne Geräte (Zonen, Blender …) ohne DMX
            continue;
        };
        let key = group_type.get(&s(&sup[fg])).cloned().unwrap_or_default();
        let ty = *type_ix.entry(key.clone()).or_insert_with(|| {
            let n = chans.iter().map(|c| (c.1 - start + 1) as usize).max().unwrap_or(1).min(512);
            let mut ch: Vec<Option<Attr>> = vec![None; n];
            let mut used = std::collections::HashSet::new();
            for c in chans.iter().filter(|c| c.0 == uni && c.1 - start < 512) {
                if let Some(a) = attr_for(&c.2.name).filter(|a| used.insert(*a)) {
                    ch[(c.1 - start) as usize] = Some(a);
                }
            }
            show.types.push(FixtureType { name: type_label.get(&key).cloned().unwrap_or_else(|| "Onyx".into()), channels: ch });
            show.types.len() - 1
        });
        let first_attr: Vec<Option<Attr>> = show.types[ty].channels.clone();
        for c in &chans {
            let rel = (c.1 - start) as usize;
            let a = first_attr.get(rel).copied().flatten().filter(|a| attr_for(&c.2.name) == Some(*a));
            logical.insert(c.2.addr, (nr, a, c.2.res));
        }
        let name = s(&sup[fname]);
        show.fixtures.push(Fixture {
            id: nr,
            name: if name.is_empty() { format!("Fixture {nr}") } else { name },
            ty,
            universe: uni.clamp(1, 65535) as u16,
            address: start.clamp(1, 512) as u16,
            x: 0.0,
            y: 0.0,
        });
    }
    if skipped > 0 {
        warn.push(format!("{skipped} Onyx-interne Geräte ohne DMX übersprungen"));
    }
    show.fixtures.sort_by_key(|f| f.id);

    // Gruppen (nur normale)
    let t = db.table("FixtureGroup")?;
    let (gn, gname, gty) = (t.col("nr"), t.col("name"), t.col("grouptype"));
    let m = db.table("FixtureGroupMember")?;
    let (mg, mf) = (m.col("groupNr"), m.col("idFixture"));
    for r in t.rows.iter().filter(|r| r[gty].int().unwrap_or(0) == 0) {
        let nr = r[gn].int();
        let mut ids: Vec<u32> = m.rows.iter().filter(|x| x[mg].int() == nr).filter_map(|x| part_fixture.get(&s(&x[mf])).copied()).collect();
        ids.sort_unstable();
        ids.dedup();
        ids.retain(|i| show.fixtures.iter().any(|f| f.id == *i));
        if !ids.is_empty() {
            show.groups.push(Group { name: s(&r[gname]), ids });
        }
    }

    // Cuelists
    let l = db.table("CueListsV3")?;
    let (li, ln) = (l.col("CueListId"), l.col("CueListName"));
    let c = db.table("CueValuesV3")?;
    let (ccl, crow, cn, cf, cd, cb, cv) =
        (c.col("CueListId"), c.col("Row"), c.col("CueName"), c.col("FadeIn"), c.col("DelayIn"), c.col("BlockingCue"), c.col("ChannelValues"));
    let mut lists: Vec<_> = l.rows.iter().collect();
    lists.sort_by_key(|r| r[li].int());
    let mut dropped = 0usize;
    for lr in lists {
        let mut rows: Vec<_> = c.rows.iter().filter(|r| r[ccl].int() == lr[li].int()).collect();
        rows.sort_by_key(|r| r[crow].int());
        let mut cues = vec![];
        for r in rows {
            let mut values: Values = Values::new();
            for rec in r[cv].bytes().unwrap_or(&[]).chunks_exact(20) {
                let w = |i: usize| u32::from_le_bytes(rec[i..i + 4].try_into().unwrap());
                match logical.get(&(w(0) as i64)) {
                    Some((fx, Some(a), res)) => {
                        let v = if *res == 16 { w(8) >> 8 } else { w(8) };
                        values.entry(*fx).or_default().insert(*a, v.min(255) as u8);
                    }
                    _ => dropped += 1,
                }
            }
            cues.push(Cue {
                name: s(&r[cn]),
                fade: r[cf].int().unwrap_or(0) as f32 / 100.0,
                follow: None,
                values,
                delay: r[cd].int().unwrap_or(0) as f32 / 100.0,
                comment: String::new(),
                block: r[cb].int().unwrap_or(0) != 0,
            });
        }
        let mut cl = Cuelist::new(&s(&lr[ln]));
        cl.cues = cues;
        show.lists.push(cl);
    }
    if dropped > 0 {
        warn.push(format!("{dropped} Cue-Werte für nicht unterstützte Kanäle verworfen"));
    }
    warn.push("Presets, Effekte, Layout, Makros und Playback-Zuordnung werden nicht importiert".into());
    Ok((show, warn))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_channel_names() {
        assert_eq!(attr_for("Intensity"), Some(Attr::Dimmer));
        assert_eq!(attr_for("Pan Fine"), Some(Attr::PanFine));
        assert_eq!(attr_for("Red Fine"), None);
    }

    #[test]
    fn rejects_garbage() {
        assert!(import(b"kein onyx").is_err());
    }

    /// Lokaler Test mit echter Show: `ONYX_TEST_SHOW=/pfad/x.OnyxShow cargo test onyx -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn read_real_show() {
        let path = std::env::var("ONYX_TEST_SHOW").unwrap();
        let db = Db::open(&std::fs::read(&path).unwrap()).unwrap();
        println!("{:?}", db.table_names());
        let cv = db.table("CueValuesV3").unwrap();
        let (a, b) = (cv.col("AmountValues"), cv.col("ChannelValues"));
        for r in &cv.rows {
            let d = r[b].bytes().unwrap_or(&[]);
            assert_eq!(d.len() as i64, r[a].int().unwrap() * 20);
        }
        let (show, warn) = import(&std::fs::read(&path).unwrap()).unwrap();
        println!("{} Geräte, {} Typen, {} Gruppen, {} Listen; {warn:?}", show.fixtures.len(), show.types.len(), show.groups.len(), show.lists.len());
        for l in &show.lists {
            println!("{}: {} Cues, Werte im 1.: {:?}", l.name, l.cues.len(), l.cues.first().map(|c| c.values.values().map(|v| v.len()).sum::<usize>()));
        }
        for t in ["Fixture", "FixtureType", "CueValuesV3", "CueListsV3", "FixtureGroup", "FixtureGroupMember"] {
            let tb = db.table(t).unwrap();
            println!("{t}: {} Zeilen", tb.rows.len());
            if let Some(r) = tb.rows.first() {
                println!("{:?}", &r[..r.len().min(8)]);
            }
        }
    }
}
