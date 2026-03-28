//! Controller-side projection revision bus synchronization.

use super::AppController;
use crate::app::controller::state::runtime::ProjectionRevisionDirtyMask;
use crate::app::state::UiProjectionRevisions;
use std::path::PathBuf;

impl AppController {
    /// Mark one or more projection revision groups dirty for frame-time draining.
    pub(crate) fn mark_projection_revision_dirty(&mut self, bits: u16) {
        self.runtime.projection_revision_dirty.0 |= bits;
    }

    /// Mark status projection revisions dirty.
    pub(crate) fn mark_status_projection_revision_dirty(&mut self) {
        self.mark_projection_revision_dirty(ProjectionRevisionDirtyMask::STATUS);
    }

    /// Mark folder-search projection revisions dirty.
    pub(crate) fn mark_folder_search_projection_revision_dirty(&mut self) {
        self.mark_projection_revision_dirty(ProjectionRevisionDirtyMask::FOLDER_SEARCH);
    }

    /// Mark browser-search projection revisions dirty.
    pub(crate) fn mark_browser_search_projection_revision_dirty(&mut self) {
        self.mark_projection_revision_dirty(ProjectionRevisionDirtyMask::BROWSER_SEARCH);
    }

    /// Mark browser-row inline metadata projection revisions dirty.
    pub(crate) fn mark_browser_row_metadata_projection_revision_dirty(&mut self) {
        self.mark_projection_revision_dirty(ProjectionRevisionDirtyMask::BROWSER_ROW_METADATA);
    }

    /// Mark map-selection projection revisions dirty.
    pub(crate) fn mark_map_selection_projection_revision_dirty(&mut self) {
        self.mark_projection_revision_dirty(ProjectionRevisionDirtyMask::MAP_SELECTION);
    }

    /// Mark map-hover projection revisions dirty.
    pub(crate) fn mark_map_hover_projection_revision_dirty(&mut self) {
        self.mark_projection_revision_dirty(ProjectionRevisionDirtyMask::MAP_HOVER);
    }

    /// Mark map-dataset projection revisions dirty.
    pub(crate) fn mark_map_dataset_projection_revision_dirty(&mut self) {
        self.mark_projection_revision_dirty(ProjectionRevisionDirtyMask::MAP_DATASET);
    }

    /// Mark map-query projection revisions dirty.
    pub(crate) fn mark_map_query_projection_revision_dirty(&mut self) {
        self.mark_projection_revision_dirty(ProjectionRevisionDirtyMask::MAP_QUERY);
    }

    /// Mark update projection revisions dirty.
    pub(crate) fn mark_update_projection_revision_dirty(&mut self) {
        self.mark_projection_revision_dirty(ProjectionRevisionDirtyMask::UPDATE);
    }

    /// Set the UI-loaded wav path and mark revisions when it changes.
    pub(crate) fn set_ui_loaded_wav(&mut self, loaded_wav: Option<PathBuf>) {
        if self.ui.loaded_wav == loaded_wav {
            return;
        }
        self.ui.loaded_wav = loaded_wav;
        self.mark_projection_revision_dirty(ProjectionRevisionDirtyMask::LOADED_WAV);
    }

    /// Set the folder-search query and mark revisions when it changes.
    pub(crate) fn set_ui_folder_search_query(&mut self, query: String) {
        if self.ui.sources.folders.search_query == query {
            return;
        }
        self.ui.sources.folders.search_query = query;
        self.mark_folder_search_projection_revision_dirty();
    }

