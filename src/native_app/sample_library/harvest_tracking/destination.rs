use super::{
    BrowserContextTargetKind, GuiMessage, HARVEST_ROOT_FOLDER, NativeAppState, Path, PathBuf,
    SampleSource, context_menu, emit_gui_action,
};

impl NativeAppState {
    pub(in crate::native_app) fn harvest_destination_for_protected_origin(
        &self,
        source_path: &Path,
    ) -> Result<Option<PathBuf>, String> {
        let Some((origin_source, _)) = self
            .library
            .folder_browser
            .sample_source_for_file_path(source_path)
        else {
            return Ok(None);
        };
        if !origin_source.is_protected() {
            return Ok(None);
        }
        self.harvest_destination_for_source(&origin_source)
            .map(Some)
            .map_err(|_| {
                String::from("Set a Primary source before extracting from a protected source")
            })
    }

    pub(in crate::native_app) fn harvest_destination_for_origin(
        &self,
        source_path: &Path,
    ) -> Result<PathBuf, String> {
        let Some((origin_source, _)) = self
            .library
            .folder_browser
            .sample_source_for_file_path(source_path)
        else {
            return Err(String::from("Sample is not in a configured harvest source"));
        };
        self.harvest_destination_for_source(&origin_source)
    }

    pub(in crate::native_app) fn open_context_sample_harvest_destination(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = std::time::Instant::now();
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        if menu.kind != BrowserContextTargetKind::Sample {
            self.ui.status.sample = String::from("Choose a sample for harvest actions");
            emit_gui_action(
                "browser.context_menu.harvest.destination",
                Some(context_menu::pane(&menu.kind)),
                Some(context_menu::target_label(&menu.path).as_str()),
                "blocked",
                started_at,
                Some("target is not a sample"),
            );
            return;
        }
        let Some((source, _)) = self
            .library
            .folder_browser
            .sample_source_for_file_path(&menu.path)
        else {
            self.ui.status.sample = String::from("Sample is not in a configured harvest source");
            emit_gui_action(
                "browser.context_menu.harvest.destination",
                Some("browser"),
                Some(context_menu::target_label(&menu.path).as_str()),
                "not_found",
                started_at,
                Some("harvest source unavailable"),
            );
            return;
        };
        self.open_harvest_destination_for_source(
            &source,
            &menu.path,
            "browser.context_menu.harvest.destination",
            started_at,
            context,
        );
    }

    pub(in crate::native_app) fn open_selected_sample_harvest_destination(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = std::time::Instant::now();
        let Some(path) = self
            .library
            .folder_browser
            .selected_file_id()
            .map(PathBuf::from)
        else {
            self.ui.status.sample = String::from("Select a sample for harvest actions");
            emit_gui_action(
                "browser.harvest_family.destination",
                Some("browser"),
                None,
                "blocked",
                started_at,
                Some("no selected sample"),
            );
            return;
        };
        let Some((source, _)) = self
            .library
            .folder_browser
            .sample_source_for_file_path(&path)
        else {
            self.ui.status.sample = String::from("Sample is not in a configured harvest source");
            emit_gui_action(
                "browser.harvest_family.destination",
                Some("browser"),
                Some(context_menu::target_label(&path).as_str()),
                "not_found",
                started_at,
                Some("harvest source unavailable"),
            );
            return;
        };
        self.open_harvest_destination_for_source(
            &source,
            &path,
            "browser.harvest_family.destination",
            started_at,
            context,
        );
    }

    fn harvest_destination_for_source(&self, source: &SampleSource) -> Result<PathBuf, String> {
        let target_source = self
            .library
            .folder_browser
            .default_writable_extraction_source(
                "Set a Primary source before using a harvest destination",
            )?;
        Ok(target_source
            .root
            .join(HARVEST_ROOT_FOLDER)
            .join(harvest_source_folder_name(source)))
    }

    fn open_harvest_destination_for_source(
        &mut self,
        source: &SampleSource,
        target_path: &Path,
        action: &'static str,
        started_at: std::time::Instant,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let destination = match self.harvest_destination_for_source(source) {
            Ok(destination) => destination,
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    action,
                    Some("browser"),
                    Some(context_menu::target_label(target_path).as_str()),
                    "blocked",
                    started_at,
                    Some(&error),
                );
                return;
            }
        };
        if let Err(error) = wavecrate::sample_sources::harvest_file_ops::ensure_dir(
            &destination,
            "Create harvest destination failed",
        ) {
            self.ui.status.sample = error.clone();
            emit_gui_action(
                action,
                Some("browser"),
                Some(context_menu::target_label(&destination).as_str()),
                "error",
                started_at,
                Some(&error),
            );
            return;
        }
        let completion_path = destination.clone();
        context.open_path(destination.clone(), move |result| {
            GuiMessage::ContextTargetOpenFinished {
                kind: BrowserContextTargetKind::Folder,
                path: completion_path.clone(),
                result,
            }
        });
        self.ui.status.sample = format!("Opening {}", destination.display());
        emit_gui_action(
            action,
            Some("browser"),
            Some(context_menu::target_label(&destination).as_str()),
            "requested",
            started_at,
            None,
        );
    }
}

fn harvest_source_folder_name(source: &SampleSource) -> String {
    source
        .root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(sanitize_harvest_folder_component)
        .unwrap_or_else(|| sanitize_harvest_folder_component(source.id.as_str()))
}

fn sanitize_harvest_folder_component(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect::<String>();
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        String::from("Source")
    } else {
        trimmed.to_string()
    }
}
