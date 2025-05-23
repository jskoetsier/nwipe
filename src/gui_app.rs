/*
 *  gui_app.rs: Graphical user interface for nwipe using egui.
 *
 *  Copyright Sebastiaan Koetsier (2025)
 *
 *  This program is free software; you can redistribute it and/or modify it under
 *  the terms of the GNU General Public License as published by the Free Software
 *  Foundation, version 2.
 */

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use eframe::{egui, CreationContext};
use egui::{Color32, RichText, Ui};
use egui_extras::{Size, StripBuilder, TableBuilder};
use rfd::FileDialog;

use crate::context::{NwipeContext, PassType, SelectStatus};
use crate::device;
use crate::logging::{self, LogLevel, nwipe_log};
use crate::method;
use crate::options::NwipeOptions;
use crate::version;

/// The main GUI application.
pub struct NwipeApp {
    /// The list of available devices.
    devices: Arc<Mutex<Vec<NwipeContext>>>,
    /// The selected wiping method.
    method: String,
    /// The selected PRNG.
    prng: String,
    /// The number of rounds.
    rounds: i32,
    /// Whether to verify the wipe.
    verify: bool,
    /// Whether wiping is in progress.
    wiping_in_progress: bool,
    /// The wiping threads.
    wipe_threads: Vec<thread::JoinHandle<()>>,
    /// The last update time.
    last_update: Instant,
    /// Log messages.
    log_messages: Arc<Mutex<Vec<String>>>,
    /// Whether to show the about dialog.
    show_about: bool,
    /// Whether to show the help dialog.
    show_help: bool,
    /// Whether to show the confirmation dialog.
    show_confirmation: bool,
    /// Whether to show the settings dialog.
    show_settings: bool,
    /// Whether to power off after wiping.
    autopoweroff: bool,
}

impl Default for NwipeApp {
    fn default() -> Self {
        // Scan for devices
        let mut devices = Vec::new();
        if let Ok(count) = device::device_scan(&mut devices) {
            nwipe_log(LogLevel::Info, &format!("Found {} devices", count));
        } else {
            nwipe_log(LogLevel::Error, "Failed to scan for devices");
        }

        Self {
            devices: Arc::new(Mutex::new(devices)),
            method: "ops2".to_string(),
            prng: "isaac".to_string(),
            rounds: 1,
            verify: true,
            wiping_in_progress: false,
            wipe_threads: Vec::new(),
            last_update: Instant::now(),
            log_messages: Arc::new(Mutex::new(Vec::new())),
            show_about: false,
            show_help: false,
            show_confirmation: false,
            show_settings: false,
            autopoweroff: false,
        }
    }
}

impl NwipeApp {
    /// Create a new NwipeApp.
    pub fn new(cc: &CreationContext<'_>) -> Self {
        // Set up custom fonts if needed
        let mut fonts = egui::FontDefinitions::default();
        // Add custom fonts here if needed
        cc.egui_ctx.set_fonts(fonts);

        // Set up custom styles
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals.window_fill = Color32::from_rgb(30, 30, 30);
        style.visuals.panel_fill = Color32::from_rgb(40, 40, 40);
        style.visuals.faint_bg_color = Color32::from_rgb(50, 50, 50);
        style.visuals.extreme_bg_color = Color32::from_rgb(20, 20, 20);
        style.visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(45, 45, 45);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(60, 60, 60);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(70, 70, 70);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(80, 80, 80);
        cc.egui_ctx.set_style(style);

        Self::default()
    }

    /// Refresh the device list.
    fn refresh_devices(&mut self) {
        let mut devices = self.devices.lock().unwrap();
        devices.clear();
        if let Ok(count) = device::device_scan(&mut *devices) {
            nwipe_log(LogLevel::Info, &format!("Found {} devices", count));
        } else {
            nwipe_log(LogLevel::Error, "Failed to scan for devices");
        }
    }

