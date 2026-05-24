use super::{DbSimilarityPrepStore, SimilarityPrepStore, state::SimilarityPrepStage};
use crate::app::controller::library::analysis_jobs::AnalysisProgress;
use crate::app::controller::{AppController, StatusTone};
use crate::app::state::ProgressTaskKind;
use crate::sample_sources::{SampleSource, SourceId, scanner::ScanMode};
use tracing::info;

mod view;

use self::view::{similarity_analysis_progress_detail, similarity_analysis_progress_snapshot};

impl AppController {
    pub(crate) fn refresh_similarity_prep_progress(&mut self) {
        let store = DbSimilarityPrepStore;
        self.refresh_similarity_prep_progress_with_store(&store);
    }

    /// Handles refresh similarity prep progress with store.
    fn refresh_similarity_prep_progress_with_store(&mut self, store: &impl SimilarityPrepStore) {
        let Some(state) = self.runtime.similarity_prep.as_ref() else {
            return;
        };
        let source_id = state.source_id.clone();
        let stage = state.stage;
        info!(
            "Similarity prep progress refresh (stage={:?}, source_id={})",
            stage,
            source_id.as_str()
        );

        match stage {
            SimilarityPrepStage::AwaitScan => {
                self.refresh_similarity_prep_await_scan(&source_id);
            }
            SimilarityPrepStage::AwaitEmbeddings => {
                self.refresh_similarity_prep_await_embeddings(store, &source_id);
            }
            SimilarityPrepStage::Finalizing => {
                self.refresh_similarity_prep_finalizing(&source_id);
            }
        }
    }

    fn refresh_similarity_prep_await_scan(&mut self, source_id: &SourceId) {
        if self.runtime.jobs.scan_in_progress() {
            info!(
                "Similarity prep waiting for scan (source_id={})",
                source_id.as_str()
            );
            if let Some(source) = self.find_source_by_id(source_id) {
                self.ensure_scan_progress_for_source(ScanMode::Hard, &source);
            }
            return;
        }

        info!(
            "Similarity prep requesting hard sync (source_id={})",
            source_id.as_str()
        );
        self.request_hard_sync();
    }

    fn refresh_similarity_prep_await_embeddings(
        &mut self,
        store: &impl SimilarityPrepStore,
        source_id: &SourceId,
    ) {
        let Some(source) = self.find_source_by_id(source_id) else {
            info!(
                "Similarity prep missing source (source_id={})",
                source_id.as_str()
            );
            return;
        };

        let progress = match self.current_similarity_analysis_progress(store, &source) {
            Some(progress) => progress,
            None => return,
        };

        if progress.pending > 0 || progress.running > 0 {
            self.show_similarity_analysis_progress(&progress);
            return;
        }

        self.refresh_similarity_embedding_progress(store, source, progress);
    }

    fn current_similarity_analysis_progress(
        &mut self,
        store: &impl SimilarityPrepStore,
        source: &SampleSource,
    ) -> Option<AnalysisProgress> {
        let progress = match store.current_analysis_progress(source) {
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
                return None;
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
        Some(progress)
    }

    fn refresh_similarity_embedding_progress(
        &mut self,
        store: &impl SimilarityPrepStore,
        source: SampleSource,
        analysis_progress: AnalysisProgress,
    ) {
        let embed_progress = match store.current_embedding_backfill_progress(&source) {
            Ok(progress) => progress,
            Err(err) => {
                self.show_similarity_embedding_progress_error(&source, err);
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
            self.show_similarity_embedding_progress(&embed_progress);
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

        self.handle_similarity_analysis_progress(&analysis_progress);
    }

    fn show_similarity_embedding_progress_error(&mut self, source: &SampleSource, err: String) {
        info!(
            "Similarity prep embedding progress unavailable (source_id={}, error={})",
            source.id.as_str(),
            err
        );
        self.ensure_similarity_prep_progress(0, true);
        self.set_similarity_embedding_detail();
        self.ui.progress.set_task_detail(
            ProgressTaskKind::Analysis,
            Some(format!("Embedding progress unavailable: {err}")),
        );
        self.ui
            .progress
            .set_task_analysis_snapshot(ProgressTaskKind::Analysis, None);
        self.set_status(
            format!("Similarity prep progress unavailable: {err}"),
            StatusTone::Warning,
        );
    }

    fn show_similarity_embedding_progress(&mut self, progress: &AnalysisProgress) {
        self.ensure_similarity_prep_progress(progress.total(), true);
        self.set_similarity_embedding_detail();
        self.ui.progress.set_task_counts(
            ProgressTaskKind::Analysis,
            progress.total(),
            progress.completed(),
        );
        let jobs_completed = progress.completed();
        let jobs_total = progress.total();
        let mut detail = format!("Embedding backfill… Jobs {jobs_completed}/{jobs_total}");
        if progress.failed > 0 {
            detail.push_str(&format!(" • {} failed", progress.failed));
        }
        self.ui
            .progress
            .set_task_detail(ProgressTaskKind::Analysis, Some(detail));
        self.ui
            .progress
            .set_task_analysis_snapshot(ProgressTaskKind::Analysis, None);
    }

    fn show_similarity_analysis_progress(&mut self, progress: &AnalysisProgress) {
        self.ensure_similarity_prep_progress(progress.total(), true);
        self.ui.progress.set_task_counts(
            ProgressTaskKind::Analysis,
            progress.total(),
            progress.completed(),
        );
        self.ui.progress.set_task_detail(
            ProgressTaskKind::Analysis,
            Some(similarity_analysis_progress_detail(progress)),
        );
        self.ui.progress.set_task_analysis_snapshot(
            ProgressTaskKind::Analysis,
            Some(similarity_analysis_progress_snapshot(progress)),
        );
    }

    fn refresh_similarity_prep_finalizing(&mut self, source_id: &SourceId) {
        info!(
            "Similarity prep finalizing (source_id={})",
            source_id.as_str()
        );
        self.ensure_similarity_finalize_progress();
        self.set_similarity_finalize_detail();
    }
}

#[cfg(test)]
mod tests;
