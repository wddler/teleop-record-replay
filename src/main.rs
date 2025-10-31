use clap::Parser;
use eframe::egui;
use serde::Deserialize;
use std::fs;
use log::{debug, error, info};
use std::process::{Child, Command};
use std::sync::Arc;
use std::path::PathBuf;

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
    #[serde(default)]
    working_directory: String,
    teleoperation: String,
    record: String,
    replay: String,
}

/// Struct for application-level settings from config.toml.
#[derive(Deserialize, Clone, Default)]
struct AppConfig {
    /// The terminal emulator to use.
    /// We use an Option so we can default if it's missing from the TOML file.
    #[serde(default)]
    terminal: Option<String>,
}

/// Struct to represent the overall configuration.
#[derive(Deserialize, Clone)]
struct Config {
    /// We use `serde(default)` so the app doesn't crash if the `[app]` table is missing.
    #[serde(default)]
    app: AppConfig,
    commands: Commands,
}

/// Holds the application state.
struct MyApp {
    /// The loaded configuration, wrapped in an Arc for efficient sharing.
    config: Result<Arc<Config>, String>,
    /// The currently running child process, if any. The tuple stores the process handle and its type.
    child_process: Option<(Child, ProcessType)>,
}

impl MyApp {
    /// Creates a new instance of the application, loading the configuration.
    fn new(config_path: PathBuf) -> Self {
        info!("Loading configuration from: {}", config_path.display());
        let config = Self::load_config(config_path).map(Arc::new);
        Self {
            config,
            child_process: None,
        }
    }

    /// Loads configuration from the specified path. Returns a `Result` indicating
    /// success or failure, with an error message if loading fails.
    /// The `config_path` is consumed to ensure it's used.
    fn load_config(config_path: PathBuf) -> Result<Config, String> {
        let config_str = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file '{}': {}", config_path.display(), e))?;
        toml::from_str(&config_str).map_err(|e| format!("Failed to parse config.toml: {}", e))
    }
}

impl Default for MyApp {
    fn default() -> Self {
        // This default is only used by eframe::run_native if we don't provide a custom constructor.
        // We will provide a custom constructor in main, so this won't be called in practice.
        // However, it's good practice to have a sensible default or panic if it's truly uncallable.
        // For now, we'll panic as it indicates a misuse of the default.
        panic!("MyApp::default() should not be called directly. Use MyApp::new(config_path) instead.");
    }
}

impl MyApp {
    /// Spawns a process in a new terminal window.
    fn spawn_process(&mut self, process_type: ProcessType) {
        // If a process is already running, do nothing.
        debug!("Attempting to spawn process of type: {:?}", process_type);
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
        let full_command = if !config.commands.working_directory.is_empty() {
            format!("cd {} && {}", config.commands.working_directory, specific_command)
        } else {
            specific_command.to_string()
        };

        debug!("Full command to execute: '{}'", full_command);
        // This command is for Linux systems with xterm.
        // You might need to change 'xterm' to your terminal emulator of choice (e.g., 'gnome-terminal').
        // For other OSes:
        // - macOS: "osascript", "-e", &format!("tell app \"Terminal\" to do script \"{}\"", command_str)
        // - Windows: "cmd", "/C", &format!("start {}", command_str)
        // Use the terminal from config, or default to "konsole".
        let terminal = config
            .app
            .terminal
            .as_deref()
            .unwrap_or("konsole");
        debug!("Using terminal: '{}'", terminal);

        // To ensure the terminal stays open for inspection, we wrap the entire command
        // in a subshell `(...)`. After the command runs (whether it succeeds or fails),
        // we print a message and use `read` to wait for the user to press Enter.
        // This is more reliable than `sleep` across different terminal emulators.
        let final_shell_command = format!(
            "({}); echo -e \"\\n\\n[INFO] Command finished. Press Enter to close this terminal.\"; read",
            full_command
        );
        debug!("Final shell command: '{}'", final_shell_command);
        let child = Command::new(terminal)
            .arg("-e")
            .arg(format!("bash -ic '{}'", final_shell_command))
            .spawn();

        match child {
            Ok(child_handle) => { // Process spawned successfully
                info!("Successfully spawned {:?} process with PID: {}", process_type, child_handle.id());
                self.child_process = Some((child_handle, process_type));
            }
            Err(e) => {
                error!("Failed to spawn {:?} process: {}", process_type, e);
                // Consider showing this error in the GUI for the user.
            }
        }
    }

    /// Kills the running process.
    fn kill_process(&mut self) {
        if let Some((mut child, _)) = self.child_process.take() {
            info!("Attempting to kill process with PID: {}", child.id());
            if let Err(e) = child.kill() {
                error!("Failed to kill process with PID {}: {}", child.id(), e);
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
                    Ok(None) => { // Process is still running.
                        // Process is still running.
                        ui.label(format!("{:?} is running...", process_type));
                        if ui.button("Stop").clicked() {
                            self.kill_process();
                        }
                    }
                    // An error occurred while trying to check the process status.
                    // This could indicate the process is no longer valid or other system issues.
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

/// Command-line arguments for the application.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration TOML file.
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

fn main() -> Result<(), eframe::Error> {
    // Initialize the logger. This allows debug messages to be printed to the console.
    env_logger::init();

    // Parse command-line arguments.
    let args = Args::parse();

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Teleop Record Replay",
        options,
        Box::new(move |_cc| {
            Box::new(MyApp::new(args.config))
        }),
    )
}
