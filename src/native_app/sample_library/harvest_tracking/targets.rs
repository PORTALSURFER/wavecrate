use super::{
    BrowserContextTargetKind, HarvestFileIdentity, HarvestState, HarvestStateTarget,
    NativeAppState, PathBuf, context_menu, emit_gui_action,
};

impl NativeAppState {
    pub(super) fn take_context_sample_harvest_state_targets(
        &mut self,
        action: &'static str,
        started_at: std::time::Instant,
    ) -> Option<Vec<HarvestStateTarget>> {
        let menu = self.take_sample_context_menu(action, started_at)?;
        let selected_paths = self.library.folder_browser.explicit_selected_file_paths();
        let paths = if selected_paths.len() > 1 {
            selected_paths
        } else {
            vec![menu.path]
        };
        let mut targets = Vec::with_capacity(paths.len());
        for path in paths {
            let Some(identity) = self.harvest_identity_for_path(&path) else {
                self.ui.status.sample =
                    String::from("Sample is not in a configured harvest source");
                emit_gui_action(
                    action,
                    Some("browser"),
                    Some(context_menu::target_label(&path).as_str()),
                    "not_found",
                    started_at,
                    Some("harvest identity unavailable"),
                );
                return None;
            };
            targets.push(HarvestStateTarget {
                path,
                identity,
                state: HarvestState::New,
            });
        }
        Some(targets)
    }

    pub(super) fn take_context_sample_harvest_target(
        &mut self,
        action: &'static str,
        started_at: std::time::Instant,
    ) -> Option<(PathBuf, HarvestFileIdentity)> {
        let menu = self.take_sample_context_menu(action, started_at)?;
        let Some(identity) = self.harvest_identity_for_path(&menu.path) else {
            self.ui.status.sample = String::from("Sample is not in a configured harvest source");
            emit_gui_action(
                action,
                Some("browser"),
                Some(context_menu::target_label(&menu.path).as_str()),
                "not_found",
                started_at,
                Some("harvest identity unavailable"),
            );
            return None;
        };
        Some((menu.path, identity))
    }

    pub(super) fn take_sample_context_menu(
        &mut self,
        action: &'static str,
        started_at: std::time::Instant,
    ) -> Option<context_menu::BrowserContextMenu> {
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return None;
        };
        if menu.kind != BrowserContextTargetKind::Sample {
            self.ui.status.sample = String::from("Choose a sample for harvest actions");
            emit_gui_action(
                action,
                Some(context_menu::pane(&menu.kind)),
                Some(context_menu::target_label(&menu.path).as_str()),
                "blocked",
                started_at,
                Some("target is not a sample"),
            );
            return None;
        }
        Some(menu)
    }

    pub(super) fn selected_sample_harvest_target(
        &mut self,
        action: &'static str,
        started_at: std::time::Instant,
    ) -> Option<(PathBuf, HarvestFileIdentity)> {
        let Some(path) = self
            .library
            .folder_browser
            .selected_file_id()
            .map(PathBuf::from)
        else {
            self.ui.status.sample = String::from("Select a sample for harvest actions");
            emit_gui_action(
                action,
                Some("browser"),
                None,
                "blocked",
                started_at,
                Some("no selected sample"),
            );
            return None;
        };
        let Some(identity) = self.harvest_identity_for_path(&path) else {
            self.ui.status.sample = String::from("Sample is not in a configured harvest source");
            emit_gui_action(
                action,
                Some("browser"),
                Some(context_menu::target_label(&path).as_str()),
                "not_found",
                started_at,
                Some("harvest identity unavailable"),
            );
            return None;
        };
        Some((path, identity))
    }

    pub(super) fn selected_harvest_state_targets(
        &mut self,
        action: &'static str,
        started_at: std::time::Instant,
    ) -> Option<Vec<HarvestStateTarget>> {
        let mut paths = self.library.folder_browser.explicit_selected_file_paths();
        if paths.is_empty() {
            let Some(path) = self
                .library
                .folder_browser
                .selected_file_id()
                .map(PathBuf::from)
            else {
                self.ui.status.sample = String::from("Select a sample for harvest actions");
                emit_gui_action(
                    action,
                    Some("browser"),
                    None,
                    "blocked",
                    started_at,
                    Some("no selected sample"),
                );
                return None;
            };
            paths.push(path);
        }

        let mut targets = Vec::with_capacity(paths.len());
        for path in paths {
            let Some(identity) = self.harvest_identity_for_path(&path) else {
                self.ui.status.sample =
                    String::from("Sample is not in a configured harvest source");
                emit_gui_action(
                    action,
                    Some("browser"),
                    Some(context_menu::target_label(&path).as_str()),
                    "not_found",
                    started_at,
                    Some("harvest identity unavailable"),
                );
                return None;
            };
            let state = match wavecrate::sample_sources::library::harvest_file(&identity.key) {
                Ok(record) => record
                    .map(|record| record.state)
                    .unwrap_or(HarvestState::New),
                Err(error) => {
                    self.ui.status.sample = format!("Update harvest state failed: {error}");
                    emit_gui_action(
                        action,
                        Some("browser"),
                        Some(context_menu::target_label(&path).as_str()),
                        "error",
                        started_at,
                        Some(&error.to_string()),
                    );
                    return None;
                }
            };
            targets.push(HarvestStateTarget {
                path,
                identity,
                state,
            });
        }
        Some(targets)
    }
}
