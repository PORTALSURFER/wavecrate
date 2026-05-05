use sempal::{
    app_core::actions::{NativeAppBridge, NativeAppModel as AppModel, NativeUiAction as UiAction},
    companion_apps::native_ui::standard_window_options,
    gui_runtime::run_native_vello_app_declarative,
    updater::{APP_NAME, UpdaterRunArgs, open_release_page},
};
use std::sync::Arc;

mod projection;
mod state;
mod tasks;

use self::state::{UiStatus, UpdateNativeBridge};

/// Run the updater UI using the native radiant runtime.
pub fn run_gui(args: UpdaterRunArgs) -> Result<(), String> {
    let options = standard_window_options(format!("{APP_NAME} updater"), None);
    run_native_vello_app_declarative(options, UpdateNativeBridge::new(args))
}

impl NativeAppBridge for UpdateNativeBridge {
    /// Project updater state into the latest shared immutable UI model snapshot.
    fn project_model(&mut self) -> Arc<AppModel> {
        self.poll_background_updates();
        self.ensure_selected_tag();
        Arc::new(self.app_model())
    }

    /// Project updater state into the latest immutable UI model snapshot.
    fn pull_model(&mut self) -> AppModel {
        Arc::unwrap_or_clone(self.project_model())
    }

    /// Reduce one runtime UI action into updater state transitions.
    fn reduce_action(&mut self, action: UiAction) {
        match action {
            UiAction::CheckForUpdates => self.refresh_release_list(),
            UiAction::InstallUpdate => {
                if !matches!(self.status, UiStatus::Updating) {
                    self.start_update();
                }
            }
            UiAction::OpenUpdateLink => {
                if let Some(url) = self
                    .selected_release()
                    .map(|release| release.html_url.clone())
                    .filter(|url| !url.is_empty())
                    && let Err(err) = open_release_page(&url)
                {
                    self.status = UiStatus::Error(err);
                }
            }
            UiAction::DismissUpdate => request_process_exit(),
            UiAction::FocusBrowserRow { visible_row } => {
                if !self.show_log_view {
                    self.select_release_by_row(visible_row);
                }
            }
            UiAction::MoveBrowserFocus { delta } => {
                if !self.show_log_view {
                    self.move_release_focus(delta);
                }
            }
            UiAction::SetBrowserTab { map } => {
                self.show_log_view = map;
            }
            _ => {}
        }
    }
}

#[cfg(not(test))]
fn request_process_exit() {
    std::process::exit(0);
}

#[cfg(test)]
fn request_process_exit() {}

#[cfg(test)]
mod tests;
