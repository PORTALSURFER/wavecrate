//! AppController-facing map-view actions and query entrypoints.

use crate::app::controller::library::analysis_jobs;
use crate::app::state::SampleBrowserTab;
use std::collections::HashMap;

use super::connections::open_cached_source_db;
use super::repository::{
    load_umap_bounds, load_umap_cluster_centroids, load_umap_point_for_sample, load_umap_points,
};
use super::*;

impl AppController {
    /// Switch between browser list/map tabs and keep map visibility in sync.
    pub fn set_browser_tab(&mut self, map: bool) {
        self.ui.browser.active_tab = if map {
            SampleBrowserTab::Map
        } else {
            SampleBrowserTab::List
        };
        self.ui.map.open = map;
    }

    /// Stage map focus/hover ids before resolving sample focus and preview.
    pub fn stage_map_sample_focus(&mut self, sample_id: &str) {
        let sample_id = sample_id.to_string();
        let selection_changed = self.ui.map.selected_sample_id.as_ref() != Some(&sample_id);
        let hover_changed = self.ui.map.hovered_sample_id.as_ref() != Some(&sample_id);
        self.ui.map.selected_sample_id = Some(sample_id.clone());
        self.ui.map.hovered_sample_id = Some(sample_id.clone());
        self.ui.map.paint_hover_active_id = Some(sample_id);
        if selection_changed {
            self.mark_map_selection_projection_revision_dirty();
        }
        if hover_changed {
            self.mark_map_hover_projection_revision_dirty();
        }
    }

    /// Focus a map sample, stage hover/selection ids, queue load preview, and start playback.
    pub fn focus_map_sample_and_preview(&mut self, sample_id: &str) {
        self.set_browser_tab(true);
        self.stage_map_sample_focus(sample_id);
        if let Err(err) = self.focus_sample_from_map(sample_id) {
            self.set_error_status(format!("Map focus failed: {err}"));
            return;
        }
        if let Err(err) = self.play_audio(false, None) {
            self.set_error_status(format!("Playback failed: {err}"));
        }
    }

    /// Open the map view panel.
    pub fn open_map(&mut self) {
        self.ui.map.open = true;
    }

    pub(crate) fn umap_bounds(
        &mut self,
        model_id: &str,
        umap_version: &str,
        source_id: Option<&SourceId>,
    ) -> Result<Option<UmapBounds>, String> {
        let conn = open_cached_source_db(self, source_id)?;
        load_umap_bounds(conn, model_id, umap_version, source_id)
    }

    pub(crate) fn umap_points_in_bounds(
        &mut self,
        query: UmapPointQuery<'_>,
    ) -> Result<Vec<UmapPoint>, String> {
        let conn = open_cached_source_db(self, query.source_id)?;
        load_umap_points(conn, &query)
    }

    /// Lookup one similarity-map point for a specific sample id.
    pub fn umap_point_for_sample(
        &mut self,
        model_id: &str,
        umap_version: &str,
        sample_id: &str,
    ) -> Result<Option<(f32, f32)>, String> {
        let (source_id, _relative) = analysis_jobs::parse_sample_id(sample_id)?;
        let source_id = SourceId::from_string(source_id);
        let conn = open_cached_source_db(self, Some(&source_id))?;
        load_umap_point_for_sample(conn, model_id, umap_version, sample_id)
    }

    /// Load cluster centroids for the requested similarity-map layout.
    pub fn umap_cluster_centroids(
        &mut self,
        model_id: &str,
        umap_version: &str,
        cluster_method: &str,
        cluster_umap_version: &str,
        source_id: Option<&SourceId>,
    ) -> Result<HashMap<i32, crate::app::state::MapClusterCentroid>, String> {
        let conn = open_cached_source_db(self, source_id)?;
        load_umap_cluster_centroids(
            conn,
            model_id,
            umap_version,
            cluster_method,
            cluster_umap_version,
            source_id,
        )
    }
}
