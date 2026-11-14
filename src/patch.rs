//! Patch: Bibliothek, Adressierung, Belegungskarte und Tabelle.

use crate::theme::{self, caption, tile};
use crate::{App, short};
use eframe::egui::{self, Color32, RichText};
use lichtpult::engine::{Action, Engine};

impl App {
    pub fn patch(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        self.patch_map(ui, e, acts);
    }

    /// Seitenleiste: Bibliothek durchsuchen und patchen.
    pub fn patch_side(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        caption(ui, "Bibliothek");
        ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Suchen…").desired_width(f32::INFINITY));
        let q = self.search.to_lowercase();
        let hits: Vec<usize> = (0..self.library.len()).filter(|&i| self.library[i].name.to_lowercase().contains(&q)).collect();
        let list_h = (ui.available_height() - 250.0).max(120.0);
        egui::ScrollArea::vertical().id_salt("lib").max_height(list_h).min_scrolled_height(list_h).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 2.0;
            for &i in hits.iter().take(200) {
                let on = self.patch.0 == i;
                let t = &self.library[i];
                let label = format!("{}   ·   {} Kanäle", short(&t.name, 28), t.channels.len());
                let b = egui::Button::new(RichText::new(label).color(if on { accent } else { theme::TEXT }))
                    .fill(if on { accent.gamma_multiply(0.16) } else { Color32::TRANSPARENT })
                    .stroke(egui::Stroke::NONE)
                    .min_size(egui::vec2(ui.available_width(), 30.0));
                if ui.add(b).clicked() {
                    self.patch.0 = i;
                }
            }
            if hits.len() > 200 {
                ui.label(RichText::new(format!("… {} weitere – Suche eingrenzen", hits.len() - 200)).color(theme::MUTED));
            }
        });
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        let Some(ty) = self.library.get(self.patch.0).cloned() else { return };
        let width = ty.channels.len() as u16;
        let free = e.show.free_address(self.patch.2, width.max(1));
        egui::Grid::new("patch_form").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
            ui.label(RichText::new("Anzahl").color(theme::MUTED));
            ui.add(egui::DragValue::new(&mut self.patch.1).range(1..=512));
            ui.end_row();
            ui.label(RichText::new("Universum").color(theme::MUTED));
            ui.add(egui::DragValue::new(&mut self.patch.2).range(1..=32768));
            ui.end_row();
            ui.label(RichText::new("Adresse").color(theme::MUTED));
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut self.patch.3).range(1..=512));
                if ui.button("Nächste freie").clicked() {
                    if let Some(a) = free {
                        self.patch.3 = a;
                    }
                }
            });
            ui.end_row();
        });
        if self.patch.3 as u32 + self.patch.1 as u32 * width as u32 > 513 {
            ui.label(RichText::new("Passt nicht ganz ins Universum – es werden so viele gepatcht wie passen.").size(12.5).color(theme::DANGER));
        }
        let go = egui::Button::new(RichText::new(format!("{} patchen", short(&ty.name, 22))).strong().color(Color32::BLACK))
            .fill(accent)
            .min_size(egui::vec2(ui.available_width(), 34.0));
        if ui.add_enabled(width > 0, go).clicked() {
            let (lib, count, universe, address) = self.patch;
            let ty = self.library[lib].clone();
            let index = e.show.types.iter().position(|t| t.name == ty.name).unwrap_or(e.show.types.len());
            acts.push(Action::AddTypes(vec![ty]));
            acts.push(Action::AddFixtures { ty: index, count, universe, address });
            self.patch.3 = (address as u32 + count as u32 * width as u32).min(512) as u16;
        }
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        caption(ui, "Importieren (.json · .gdtf)");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.import_path).hint_text("Pfad oder Datei hierher ziehen").desired_width(ui.available_width() - 80.0));
            if ui.button("Import").clicked() {
                let p = std::path::PathBuf::from(self.import_path.trim());
                self.import(vec![p]);
            }
        });
        if !self.status.is_empty() {
            theme::hint(ui, &self.status);
        }
    }

    fn patch_map(&mut self, ui: &mut egui::Ui, e: &Engine, acts: &mut Vec<Action>) {
        let accent = self.accent();
        let mut unis: Vec<u16> = e.show.fixtures.iter().map(|f| f.universe).collect();
        unis.push(self.patch.2);
        unis.sort_unstable();
        unis.dedup();
        let conflicts = e.conflicts();
        ui.horizontal_wrapped(|ui| {
            caption(ui, "Universum");
            for &u in &unis {
                if tile(ui, u.to_string(), u == self.patch.2, accent).clicked() {
                    self.patch.2 = u;
                    self.out_universe = u;
                }
            }
            if theme::icon_btn(ui, "plus", "Nächstes Universum").clicked() {
                self.patch.2 = unis.last().copied().unwrap_or(0) + 1;
            }
            if ui.button("Lückenlos adressieren").on_hover_text("Alle Fixtures dieses Universums ab 1 neu anordnen").clicked() {
                acts.push(Action::AutoAddress(self.patch.2));
            }
            if !conflicts.is_empty() {
                ui.label(RichText::new(format!("{} Konflikte", conflicts.len())).color(theme::DANGER));
            }
        });

        // Belegungskarte: 512 Kanäle, 32 pro Zeile.
        let uni = self.patch.2;
        let mut owner: Vec<Vec<usize>> = vec![Vec::new(); 512];
        for (i, f) in e.show.fixtures.iter().enumerate().filter(|(_, f)| f.universe == uni) {
            for a in e.show.range(f) {
                if let Some(o) = owner.get_mut(a as usize - 1) {
                    o.push(i);
                }
            }
        }
        let used = owner.iter().filter(|o| !o.is_empty()).count();
        ui.label(RichText::new(format!("{used} von 512 Kanälen belegt")).size(12.5).color(theme::MUTED));
        let cell = ((ui.available_width() - 4.0) / 32.0).clamp(16.0, 32.0);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(cell * 32.0, cell * 16.0 * 0.75), egui::Sense::click());
        let ch = cell * 0.75;
        let p = ui.painter_at(rect);
        let hover = resp.hover_pos().filter(|h| rect.contains(*h)).map(|h| {
            let (c, r) = (((h.x - rect.left()) / cell) as usize, ((h.y - rect.top()) / ch) as usize);
            r * 32 + c
        });
        for (n, o) in owner.iter().enumerate() {
            let r = egui::Rect::from_min_size(rect.min + egui::vec2((n % 32) as f32 * cell, (n / 32) as f32 * ch), egui::vec2(cell - 1.5, ch - 1.5));
            let Some(&i) = o.first() else {
                p.rect_filled(r, 2.0, theme::RAISED);
                continue;
            };
            let f = &e.show.fixtures[i];
            let bad = o.len() > 1;
            let hue = (f.id as f32 * 0.381966) % 1.0;
            let fill = if bad { theme::DANGER } else { Color32::from(egui::ecolor::Hsva::new(hue, 0.6, 0.55, 1.0)) };
            p.rect_filled(r, 2.0, fill);
            if e.selection.contains(&f.id) {
                p.rect_stroke(r, 2.0, egui::Stroke::new(1.5, Color32::WHITE), egui::StrokeKind::Inside);
            }
            if f.universe == uni && e.show.range(f).start as usize == n + 1 {
                p.text(r.left_center() + egui::vec2(3.0, 0.0), egui::Align2::LEFT_CENTER, f.id.to_string(), egui::FontId::proportional(11.0), Color32::WHITE);
            }
        }
        if let Some(n) = hover.filter(|n| *n < 512) {
            if let Some(&i) = owner[n].first() {
                let f = &e.show.fixtures[i];
                let r = e.show.range(f);
                resp.clone().on_hover_text(format!("{} · {}–{}", f.name, r.start, r.end - 1));
                if resp.clicked() {
                    acts.push(Action::Select(f.id));
                }
            } else {
                resp.clone().on_hover_text(format!("Kanal {} frei", n + 1));
                if resp.clicked() {
                    self.patch.3 = n as u16 + 1;
                }
            }
        }

        ui.add_space(6.0);
        caption(ui, "Gepatchte Fixtures");
        egui::ScrollArea::vertical().id_salt("patched").show(ui, |ui| {
            egui::Grid::new("patch_grid").striped(true).spacing([16.0, 6.0]).show(ui, |ui| {
                for h in ["ID", "Name", "Univ.", "Adresse", "Bereich", ""] {
                    ui.label(RichText::new(h).size(12.0).color(theme::MUTED));
                }
                ui.end_row();
                for f in &e.show.fixtures {
                    let bad = conflicts.contains(&f.id);
                    let sel = e.selection.contains(&f.id);
                    if ui.selectable_label(sel, f.id.to_string()).clicked() {
                        acts.push(Action::Select(f.id));
                        self.patch.2 = f.universe;
                    }
                    let mut name = f.name.clone();
                    let tn = e.show.types[f.ty].name.clone();
                    if ui.add_sized([150.0, 26.0], egui::TextEdit::singleline(&mut name)).on_hover_text(tn).changed() {
                        acts.push(Action::RenameFixture(f.id, name));
                    }
                    let (mut uni, mut adr) = (f.universe, f.address);
                    let a = ui.add(egui::DragValue::new(&mut uni).range(1..=32768)).changed();
                    let b = ui.add(egui::DragValue::new(&mut adr).range(1..=512)).changed();
                    if a || b {
                        acts.push(Action::SetPatch { id: f.id, universe: uni, address: adr });
                    }
                    let span = format!("{}–{}", f.address, e.show.range(f).end.saturating_sub(1));
                    ui.label(if bad { RichText::new(format!("{span}  Konflikt")).color(theme::DANGER) } else { RichText::new(span) });
                    if theme::icon_btn(ui, "x", "Fixture entfernen").clicked() {
                        acts.push(Action::RemoveFixture(f.id));
                    }
                    ui.end_row();
                }
            });
        });
    }

}
