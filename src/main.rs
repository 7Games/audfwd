use egui::{Align2, Vec2, Widget};
use eframe::egui;
use tokio::sync::oneshot;
use rodio::microphone::available_inputs;
use rodio::speakers::available_outputs;

mod audio;
mod ui;

#[tokio::main]
async fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 220.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "audfwd",
        options,
        Box::new(|cc| {
            // ui:: ::configure_text_styles(&cc.egui_ctx);


            Ok(Box::<MyApp>::default())
        }),
    )
}

struct MyApp {
    is_listening: bool,

    audio_inputs: Vec<rodio::microphone::Input>,
    audio_outputs: Vec<rodio::speakers::Output>,

    selected_input: String,
    selected_output: String,

    button_text: String,

    mono: bool,

    show_exit_confirmation_dialog: bool,
    show_settings: bool,

    tx: Option<oneshot::Sender<()>>,
    rx: Option<oneshot::Receiver<()>>,
}

impl Default for MyApp {
    fn default() -> Self {
        let (t_tx, t_rx) = oneshot::channel::<()>();

        let inputs = available_inputs().unwrap();
        let outputs = available_outputs().unwrap();

        Self {
            is_listening: false,

            audio_inputs: inputs.clone(),
            audio_outputs: outputs.clone(),

            selected_input: String::from(format!("0: {}", inputs[0])),
            selected_output: String::from(format!("0: {}", outputs[0])),

            button_text: String::from("Start"),

            mono: false,

            show_exit_confirmation_dialog: false,
            show_settings: false,

            tx: Some(t_tx),
            rx: Some(t_rx),
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if ui.input(|i| i.viewport().close_requested()) {
            if !self.is_listening {
                // do nothing - we will close
            } else {
                ui.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.show_exit_confirmation_dialog = true;
            }
        }

        if self.show_exit_confirmation_dialog {
            egui::Window::new("Are you sure?")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, 0.0))
                .show(ui.ctx(), |ui| {
                    ui.label("The forwarding audio will stop");
                    ui.horizontal(|ui| {
                        let close = egui::Button::new("Close").fill(egui::Color32::RED).ui(ui);

                        if close.clicked() {
                            self.show_exit_confirmation_dialog = false;
                            self.is_listening = false;

                            if let Some(tx) = self.tx.take() {
                                audio::stop_mic_to_speaker(tx);
                            }

                            ui.send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        if ui.button("Don't Close").clicked() {
                            self.show_exit_confirmation_dialog = false;
                        }
                    });
                });
        }

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("📢 audfwd");
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                let _input_combo = egui::ComboBox::from_label("Input Device")
                    .selected_text(format!("{:?}", self.selected_input))
                    .show_ui(ui, |ui| {
                        for (i, input) in self.audio_inputs.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_input,
                                                format!("{}: {}", i, input),
                                                format!("{}: {}", i, input));
                        }
                    });

            });
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                let _output_combo = egui::ComboBox::from_label("Output Device")
                    .selected_text(format!("{:?}", self.selected_output))
                    .show_ui(ui, |ui| {
                        for (i, output) in self.audio_outputs.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_output,
                                                format!("{}: {}", i, output),
                                                format!("{}: {}", i, output));
                        }
                    });
            });
            ui.add_space(10.0);
            ui.add(egui::Checkbox::new(&mut self.mono, "Mono Input"));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button(&self.button_text).clicked() {
                    if self.is_listening {
                        self.button_text = String::from("Start");
                        self.is_listening = false;

                        if let Some(tx) = self.tx.take() {
                            audio::stop_mic_to_speaker(tx);
                        }
                    } else {
                        self.button_text = String::from("Stop");
                        self.is_listening = true;

                        let (t_tx, t_rx) = oneshot::channel::<()>();
                        self.tx = Some(t_tx);
                        self.rx = Some(t_rx);

                        let mic_num = self.selected_input.split(":").collect::<Vec<_>>()[0].parse::<usize>().unwrap();
                        let out_num = self.selected_output.split(":").collect::<Vec<_>>()[0].parse::<usize>().unwrap();

                        let stop_rx = self.rx.take().unwrap(); // move into task
                        let mic = self.audio_inputs[mic_num].clone();
                        let speaker = self.audio_outputs[out_num].clone();
                        let mono = (!self.mono) as u16 + 1;

                        tokio::spawn(async move {
                            audio::run_mic_to_speaker(stop_rx, mic, speaker, mono).await;
                        });
                    }
                }
                if ui.button("⚙️").clicked() {
                    self.show_settings = !self.show_settings;
                }
            });
            ui.add_space(8.0);
            if self.is_listening {
                ui.label(egui::RichText::new(format!("Forwarding!")).color(egui::Color32::GREEN));
            } else {
                ui.label(egui::RichText::new(format!("Not Forwarding...")).color(egui::Color32::RED));
            }
        });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("⚙️ Settings");
            ui.add_space(20.0);
        });
    }
}
