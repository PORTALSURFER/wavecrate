use crate::app::controller::AppController;
use crate::app::controller::jobs;
use crate::app::controller::ui::status_message::StatusMessage;
use crate::sample_sources::SampleSource;
use crate::sample_sources::scanner::ScanMode;

impl AppController {
    pub(crate) fn show_similarity_prep_start(&mut self, source: &SampleSource, skip_scan: bool) {
        self.runtime.similarity_prep_last_error = None;
        self.set_status_message(StatusMessage::PreparingSimilarity {
            source: source.root.display().to_string(),
        });
        if skip_scan {
            self.show_similarity_prep_progress(0, false);
            self.set_similarity_analysis_detail();
            return;
        }
        self.ensure_scan_progress_for_source(ScanMode::Hard, source);
    }

    pub(crate) fn show_similarity_prep_finalizing(&mut self) {
        self.set_status_message(StatusMessage::FinalizingSimilarityPrep);
        self.show_similarity_finalize_progress();
        self.set_similarity_finalize_detail();
    }

    pub(crate) fn show_similarity_prep_ready(&mut self, outcome: &jobs::SimilarityPrepOutcome) {
        self.runtime.similarity_prep_last_error = None;
        self.ui.map.bounds = None;
        self.ui.map.cached_bounds_source_id = None;
        self.ui.map.cached_bounds_umap_version = None;
        self.ui.map.last_query = None;
        self.ui.map.cached_points.clear();
        self.ui.map.cached_points_source_id = None;
        self.ui.map.cached_points_umap_version = None;
        self.ui.map.cached_cluster_centroids_key = None;
        self.ui.map.cached_cluster_centroids = None;
        self.ui.map.auto_cluster_build_requested_key = None;
        self.ui.map.outdated = false;
        self.mark_map_dataset_projection_revision_dirty();
        self.mark_map_query_projection_revision_dirty();
        self.set_status_message(StatusMessage::SimilarityReady {
            cluster_count: outcome.cluster_stats.cluster_count,
            noise_ratio: outcome.cluster_stats.noise_ratio,
        });
    }

    pub(crate) fn show_similarity_prep_failed(&mut self, err: String) {
        self.runtime.similarity_prep_last_error = Some(err.clone());
        self.set_status_message(StatusMessage::SimilarityPrepFailed { err });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::WaveformRenderer;
    use crate::app::state::ProgressTaskKind;
    use tempfile::tempdir;

    #[test]
    fn add_source_starts_footer_scan_progress_for_similarity_prep() {
        let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
        let dir = tempdir().expect("tempdir");
        let source_root = dir.path().join("source");
        std::fs::create_dir_all(&source_root).expect("create source root");

        controller
            .add_source_from_path(source_root.clone())
            .expect("add source");

        assert!(controller.ui.progress.visible);
        assert!(!controller.ui.progress.modal);
        assert_eq!(controller.ui.progress.task, Some(ProgressTaskKind::Scan));
        assert_eq!(controller.ui.progress.title, "Scanning source");
        assert_eq!(
            controller.ui.progress.detail.as_deref(),
            Some(format!("Hard sync • {}", source_root.display()).as_str())
        );
    }
}
