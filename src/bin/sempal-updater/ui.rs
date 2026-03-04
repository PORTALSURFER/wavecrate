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
    gui_runtime::{NativeRunOptions, run_native_vello_app_declarative},
    updater::{
        APP_NAME, ApplyPlan, ReleaseSummary, UpdateChannel, UpdateProgress, UpdaterRunArgs,
        apply_update_with_progress, list_recent_releases, open_release_page,
    },
};
use std::{
    sync::{
        Arc,
        mpsc::{self, Receiver},
    },
    thread,
};

const MAX_LOG_LINES: usize = 200;
const RELEASE_LIST_LIMIT: usize = 5;

/// Run the updater UI using the native radiant runtime.
pub fn run_gui(args: UpdaterRunArgs) -> Result<(), String> {
    let options = NativeRunOptions {
        title: format!("{APP_NAME} updater"),
        inner_size: Some([860.0, 620.0]),
        min_inner_size: Some([640.0, 420.0]),
        maximized: false,
        target_fps: 120,
        icon: None,
    };
    run_native_vello_app_declarative(options, UpdateNativeBridge::new(args))
}

#[derive(Debug, Clone)]
struct ReleaseOption {
    tag: String,
    label: String,
    html_url: String,
}

#[derive(Debug, Clone)]
enum ReleaseState {
    Idle,
    Loading,
    Loaded(Vec<ReleaseOption>),
    Error(String),
}

#[derive(Debug, Clone)]
enum UiStatus {
    Idle,
    Updating,
    Success(String),
    Error(String),
}

struct UpdateNativeBridge {
    args: UpdaterRunArgs,
    release_state: ReleaseState,
    release_rx: Option<Receiver<Result<Vec<ReleaseSummary>, String>>>,
    selected_tag: Option<String>,
    status: UiStatus,
    log: Vec<String>,
    progress_rx: Option<Receiver<UpdateProgress>>,
    result_rx: Option<Receiver<Result<ApplyPlan, String>>>,
    show_log_view: bool,
}

impl UpdateNativeBridge {
    fn new(args: UpdaterRunArgs) -> Self {
        let mut bridge = Self {
            args,
            release_state: ReleaseState::Idle,
            release_rx: None,
            selected_tag: None,
            status: UiStatus::Idle,
            log: Vec::new(),
            progress_rx: None,
            result_rx: None,
            show_log_view: false,
        };
        bridge.refresh_release_list();
        bridge
    }

    fn refresh_release_list(&mut self) {
        if self.args.identity.channel == UpdateChannel::Nightly {
            self.release_state = ReleaseState::Loaded(vec![ReleaseOption {
                tag: "nightly".to_string(),
                label: "nightly".to_string(),
                html_url: String::new(),
            }]);
            self.selected_tag = Some("nightly".to_string());
            return;
        }
        let repo = self.args.repo.clone();
        let identity = self.args.identity.clone();
        let channel = self.args.identity.channel;
        let (tx, rx) = mpsc::channel();
        self.release_state = ReleaseState::Loading;
        self.release_rx = Some(rx);
        thread::spawn(move || {
            let result = list_recent_releases(&repo, channel, &identity, RELEASE_LIST_LIMIT)
                .map_err(|err| err.to_string());
            let _ = tx.send(result);
        });
    }

    fn ensure_selected_tag(&mut self) {
        if self.selected_tag.is_some() {
            return;
        }
        if let ReleaseState::Loaded(options) = &self.release_state
            && let Some(first) = options.first()
        {
            self.selected_tag = Some(first.tag.clone());
        }
    }

    fn selected_release(&self) -> Option<&ReleaseOption> {
        let selected = self.selected_tag.as_deref()?;
        let ReleaseState::Loaded(options) = &self.release_state else {
            return None;
        };
        options.iter().find(|option| option.tag == selected)
    }

