use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
    thread,
};

use sempal::{
    app_core::actions::{
        NativeAppBridge, NativeAppModel as AppModel,
        NativeBrowserActionsModel as BrowserActionsModel,
        NativeBrowserChromeModel as BrowserChromeModel,
        NativeBrowserPanelModel as BrowserPanelModel, NativeBrowserRowModel as BrowserRowModel,
        NativeSourceRowModel as SourceRowModel, NativeStatusBarModel as StatusBarModel,
        NativeUiAction as UiAction, NativeUpdatePanelModel as UpdatePanelModel,
        NativeUpdateStatusModel as UpdateStatusModel,
    },
    gui_runtime::{NativeRunOptions, WindowIconRgba, run_native_vello_app_declarative},
};

use crate::{APP_NAME, install, paths};

/// Events emitted by installer worker threads and consumed by the UI bridge.
pub(crate) enum InstallerEvent {
    Started { total_files: usize },
    FileCopied { copied_files: usize, name: String },
    Log(String),
    Finished,
    Failed(String),
}

#[derive(Clone, Copy, PartialEq)]
enum InstallStep {
    Welcome,
    License,
    Location,
    Installing,
    Done,
    Error,
}

#[derive(Default)]
struct InstallProgress {
    total_files: usize,
    copied_files: usize,
    current: Option<String>,
}

struct InstallerNativeBridge {
    step: InstallStep,
    install_dir: PathBuf,
    bundle_dir: PathBuf,
    license_text: String,
    progress: InstallProgress,
    receiver: Option<mpsc::Receiver<InstallerEvent>>,
    error: Option<String>,
    open_folder_on_finish: bool,
    launch_on_finish: bool,
    finish_errors: Vec<String>,
    logs: Vec<String>,
    install_finished: bool,
}

impl InstallerNativeBridge {
    fn new() -> Self {
        Self {
            step: InstallStep::Welcome,
            install_dir: paths::default_install_dir(),
            bundle_dir: paths::default_bundle_dir(),
            license_text: include_str!("../../../LICENSE").to_string(),
            progress: InstallProgress::default(),
            receiver: None,
            error: None,
            open_folder_on_finish: true,
            launch_on_finish: true,
            finish_errors: Vec::new(),
            logs: Vec::new(),
            install_finished: false,
        }
    }

    fn start_install(&mut self) {
        let bundle_dir = self.bundle_dir.clone();
        let install_dir = self.install_dir.clone();
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        self.progress = InstallProgress::default();
        self.step = InstallStep::Installing;
        self.install_finished = false;
        self.finish_errors.clear();
        self.logs.clear();
        thread::spawn(move || {
            if let Err(err) = install::run_install(&bundle_dir, &install_dir, tx.clone()) {
                let _ = tx.send(InstallerEvent::Failed(err));
            }
        });
    }

    fn poll_installer(&mut self) {
        let Some(receiver) = &self.receiver else {
            return;
        };
        while let Ok(event) = receiver.try_recv() {
            match event {
                InstallerEvent::Started { total_files } => {
                    self.progress.total_files = total_files;
                }
                InstallerEvent::FileCopied { copied_files, name } => {
                    self.progress.copied_files = copied_files;
                    self.progress.current = Some(name);
                }
                InstallerEvent::Log(message) => {
                    self.logs.push(message);
                }
                InstallerEvent::Finished => {
                    self.install_finished = true;
                    self.step = InstallStep::Done;
                }
                InstallerEvent::Failed(err) => {
                    self.error = Some(err);
                    self.step = InstallStep::Error;
                }
            }
        }
    }

