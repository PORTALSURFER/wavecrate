use super::*;
use crate::app::controller::WaveformRenderer;
use crate::app::controller::library::analysis_jobs;
use crate::app::controller::library::similarity_prep::state;
use crate::app::state::ProgressTaskKind;
use crate::sample_sources::{SampleSource, SourceId};
use tempfile::tempdir;

struct EmbeddingProgressErrorStore;

impl SimilarityPrepStore for EmbeddingProgressErrorStore {
    fn read_scan_timestamp(&self, _source: &SampleSource) -> Option<i64> {
        Some(10)
    }

    fn read_prep_timestamp(&self, _source: &SampleSource) -> Option<i64> {
        None
    }

    fn source_has_embeddings(&self, _source: &SampleSource) -> bool {
        panic!("embedding coverage must not be checked when progress is unavailable")
    }

    fn source_has_aspect_descriptors(&self, _source: &SampleSource) -> bool {
        panic!("aspect coverage must not be checked when progress is unavailable")
    }

    fn record_prep_scan_timestamp(
        &self,
        _source: &SampleSource,
        _scan_completed_at: i64,
    ) -> Result<(), String> {
        Err("not needed".to_string())
    }

    fn current_analysis_progress(
        &self,
        _source: &SampleSource,
    ) -> Result<analysis_jobs::AnalysisProgress, String> {
        Ok(analysis_jobs::AnalysisProgress::default())
    }

    fn current_embedding_backfill_progress(
        &self,
        _source: &SampleSource,
    ) -> Result<analysis_jobs::AnalysisProgress, String> {
        Err("progress db busy".to_string())
    }

    fn open_source_db_for_similarity(
        &self,
        _source_id: &SourceId,
    ) -> Result<rusqlite::Connection, String> {
        Err("not needed".to_string())
    }

    fn count_umap_layout_rows(
        &self,
        _conn: &rusqlite::Connection,
        _model_id: &str,
        _umap_version: &str,
        _sample_id_prefix: &str,
    ) -> Result<i64, String> {
        Err("not needed".to_string())
    }
}

fn controller_with_similarity_prep_source() -> (AppController, tempfile::TempDir) {
    let renderer = WaveformRenderer::new(10, 10);
    let mut controller = AppController::new(renderer, None);
    let dir = tempdir().unwrap();
    let root = dir.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let source = SampleSource::new(root);
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.library.sources.push(source.clone());
    controller.runtime.similarity.prep =
        Some(state::build_initial_state(state::SimilarityPrepInit {
            source_id: source.id,
            umap_version: "test-umap".to_string(),
            scan_completed_at: Some(10),
            skip_scan: true,
            skip_backfill: true,
            force_full_analysis: false,
        }));
    (controller, dir)
}

#[test]
fn embedding_progress_error_keeps_similarity_prep_waiting() {
    let (mut controller, _dir) = controller_with_similarity_prep_source();

    controller.refresh_similarity_prep_progress_with_store(&EmbeddingProgressErrorStore);

    assert_eq!(
        controller
            .runtime
            .similarity
            .prep
            .as_ref()
            .map(|state| state.stage),
        Some(SimilarityPrepStage::AwaitEmbeddings)
    );
    assert_eq!(
        controller.ui.progress.task,
        Some(ProgressTaskKind::Analysis)
    );
    assert!(
        controller
            .ui
            .progress
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("Embedding progress unavailable"))
    );
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);
    assert!(controller.ui.status.text.contains("progress unavailable"));
}