    fn select_release_by_row(&mut self, visible_row: usize) {
        let ReleaseState::Loaded(options) = &self.release_state else {
            return;
        };
        if let Some(option) = options.get(visible_row) {
            self.selected_tag = Some(option.tag.clone());
        }
    }

    fn move_release_focus(&mut self, delta: i8) {
        let ReleaseState::Loaded(options) = &self.release_state else {
            return;
        };
        if options.is_empty() {
            return;
        }
        let current_index = self
            .selected_tag
            .as_ref()
            .and_then(|tag| options.iter().position(|option| option.tag == *tag))
            .unwrap_or(0);
        let max_index = options.len() - 1;
        let next_index = if delta.is_negative() {
            current_index.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            (current_index + delta as usize).min(max_index)
        };
        if let Some(option) = options.get(next_index) {
            self.selected_tag = Some(option.tag.clone());
        }
    }

    fn start_update(&mut self) {
        if matches!(self.status, UiStatus::Updating) {
            return;
        }
        let mut args = self.args.clone();
        args.requested_tag = self.selected_tag.clone();
        let (progress_tx, progress_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        self.progress_rx = Some(progress_rx);
        self.result_rx = Some(result_rx);
        self.log.clear();
        self.push_log("Starting update...");
        self.status = UiStatus::Updating;
        self.show_log_view = true;
        thread::spawn(move || {
            let result = apply_update_with_progress(args, |progress| {
                let _ = progress_tx.send(progress);
            })
            .map_err(|err| err.to_string());
            let _ = result_tx.send(result);
        });
    }

    fn push_log(&mut self, message: impl Into<String>) {
        self.log.push(message.into());
        if self.log.len() > MAX_LOG_LINES {
            let trim = self.log.len() - MAX_LOG_LINES;
            self.log.drain(0..trim);
        }
    }

    fn poll_background_updates(&mut self) {
        if let Some(rx) = &self.release_rx
            && let Ok(result) = rx.try_recv()
        {
            self.release_rx = None;
            match result {
                Ok(list) => {
                    let options = list.into_iter().map(format_release_option).collect();
                    self.release_state = ReleaseState::Loaded(options);
                    self.ensure_selected_tag();
                }
                Err(err) => {
                    self.release_state = ReleaseState::Error(err);
                }
            }
        }

        if let Some(rx) = &self.progress_rx {
            let messages: Vec<String> = rx.try_iter().map(|progress| progress.message).collect();
            for message in messages {
                self.push_log(message);
            }
        }

        if let Some(rx) = &self.result_rx
            && let Ok(result) = rx.try_recv()
        {
            self.result_rx = None;
            self.progress_rx = None;
            match result {
                Ok(plan) => {
                    self.push_log(format!(
                        "Installed {} into {}",
                        plan.release_tag,
                        plan.install_dir.display()
                    ));
                    if !plan.stale_removal_failures.is_empty() {
                        self.push_log(format!(
                            "Warning: failed to remove {} stale paths",
                            plan.stale_removal_failures.len()
                        ));
                        for failure in &plan.stale_removal_failures {
                            self.push_log(format!(
                                "Stale remove failed: {} ({})",
                                failure.path.display(),
                                failure.error
                            ));
                        }
                    }
                    self.status = UiStatus::Success(format!("Updated to {}", plan.release_tag));
                }
                Err(err) => {
                    self.push_log(format!("Update failed: {err}"));
                    self.status = UiStatus::Error(err);
                }
            }
        }
    }

    fn update_panel_model(&self) -> UpdatePanelModel {
        let mut model = UpdatePanelModel::default();
        match &self.status {
            UiStatus::Updating => {
                model.status = UpdateStatusModel::Checking;
                model.status_label = String::from("Applying update");
                model.action_hint_label = String::from("Please wait for completion");
            }
            UiStatus::Error(message) => {
                model.status = UpdateStatusModel::Error;
                model.status_label = String::from("Update failed");
                model.last_error = Some(message.clone());
                model.action_hint_label = String::from("Retry check");
            }
            UiStatus::Success(message) => {
                model.status = UpdateStatusModel::Available;
                model.status_label = message.clone();
                if let Some(release) = self.selected_release() {
                    model.available_tag = Some(release.tag.clone());
                    if !release.html_url.is_empty() {
                        model.available_url = Some(release.html_url.clone());
                    }
                }
                model.action_hint_label = String::from("Open release notes or dismiss");
            }
            UiStatus::Idle => match &self.release_state {
                ReleaseState::Loading => {
                    model.status = UpdateStatusModel::Checking;
                    model.status_label = String::from("Loading releases");
                    model.action_hint_label = String::from("Fetching recent releases");
                }
                ReleaseState::Error(message) => {
                    model.status = UpdateStatusModel::Error;
                    model.status_label = String::from("Release list unavailable");
                    model.last_error = Some(message.clone());
                    model.action_hint_label = String::from("Retry check");
                }
                ReleaseState::Loaded(_) => {
                    model.status = UpdateStatusModel::Available;
                    model.status_label = String::from("Ready to update");
                    if let Some(release) = self.selected_release() {
                        model.available_tag = Some(release.tag.clone());
                        if !release.html_url.is_empty() {
                            model.available_url = Some(release.html_url.clone());
                        }
                    }
                    model.action_hint_label =
                        String::from("Install selected release or open notes");
                }
                ReleaseState::Idle => {
                    model.status = UpdateStatusModel::Idle;
                    model.status_label = String::from("Idle");
                    model.action_hint_label = String::from("Check for updates");
                }
            },
        }
        model.release_notes_label = self
            .selected_release()
            .map(|release| format!("Selected: {}", release.label))
            .unwrap_or_else(|| String::from("Selected: none"));
        model
    }

    fn browser_rows(&self) -> Vec<BrowserRowModel> {
        if self.show_log_view {
            return self
                .log
                .iter()
                .enumerate()
                .map(|(index, line)| BrowserRowModel::new(index, line, 1, false, false))
                .collect();
        }

        let ReleaseState::Loaded(options) = &self.release_state else {
            return Vec::new();
        };
        let selected_tag = self.selected_tag.clone();
        options
            .iter()
            .enumerate()
            .map(|(index, option)| {
                let focused = selected_tag.as_ref().is_some_and(|tag| *tag == option.tag);
                BrowserRowModel::new(index, option.label.clone(), 1, focused, focused)
                    .with_bucket_label(option.tag.clone())
            })
            .collect()
    }

    fn app_model(&self) -> AppModel {
        let mut model = AppModel {
            title: format!("{APP_NAME} updater"),
            backend_label: format!(
                "{} | {}",
                channel_label(self.args.identity.channel),
                self.args.install_dir.display()
            ),
            sources_label: String::from("Updater"),
            status_text: String::new(),
            status: StatusBarModel {
                left: format!("channel: {}", channel_label(self.args.identity.channel)),
                center: self
                    .selected_tag
                    .clone()
                    .map(|tag| format!("selected: {tag}"))
                    .unwrap_or_else(|| String::from("selected: none")),
                right: match self.status {
                    UiStatus::Updating => String::from("updating"),
                    UiStatus::Success(_) => String::from("updated"),
                    UiStatus::Error(_) => String::from("error"),
                    UiStatus::Idle => String::from("idle"),
                },
            },
            transport_running: true,
            ..AppModel::default()
        };

        model.browser_actions = BrowserActionsModel::default();
        model.browser = BrowserPanelModel {
            visible_count: self.browser_rows().len(),
            selected_visible_row: self.selected_tag.as_ref().and_then(|selected| {
                let ReleaseState::Loaded(options) = &self.release_state else {
                    return None;
                };
                options.iter().position(|option| option.tag == *selected)
            }),
            selected_path_count: usize::from(self.selected_tag.is_some()),
            search_query: if self.show_log_view {
                String::from("log view")
            } else {
                String::from("release list")
            },
            search_placeholder: Some(String::from("Arrows + enter to select release")),
            busy: matches!(self.release_state, ReleaseState::Loading)
                || matches!(self.status, UiStatus::Updating),
            sort_label: Some(String::from("recent first")),
            active_tab_label: Some(if self.show_log_view {
                String::from("Log")
            } else {
                String::from("Versions")
            }),
            focused_sample_label: self.selected_tag.clone(),
            anchor_visible_row: None,
            rows: self.browser_rows(),
        };
        model.browser_chrome = BrowserChromeModel {
            samples_tab_label: String::from("Versions"),
            map_tab_label: String::from("Progress log"),
            search_prefix_label: String::from("Mode"),
            search_placeholder: String::from("Use top actions"),
            activity_ready_label: String::from("Ready"),
            activity_busy_label: String::from("Updating"),
            sort_prefix_label: String::from("Order"),
            sort_order_label: String::from("Recent"),
            similarity_toggle_label: String::from("n/a"),
            item_count_label: format!("{} rows", model.browser.visible_count),
        };
        model.sources.rows = vec![
            SourceRowModel::new(
                "Install dir",
                self.args.install_dir.display().to_string(),
                false,
                false,
            ),
            SourceRowModel::new("Target", self.args.identity.target.clone(), false, false),
        ];
        model.update = self.update_panel_model();
        model
    }
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
    fn on_action(&mut self, action: UiAction) {
        match action {
            UiAction::CheckForUpdates => {
                self.refresh_release_list();
            }
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
            UiAction::DismissUpdate => {
                request_process_exit();
            }
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

fn format_release_option(summary: ReleaseSummary) -> ReleaseOption {
    let label = match summary.published_at.as_deref() {
        Some(date) => format!("{} ({})", summary.tag, short_date(date)),
        None => summary.tag.clone(),
    };
    ReleaseOption {
        tag: summary.tag,
        label,
        html_url: summary.html_url,
    }
}

fn short_date(value: &str) -> String {
    value.get(0..10).unwrap_or(value).to_string()
}

fn channel_label(channel: UpdateChannel) -> &'static str {
    match channel {
        UpdateChannel::Stable => "stable",
        UpdateChannel::Nightly => "nightly",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sempal::updater::{RuntimeIdentity, UpdateChannel, UpdaterRunArgs};
    use std::path::PathBuf;

    fn test_args() -> UpdaterRunArgs {
        UpdaterRunArgs {
            repo: "owner/repo".to_string(),
            identity: RuntimeIdentity {
                app: "Sempal".to_string(),
                channel: UpdateChannel::Stable,
                target: "x86_64".to_string(),
                platform: "windows".to_string(),
                arch: "x86_64".to_string(),
            },
            install_dir: PathBuf::from("/tmp/sempal"),
            relaunch: true,
            requested_tag: None,
        }
    }

    #[test]
    fn focus_action_selects_loaded_release() {
        let mut bridge = UpdateNativeBridge::new(test_args());
        bridge.release_state = ReleaseState::Loaded(vec![
            ReleaseOption {
                tag: "v1.0.0".to_string(),
                label: "v1.0.0".to_string(),
                html_url: String::new(),
            },
            ReleaseOption {
                tag: "v1.1.0".to_string(),
                label: "v1.1.0".to_string(),
                html_url: String::new(),
            },
        ]);
        bridge.on_action(UiAction::FocusBrowserRow { visible_row: 1 });
        assert_eq!(bridge.selected_tag.as_deref(), Some("v1.1.0"));
    }

    #[test]
    fn app_model_switches_tabs_for_log_view() {
        let mut bridge = UpdateNativeBridge::new(test_args());
        bridge.on_action(UiAction::SetBrowserTab { map: true });
        let model = bridge.pull_model();
        assert_eq!(model.browser.active_tab_label.as_deref(), Some("Log"));
    }
}
