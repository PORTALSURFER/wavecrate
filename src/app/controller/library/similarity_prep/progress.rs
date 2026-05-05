use super::{DbSimilarityPrepStore, SimilarityPrepStore, state::SimilarityPrepStage};
use crate::app::controller::{AppController, StatusTone};
use crate::app::state::AnalysisProgressSnapshot;
use crate::sample_sources::scanner::ScanMode;
use tracing::info;

impl AppController {
    pub(crate) fn refresh_similarity_prep_progress(&mut self) {
        let store = DbSimilarityPrepStore;
        self.refresh_similarity_prep_progress_with_store(&store);
    }

    fn refresh_similarity_prep_progress_with_store(&mut self, store: &impl SimilarityPrepStore) {
        let Some(state) = self.runtime.similarity_prep.as_ref() else {
            return;
        };
        info!(
            "Similarity prep progress refresh (stage={:?}, source_id={})",
            state.stage,
            state.source_id.as_str()
        );
        match state.stage {
            SimilarityPrepStage::AwaitScan => {
                if self.runtime.jobs.scan_in_progress() {
                    info!(
                        "Similarity prep waiting for scan (source_id={})",
                        state.source_id.as_str()
                    );
                    if let Some(source) = self.find_source_by_id(&state.source_id) {
                        self.ensure_scan_progress_for_source(ScanMode::Hard, &source);
                    }
                    return;
                }
                info!(
                    "Similarity prep requesting hard sync (source_id={})",
                    state.source_id.as_str()
                );
                self.request_hard_sync();
            }
            SimilarityPrepStage::AwaitEmbeddings => {
                let Some(source) = self.find_source_by_id(&state.source_id) else {
                    info!(
                        "Similarity prep missing source (source_id={})",
                        state.source_id.as_str()
                    );
                    return;
                };
                let progress = match store.current_analysis_progress(&source) {
                    Ok(progress) => progress,
                    Err(_) => {
                        info!(
                            "Similarity prep analysis progress unavailable (source_id={})",
                            source.id.as_str()
                        );
                        if !self.ui.progress.visible {
                            self.show_similarity_prep_progress(0, false);
                            self.set_similarity_analysis_detail();
                        }
                        return;
                    }
                };
                info!(
                    "Similarity prep analysis progress (pending={}, running={}, failed={}, completed={}, total={}, source_id={})",
                    progress.pending,
                    progress.running,
                    progress.failed,
                    progress.completed(),
                    progress.total(),
                    source.id.as_str()
                );
                if progress.pending == 0 && progress.running == 0 {
                    let embed_progress = match store.current_embedding_backfill_progress(&source) {
                        Ok(progress) => progress,
                        Err(err) => {
                            info!(
                                "Similarity prep embedding progress unavailable (source_id={}, error={})",
                                source.id.as_str(),
                                err
                            );
                            self.ensure_similarity_prep_progress(0, true);
                            self.set_similarity_embedding_detail();
                            self.ui.progress.set_task_detail(
                                crate::app::state::ProgressTaskKind::Analysis,
                                Some(format!("Embedding progress unavailable: {err}")),
                            );
                            self.ui.progress.set_task_analysis_snapshot(
                                crate::app::state::ProgressTaskKind::Analysis,
                                None,
                            );
                            self.set_status(
                                format!("Similarity prep progress unavailable: {err}"),
                                StatusTone::Warning,
                            );
                            return;
                        }
                    };
                    info!(
                        "Similarity prep embedding progress (pending={}, running={}, failed={}, completed={}, total={}, source_id={})",
                        embed_progress.pending,
                        embed_progress.running,
                        embed_progress.failed,
                        embed_progress.completed(),
                        embed_progress.total(),
                        source.id.as_str()
                    );
                    if embed_progress.pending > 0 || embed_progress.running > 0 {
                        self.ensure_similarity_prep_progress(embed_progress.total(), true);
                        self.set_similarity_embedding_detail();
                        self.ui.progress.set_task_counts(
                            crate::app::state::ProgressTaskKind::Analysis,
                            embed_progress.total(),
                            embed_progress.completed(),
                        );
                        let jobs_completed = embed_progress.completed();
                        let jobs_total = embed_progress.total();
                        let mut detail =
                            format!("Embedding backfill… Jobs {jobs_completed}/{jobs_total}");
                        if embed_progress.failed > 0 {
                            detail.push_str(&format!(" • {} failed", embed_progress.failed));
                        }
                        self.ui.progress.set_task_detail(
                            crate::app::state::ProgressTaskKind::Analysis,
                            Some(detail),
                        );
                        self.ui.progress.set_task_analysis_snapshot(
                            crate::app::state::ProgressTaskKind::Analysis,
                            None,
                        );
                        return;
                    }
                    if !store.source_has_embeddings(&source) {
                        info!(
                            "Similarity prep enqueueing embedding backfill (source_id={})",
                            source.id.as_str()
                        );
                        self.ensure_similarity_prep_progress(0, true);
                        self.set_similarity_embedding_detail();
                        self.enqueue_similarity_backfill(source, false);
                        return;
                    }
                    self.handle_similarity_analysis_progress(&progress);
                    return;
                }
                self.ensure_similarity_prep_progress(progress.total(), true);
                self.ui.progress.set_task_counts(
                    crate::app::state::ProgressTaskKind::Analysis,
                    progress.total(),
                    progress.completed(),
                );
                let jobs_completed = progress.completed();
                let jobs_total = progress.total();
                let samples_completed = progress.samples_completed();
                let samples_total = progress.samples_total;
                let mut detail = format!(
                    "Analyzing audio features… Jobs {jobs_completed}/{jobs_total} • Samples {samples_completed}/{samples_total}"
                );
                if progress.running == 0 && progress.pending > 0 {
                    detail.push_str(" • Waiting for workers");
                }
                if progress.failed > 0 {
                    detail.push_str(&format!(" • {} failed", progress.failed));
                }
                self.ui
                    .progress
                    .set_task_detail(crate::app::state::ProgressTaskKind::Analysis, Some(detail));
                self.ui.progress.set_task_analysis_snapshot(
                    crate::app::state::ProgressTaskKind::Analysis,
                    Some(AnalysisProgressSnapshot {
                    pending: progress.pending,
                    running: progress.running,
                    failed: progress.failed,
                    samples_completed,
                    samples_total,
                    running_jobs: Vec::new(),
                    stale_after_secs: Some(
                        crate::app::controller::library::analysis_jobs::stale_running_job_seconds(),
                    ),
                }),
                );
            }
            SimilarityPrepStage::Finalizing => {
                info!(
                    "Similarity prep finalizing (source_id={})",
                    state.source_id.as_str()
                );
                self.ensure_similarity_finalize_progress();
                self.set_similarity_finalize_detail();
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
        controller.runtime.similarity_prep =
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
                .similarity_prep
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
}
