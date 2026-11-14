# Lichtpult

Open-Source-Lichtsteuerung für Bühnentechnik im Stil von Obsidian Onyx.
**Tauri 2 + Svelte 5 + TypeScript** (UI) auf einem Rust-Kern (Engine, DMX, Art-Net/sACN, MIDI, GDTF/OFL). Die alte egui-Version liegt unverändert in `archive/egui/`.

## Aufbau
- `src/` – Rust-Bibliothek `lichtpult`: `engine.rs` (Show, Programmer, Cues, Effekte), `cmd.rs` (Befehlszeile), `api.rs` (JSON-API, `Core::call`).
- `src/bin/server.rs` – Browser-Entwicklungsserver (`/api/<methode>`, liefert `ui/dist`).
- `src-tauri/` – Desktop-Shell, ein Tauri-Command `api`.
- `ui/` – Svelte-Frontend (Compose, Playback, Plan, Patch, Output).

Alle Bedienelemente steuern die Engine nur über `engine::Action`.

## Oberfläche (`ui/src`)
- `store.svelte.ts` – Zustand: Polling der Engine, `act`/`cmd`, Rechtsklick-Menü, Fenster, Dialoge.
- `App.svelte` – Rahmen: Kopfzeile, linke Navigation, Bank, Befehlszeile.
- Ansichten: `Compose` (Fixture Center, Programmer, Parameter), `Presets`, `Playback`, `Plan`, `Patch`, `Output`, `Universes` (Belegung, Live-Raster, Auto-Adresse).
- `Windows.svelte` – frei verschiebbare Fenster: Editoren (Cue, Cuelist, FX, Gruppe, Preset, Fixture), Keypad und alle Ansichten
  (Menü *Windows*, Liste in `WIN_VIEWS`). `CueValues` und `Status` gibt es nur als Fenster.
- Auswahl: Klick, Ziehen, Strg (umschalten), Shift (Bereich); Gruppen mit Strg/Shift hinzufügen/entfernen; Tabellen haben Mehrfachauswahl.
- Anpassen: Trennlinien ziehen (über der Bank, zwischen den Compose-Spalten/-Zeilen; Doppelklick = Standard, *Windows → Reset layout*), gespeichert im Browser-Speicher. Patch, Playback, Output, Stage und Universes haben eine ziehbare rechte Spalte. Tastatur: Strg+1–7 Ansicht, Strg+A alles, Esc schließt Menü/Fenster/Auswahl. Ansicht und Fenster werden gemerkt; *Windows* bietet Workspaces speichern/laden sowie Tile/Cascade. Eigene Fixture-Typen (bearbeiten/löschen im Patch): *Patch → Create fixture type…* (`FixtureEditor`).

## Starten
```
cd ui && npm install && npm run build && cd ..
cargo tauri dev                       # Desktop-App (cargo install tauri-cli --version "^2")
cargo tauri build                     # fertige Desktop-App / Installer
cargo run --bin lichtpult-server      # nur zum Testen im Browser (http://127.0.0.1:8787)
cargo test --lib
```

## Befehlszeile
Mehrere Befehle mit `;`: `1 thru 8 @ 50` · `1 + 3 - 2 red 255` · `group 1 pal 2` · `go 2` · `back` · `release 1` · `goto 3` · `level 1 200` · `master 128` · `blackout on` · `fx 1` / `fx stop` · `record scene|cue|group|palette color` · `undo` · `clear`

## Funktionen
Mehrere Cuelisten mit Playback-Fadern, Follow/Loop, HTP/LTP-Mischung, Effekt-Engine (Intensität/Farbe/Position), 2D-Bühnenansicht, Undo/Redo, Farbrad/Gobo/Prisma/Iris/Fokus/Frost, 16-Bit-Pan/Tilt (Fine-Kanäle aus OFL), ESP32-Art-Net→DMX-Node (`firmware/esp32-dmx`).

- **Layout:** Navigationsleiste links, schmale Kopfzeile (GM, Blackout, Ausgabe-Status), unten die Playback-Leiste mit Szenen-Pads (Klick = abrufen, grün = aktiv) und GO/Zurück/Stop.
- **Tabs:** *Pult* (Pads für Fixtures und Gruppen; unten Attribut-Reiter A Intensität, B Position, C Farbe, D Beam mit Reglern und Paletten, E Effekte), *Szenen* (Liste oder Raster, Inspector für Name/Überblendzeit/Auto-Follow, GO/Back), *Bühne* (Fixtures plus Objekte wie Podest, Traverse, Lautsprecher, Pult, Publikum, Beschriftung), *Patch*, *Ausgabe*.
- **Szenen:** „Als Szene speichern“ sichert den vollständigen Look aller Fixtures als unabhängige Szene und leert den Programmer. GO/Szenen-Klick leeren den Programmer automatisch, damit manuelle Werte die Szene nicht überdecken.
- **Autosave:** Die Show wird alle 3 s gespeichert, wenn sie sich geändert hat.
- **Patch:** Bibliothek mit Kanalvorschau, „Nächste freie“ Adresse, Belegungskarte (512 Kanäle je Universum, Konflikte rot, Klick wählt/setzt Adresse), lückenloses Neuadressieren, Import von OFL-`.json` und GDTF-`.gdtf` (Datei/Ordner/Drag & Drop).
- **Output:** Art-Net, sACN (E1.31) oder beides, optional mit Ziel-IP (Unicast, z. B. ESP32); DMX-Monitor.
- **MIDI:** Eingänge werden automatisch verbunden; Taste/Regler „lernen“ für GO, Back, Freigeben, Playback-Fader, Master, Blackout und Effekte. Zuordnungen in `~/.local/share/lichtpult/midi.json`.

## Roadmap
Trigger/Timecode, 3D-Visualizer, herstellerspezifische Pult-Protokolle (z. B. NX-Touch)

Lizenz: MIT

## Onyx-Werkzeuge

- Auswahl: Invert, Jedes 2., Gegenreihe, Last/Next (Bild ab/auf), zuletzt gewähltes Fixture rot.
- Programmer: Blind (Strg+P), Park/Unpark, Fan (`fan red 0 thru 100`), Entf = Programmer leeren.
- Cues: Verzögerung, Kommentar, Snap (Pos1), globale Überblendgeschwindigkeit; Chase-Listen mit BPM.
- Effekte: 8 Wellenformen, Tempo in BPM.
- Befehlszeile (Strg+L): `invert`, `every 2`, `full`, `blind`, `park`, `snap`, `delete cue 2`, `speed 50` u. a.