    fn browse_install_dir(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.install_dir = folder;
        }
    }

    fn advance_step(&mut self) {
        match self.step {
            InstallStep::Welcome => self.step = InstallStep::License,
            InstallStep::License => self.step = InstallStep::Location,
            InstallStep::Location => self.start_install(),
            InstallStep::Done => self.run_finish_actions(),
            InstallStep::Error => self.start_install(),
            InstallStep::Installing => {}
        }
    }

    fn back_step(&mut self) {
        match self.step {
            InstallStep::Welcome => request_process_exit(),
            InstallStep::License => self.step = InstallStep::Welcome,
            InstallStep::Location => self.step = InstallStep::License,
            InstallStep::Done => request_process_exit(),
            InstallStep::Error => request_process_exit(),
            InstallStep::Installing => {}
        }
    }

    fn run_finish_actions(&mut self) {
        self.finish_errors.clear();
        if self.open_folder_on_finish
            && let Err(err) = open::that(&self.install_dir)
        {
            self.finish_errors
                .push(format!("Failed to open install folder: {err}"));
        }
        if self.launch_on_finish {
            let exe = self.install_dir.join("sempal.exe");
            if let Err(err) = std::process::Command::new(exe).spawn() {
                self.finish_errors
                    .push(format!("Failed to launch SemPal: {err}"));
            }
        }
        if self.finish_errors.is_empty() {
            request_process_exit();
        }
    }

    fn update_panel(&self) -> UpdatePanelModel {
        let mut model = UpdatePanelModel::default();
        match self.step {
            InstallStep::Welcome => {
                model.status = UpdateStatusModel::Available;
                model.status_label = String::from("Installer ready");
                model.action_hint_label = String::from("Install=Next | Dismiss=Exit");
                model.available_url = Some(String::from("internal://welcome"));
            }
            InstallStep::License => {
                model.status = UpdateStatusModel::Available;
                model.status_label = String::from("Review license");
                model.action_hint_label = String::from("Open=Back | Install=Next");
                model.available_url = Some(String::from("internal://license"));
                model.release_notes_label = String::from("Press Install to continue");
            }
            InstallStep::Location => {
                model.status = UpdateStatusModel::Available;
                model.status_label = String::from("Choose install location");
                model.action_hint_label = String::from("Open=Browse | Install=Start");
                model.available_url = Some(String::from("internal://location"));
                model.release_notes_label = self.install_dir.display().to_string();
            }
            InstallStep::Installing => {
                model.status = UpdateStatusModel::Checking;
                model.status_label = String::from("Installing");
                model.action_hint_label = String::from("Please wait");
                model.release_notes_label = self
                    .progress
                    .current
                    .as_ref()
                    .map(|name| format!("Copying {name}"))
                    .unwrap_or_else(|| String::from("Preparing"));
            }
            InstallStep::Done => {
                model.status = UpdateStatusModel::Available;
                model.status_label = String::from("Installation complete");
                model.action_hint_label =
                    String::from("Open=Folder | Install=Launch | Dismiss=Exit");
                model.available_url = Some(String::from("internal://done"));
                model.release_notes_label = format!(
                    "Open folder: {} | Launch app: {}",
                    bool_word(self.open_folder_on_finish),
                    bool_word(self.launch_on_finish)
                );
            }
            InstallStep::Error => {
                model.status = UpdateStatusModel::Error;
                model.status_label = String::from("Installation failed");
                model.action_hint_label = String::from("Retry check to reinstall");
                model.last_error = self.error.clone();
            }
        }
        model
    }

    fn browser_rows(&self) -> Vec<BrowserRowModel> {
        match self.step {
            InstallStep::Welcome => vec![
                BrowserRowModel::new(0, "Welcome to the SemPal installer", 1, false, false),
                BrowserRowModel::new(
                    1,
                    "Install includes app binaries and required ML assets",
                    1,
                    false,
                    false,
                ),
            ],
            InstallStep::License => self
                .license_text
                .lines()
                .take(120)
                .enumerate()
                .map(|(index, line)| BrowserRowModel::new(index, line.to_string(), 1, false, false))
                .collect(),
            InstallStep::Location => vec![
                BrowserRowModel::new(
                    0,
                    format!("Install path: {}", self.install_dir.display()),
                    1,
                    true,
                    true,
                )
                .with_bucket_label("browse"),
                BrowserRowModel::new(
                    1,
                    format!("Bundle source: {}", self.bundle_dir.display()),
                    1,
                    false,
                    false,
                ),
            ],
            InstallStep::Installing | InstallStep::Done | InstallStep::Error => self
                .logs
                .iter()
                .enumerate()
                .map(|(index, line)| BrowserRowModel::new(index, line, 1, false, false))
                .collect(),
        }
    }

    fn app_model(&self) -> AppModel {
        let rows = self.browser_rows();
        let mut model = AppModel {
            title: format!("{APP_NAME} installer"),
            backend_label: format!("install dir: {}", self.install_dir.display()),
            sources_label: String::from("Installer"),
            status_text: String::new(),
            status: StatusBarModel {
                left: step_label(self.step).to_string(),
                center: match self.step {
                    InstallStep::Installing => format!(
                        "{}/{} files",
                        self.progress.copied_files, self.progress.total_files
                    ),
                    _ => format!("logs: {}", self.logs.len()),
                },
                right: if self.install_finished {
                    String::from("finished")
                } else {
                    String::from("active")
                },
            },
            transport_running: true,
            ..AppModel::default()
        };
        model.browser_actions = BrowserActionsModel::default();
        model.sources.rows = vec![
            SourceRowModel::new(
                "Install dir",
                self.install_dir.display().to_string(),
                false,
                false,
            ),
            SourceRowModel::new(
                "Bundle dir",
                self.bundle_dir.display().to_string(),
                false,
                false,
            ),
        ];
        model.browser = BrowserPanelModel {
            visible_count: rows.len(),
            selected_visible_row: if matches!(self.step, InstallStep::Location) {
                Some(0)
            } else {
                None
            },
            autoscroll: true,
            view_start_row: 0,
            selected_path_count: 0,
            search_query: step_label(self.step).to_string(),
            active_rating_filters: [false; 7],
            search_placeholder: Some(String::from("Use top action buttons")),
            busy: matches!(self.step, InstallStep::Installing),
            sort_label: Some(String::from("installer")),
            active_tab_label: Some(String::from("Flow")),
            focused_sample_label: None,
            anchor_visible_row: None,
            rows,
        };
        model.browser_chrome = BrowserChromeModel {
            samples_tab_label: String::from("Flow"),
            map_tab_label: String::from("Log"),
            search_prefix_label: String::from("Step"),
            search_placeholder: String::from("Installer flow"),
            activity_ready_label: String::from("Ready"),
            activity_busy_label: String::from("Installing"),
            sort_prefix_label: String::from("Mode"),
            sort_order_label: String::from("Installer"),
            similarity_toggle_label: String::from("n/a"),
            item_count_label: format!("{} rows", model.browser.visible_count),
        };
        model.update = self.update_panel();
        model
    }
}