    /// Start wiping the selected devices.
    fn start_wiping(&mut self) {
        if self.wiping_in_progress {
            return;
        }

        // Get the selected devices
        let devices = self.devices.lock().unwrap();
        let selected_devices: Vec<NwipeContext> = devices
            .iter()
            .filter(|d| d.select == SelectStatus::True)
            .cloned()
            .collect();

        if selected_devices.is_empty() {
            nwipe_log(LogLevel::Warning, "No devices selected for wiping");
            return;
        }

        // Clone the devices for the wiping threads
        let devices_arc = Arc::clone(&self.devices);
        let log_messages = Arc::clone(&self.log_messages);

        // Start wiping threads
        for device in selected_devices {
            let devices_arc = Arc::clone(&devices_arc);
            let log_messages = Arc::clone(&log_messages);
            let method = self.method.clone();
            let prng = self.prng.clone();
            let rounds = self.rounds;
            let verify = self.verify;

            let handle = thread::spawn(move || {
                // Set up the context for wiping
                let mut context = device.clone();
                context.prng = prng;
                context.round_count = rounds;
                context.verify = verify;

                // Log the start of wiping
                nwipe_log(
                    LogLevel::Notice,
                    &format!("Starting wipe of device {}", context.device_name),
                );

                // Perform the wipe
                let result = match method.as_str() {
                    "ops2" => method::ops2_wipe(&mut context),
                    "dod" => method::dod_wipe(&mut context),
                    "gutmann" => method::gutmann_wipe(&mut context),
                    "random" => method::random_wipe(&mut context),
                    "zero" => method::zero_wipe(&mut context),
                    _ => {
                        nwipe_log(
                            LogLevel::Error,
                            &format!("Unknown wipe method: {}", method),
                        );
                        -1
                    }
                };

                // Update the device status
                let mut devices = devices_arc.lock().unwrap();
                for d in devices.iter_mut() {
                    if d.device_name == context.device_name {
                        d.result = result;
                        d.wipe_status = 0; // Completed
                        d.round_percent = 100.0;
                        break;
                    }
                }

                // Log the completion of wiping
                if result == 0 {
                    nwipe_log(
                        LogLevel::Notice,
                        &format!("Wipe of device {} completed successfully", context.device_name),
                    );
                } else {
                    nwipe_log(
                        LogLevel::Error,
                        &format!(
                            "Wipe of device {} failed with error code {}",
                            context.device_name, result
                        ),
                    );
                }
            });

            self.wipe_threads.push(handle);
        }

        self.wiping_in_progress = true;
    }

    /// Stop wiping.
    fn stop_wiping(&mut self) {
        if !self.wiping_in_progress {
            return;
        }

        // Set the termination flag
        unsafe {
            crate::TERMINATE_SIGNAL = true;
            crate::USER_ABORT = true;
        }

        // Wait for the wiping threads to finish
        for handle in self.wipe_threads.drain(..) {
            let _ = handle.join();
        }

        // Reset the termination flag
        unsafe {
            crate::TERMINATE_SIGNAL = false;
            crate::USER_ABORT = false;
        }

        self.wiping_in_progress = false;
    }

    /// Check if all wiping threads have finished.
    fn check_wiping_finished(&mut self) {
        if !self.wiping_in_progress {
            return;
        }

        // Check if all threads have finished
        let mut all_finished = true;
        for handle in &self.wipe_threads {
            if !handle.is_finished() {
                all_finished = false;
                break;
            }
        }

        if all_finished {
            // Clean up the threads
            for handle in self.wipe_threads.drain(..) {
                let _ = handle.join();
            }

            self.wiping_in_progress = false;

            // Log the completion of all wiping
            nwipe_log(LogLevel::Notice, "All wiping operations completed");

            // Check if we should power off
            if self.autopoweroff {
                nwipe_log(LogLevel::Notice, "Powering off system as requested");
                let _ = std::process::Command::new("shutdown")
                    .args(["-P", "+1", "System going down in one minute"])
                    .output();
            }
        }
    }
}

