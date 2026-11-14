# Lichtpult

Open-Source-Lichtsteuerung für Bühnentechnik, nativ für Linux, geschrieben in Rust (egui). OLED-Dunkeldesign mit einstellbarer Akzentfarbe (⚙ oben rechts).

```
cargo run --release
cargo test
```

## Aufbau
- `engine.rs` – Show-Daten, Programmer, Cues, Fades/Tracking, Master/Blackout, DMX-Rendering.
- `fixture.rs` – Attribute und Fixture-Typen (eingebaut; OFL/GDTF-Import folgt).
- `lib.rs` – Art-Net-Ausgabe (44 Hz), Speichern/Laden (`~/.local/share/lichtpult/show.json`).
- `main.rs`, `theme.rs` – UI.

**Alle Bedienelemente steuern die Engine nur über `engine::Action`.** UI, MIDI und später
Hardware-Pulte (z. B. Behringer X-Touch, Onyx NX-Touch) werden auf dieselben Aktionen abgebildet.

## Tastatur
`Leertaste` GO · `Backspace` Back · `Esc` Auswahl löschen · `Strg/Shift`+Klick Mehrfachauswahl

## Roadmap
1. ✅ Patch, Programmer, Gruppen, Paletten, Cue-Liste mit Fades, DMX-Monitor
2. Fixture-Import (OFL/GDTF), Effekte, mehrere Cuelisten/Playbacks, Trigger/Timecode
3. sACN, USB-DMX, MIDI/OSC-Controller, 2D-/3D-Visualizer

Lizenz: MIT
