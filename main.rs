#![windows_subsystem = "windows"]

use eframe::egui;
use std::fs;
use std::sync::{Arc};
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::thread;
use std::time::{Instant, Duration};
use midir::MidiOutput;

fn main() -> eframe::Result<()> {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([300.0, 155.0]);
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let midiot = MidiOutput::new("Rust Midi Output").expect("Failed to create MIDI output");
    let outports = midiot.ports();

    if outports.is_empty() {
        println!("No MIDI output ports found");
        return Ok(());
    }

    // Gather available port names for GUI dropdown
    let guiout: Vec<String> = outports.iter()
        .map(|p| midiot.port_name(p).unwrap_or("Unknown".to_string()))
        .collect();

    let bpm = Arc::new(AtomicI32::new(0));
    let guibpm = Arc::clone(&bpm);

    let sharedmidiport = Arc::new(AtomicUsize::new(1));
    let guimidiport = Arc::clone(&sharedmidiport);

    // Spawn MIDI clock thread
    let threadbpm = Arc::clone(&bpm);
    let threadmidiport = Arc::clone(&sharedmidiport);
    thread::spawn(move || {
        let mut conn_out: Option<midir::MidiOutputConnection> = None;
        let mut oldval = usize::MAX;
        let mut next_tick = Instant::now();

        loop {
            let val = threadbpm.load(Ordering::SeqCst);
            let indval = threadmidiport.load(Ordering::SeqCst);

            // reconnect only if port changed or connection lost
            if indval != oldval || conn_out.is_none() {
                conn_out = None; // close any existing connection
                if indval < outports.len() {
                    let port = &outports[indval]; // skip port 0
                    match MidiOutput::new("Rust Midi Output Thread").unwrap().connect(port, "midir-selected") {
                        Ok(c) => {
                            conn_out = Some(c);
                            oldval = indval;
                        }
                        Err(e) => {
                            eprintln!("Failed to connect to port {}: {}", indval + 1, e);
                            conn_out = None;
                            thread::sleep(Duration::from_secs(1));
                            continue;
                        }
                    }
                }
            }

            if val == 0 || conn_out.is_none() {
                thread::sleep(Duration::from_millis(500));
                continue;
            }

            let interval_ms = 60000.0 / (val as f64 * 24.0);
            let interval = Duration::from_secs_f64(interval_ms / 1000.0);

            let now = Instant::now();
            if now < next_tick {
                thread::sleep(next_tick - now);
            }

            if let Some(conn) = conn_out.as_mut() {
                let _ = conn.send(&[0xF8]);
            }

            next_tick += interval;
        }
    });

    // Launch GUI
    eframe::run_native(
        "MidiClock",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc, guibpm, guimidiport, guiout)))),
    )
}

struct MyApp {
    bpm: Arc<AtomicI32>,
    last_press: Option<Instant>,
    dropdown_index: Arc<AtomicUsize>,
    parrot_names: Vec<String>,
    current_index: usize,
    impact_font: eframe::egui::FontId,
}

impl MyApp {
    fn new(
        cc: &eframe::CreationContext<'_>,
        bpm: Arc<AtomicI32>,
        dropdown_index: Arc<AtomicUsize>,
        parrot_names: Vec<String>,
    ) -> Self {
        let font_data = fs::read(r"C:\Windows\Fonts\Impact.ttf").expect("Failed to read Impact.ttf");
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "ImpactFont".to_owned(),
            egui::FontData::from_owned(font_data).into(),
        );
        fonts
            .families
            .insert(egui::FontFamily::Name("BPM".into()), vec!["ImpactFont".to_owned()]);
        cc.egui_ctx.set_fonts(fonts);
        let impact_font = egui::FontId::new(90.0, egui::FontFamily::Name("BPM".into()));

        Self {
            bpm,
            last_press: None,
            dropdown_index,
            parrot_names,
            current_index: 0,
            impact_font,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            let now = Instant::now();
            if let Some(last) = self.last_press {
                let elapsed = now.duration_since(last);
                let interval_secs = elapsed.as_secs_f32();
                if interval_secs > 0.0 {
                    let bpm = (60.0 / interval_secs).round() as u32;
                    if bpm >= 40 && bpm <= 300 {
                        self.bpm.store(bpm.try_into().unwrap(), Ordering::SeqCst);
                    }
                }
            }
            self.last_press = Some(now);
        }
        let mut bpm = self.bpm.load(Ordering::SeqCst);
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            
            if bpm < 300 {
                bpm += 1;
                self.bpm.store(bpm, Ordering::SeqCst);
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            
            if bpm <= 290 {
                bpm += 10;
                self.bpm.store(bpm, Ordering::SeqCst);
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
           
            if bpm > 40 {
                bpm -= 1;
                self.bpm.store(bpm, Ordering::SeqCst);
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
           
            if bpm >= 50 {
                bpm -= 10;
                self.bpm.store(bpm, Ordering::SeqCst);
            }
        }


        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let value = self.bpm.load(Ordering::SeqCst);
                if value != 0 {
                    ui.label(
                        eframe::egui::RichText::new(format!("{}", value))
                            .font(self.impact_font.clone()),
                    );
                } else {
                    ui.label(
                        eframe::egui::RichText::new("--").font(self.impact_font.clone()),
                    );
                }

                ui.separator();
                ui.horizontal_centered(|ui| {
                egui::ComboBox::from_label("")
                    .selected_text(self.parrot_names[self.current_index].clone())
                    .show_ui(ui, |ui| {
                        for (index, name) in self.parrot_names.iter().enumerate().skip(1) {
                            if ui.selectable_label(self.current_index == index, name.clone()).clicked() {
                                self.current_index = index;
                                self.dropdown_index.store(index, Ordering::SeqCst);
                            }
                        }
                    });
                });
            });
        });
    }
}