impl eframe::App for NwipeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if wiping has finished
        self.check_wiping_finished();

        // Update the UI at a reasonable rate
        if self.last_update.elapsed() > Duration::from_millis(500) {
            self.last_update = Instant::now();
            ctx.request_repaint();
        }

        // Top panel with menu
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Refresh Devices").clicked() {
                        self.refresh_devices();
                        ui.close_menu();
                    }
                    if ui.button("Settings").clicked() {
                        self.show_settings = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Help").clicked() {
                        self.show_help = true;
                        ui.close_menu();
                    }
                    if ui.button("About").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });
            });
        });

        // Bottom panel with status
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Status: ");
                if self.wiping_in_progress {
                    ui.label(RichText::new("Wiping in progress").color(Color32::YELLOW));
                } else {
                    ui.label(RichText::new("Ready").color(Color32::GREEN));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(version::version_string());
                });
            });
        });

        // Left panel with device list
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Devices");
                ui.separator();

                if ui.button("Refresh").clicked() {
                    self.refresh_devices();
                }

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut devices = self.devices.lock().unwrap();
                    for device in devices.iter_mut() {
                        let mut selected = device.select == SelectStatus::True;
                        let device_name = &device.device_name;
                        let model = &device.identity.model_no;
                        let size_gb = device.device_size / (1024 * 1024 * 1024);

                        ui.horizontal(|ui| {
                            if ui.checkbox(&mut selected, "").clicked() {
                                device.select = if selected {
                                    SelectStatus::True
                                } else {
                                    SelectStatus::False
                                };
                            }

                            ui.vertical(|ui| {
                                ui.label(RichText::new(device_name).strong());
                                ui.label(format!("Model: {}", model));
                                ui.label(format!("Size: {} GB", size_gb));
                            });
                        });
                        ui.separator();
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Select All").clicked() {
                        let mut devices = self.devices.lock().unwrap();
                        for device in devices.iter_mut() {
                            device.select = SelectStatus::True;
                        }
                    }
                    if ui.button("Deselect All").clicked() {
                        let mut devices = self.devices.lock().unwrap();
                        for device in devices.iter_mut() {
                            device.select = SelectStatus::False;
                        }
                    }
                });
            });

        // Right panel with options and controls
        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Wipe Options");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Method:");
                    egui::ComboBox::from_id_source("method_combo")
                        .selected_text(&self.method)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.method, "ops2".to_string(), "OPS-II (DoD 5220.22-M)");
                            ui.selectable_value(&mut self.method, "dod".to_string(), "DoD 5220.22-M");
                            ui.selectable_value(&mut self.method, "gutmann".to_string(), "Gutmann (35 passes)");
                            ui.selectable_value(&mut self.method, "random".to_string(), "Random");
                            ui.selectable_value(&mut self.method, "zero".to_string(), "Zero");
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("PRNG:");
                    egui::ComboBox::from_id_source("prng_combo")
                        .selected_text(&self.prng)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.prng, "isaac".to_string(), "ISAAC");
                            ui.selectable_value(&mut self.prng, "mt19937".to_string(), "MT19937");
                            ui.selectable_value(&mut self.prng, "random".to_string(), "System Random");
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Rounds:");
                    ui.add(egui::DragValue::new(&mut self.rounds).speed(0.1).clamp_range(1..=100));
                });

                ui.checkbox(&mut self.verify, "Verify wipe");
                ui.checkbox(&mut self.autopoweroff, "Power off when complete");

                ui.separator();

                ui.horizontal(|ui| {
                    if !self.wiping_in_progress {
                        if ui.button("Start Wiping").clicked() {
                            // Check if any devices are selected
                            let devices = self.devices.lock().unwrap();
                            let selected_count = devices.iter().filter(|d| d.select == SelectStatus::True).count();
                            if selected_count > 0 {
                                self.show_confirmation = true;
                            } else {
                                nwipe_log(LogLevel::Warning, "No devices selected for wiping");
                            }
                        }
                    } else {
                        if ui.button("Stop Wiping").clicked() {
                            self.stop_wiping();
                        }
                    }
                });
            });

        // Central panel with progress and logs
        egui::CentralPanel::default().show(ctx, |ui| {
            // Use a strip to divide the central panel
            StripBuilder::new(ui)
                .size(Size::remainder()) // Progress area
                .size(Size::exact(200.0)) // Log area
                .vertical(|mut strip| {
                    // Progress area
                    strip.cell(|ui| {
                        ui.heading("Wipe Progress");
                        ui.separator();

                        // Create a table for the progress
                        TableBuilder::new(ui)
                            .column(egui_extras::Column::auto().at_least(100.0)) // Device name
                            .column(egui_extras::Column::remainder()) // Progress bar
                            .column(egui_extras::Column::exact(100.0)) // Status
                            .header(20.0, |mut header| {
                                header.col(|ui| {
                                    ui.heading("Device");
                                });
                                header.col(|ui| {
                                    ui.heading("Progress");
                                });
                                header.col(|ui| {
                                    ui.heading("Status");
                                });
                            })
                            .body(|mut body| {
                                let devices = self.devices.lock().unwrap();
                                for device in devices.iter() {
                                    if device.select == SelectStatus::True {
                                        body.row(30.0, |mut row| {
                                            row.col(|ui| {
                                                ui.label(&device.device_name);
                                            });
                                            row.col(|ui| {
                                                let progress = device.round_percent / 100.0;
                                                ui.add(egui::ProgressBar::new(progress as f32).show_percentage());
                                            });
                                            row.col(|ui| {
                                                let status = match device.wipe_status {
                                                    -1 => "Not started",
                                                    0 => {
                                                        if device.result == 0 {
                                                            "Completed"
                                                        } else {
                                                            "Failed"
                                                        }
                                                    },
                                                    _ => "In progress",
                                                };

                                                let status_color = match device.wipe_status {
                                                    0 => {
                                                        if device.result == 0 {
                                                            Color32::GREEN
                                                        } else {
                                                            Color32::RED
                                                        }
                                                    },
                                                    -1 => Color32::GRAY,
                                                    _ => Color32::YELLOW,
                                                };

                                                ui.label(RichText::new(status).color(status_color));
                                            });
                                        });
                                    }
                                }
                            });
                    });

                    // Log area
                    strip.cell(|ui| {
                        ui.heading("Log");
                        ui.separator();

                        egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                            // Get the log messages
                            let log_messages = self.log_messages.lock().unwrap();

                            // Display the log messages
                            for message in log_messages.iter() {
                                ui.label(message);
                            }

                            // Also display the global log messages
                            // This is a simplified approach; in a real implementation,
                            // you would need to capture log messages from the logging system
                            // and display them here.
                        });
                    });
                });
        });

        // Show confirmation dialog
        if self.show_confirmation {
            egui::Window::new("Confirmation")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("WARNING: This will permanently erase all data on the selected devices!");
                    ui.label("There is NO WAY to recover the data after wiping.");
                    ui.label("Are you sure you want to continue?");

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_confirmation = false;
                        }

                        if ui.button("Yes, Wipe the Devices").clicked() {
                            self.show_confirmation = false;
                            self.start_wiping();
                        }
                    });
                });
        }

        // Show about dialog
        if self.show_about {
            egui::Window::new("About")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("nwipe - Secure Disk Eraser");
                    ui.label(version::version_string());
                    ui.label(version::copyright_string());
                    ui.separator();
                    ui.label("A secure disk wiping utility implemented in Rust.");
                    ui.label("This program securely erases disks using various methods to ensure data cannot be recovered.");

                    ui.separator();

                    if ui.button("Close").clicked() {
                        self.show_about = false;
                    }
                });
        }

        // Show help dialog
        if self.show_help {
            egui::Window::new("Help")
                .collapsible(false)
                .resizable(true)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("nwipe Help");
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading("Wiping Methods");
                        ui.label("OPS-II (DoD 5220.22-M): Three rounds of wiping with zeros, ones, and random data.");
                        ui.label("DoD 5220.22-M: One round of wiping with zeros, ones, and random data.");
                        ui.label("Gutmann: 35 passes with various patterns.");
                        ui.label("Random: One pass of random data.");
                        ui.label("Zero: One pass of zeros.");

                        ui.separator();

                        ui.heading("PRNG Options");
                        ui.label("ISAAC: A cryptographically secure PRNG.");
                        ui.label("MT19937: Mersenne Twister PRNG.");
                        ui.label("System Random: The system's default PRNG.");

                        ui.separator();

                        ui.heading("Safety Warnings");
                        ui.label("IMPORTANT: nwipe will permanently destroy all data on the selected disks.");
                        ui.label("There is NO RECOVERY possible after wiping.");
                        ui.label("Always double-check device names before wiping.");
                        ui.label("Never wipe your system disk while the system is running from it.");
                    });

                    ui.separator();

                    if ui.button("Close").clicked() {
                        self.show_help = false;
                    }
                });
        }

        // Show settings dialog
        if self.show_settings {
            egui::Window::new("Settings")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("Settings");
                    ui.separator();

                    // Add settings here
                    ui.checkbox(&mut self.autopoweroff, "Power off system when wiping completes");

                    ui.separator();

                    if ui.button("Close").clicked() {
                        self.show_settings = false;
                    }
                });
        }
    }
}

/// Run the GUI application.
pub fn run_gui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([800.0, 600.0])
            .with_position(egui::pos2(300.0, 200.0)),
        ..Default::default()
    };

    eframe::run_native(
        "nwipe - Secure Disk Eraser",
        options,
        Box::new(|cc| Box::new(NwipeApp::new(cc))),
    )
}
