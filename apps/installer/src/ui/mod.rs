//! Native installer bridge and shared worker-event helpers.

use std::{sync::Arc, sync::mpsc};

use sempal::{
    app_core::actions::{NativeAppBridge, NativeAppModel as AppModel, NativeUiAction as UiAction},
    companion_apps::native_ui::decode_first_window_icon,
    gui_runtime::run_native_vello_app_declarative,
};

mod projection;
mod state;

use self::projection::step_label;
pub(crate) use self::state::{InstallerEvent, InstallerNativeBridge, InstallerSender};

/// Project installer state into the shared native app model and reduce runtime actions.
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
                if matches!(self.step, state::InstallStep::Location) {
                    self.browse_install_dir();
                } else if matches!(self.step, state::InstallStep::Done)
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
                if matches!(self.step, state::InstallStep::Error) {
                    self.start_install();
                } else {
                    self.advance_step();
                }
            }
            UiAction::SelectSourceRow { index } => {
                if index == 0 && matches!(self.step, state::InstallStep::Location) {
                    self.browse_install_dir();
                }
            }
            _ => {}
        }
    }
}

/// Run the installer UI using the native radiant runtime.
pub(crate) fn run_installer_app() -> Result<(), String> {
    let options = sempal::companion_apps::native_ui::standard_window_options(
        "SemPal Installer",
        load_installer_icon(),
    );
    run_native_vello_app_declarative(options, InstallerNativeBridge::new())
}

fn load_installer_icon() -> Option<sempal::gui_runtime::WindowIconRgba> {
    let icon = decode_first_window_icon(&[
        include_bytes!("../../../../assets/logo3.ico"),
        include_bytes!("../../../../assets/logo3.png"),
    ]);
    if icon.is_none() {
        eprintln!("Failed to decode installer icon assets.");
    }
    icon
}

#[cfg(not(test))]
fn request_process_exit() {
    std::process::exit(0);
}

#[cfg(test)]
fn request_process_exit() {}

/// Report the total number of files the installer expects to copy.
pub(crate) fn send_started(
    sender: &mpsc::Sender<InstallerEvent>,
    total_files: usize,
) -> Result<(), String> {
    sender
        .send(InstallerEvent::Started { total_files })
        .map_err(|err| format!("Failed to report install start: {err}"))
}

/// Report one copied file and the current completion count.
pub(crate) fn send_file_copied(
    sender: &mpsc::Sender<InstallerEvent>,
    copied_files: usize,
    name: String,
) -> Result<(), String> {
    sender
        .send(InstallerEvent::FileCopied { copied_files, name })
        .map_err(|err| format!("Failed to report install progress: {err}"))
}

/// Report a successful installer completion event.
pub(crate) fn send_finished(sender: &mpsc::Sender<InstallerEvent>) -> Result<(), String> {
    sender
        .send(InstallerEvent::Finished)
        .map_err(|err| format!("Failed to report completion: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sempal::app_core::actions::NativeUiAction as UiAction;

    #[test]
    fn step_transitions_follow_expected_order() {
        let mut bridge = InstallerNativeBridge::new();
        bridge.advance_step();
        assert_eq!(step_label(bridge.step), "license");
        bridge.advance_step();
        assert_eq!(step_label(bridge.step), "location");
    }

    #[test]
    fn error_retry_restarts_install_state() {
        let mut bridge = InstallerNativeBridge::new();
        bridge.step = state::InstallStep::Error;
        bridge.logs.push(String::from("old log"));
        bridge.finish_errors.push(String::from("old finish error"));
        bridge.install_finished = true;

        bridge.reduce_action(UiAction::CheckForUpdates);

        assert!(matches!(bridge.step, state::InstallStep::Installing));
        assert!(bridge.logs.is_empty());
        assert!(bridge.finish_errors.is_empty());
        assert!(!bridge.install_finished);
    }

    #[test]
    fn location_projection_selects_install_path_row() {
        let mut bridge = InstallerNativeBridge::new();
        bridge.advance_step();
        bridge.advance_step();

        let model = bridge.app_model();

        assert_eq!(model.status.left, "location");
        assert_eq!(model.browser.selected_visible_row, Some(0));
        assert_eq!(
            model.update.release_notes_label,
            bridge.install_dir.display().to_string()
        );
    }
}
