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
`LICHTPULT_SHOT=/tmp/x.ppm [LICHTPULT_VIEW=stage|patch|output] [LICHTPULT_SELECT=1] cargo run` speichert den Fensterinhalt und beendet die App.

## Tastatur
`Leertaste` GO · `Backspace` Back · `Esc` Auswahl löschen · `Strg+A` alle · `Strg+Z` / `Strg+Shift+Z` Undo/Redo · `Strg+L` Befehlszeile

## Befehlszeile
Mehrere Befehle mit `;`: `1 thru 8 @ 50` · `1 + 3 - 2 red 255` · `group 1 pal 2` · `go 2` · `back` · `release 1` · `goto 3` · `level 1 200` · `master 128` · `blackout on` · `fx 1` / `fx stop` · `record cue|group|palette color` · `undo` · `clear`

## Funktionen
Mehrere Cuelisten mit Playback-Fadern, Follow/Loop, HTP/LTP-Mischung, Effekt-Engine (Intensität/Farbe/Position), 2D-Bühnenansicht, Undo/Redo, Farbrad/Gobo/Prisma/Iris/Fokus/Frost, 16-Bit-Pan/Tilt (Fine-Kanäle aus OFL), ESP32-Art-Net→DMX-Node (`firmware/esp32-dmx`).

## Roadmap
GDTF-Import, sACN, MIDI/OSC-Controller, Trigger/Timecode, 3D-Visualizer

Lizenz: MIT
