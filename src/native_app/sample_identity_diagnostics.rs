use std::path::{Path, PathBuf};

use crate::native_app::app::NativeAppState;

impl NativeAppState {
    pub(in crate::native_app) fn log_sample_identity_checkpoint(
        &self,
        event: &'static str,
        trigger: &'static str,
        requested_path: Option<&Path>,
        note: Option<&str>,
    ) {
        let selected_file_ids = self
            .library
            .folder_browser
            .selected_file_ids_for_diagnostics();
        let active_file_ids = self
            .library
            .folder_browser
            .active_file_ids_for_diagnostics();
        let normalizing_paths = sorted_path_strings(
            self.background
                .normalization_active_paths
                .iter()
                .map(|path| path.as_path()),
        );
        let loaded_path = self.waveform.current.path();
        tracing::debug!(
            target: "wavecrate::debug::sample_identity",
            event,
            trigger,
            requested_path = requested_path.map(|path| path.display().to_string()).as_deref(),
            selected_file_id = self.library.folder_browser.selected_file_id(),
            selected_file_ids = ?selected_file_ids,
            active_file_ids = ?active_file_ids,
            selected_file_count = selected_file_ids.len(),
            selected_file_ids_explicit = self
                .library
                .folder_browser
                .selected_file_ids_explicit_for_diagnostics(),
            selected_source_id = self.library.folder_browser.selected_source_id(),
            loaded_waveform_path = %loaded_path.display(),
            loaded_waveform_has_sample = self.waveform.current.has_loaded_sample(),
            load_selection_path = self.waveform.load.selection.selected_path.as_deref(),
            active_sample_load = self.active_sample_load_task().is_some(),
            deferred_sample_load = self.background.deferred_sample_load_task.active().is_some(),
            sample_load_validation = self.background.sample_load_validation_task.active().is_some(),
            pending_sample_playback = ?self.audio.pending_sample_playback,
            normalizing_paths = ?normalizing_paths,
            note,
            "Sample identity checkpoint"
        );
    }

    pub(in crate::native_app) fn log_sample_identity_paths_checkpoint(
        &self,
        event: &'static str,
        trigger: &'static str,
        requested_paths: &[PathBuf],
        note: Option<&str>,
    ) {
        let selected_file_ids = self
            .library
            .folder_browser
            .selected_file_ids_for_diagnostics();
        let active_file_ids = self
            .library
            .folder_browser
            .active_file_ids_for_diagnostics();
        let requested_paths =
            sorted_path_strings(requested_paths.iter().map(|path| path.as_path()));
        let normalizing_paths = sorted_path_strings(
            self.background
                .normalization_active_paths
                .iter()
                .map(|path| path.as_path()),
        );
        let loaded_path = self.waveform.current.path();
        tracing::debug!(
            target: "wavecrate::debug::sample_identity",
            event,
            trigger,
            requested_paths = ?requested_paths,
            selected_file_id = self.library.folder_browser.selected_file_id(),
            selected_file_ids = ?selected_file_ids,
            active_file_ids = ?active_file_ids,
            selected_file_count = selected_file_ids.len(),
            selected_file_ids_explicit = self
                .library
                .folder_browser
                .selected_file_ids_explicit_for_diagnostics(),
            selected_source_id = self.library.folder_browser.selected_source_id(),
            loaded_waveform_path = %loaded_path.display(),
            loaded_waveform_has_sample = self.waveform.current.has_loaded_sample(),
            load_selection_path = self.waveform.load.selection.selected_path.as_deref(),
            active_sample_load = self.active_sample_load_task().is_some(),
            deferred_sample_load = self.background.deferred_sample_load_task.active().is_some(),
            sample_load_validation = self.background.sample_load_validation_task.active().is_some(),
            pending_sample_playback = ?self.audio.pending_sample_playback,
            normalizing_paths = ?normalizing_paths,
            note,
            "Sample identity checkpoint"
        );
    }
}

fn sorted_path_strings<'a>(paths: impl IntoIterator<Item = &'a Path>) -> Vec<String> {
    let mut paths = paths
        .into_iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    paths.sort();
    paths
}
