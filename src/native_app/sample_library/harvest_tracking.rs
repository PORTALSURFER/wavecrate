use std::path::{Path, PathBuf};

use wavecrate::{
    sample_sources::{
        HarvestDerivationOperation, HarvestFileIdentity, HarvestFileKey, HarvestMetadataSnapshot,
        HarvestSourceRange, HarvestState, NewHarvestDerivation, SampleSource, SourceDatabase,
        SourceId, WavEntry,
    },
    selection::SelectionRange,
};

use crate::native_app::{
    app::{GuiMessage, NativeAppState, emit_gui_action, sample_path_label},
    audio::playback::tagged_playback_mode_for_tag,
    sample_library::{
        context_menu_target::{self as context_menu, BrowserContextTargetKind},
        folder_browser::commands::FolderMoveRequest,
        sample_list::{
            SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_ROW_HEIGHT,
            SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS,
        },
    },
};

const HARVEST_ROOT_FOLDER: &str = "_Harvests";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct HarvestFamilySummary {
    pub(in crate::native_app) state_label: String,
    pub(in crate::native_app) origin_count: usize,
    pub(in crate::native_app) derivative_count: usize,
    pub(in crate::native_app) missing_origin_count: usize,
    pub(in crate::native_app) missing_derivative_count: usize,
    pub(in crate::native_app) first_origin_label: Option<String>,
    pub(in crate::native_app) first_derivative_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct HarvestSeenPersistResult {
    pub(in crate::native_app) file_id: String,
    pub(in crate::native_app) result: Result<(), String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HarvestSeenPersistRequest {
    file_id: String,
    source_id: SourceId,
    source_root: PathBuf,
    source_database_root: PathBuf,
    relative_path: PathBuf,
}

impl NativeAppState {
    pub(in crate::native_app) fn selected_harvest_family_summary(
        &self,
    ) -> Option<HarvestFamilySummary> {
        let selected_path = PathBuf::from(self.library.folder_browser.selected_file_id()?);
        let identity = self.harvest_identity_for_path(&selected_path)?;
        let record = match wavecrate::sample_sources::library::harvest_file(&identity.key) {
            Ok(record) => record,
            Err(error) => {
                tracing::warn!(
                    path = %selected_path.display(),
                    "failed to load selected harvest file record: {error}"
                );
                None
            }
        };
        let parents =
            match wavecrate::sample_sources::library::harvest_parents_for_child(&identity.key) {
                Ok(parents) => parents,
                Err(error) => {
                    tracing::warn!(
                        path = %selected_path.display(),
                        "failed to load selected harvest origin records: {error}"
                    );
                    Vec::new()
                }
            };
        let derivatives =
            match wavecrate::sample_sources::library::harvest_derivations_for_parent(&identity.key)
            {
                Ok(derivatives) => derivatives,
                Err(error) => {
                    tracing::warn!(
                        path = %selected_path.display(),
                        "failed to load selected harvest derivative records: {error}"
                    );
                    Vec::new()
                }
            };
        if record.is_none()
            && parents.is_empty()
            && derivatives.is_empty()
            && self.library.folder_browser.harvest_filter().is_none()
        {
            return None;
        }

        let origin_paths = parents
            .iter()
            .map(|edge| self.harvest_path_for_key(&edge.parent.key))
            .collect::<Vec<_>>();
        let derivative_paths = derivatives
            .iter()
            .map(|edge| self.harvest_path_for_key(&edge.child.key))
            .collect::<Vec<_>>();
        Some(HarvestFamilySummary {
            state_label: harvest_state_display_label(
                record
                    .as_ref()
                    .map(|record| record.state)
                    .unwrap_or(HarvestState::New),
            )
            .to_owned(),
            origin_count: parents.len(),
            derivative_count: derivatives.len(),
            missing_origin_count: missing_related_harvest_paths(&origin_paths),
            missing_derivative_count: missing_related_harvest_paths(&derivative_paths),
            first_origin_label: origin_paths
                .iter()
                .find_map(|path| path.as_ref().map(|path| harvest_family_path_label(path))),
            first_derivative_label: derivative_paths
                .iter()
                .find_map(|path| path.as_ref().map(|path| harvest_family_path_label(path))),
        })
    }

    pub(in crate::native_app) fn schedule_harvest_seen_for_path(
        &self,
        path: &Path,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let Some(request) = self.harvest_seen_persist_request_for_path(path) else {
            return;
        };
        context
            .business()
            .priority(
                "gui-harvest-seen-persist",
                radiant::prelude::TaskPriority::Idle,
            )
            .run(
                move |_| persist_harvest_seen(request),
                GuiMessage::HarvestSeenPersisted,
            );
    }

    pub(in crate::native_app) fn finish_harvest_seen_persist(
        &mut self,
        result: HarvestSeenPersistResult,
    ) {
        if let Err(error) = result.result {
            tracing::warn!(
                file_id = %result.file_id,
                "failed to mark harvest file as seen in background: {error}"
            );
        }
    }

    pub(in crate::native_app) fn selected_harvest_family_available(&self) -> bool {
        self.library
            .folder_browser
            .selected_file_id()
            .map(PathBuf::from)
            .and_then(|path| self.harvest_key_for_path(&path))
            .is_some()
    }

    pub(in crate::native_app) fn mark_harvest_touched_for_path(&self, path: &Path) {
        let Some(identity) = self.harvest_identity_for_path(path) else {
            return;
        };
        if let Err(error) = wavecrate::sample_sources::library::mark_harvest_touched(&identity) {
            tracing::warn!(path = %path.display(), "failed to mark harvest file as touched: {error}");
        }
    }

    pub(in crate::native_app) fn mark_harvest_touched_for_paths<I>(&self, paths: I)
    where
        I: IntoIterator,
        I::Item: AsRef<Path>,
    {
        for path in paths {
            self.mark_harvest_touched_for_path(path.as_ref());
        }
    }

    pub(in crate::native_app) fn record_harvest_discovered_for_paths<I>(&self, paths: I)
    where
        I: IntoIterator,
        I::Item: AsRef<Path>,
    {
        for path in paths {
            self.record_harvest_discovered_for_path(path.as_ref());
        }
    }

    pub(in crate::native_app) fn record_harvest_extraction_with_source_duration(
        &self,
        source_path: &Path,
        selection: SelectionRange,
        child_path: &Path,
        source_duration_seconds: f64,
    ) {
        self.record_harvest_selection_derivation_with_source_duration(
            source_path,
            selection,
            child_path,
            source_duration_seconds,
            HarvestDerivationOperation::Extract,
        );
    }

    pub(in crate::native_app) fn record_harvest_selection_derivation_with_source_duration(
        &self,
        source_path: &Path,
        selection: SelectionRange,
        child_path: &Path,
        source_duration_seconds: f64,
        operation: HarvestDerivationOperation,
    ) {
        let Some(parent) = self.harvest_identity_for_path(source_path) else {
            return;
        };
        let Some(child) = self.harvest_identity_for_path(child_path) else {
            return;
        };
        let duration = source_duration_seconds.max(0.0);
        let source_range = HarvestSourceRange {
            start_seconds: selection.start() as f64 * duration,
            end_seconds: selection.end() as f64 * duration,
        };
        let edge = NewHarvestDerivation {
            parent,
            child,
            operation,
            source_range: Some(source_range),
            output_duration_seconds: Some(
                (source_range.end_seconds - source_range.start_seconds).max(0.0),
            ),
            destination_folder: child_path.parent().map(Path::to_path_buf),
            inherited_metadata: self.harvest_metadata_snapshot_for_path(source_path),
            tool_version: format!("wavecrate-{}", env!("CARGO_PKG_VERSION")),
        };
        if let Err(error) = wavecrate::sample_sources::library::record_harvest_derivation(&edge) {
            tracing::warn!(
                source = %source_path.display(),
                child = %child_path.display(),
                "failed to record harvest derivation: {error}"
            );
        }
    }

    pub(in crate::native_app) fn record_harvest_whole_file_derivation(
        &self,
        source_path: &Path,
        child_path: &Path,
        operation: HarvestDerivationOperation,
    ) {
        let Some(parent) = self.harvest_identity_for_path(source_path) else {
            return;
        };
        let Some(child) = self.harvest_identity_for_path(child_path) else {
            return;
        };
        let edge = NewHarvestDerivation {
            parent,
            child,
            operation,
            source_range: None,
            output_duration_seconds: None,
            destination_folder: child_path.parent().map(Path::to_path_buf),
            inherited_metadata: self.harvest_metadata_snapshot_for_path(source_path),
            tool_version: format!("wavecrate-{}", env!("CARGO_PKG_VERSION")),
        };
        if let Err(error) = wavecrate::sample_sources::library::record_harvest_derivation(&edge) {
            tracing::warn!(
                source = %source_path.display(),
                child = %child_path.display(),
                "failed to record harvest whole-file derivation: {error}"
            );
        }
    }

    pub(in crate::native_app) fn reconcile_harvest_graph_after_folder_move(
        &self,
        request: &FolderMoveRequest,
        moved_paths: &[(PathBuf, PathBuf)],
    ) {
        match request {
            FolderMoveRequest::Folder {
                source_root, moves, ..
            } => {
                for (old_prefix, new_prefix) in moves {
                    self.remap_harvest_file_prefix_for_folder_move(
                        source_root,
                        old_prefix,
                        new_prefix,
                    );
                }
            }
            FolderMoveRequest::Files { .. } => {
                for (old_path, new_path) in moved_paths {
                    self.remap_harvest_file_key_for_move(old_path, new_path);
                }
            }
            FolderMoveRequest::SourcedFiles { file_moves, .. } => {
                for (old_path, new_path) in moved_paths {
                    let Some(file_move) = file_moves
                        .iter()
                        .find(|file_move| Path::new(&file_move.file_id) == old_path.as_path())
                    else {
                        continue;
                    };
                    if file_move.copy_only {
                        self.record_harvest_copy_derivation(old_path, new_path);
                    } else {
                        self.remap_harvest_file_key_for_move(old_path, new_path);
                    }
                }
            }
            FolderMoveRequest::ExtractedFile { .. } => {}
        }
    }

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

    pub(in crate::native_app) fn optional_harvest_destination_for_protected_origin(
        &self,
        source_path: &Path,
    ) -> Option<PathBuf> {
        let (origin_source, _) = self
            .library
            .folder_browser
            .sample_source_for_file_path(source_path)?;
        if !origin_source.is_protected() {
            return None;
        }
        let primary_source = self.library.folder_browser.primary_sample_source()?;
        Some(
            primary_source
                .root
                .join(HARVEST_ROOT_FOLDER)
                .join(harvest_source_folder_name(&origin_source)),
        )
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

    pub(in crate::native_app) fn mark_context_sample_harvest_done(&mut self) {
        self.set_context_sample_harvest_state(HarvestState::Done, "Marked harvest done", "done");
    }

    pub(in crate::native_app) fn mark_context_sample_harvest_ignored(&mut self) {
        self.set_context_sample_harvest_state(
            HarvestState::Ignored,
            "Ignored in harvest",
            "ignored",
        );
    }

    pub(in crate::native_app) fn reset_context_sample_harvest(&mut self) {
        self.set_context_sample_harvest_state(HarvestState::New, "Reset harvest state", "reset");
    }

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

    fn harvest_identity_for_path(&self, path: &Path) -> Option<HarvestFileIdentity> {
        let (source, relative_path) = self
            .library
            .folder_browser
            .sample_source_for_file_path(path)?;
        let (file_size, modified_ns) = file_identity_metadata(path);
        let entry = source_db_entry_for_path(&source, &relative_path);
        Some(HarvestFileIdentity {
            key: HarvestFileKey::new(source.id, relative_path),
            file_size: file_size.or_else(|| entry.as_ref().map(|entry| entry.file_size)),
            modified_ns: modified_ns.or_else(|| entry.as_ref().map(|entry| entry.modified_ns)),
            content_hash: entry.and_then(|entry| entry.content_hash),
        })
    }

    fn harvest_key_for_path(&self, path: &Path) -> Option<HarvestFileKey> {
        let (source, relative_path) = self
            .library
            .folder_browser
            .sample_source_for_file_path(path)?;
        Some(HarvestFileKey::new(source.id, relative_path))
    }

    fn harvest_seen_persist_request_for_path(
        &self,
        path: &Path,
    ) -> Option<HarvestSeenPersistRequest> {
        let (source, relative_path) = self
            .library
            .folder_browser
            .sample_source_for_file_path(path)?;
        let source_database_root = source.database_root().ok()?;
        Some(HarvestSeenPersistRequest {
            file_id: path.display().to_string(),
            source_id: source.id,
            source_root: source.root,
            source_database_root,
            relative_path,
        })
    }

    fn record_harvest_discovered_for_path(&self, path: &Path) {
        let Some(identity) = self.harvest_identity_for_path(path) else {
            return;
        };
        if let Err(error) = wavecrate::sample_sources::library::upsert_harvest_file(&identity) {
            tracing::warn!(path = %path.display(), "failed to record discovered harvest file: {error}");
        }
    }

    fn harvest_metadata_snapshot_for_path(&self, path: &Path) -> HarvestMetadataSnapshot {
        let file_id = path.to_string_lossy().to_string();
        let tags = self
            .metadata
            .tags_by_file
            .get(&file_id)
            .cloned()
            .unwrap_or_default();
        let playback_type = tags
            .iter()
            .find(|tag| tagged_playback_mode_for_tag(tag).is_some())
            .cloned();
        let rating = self
            .library
            .folder_browser
            .sample_source_for_file_path(path)
            .and_then(|(source, relative)| source_db_entry_for_path(&source, &relative))
            .map(|entry| entry.tag.as_i64());
        HarvestMetadataSnapshot {
            rating,
            tags,
            playback_type,
        }
    }

    fn harvest_destination_for_source(&self, source: &SampleSource) -> Result<PathBuf, String> {
        let Some(primary_source) = self.library.folder_browser.primary_sample_source() else {
            return Err(String::from(
                "Set a Primary source before using a harvest destination",
            ));
        };
        Ok(primary_source
            .root
            .join(HARVEST_ROOT_FOLDER)
            .join(harvest_source_folder_name(source)))
    }

    fn record_harvest_copy_derivation(&self, source_path: &Path, child_path: &Path) {
        let Some((source, _)) = self
            .library
            .folder_browser
            .sample_source_for_file_path(source_path)
        else {
            return;
        };
        let Some((child_source, _)) = self
            .library
            .folder_browser
            .sample_source_for_file_path(child_path)
        else {
            return;
        };
        if !source.is_protected() {
            return;
        }
        let Some(parent) = self.harvest_identity_for_path(source_path) else {
            return;
        };
        let Some(child) = self.harvest_identity_for_path(child_path) else {
            return;
        };
        let operation = if child_source.is_primary() {
            HarvestDerivationOperation::CopyToPrimary
        } else {
            HarvestDerivationOperation::Copy
        };
        let edge = NewHarvestDerivation {
            parent,
            child,
            operation,
            source_range: None,
            output_duration_seconds: None,
            destination_folder: child_path.parent().map(Path::to_path_buf),
            inherited_metadata: self.harvest_metadata_snapshot_for_path(source_path),
            tool_version: format!("wavecrate-{}", env!("CARGO_PKG_VERSION")),
        };
        if let Err(error) = wavecrate::sample_sources::library::record_harvest_derivation(&edge) {
            tracing::warn!(
                source = %source_path.display(),
                child = %child_path.display(),
                "failed to record harvest copy derivation: {error}"
            );
        }
    }

    fn remap_harvest_file_key_for_move(&self, old_path: &Path, new_path: &Path) {
        let Some(old_key) = self.harvest_key_for_path(old_path) else {
            return;
        };
        let Some(new_key) = self.harvest_key_for_path(new_path) else {
            return;
        };
        if let Err(error) =
            wavecrate::sample_sources::library::remap_harvest_file_key(&old_key, &new_key)
        {
            tracing::warn!(
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                "failed to remap harvest file identity after move: {error}"
            );
        }
    }

    fn remap_harvest_file_prefix_for_folder_move(
        &self,
        source_root: &Path,
        old_prefix: &Path,
        new_prefix: &Path,
    ) {
        let old_path = source_root.join(old_prefix);
        let new_path = source_root.join(new_prefix);
        let Some(old_key) = self.harvest_key_for_path(&old_path) else {
            return;
        };
        let Some(new_key) = self.harvest_key_for_path(&new_path) else {
            return;
        };
        if old_key.source_id != new_key.source_id {
            tracing::warn!(
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                "skipped harvest folder remap across source boundary"
            );
            return;
        }
        if let Err(error) = wavecrate::sample_sources::library::remap_harvest_file_prefix(
            &old_key.source_id,
            &old_key.relative_path,
            &new_key.relative_path,
        ) {
            tracing::warn!(
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                "failed to remap harvest folder identity after move: {error}"
            );
        }
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

    fn set_context_sample_harvest_state(
        &mut self,
        state: HarvestState,
        status_prefix: &'static str,
        outcome: &'static str,
    ) {
        let started_at = std::time::Instant::now();
        let Some((path, identity)) = self
            .take_context_sample_harvest_target("browser.context_menu.harvest.state", started_at)
        else {
            return;
        };
        if let Err(error) = wavecrate::sample_sources::library::upsert_harvest_file(&identity) {
            self.ui.status.sample = format!("Update harvest state failed: {error}");
            emit_gui_action(
                "browser.context_menu.harvest.state",
                Some("browser"),
                Some(context_menu::target_label(&path).as_str()),
                "error",
                started_at,
                Some(&error.to_string()),
            );
            return;
        }
        match wavecrate::sample_sources::library::set_harvest_state(&identity.key, state) {
            Ok(_) => {
                self.library
                    .folder_browser
                    .refresh_after_harvest_state_change();
                self.ui.status.sample = format!("{status_prefix} {}", sample_path_label(&path));
                emit_gui_action(
                    "browser.context_menu.harvest.state",
                    Some("browser"),
                    Some(context_menu::target_label(&path).as_str()),
                    outcome,
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = format!("Update harvest state failed: {error}");
                emit_gui_action(
                    "browser.context_menu.harvest.state",
                    Some("browser"),
                    Some(context_menu::target_label(&path).as_str()),
                    "error",
                    started_at,
                    Some(&error.to_string()),
                );
            }
        }
    }

    fn take_context_sample_harvest_target(
        &mut self,
        action: &'static str,
        started_at: std::time::Instant,
    ) -> Option<(PathBuf, HarvestFileIdentity)> {
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

    fn selected_sample_harvest_target(
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

    fn harvest_path_for_key(&self, key: &HarvestFileKey) -> Option<PathBuf> {
        self.library
            .folder_browser
            .source_root_path(key.source_id.as_str())
            .map(|root| root.join(&key.relative_path))
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

fn missing_related_harvest_paths(paths: &[Option<PathBuf>]) -> usize {
    paths
        .iter()
        .filter(|path| {
            path.as_ref()
                .is_none_or(|path| !wavecrate::sample_sources::harvest_file_ops::path_exists(path))
        })
        .count()
}

fn harvest_state_display_label(state: HarvestState) -> &'static str {
    match state {
        HarvestState::New => "New",
        HarvestState::Seen => "Seen",
        HarvestState::Touched => "Touched",
        HarvestState::Done => "Done",
        HarvestState::Ignored => "Ignored",
    }
}

fn harvest_family_path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn source_db_entry_for_path(source: &SampleSource, relative_path: &Path) -> Option<WavEntry> {
    source
        .open_db()
        .ok()
        .and_then(|db| db.entry_for_path(relative_path).ok().flatten())
}

fn persist_harvest_seen(request: HarvestSeenPersistRequest) -> HarvestSeenPersistResult {
    let result = persist_harvest_seen_inner(&request);
    HarvestSeenPersistResult {
        file_id: request.file_id,
        result,
    }
}

fn persist_harvest_seen_inner(request: &HarvestSeenPersistRequest) -> Result<(), String> {
    let path = request.source_root.join(&request.relative_path);
    let (file_size, modified_ns) = file_identity_metadata(&path);
    let entry = SourceDatabase::open_read_only_with_database_root(
        &request.source_root,
        &request.source_database_root,
    )
    .ok()
    .and_then(|db| db.entry_for_path(&request.relative_path).ok().flatten());
    let identity = HarvestFileIdentity {
        key: HarvestFileKey::new(request.source_id.clone(), request.relative_path.clone()),
        file_size: file_size.or_else(|| entry.as_ref().map(|entry| entry.file_size)),
        modified_ns: modified_ns.or_else(|| entry.as_ref().map(|entry| entry.modified_ns)),
        content_hash: entry.and_then(|entry| entry.content_hash),
    };
    wavecrate::sample_sources::library::mark_harvest_seen(&identity)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn file_identity_metadata(path: &Path) -> (Option<u64>, Option<i64>) {
    wavecrate::sample_sources::harvest_file_ops::file_identity_metadata(path)
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
