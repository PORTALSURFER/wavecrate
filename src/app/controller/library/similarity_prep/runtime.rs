use super::DEFAULT_CLUSTER_MIN_SIZE;
use super::store::{DbSimilarityPrepStore, SimilarityPrepStore};
use crate::app::controller::AppController;
use crate::app::controller::jobs;
use crate::sample_sources::{SampleSource, SourceId};
use std::thread;
use wavecrate_analysis::hdbscan::{HdbscanConfig, HdbscanMethod};

impl AppController {
    pub(crate) fn apply_similarity_prep_duration_cap(&mut self) {
        let max_duration = if self.similarity_prep_duration_cap_enabled() {
            self.settings.analysis.max_analysis_duration_seconds
        } else {
            0.0
        };
        self.runtime
            .analysis
            .set_max_analysis_duration_seconds(max_duration);
    }

    pub(crate) fn restore_similarity_prep_duration_cap(&mut self) {
        self.runtime.analysis.set_max_analysis_duration_seconds(
            self.settings.analysis.max_analysis_duration_seconds,
        );
    }

    pub(crate) fn apply_similarity_prep_fast_mode(&mut self) {
        if !self.similarity_prep_fast_mode_enabled() {
            return;
        }
        let sample_rate = self.similarity_prep_fast_sample_rate();
        let version = wavecrate_analysis::analysis_version_for_sample_rate(sample_rate);
        self.runtime.analysis.set_analysis_sample_rate(sample_rate);
        self.runtime
            .analysis
            .set_analysis_version_override(Some(version));
    }

    pub(crate) fn restore_similarity_prep_fast_mode(&mut self) {
        self.runtime
            .analysis
            .set_analysis_sample_rate(wavecrate_analysis::ANALYSIS_SAMPLE_RATE);
        self.runtime.analysis.set_analysis_version_override(None);
    }

    pub(crate) fn apply_similarity_prep_full_analysis(&mut self, force_full_analysis: bool) {
        if !force_full_analysis {
            return;
        }
        self.runtime.analysis.set_analysis_cache_enabled(false);
    }

    pub(crate) fn restore_similarity_prep_full_analysis(&mut self) {
        self.runtime.analysis.set_analysis_cache_enabled(true);
    }

    pub(crate) fn apply_similarity_prep_worker_boost(&mut self) {
        if self.settings.analysis.analysis_worker_count != 0 {
            return;
        }
        let boosted = thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1)
            .clamp(1, 64);
        self.runtime.performance.idle_worker_override = Some(boosted);
        self.runtime.analysis.set_worker_count(boosted);
    }

    pub(crate) fn restore_similarity_prep_worker_count(&mut self) {
        self.runtime.performance.idle_worker_override = None;
        self.runtime
            .analysis
            .set_worker_count(self.settings.analysis.analysis_worker_count);
    }

    pub(crate) fn enqueue_similarity_backfill(
        &mut self,
        source: SampleSource,
        force_full_analysis: bool,
    ) {
        self.enqueue_similarity_prep_bootstrap(source, force_full_analysis);
    }

    pub(crate) fn start_similarity_finalize(&mut self, source_id: SourceId, umap_version: String) {
        self.runtime.similarity.begin_finalize(&source_id);
        let tx = self.runtime.jobs.message_sender();
        thread::spawn(move || {
            let started_at = std::time::Instant::now();
            let result =
                std::panic::catch_unwind(|| run_similarity_finalize(&source_id, &umap_version))
                    .unwrap_or_else(|_| Err("Similarity finalize panicked".to_string()));
            tracing::info!(
                "Similarity finalize finished in {:.2?} (source_id={})",
                started_at.elapsed(),
                source_id.as_str()
            );
            let _ = tx.send(jobs::JobMessage::SimilarityPrepared(
                jobs::SimilarityPrepResult { source_id, result },
            ));
        });
    }

    pub(crate) fn find_source_by_id(&self, source_id: &SourceId) -> Option<SampleSource> {
        self.library
            .sources
            .iter()
            .find(|source| &source.id == source_id)
            .cloned()
    }
}

fn run_similarity_finalize(
    source_id: &SourceId,
    umap_version: &str,
) -> Result<jobs::SimilarityPrepOutcome, String> {
    let store = DbSimilarityPrepStore;
    let mut conn = store.open_source_db_for_similarity(source_id)?;
    let sample_id_prefix = format!("{}::%", source_id.as_str());
    wavecrate_analysis::build_map_layout(
        &mut conn,
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
        umap_version,
        0,
        0.95,
    )?;
    let layout_rows = store.count_umap_layout_rows(
        &conn,
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
        umap_version,
        &sample_id_prefix,
    )?;
    if layout_rows == 0 {
        return Err(format!(
            "No Starmap layout rows for source {} (check embeddings)",
            source_id.as_str()
        ));
    }
    let cluster_stats = wavecrate_analysis::hdbscan::build_hdbscan_clusters_for_sample_id_prefix(
        &mut conn,
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
        HdbscanMethod::Umap,
        Some(umap_version),
        Some(sample_id_prefix.as_str()),
        HdbscanConfig {
            min_cluster_size: DEFAULT_CLUSTER_MIN_SIZE,
            min_samples: None,
            allow_single_cluster: false,
        },
    )?;
    wavecrate_analysis::flush_ann_index(&conn)?;
    Ok(jobs::SimilarityPrepOutcome { cluster_stats })
}
