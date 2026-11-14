# Lichtpult

Open-Source-Lichtsteuerung für Bühnentechnik, nativ für Linux, geschrieben in Rust (egui). OLED-Dunkeldesign mit einstellbarer Akzentfarbe (⚙ oben rechts).

```
cargo run --release
cargo test
```

## Aufbau
- `engine.rs` – Show-Daten, Programmer, Cues, Fades/Tracking, Master/Blackout, DMX-Rendering.
- `fixture.rs` – Attribute, Fixture-Typen und Import aus der [Open Fixture Library](https://open-fixture-library.org) (JSON).
- `lib.rs` – Art-Net-Ausgabe (44 Hz), Speichern/Laden (`~/.local/share/lichtpult/show.json`).
- `main.rs`, `theme.rs` – UI.

**Alle Bedienelemente steuern die Engine nur über `engine::Action`.** UI, MIDI und später
Hardware-Pulte (z. B. Behringer X-Touch, Onyx NX-Touch) werden auf dieselben Aktionen abgebildet.

## Fixtures importieren
OFL-JSON-Dateien oder ganze Ordner (z. B. ein Klon des OFL-Repos) ins Fenster ziehen oder im Patch-Tab den Pfad eintragen.
Jeder Modus wird ein eigener Typ. Fine-Kanäle (16 Bit) und noch nicht unterstützte Funktionen (Gobos, Farbräder, Strobe …)
belegen ihre DMX-Adresse, bleiben aber bei 0.

## Entwicklung
`LICHTPULT_SHOT=/tmp/x.ppm [LICHTPULT_VIEW=patch|cues|output] [LICHTPULT_SELECT=1] cargo run` speichert den Fensterinhalt und beendet die App.

## Tastatur
`Leertaste` GO · `Backspace` Back · `Esc` Auswahl löschen · `Strg/Shift`+Klick Mehrfachauswahl

## Roadmap
1. ✅ Patch, Programmer, Gruppen, Paletten, Cue-Liste mit Fades, DMX-Monitor
2. GDTF-Import, Farbräder/Gobos/Strobe, 16-Bit-Pan/Tilt, Effekte, mehrere Cuelisten/Playbacks, Trigger/Timecode
3. sACN, USB-DMX, MIDI/OSC-Controller, 2D-/3D-Visualizer

Lizenz: MIT