impl NativeAppBridge for InstallerNativeBridge {
    /// Project the current installer workflow state into a shared model snapshot.
    fn project_model(&mut self) -> Arc<AppModel> {
        self.poll_installer();
        Arc::new(self.app_model())
    }

    /// Project the current installer workflow state into a UI model snapshot.
    fn pull_model(&mut self) -> AppModel {
        Arc::unwrap_or_clone(self.project_model())
    }

    /// Reduce one emitted UI action into installer workflow state.
    fn reduce_action(&mut self, action: UiAction) {
        match action {
            UiAction::InstallUpdate => self.advance_step(),
            UiAction::OpenUpdateLink => {
                if matches!(self.step, InstallStep::Location) {
                    self.browse_install_dir();
                } else if matches!(self.step, InstallStep::Done)
                    && let Err(err) = open::that(&self.install_dir)
                {
                    self.finish_errors
                        .push(format!("Failed to open install folder: {err}"));
                } else {
                    self.back_step();
                }
            }
            UiAction::DismissUpdate => self.back_step(),
            UiAction::CheckForUpdates => {
                if matches!(self.step, InstallStep::Error) {
                    self.start_install();
                } else {
                    self.advance_step();
                }
            }
            UiAction::SelectSourceRow { index } => {
                if index == 0 && matches!(self.step, InstallStep::Location) {
                    self.browse_install_dir();
                }
            }
            _ => {}
        }
    }
}

/// Run the installer UI using the native radiant runtime.
pub(crate) fn run_installer_app() -> Result<(), String> {
    let options = NativeRunOptions {
        title: String::from("SemPal Installer"),
        inner_size: Some([860.0, 620.0]),
        min_inner_size: Some([640.0, 420.0]),
        maximized: false,
        target_fps: 120,
        icon: load_installer_icon(),
    };
    run_native_vello_app_declarative(options, InstallerNativeBridge::new())
}

fn load_installer_icon() -> Option<WindowIconRgba> {
    decode_icon(include_bytes!("../../../assets/logo3.ico")).or_else(|| {
        let fallback = decode_icon(include_bytes!("../../../assets/logo3.png"));
        if fallback.is_none() {
            eprintln!("Failed to decode installer icon assets.");
        }
        fallback
    })
}

fn decode_icon(bytes: &[u8]) -> Option<WindowIconRgba> {
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    Some(WindowIconRgba {
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn step_label(step: InstallStep) -> &'static str {
    match step {
        InstallStep::Welcome => "welcome",
        InstallStep::License => "license",
        InstallStep::Location => "location",
        InstallStep::Installing => "installing",
        InstallStep::Done => "done",
        InstallStep::Error => "error",
    }
}

fn bool_word(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(not(test))]
fn request_process_exit() {
    std::process::exit(0);
}

#[cfg(test)]
fn request_process_exit() {}

pub(crate) fn send_started(
    sender: &mpsc::Sender<InstallerEvent>,
    total_files: usize,
) -> Result<(), String> {
    sender
        .send(InstallerEvent::Started { total_files })
        .map_err(|err| format!("Failed to report install start: {err}"))
}

pub(crate) fn send_file_copied(
    sender: &mpsc::Sender<InstallerEvent>,
    copied_files: usize,
    name: String,
) -> Result<(), String> {
    sender
        .send(InstallerEvent::FileCopied { copied_files, name })
        .map_err(|err| format!("Failed to report install progress: {err}"))
}

pub(crate) fn send_finished(sender: &mpsc::Sender<InstallerEvent>) -> Result<(), String> {
    sender
        .send(InstallerEvent::Finished)
        .map_err(|err| format!("Failed to report completion: {err}"))
}

pub(crate) type InstallerSender = mpsc::Sender<InstallerEvent>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_transitions_follow_expected_order() {
        let mut bridge = InstallerNativeBridge::new();
        bridge.advance_step();
        assert_eq!(step_label(bridge.step), "license");
        bridge.advance_step();
        assert_eq!(step_label(bridge.step), "location");
    }
}
