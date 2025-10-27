use eframe::egui;
use std::process::{Child, Command};

/// Enum to represent the different types of processes we can run.
#[derive(Debug, PartialEq, Clone, Copy)]
enum ProcessType {
    Teleoperation,
    Record,
    Replay,
}

/// Holds the application state.
struct MyApp {
    /// The currently running child process, if any.
    /// We use an Option to represent that a process might not be running.
    /// The tuple stores the process handle and its type.
    child_process: Option<(Child, ProcessType)>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            child_process: None,
        }
    }
}

impl MyApp {
    /// Spawns a process in a new terminal window.
    fn spawn_process(&mut self, process_type: ProcessType) {
        // If a process is already running, do nothing.
        if self.child_process.is_some() {
            return;
        }

        // Define the command to be executed for each process type.
        // These are example commands; you can replace them with your actual commands.
        let command_str = match process_type {
            ProcessType::Teleoperation => "echo 'Starting Teleoperation...'; sleep 10; echo 'Teleop finished.'",
            ProcessType::Record => "echo 'Starting Record...'; sleep 10; echo 'Record finished.'",
            ProcessType::Replay => "echo 'Starting Replay...'; sleep 10; echo 'Replay finished.'",
        };

        // This command is for Linux systems with xterm.
        // You might need to change 'xterm' to your terminal emulator of choice (e.g., 'gnome-terminal').
        // For other OSes:
        // - macOS: "osascript", "-e", &format!("tell app \"Terminal\" to do script \"{}\"", command_str)
        // - Windows: "cmd", "/C", &format!("start {}", command_str)
        let child = Command::new("konsole")
            .arg("-e")
            .arg(format!("bash -c '{}'", command_str))
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
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}
