use eframe::egui;
use serde::Deserialize;
use std::fs;
use std::process::{Child, Command};
use std::sync::Arc;

/// Enum to represent the different types of processes we can run.
#[derive(Debug, PartialEq, Clone, Copy)]
enum ProcessType {
    Teleoperation,
    Record,
    Replay,
}

/// Struct to hold the command strings from config.toml.
#[derive(Deserialize, Clone)]
struct Commands {
    prefix: String,
    teleoperation: String,
    record: String,
    replay: String,
}

/// Struct to represent the overall configuration.
#[derive(Deserialize, Clone)]
struct Config {
    commands: Commands,
}

/// Holds the application state.
struct MyApp {
    /// The loaded configuration, wrapped in an Arc for efficient sharing.
    config: Result<Arc<Config>, String>,
    /// The currently running child process, if any.
    /// We use an Option to represent that a process might not be running.
    /// The tuple stores the process handle and its type.
    child_process: Option<(Child, ProcessType)>,
}

impl MyApp {
    /// Creates a new instance of the application, loading the configuration.
    fn new() -> Self {
        let config = Self::load_config().map(Arc::new);
        Self {
            config,
            child_process: None,
        }
    }

    /// Loads configuration from `config.toml`.
    fn load_config() -> Result<Config, String> {
        let config_str = fs::read_to_string("config.toml")
            .map_err(|e| format!("Failed to read config.toml: {}", e))?;
        toml::from_str(&config_str).map_err(|e| format!("Failed to parse config.toml: {}", e))
    }
}

impl Default for MyApp {
    fn default() -> Self {
        Self::new()
    }
}

impl MyApp {
    /// Spawns a process in a new terminal window.
    fn spawn_process(&mut self, process_type: ProcessType) {
        // If a process is already running, do nothing.
        if self.child_process.is_some() || self.config.is_err() {
            return;
        }
        let config = self.config.as_ref().unwrap().clone();

        // Get the specific command for the process type from the loaded config.
        let specific_command = match process_type {
            ProcessType::Teleoperation => &config.commands.teleoperation,
            ProcessType::Record => &config.commands.record,
            ProcessType::Replay => &config.commands.replay,
        };

        // Combine the prefix and the specific command.
        let full_command = format!("{} && {}", config.commands.prefix, specific_command);

        // This command is for Linux systems with xterm.
        // You might need to change 'xterm' to your terminal emulator of choice (e.g., 'gnome-terminal').
        // For other OSes:
        // - macOS: "osascript", "-e", &format!("tell app \"Terminal\" to do script \"{}\"", command_str)
        // - Windows: "cmd", "/C", &format!("start {}", command_str)
        let child = Command::new("xterm")
            .arg("-e")
            .arg(format!("bash -c '{}'", full_command))
            .spawn();

        match child {
            Ok(child_handle) => {
                self.child_process = Some((child_handle, process_type));
            }
            Err(e) => {
                // It's good practice to log errors.
                // For a real app, you might want to show this in the UI.
                eprintln!("Failed to spawn process: {}", e);
            }
        }
    }

    /// Kills the running process.
    fn kill_process(&mut self) {
        if let Some((mut child, _)) = self.child_process.take() {
            if let Err(e) = child.kill() {
                eprintln!("Failed to kill process: {}", e);
            }
            // We can also wait for the process to ensure it's cleaned up,
            // but for killing it, this is often sufficient.
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Teleop Record Replay");
            ui.separator();

            // Display an error message if the configuration failed to load.
            if let Err(e) = &self.config {
                ui.colored_label(egui::Color32::RED, e);
                return;
            }

            if let Some((child, process_type)) = &mut self.child_process {
                // Check if the process has finished.
                match child.try_wait() {
                    Ok(Some(_status)) => self.child_process = None, // Process finished.
                    Ok(None) => {
                        // Process is still running.
                        ui.label(format!("{:?} is running...", process_type));
                        if ui.button("Stop").clicked() {
                            self.kill_process();
                        }
                    }
                    Err(e) => {
                        eprintln!("Error waiting for child process: {}", e);
                        self.child_process = None;
                    }
                }
            } else {
                // No process is running, show the main buttons.
                if ui.button("Teleoperation").clicked() {
                    self.spawn_process(ProcessType::Teleoperation);
                }
                if ui.button("Record").clicked() {
                    self.spawn_process(ProcessType::Record);
                }
                if ui.button("Replay").clicked() {
                    self.spawn_process(ProcessType::Replay);
                }
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Teleop Record Replay",
        options,
        Box::new(|_cc| Box::new(MyApp::new())),
    )
}
