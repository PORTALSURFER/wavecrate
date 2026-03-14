//! Projection helpers for the installer's shared native app model.

use sempal::{
    app_core::actions::{
        NativeAppModel as AppModel, NativeBrowserRowModel as BrowserRowModel,
        NativeSourceRowModel as SourceRowModel, NativeStatusBarModel as StatusBarModel,
        NativeUpdatePanelModel as UpdatePanelModel, NativeUpdateStatusModel as UpdateStatusModel,
    },
    companion_apps::native_ui::{
        CompanionAppModelConfig, CompanionBrowserChromeConfig, CompanionBrowserPanelConfig,
        standard_app_model, standard_browser_chrome, standard_browser_panel,
    },
};

use crate::APP_NAME;

use super::state::{InstallStep, InstallerNativeBridge};

impl InstallerNativeBridge {
    pub(crate) fn app_model(&self) -> AppModel {
        let rows = self.browser_rows();
        let browser = standard_browser_panel(CompanionBrowserPanelConfig {
            selected_visible_row: if matches!(self.step, InstallStep::Location) {
                Some(0)
            } else {
                None
            },
            selected_path_count: 0,
            search_query: step_label(self.step).to_string(),
            search_placeholder: Some(String::from("Use top action buttons")),
            busy: matches!(self.step, InstallStep::Installing),
            active_tab_label: Some(String::from("Flow")),
            focused_sample_label: None,
            rows,
            sort_label: Some(String::from("installer")),
        });
        let browser_chrome = standard_browser_chrome(CompanionBrowserChromeConfig {
            samples_tab_label: String::from("Flow"),
            map_tab_label: String::from("Log"),
            search_prefix_label: String::from("Step"),
            search_placeholder: String::from("Installer flow"),
            activity_ready_label: String::from("Ready"),
            activity_busy_label: String::from("Installing"),
            sort_prefix_label: String::from("Mode"),
            sort_order_label: String::from("Installer"),
            item_count: browser.visible_count,
        });
        standard_app_model(CompanionAppModelConfig {
            title: format!("{APP_NAME} installer"),
            backend_label: format!("install dir: {}", self.install_dir.display()),
            sources_label: String::from("Installer"),
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
            browser,
            browser_chrome,
            source_rows: vec![
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
            ],
            update: self.update_panel(),
        })
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
}

pub(super) fn step_label(step: InstallStep) -> &'static str {
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
