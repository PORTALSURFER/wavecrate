use super::{
    GuiMessage, HarvestFileIdentity, NativeAppState, PathBuf, SAMPLE_BROWSER_LIST_ID,
    SAMPLE_BROWSER_ROW_HEIGHT, SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS, context_menu,
    emit_gui_action, sample_path_label,
};

impl NativeAppState {
    pub(in crate::native_app) fn show_context_sample_harvest_origin(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = std::time::Instant::now();
        let Some((path, identity)) = self
            .take_context_sample_harvest_target("browser.context_menu.harvest.origin", started_at)
        else {
            return;
        };
        self.show_harvest_origin_for_target(
            path,
            identity,
            "browser.context_menu.harvest.origin",
            started_at,
            context,
        );
    }

    pub(in crate::native_app) fn show_selected_sample_harvest_origin(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = std::time::Instant::now();
        let Some((path, identity)) =
            self.selected_sample_harvest_target("browser.harvest_family.origin", started_at)
        else {
            return;
        };
        self.show_harvest_origin_for_target(
            path,
            identity,
            "browser.harvest_family.origin",
            started_at,
            context,
        );
    }

    pub(in crate::native_app) fn show_selected_sample_harvest_derivatives(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = std::time::Instant::now();
        let Some((path, identity)) =
            self.selected_sample_harvest_target("browser.harvest_family.derivatives", started_at)
        else {
            return;
        };
        self.show_harvest_derivatives_for_target(
            path,
            identity,
            "browser.harvest_family.derivatives",
            started_at,
            context,
        );
    }

    fn show_harvest_origin_for_target(
        &mut self,
        path: PathBuf,
        identity: HarvestFileIdentity,
        action: &'static str,
        started_at: std::time::Instant,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let parents =
            match wavecrate::sample_sources::library::harvest_parents_for_child(&identity.key) {
                Ok(parents) => parents,
                Err(error) => {
                    self.ui.status.sample = format!("Show harvest origin failed: {error}");
                    emit_gui_action(
                        action,
                        Some("browser"),
                        Some(context_menu::target_label(&path).as_str()),
                        "error",
                        started_at,
                        Some(&error.to_string()),
                    );
                    return;
                }
            };
        let Some(parent_path) = parents
            .iter()
            .find_map(|edge| self.harvest_path_for_key(&edge.parent.key))
        else {
            self.ui.status.sample = String::from("No harvest origin recorded");
            emit_gui_action(
                action,
                Some("browser"),
                Some(context_menu::target_label(&path).as_str()),
                "empty",
                started_at,
                None,
            );
            return;
        };
        self.focus_harvest_related_sample(
            parent_path,
            "origin",
            parents.len(),
            started_at,
            action,
            context,
        );
    }

    pub(in crate::native_app) fn show_context_sample_harvest_derivatives(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = std::time::Instant::now();
        let Some((path, identity)) = self.take_context_sample_harvest_target(
            "browser.context_menu.harvest.derivatives",
            started_at,
        ) else {
            return;
        };
        self.show_harvest_derivatives_for_target(
            path,
            identity,
            "browser.context_menu.harvest.derivatives",
            started_at,
            context,
        );
    }

    fn show_harvest_derivatives_for_target(
        &mut self,
        path: PathBuf,
        identity: HarvestFileIdentity,
        action: &'static str,
        started_at: std::time::Instant,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let derivatives =
            match wavecrate::sample_sources::library::harvest_derivations_for_parent(&identity.key)
            {
                Ok(derivatives) => derivatives,
                Err(error) => {
                    self.ui.status.sample = format!("Show harvest derivatives failed: {error}");
                    emit_gui_action(
                        action,
                        Some("browser"),
                        Some(context_menu::target_label(&path).as_str()),
                        "error",
                        started_at,
                        Some(&error.to_string()),
                    );
                    return;
                }
            };
        let Some(child_path) = derivatives.iter().find_map(|edge| {
            let path = self.harvest_path_for_key(&edge.child.key)?;
            wavecrate::sample_sources::harvest_file_ops::path_exists(&path).then_some(path)
        }) else {
            self.ui.status.sample = if derivatives.is_empty() {
                String::from("No harvest derivatives recorded")
            } else {
                String::from("Harvest derivatives are missing or outside configured sources")
            };
            emit_gui_action(
                action,
                Some("browser"),
                Some(context_menu::target_label(&path).as_str()),
                if derivatives.is_empty() {
                    "empty"
                } else {
                    "missing"
                },
                started_at,
                None,
            );
            return;
        };
        self.focus_harvest_related_sample(
            child_path,
            "derivative",
            derivatives.len(),
            started_at,
            action,
            context,
        );
    }

    fn focus_harvest_related_sample(
        &mut self,
        path: PathBuf,
        label: &'static str,
        related_count: usize,
        started_at: std::time::Instant,
        action: &'static str,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        if !wavecrate::sample_sources::harvest_file_ops::path_exists(&path) {
            self.ui.status.sample = format!("Harvest {label} is missing: {}", path.display());
            emit_gui_action(
                action,
                Some("browser"),
                Some(context_menu::target_label(&path).as_str()),
                "missing",
                started_at,
                None,
            );
            return;
        }
        if !self
            .library
            .folder_browser
            .focus_file_across_sources_matching_tags(&path, &self.metadata.tags_by_file)
        {
            self.ui.status.sample = format!("Harvest {label} is not visible in configured sources");
            emit_gui_action(
                action,
                Some("browser"),
                Some(context_menu::target_label(&path).as_str()),
                "not_found",
                started_at,
                None,
            );
            return;
        }
        if let Some(index) = self
            .library
            .folder_browser
            .selected_audio_file_index_matching_tags(&self.metadata.tags_by_file)
        {
            context.scroll_into_view_snapped(
                SAMPLE_BROWSER_LIST_ID,
                index as f32 * SAMPLE_BROWSER_ROW_HEIGHT,
                SAMPLE_BROWSER_ROW_HEIGHT,
                SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS as f32 * SAMPLE_BROWSER_ROW_HEIGHT,
                SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS as f32 * SAMPLE_BROWSER_ROW_HEIGHT,
                SAMPLE_BROWSER_ROW_HEIGHT,
            );
        }
        self.load_sample_without_autoplay(path.display().to_string(), context);
        self.ui.status.sample = format!(
            "Showing harvest {label} {}{}",
            sample_path_label(&path),
            related_count_label(related_count)
        );
        emit_gui_action(
            action,
            Some("browser"),
            Some(context_menu::target_label(&path).as_str()),
            "success",
            started_at,
            None,
        );
    }
}

fn related_count_label(count: usize) -> String {
    if count <= 1 {
        String::new()
    } else {
        format!(" ({count} recorded)")
    }
}
