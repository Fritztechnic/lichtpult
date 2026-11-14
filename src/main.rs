use eframe::egui;
use lichtpult::{Universe, new_universe, spawn_output};

const BANK: usize = 24;

struct App {
    universe: Universe,
    page: usize,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("top").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Lichtpult");
                for p in 0..512 / BANK + 1 {
                    let first = p * BANK + 1;
                    ui.selectable_value(&mut self.page, p, format!("{first}"));
                }
                if ui.button("Blackout").clicked() {
                    *self.universe.lock().unwrap() = [0; 512];
                }
            });
        });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let mut dmx = self.universe.lock().unwrap();
            ui.horizontal_top(|ui| {
                for ch in (self.page * BANK..(self.page + 1) * BANK).take_while(|c| *c < 512) {
                    ui.vertical(|ui| {
                        ui.label(format!("{}", ch + 1));
                        let mut v = dmx[ch];
                        if ui.add(egui::Slider::new(&mut v, 255..=0).vertical()).changed() {
                            dmx[ch] = v;
                        }
                    });
                }
            });
        });
    }
}

fn main() -> eframe::Result {
    let universe = new_universe();
    spawn_output(universe.clone(), "255.255.255.255:6454");
    eframe::run_native(
        "Lichtpult",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(App { universe, page: 0 }))),
    )
}