    /// Refresh canonical projection revisions from current UI state snapshots.
    ///
    /// This centralizes revision bumps so native projection keys depend on
    /// scalar revisions instead of container hashing.
    pub(crate) fn refresh_projection_revision_bus(&mut self) -> bool {
        let dirty = self.runtime.projection_revision_dirty.0;
        if dirty == ProjectionRevisionDirtyMask::NONE {
            return false;
        }
        self.runtime.projection_revision_dirty.0 = ProjectionRevisionDirtyMask::NONE;
        let revisions = &mut self.ui.projection_revisions;
        if (dirty & ProjectionRevisionDirtyMask::STATUS) != 0 {
            UiProjectionRevisions::bump(&mut revisions.status);
        }
        if (dirty & ProjectionRevisionDirtyMask::FOLDER_SEARCH) != 0 {
            UiProjectionRevisions::bump(&mut revisions.folder_search);
        }
        if (dirty & ProjectionRevisionDirtyMask::BROWSER_SEARCH) != 0 {
            UiProjectionRevisions::bump(&mut revisions.browser_search);
        }
        if (dirty & ProjectionRevisionDirtyMask::BROWSER_ROW_METADATA) != 0 {
            UiProjectionRevisions::bump(&mut revisions.browser_row_metadata);
        }
        if (dirty & ProjectionRevisionDirtyMask::MAP_SELECTION) != 0 {
            UiProjectionRevisions::bump(&mut revisions.map_selection);
        }
        if (dirty & ProjectionRevisionDirtyMask::MAP_HOVER) != 0 {
            UiProjectionRevisions::bump(&mut revisions.map_hover);
        }
        if (dirty & ProjectionRevisionDirtyMask::MAP_DATASET) != 0 {
            UiProjectionRevisions::bump(&mut revisions.map_dataset);
        }
        if (dirty & ProjectionRevisionDirtyMask::MAP_QUERY) != 0 {
            UiProjectionRevisions::bump(&mut revisions.map_query);
        }
        if (dirty & ProjectionRevisionDirtyMask::UPDATE) != 0 {
            UiProjectionRevisions::bump(&mut revisions.update);
        }
        if (dirty & ProjectionRevisionDirtyMask::LOADED_WAV) != 0 {
            UiProjectionRevisions::bump(&mut revisions.loaded_wav);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::AppController;
    use crate::waveform::WaveformRenderer;

    #[test]
    /// Refresh should remain a no-op when no mutation marked revision bits dirty.
    fn revision_bus_noop_sync_without_dirty_mask_is_stable() {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

        let changed = controller.refresh_projection_revision_bus();
        let first = controller.ui.projection_revisions;

        assert!(!changed);
        assert_eq!(controller.ui.projection_revisions, first);
    }

    #[test]
    fn revision_bus_bumps_search_revisions_when_queries_change() {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let before = controller.ui.projection_revisions;

        controller.set_browser_search("kick");
        controller.set_folder_search(String::from("drums"));
        let _ = controller.refresh_projection_revision_bus();

        assert_eq!(
            controller.ui.projection_revisions.browser_search,
            before.browser_search.wrapping_add(1)
        );
        assert_eq!(
            controller.ui.projection_revisions.folder_search,
            before.folder_search.wrapping_add(1)
        );
    }

    #[test]
    /// Browser-row metadata revision should bump independently from browser search state.
    fn revision_bus_bumps_browser_row_metadata_without_touching_search_revision() {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let before = controller.ui.projection_revisions;

        controller.mark_browser_row_metadata_projection_revision_dirty();
        let changed = controller.refresh_projection_revision_bus();

        assert!(changed);
        assert_eq!(
            controller.ui.projection_revisions.browser_row_metadata,
            before.browser_row_metadata.wrapping_add(1)
        );
        assert_eq!(
            controller.ui.projection_revisions.browser_search,
            before.browser_search
        );
    }

    #[test]
    /// Loaded-wav revision should bump once per distinct loaded path transition.
    fn revision_bus_bumps_loaded_wav_only_when_path_changes() {
        use std::path::PathBuf;

        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let before = controller.ui.projection_revisions;

        controller.set_ui_loaded_wav(Some(PathBuf::from("kick.wav")));
        let changed = controller.refresh_projection_revision_bus();

        assert!(changed);
        assert_eq!(
            controller.ui.projection_revisions.loaded_wav,
            before.loaded_wav.wrapping_add(1)
        );

        let stable = controller.refresh_projection_revision_bus();
        assert!(!stable);
    }
}
