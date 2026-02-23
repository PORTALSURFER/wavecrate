//! Controller-side projection revision bus synchronization.

use super::AppController;
use crate::app::controller::state::runtime::MapQueryBoundsRevisionKey;
use crate::app::state::UiProjectionRevisions;

impl AppController {
    /// Refresh canonical projection revisions from current UI state snapshots.
    ///
    /// This centralizes revision bumps so native projection keys depend on
    /// scalar revisions instead of container hashing.
    pub(crate) fn refresh_projection_revision_bus(&mut self) -> bool {
        let revisions = &mut self.ui.projection_revisions;
        let snapshot = &mut self.runtime.projection_revision_snapshot;
        let mut changed = false;

        if snapshot.status_text != self.ui.status.text
            || snapshot.status_tone != Some(self.ui.status.status_tone)
        {
            UiProjectionRevisions::bump(&mut revisions.status);
            changed = true;
            snapshot.status_text = self.ui.status.text.clone();
            snapshot.status_tone = Some(self.ui.status.status_tone);
        }

        if snapshot.folder_search_query != self.ui.sources.folders.search_query {
            UiProjectionRevisions::bump(&mut revisions.folder_search);
            changed = true;
            snapshot.folder_search_query = self.ui.sources.folders.search_query.clone();
        }

        if snapshot.browser_search_query != self.ui.browser.search_query {
            UiProjectionRevisions::bump(&mut revisions.browser_search);
            changed = true;
            snapshot.browser_search_query = self.ui.browser.search_query.clone();
        }

        if snapshot.map_selected_sample_id != self.ui.map.selected_sample_id {
            UiProjectionRevisions::bump(&mut revisions.map_selection);
            changed = true;
            snapshot.map_selected_sample_id = self.ui.map.selected_sample_id.clone();
        }

        if snapshot.map_hovered_sample_id != self.ui.map.hovered_sample_id {
            UiProjectionRevisions::bump(&mut revisions.map_hover);
            changed = true;
            snapshot.map_hovered_sample_id = self.ui.map.hovered_sample_id.clone();
        }

        let map_dataset_changed = snapshot.map_umap_version != self.ui.map.umap_version
            || snapshot.map_cached_bounds_source_id != self.ui.map.cached_bounds_source_id
            || snapshot.map_cached_bounds_umap_version != self.ui.map.cached_bounds_umap_version
            || snapshot.map_cached_points_source_id != self.ui.map.cached_points_source_id
            || snapshot.map_cached_points_umap_version != self.ui.map.cached_points_umap_version;
        if map_dataset_changed {
            UiProjectionRevisions::bump(&mut revisions.map_dataset);
            changed = true;
            snapshot.map_umap_version = self.ui.map.umap_version.clone();
            snapshot.map_cached_bounds_source_id = self.ui.map.cached_bounds_source_id.clone();
            snapshot.map_cached_bounds_umap_version =
                self.ui.map.cached_bounds_umap_version.clone();
            snapshot.map_cached_points_source_id = self.ui.map.cached_points_source_id.clone();
            snapshot.map_cached_points_umap_version =
                self.ui.map.cached_points_umap_version.clone();
        }

        let map_last_query = self
            .ui
            .map
            .last_query
            .map(MapQueryBoundsRevisionKey::from_bounds);
        if snapshot.map_last_query != map_last_query {
            UiProjectionRevisions::bump(&mut revisions.map_query);
            changed = true;
            snapshot.map_last_query = map_last_query;
        }

        let update_changed = snapshot.update_status != Some(self.ui.update.status.clone())
            || snapshot.update_available_tag != self.ui.update.available_tag
            || snapshot.update_available_url != self.ui.update.available_url
            || snapshot.update_last_error != self.ui.update.last_error;
        if update_changed {
            UiProjectionRevisions::bump(&mut revisions.update);
            changed = true;
            snapshot.update_status = Some(self.ui.update.status.clone());
            snapshot.update_available_tag = self.ui.update.available_tag.clone();
            snapshot.update_available_url = self.ui.update.available_url.clone();
            snapshot.update_last_error = self.ui.update.last_error.clone();
        }

        if snapshot.loaded_wav != self.ui.loaded_wav {
            UiProjectionRevisions::bump(&mut revisions.loaded_wav);
            changed = true;
            snapshot.loaded_wav = self.ui.loaded_wav.clone();
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::AppController;
    use crate::waveform::WaveformRenderer;

    #[test]
    fn revision_bus_noop_sync_is_stable_after_initial_snapshot() {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

        let _ = controller.refresh_projection_revision_bus();
        let first = controller.ui.projection_revisions;
        let _ = controller.refresh_projection_revision_bus();

        assert_eq!(controller.ui.projection_revisions, first);
    }

    #[test]
    fn revision_bus_bumps_search_revisions_when_queries_change() {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

        let _ = controller.refresh_projection_revision_bus();
        let before = controller.ui.projection_revisions;

        controller.ui.browser.search_query = String::from("kick");
        controller.ui.sources.folders.search_query = String::from("drums");
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
}
