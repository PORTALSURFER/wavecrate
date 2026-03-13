use super::state::{ReleaseState, UiStatus, UpdateNativeBridge, channel_label};
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
    updater::APP_NAME,
};

impl UpdateNativeBridge {
    pub(super) fn update_panel_model(&self) -> UpdatePanelModel {
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

    pub(super) fn browser_rows(&self) -> Vec<BrowserRowModel> {
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

    pub(super) fn app_model(&self) -> AppModel {
        let rows = self.browser_rows();
        let browser = standard_browser_panel(CompanionBrowserPanelConfig {
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
            active_tab_label: Some(if self.show_log_view {
                String::from("Log")
            } else {
                String::from("Versions")
            }),
            focused_sample_label: self.selected_tag.clone(),
            rows,
            sort_label: Some(String::from("recent first")),
        });
        let browser_chrome = standard_browser_chrome(CompanionBrowserChromeConfig {
            samples_tab_label: String::from("Versions"),
            map_tab_label: String::from("Progress log"),
            search_prefix_label: String::from("Mode"),
            search_placeholder: String::from("Use top actions"),
            activity_ready_label: String::from("Ready"),
            activity_busy_label: String::from("Updating"),
            sort_prefix_label: String::from("Order"),
            sort_order_label: String::from("Recent"),
            item_count: browser.visible_count,
        });
        standard_app_model(CompanionAppModelConfig {
            title: format!("{APP_NAME} updater"),
            backend_label: format!(
                "{} | {}",
                channel_label(self.args.identity.channel),
                self.args.install_dir.display()
            ),
            sources_label: String::from("Updater"),
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
            browser,
            browser_chrome,
            source_rows: vec![
                SourceRowModel::new(
                    "Install dir",
                    self.args.install_dir.display().to_string(),
                    false,
                    false,
                ),
                SourceRowModel::new("Target", self.args.identity.target.clone(), false, false),
            ],
            update: self.update_panel_model(),
        })
    }
}
