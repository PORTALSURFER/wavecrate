use std::{collections::VecDeque, path::Path};

use radiant::prelude as ui;
use wavecrate::sample_sources::{
    SampleSource, SourceId, StarmapLayoutLoadResult, load_starmap_layout,
};

use crate::native_app::app::{
    GuiMessage, NativeAppState, SampleBrowserDisplayMode, emit_gui_action,
};

mod worker;
use worker::{enqueue_similarity_prep_inner_with_cancel, resolve_similarity_prep_status};

pub(in crate::native_app) use worker::{
    NATIVE_SIMILARITY_UMAP_VERSION, SimilarityPublicationFence, finalize_similarity_prep_if_ready,
    reset_interrupted_similarity_prep_jobs, run_internal_similarity_finalizer_from_args,
    run_similarity_prep_job, similarity_prep_needs_finalization,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct NativeSimilarityPrepState {
    pub(in crate::native_app) status: Option<NativeSimilarityPrepStatus>,
    pub(in crate::native_app) running: bool,
    pub(in crate::native_app) running_source_id: Option<String>,
    pub(in crate::native_app) pending_source_ids: VecDeque<String>,
    pub(in crate::native_app) summary: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum SimilarityPrepTrigger {
    Automatic,
    UserRequested,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum NativeSimilarityPrepStatus {
    UpToDate,
    Outdated,
    Blocked {
        failed_count: usize,
        unsupported_count: usize,
    },
    MissingArtifacts {
        missing_embeddings: bool,
        missing_aspects: bool,
        missing_layout: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct SimilarityPrepStatusResult {
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) status: Result<NativeSimilarityPrepStatus, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct SimilarityPrepEnqueueResult {
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) trigger: SimilarityPrepTrigger,
    pub(in crate::native_app) result: Result<SimilarityPrepEnqueueSummary, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct SimilarityPrepEnqueueSummary {
    pub(in crate::native_app) analysis_inserted: usize,
    pub(in crate::native_app) embedding_inserted: usize,
    pub(in crate::native_app) jobs_processed: usize,
    pub(in crate::native_app) jobs_failed: usize,
    pub(in crate::native_app) finalized: bool,
    pub(in crate::native_app) status: NativeSimilarityPrepStatus,
}

#[derive(Clone, Debug)]
struct SimilarityPrepSource {
    source: SampleSource,
}

impl SimilarityPrepSource {
    fn id(&self) -> &SourceId {
        &self.source.id
    }

    fn sample_source(&self) -> SampleSource {
        self.source.clone()
    }
}

impl NativeAppState {
    pub(in crate::native_app) fn maybe_start_starmap_layout_load(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if self.ui.chrome.sample_browser_display != SampleBrowserDisplayMode::Map {
            return;
        }
        let Some(request) = self
            .library
            .folder_browser
            .take_starmap_layout_load_request(&self.metadata.tags_by_file)
        else {
            return;
        };
        context.business().idle("gui-starmap-layout-load").run(
            move |_| load_starmap_layout(request),
            GuiMessage::StarmapLayoutLoaded,
        );
    }

    pub(in crate::native_app) fn finish_starmap_layout_load(
        &mut self,
        result: StarmapLayoutLoadResult,
    ) {
        self.library
            .folder_browser
            .apply_starmap_layout_load_result(result);
    }

    pub(in crate::native_app) fn refresh_selected_similarity_prep_status(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(source) = self.selected_similarity_prep_source() else {
            self.library.similarity_prep.status = None;
            self.library.similarity_prep.summary = None;
            return;
        };
        context
            .business()
            .background("gui-similarity-prep-status")
            .run(
                move |_| resolve_status_result(source),
                GuiMessage::SimilarityPrepStatusResolved,
            );
    }

    pub(in crate::native_app) fn prepare_similarity_for_source(
        &mut self,
        source_id: &str,
        trigger: SimilarityPrepTrigger,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(source) = self.similarity_prep_source_for_id(source_id) else {
            return;
        };
        self.queue_similarity_prep_for_source(source, trigger, context);
    }

    pub(in crate::native_app) fn prepare_similarity_for_anchor_path(
        &mut self,
        file_id: &str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some((source, _)) = self
            .library
            .folder_browser
            .sample_source_for_file_path(Path::new(file_id))
        else {
            return;
        };
        self.queue_similarity_prep_for_source(
            SimilarityPrepSource { source },
            SimilarityPrepTrigger::Automatic,
            context,
        );
    }

    fn queue_similarity_prep_for_source(
        &mut self,
        source: SimilarityPrepSource,
        trigger: SimilarityPrepTrigger,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if self.library.similarity_prep.running {
            if trigger == SimilarityPrepTrigger::UserRequested {
                self.ui.status.sample = String::from("Similarity prep already running");
            }
            self.queue_pending_similarity_prep_source(source.id().as_str());
            return;
        }
        let source_id = source.id().as_str().to_string();
        let selected_source = source_id == self.library.folder_browser.selected_source_id();
        self.library.similarity_prep.running = true;
        self.library.similarity_prep.running_source_id = Some(source_id);
        if trigger == SimilarityPrepTrigger::UserRequested {
            self.library.similarity_prep.summary = Some(String::from("Similarity prep queued"));
        } else if selected_source {
            self.library.similarity_prep.summary = None;
        }
        let budget = self.background.source_processing.budget_handle();
        context.business().background("gui-similarity-prep").run(
            move |_| {
                let source_id = source.id().as_str().to_string();
                let Some(permit) = budget.acquire_scan(&source_id) else {
                    return SimilarityPrepEnqueueResult {
                        source_id,
                        trigger,
                        result: Err(String::from("Similarity preparation canceled")),
                    };
                };
                let cancel = permit.cancel_token();
                let result = enqueue_similarity_prep(source, trigger, Some(cancel.as_ref()));
                drop(permit);
                result
            },
            GuiMessage::SimilarityPrepEnqueueFinished,
        );
    }

    pub(in crate::native_app) fn finish_similarity_prep_status(
        &mut self,
        result: SimilarityPrepStatusResult,
    ) {
        if result.source_id != self.library.folder_browser.selected_source_id() {
            return;
        }
        match result.status {
            Ok(status) => {
                self.library.similarity_prep.status = Some(status.clone());
                self.library.similarity_prep.summary = Some(status.summary().to_string());
            }
            Err(error) => {
                self.library.similarity_prep.status = None;
                self.library.similarity_prep.summary = Some(format!("Similarity status: {error}"));
            }
        }
    }

    pub(in crate::native_app) fn finish_similarity_prep_enqueue(
        &mut self,
        result: SimilarityPrepEnqueueResult,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.finish_running_similarity_prep(&result.source_id);
        let selected_source = result.source_id == self.library.folder_browser.selected_source_id();
        match result.result {
            Ok(summary) => {
                if starmap_layout_may_have_changed(&summary) {
                    self.library.folder_browser.invalidate_starmap_layout();
                }
                let refresh_anchor_scores =
                    selected_source && summary.should_refresh_anchor_scores();
                if selected_source {
                    let has_work = summary.has_work();
                    let message = summary.message();
                    let footer_message = summary.footer_message();
                    self.library.similarity_prep.status = Some(summary.status);
                    self.library.similarity_prep.summary = Some(message.clone());
                    if (result.trigger == SimilarityPrepTrigger::UserRequested || has_work)
                        && let Some(footer_message) = footer_message
                    {
                        self.ui.status.sample = footer_message;
                    }
                    self.refresh_selected_similarity_prep_status(context);
                }
                if refresh_anchor_scores {
                    self.queue_active_similarity_score_resolution(context);
                }
            }
            Err(error) => {
                if selected_source {
                    self.library.similarity_prep.summary =
                        Some(format!("Similarity prep failed: {error}"));
                    self.ui.status.sample = format!("Similarity prep failed: {error}");
                    emit_gui_action(
                        "similarity_prep.native.enqueue",
                        Some("browser"),
                        Some(result.source_id.as_str()),
                        "error",
                        std::time::Instant::now(),
                        Some(&error),
                    );
                }
            }
        }
        self.background
            .source_processing
            .wake_source(&result.source_id, "similarity_prep_commit");
        self.start_next_pending_similarity_prep(context);
    }

    fn finish_running_similarity_prep(&mut self, source_id: &str) {
        if self
            .library
            .similarity_prep
            .running_source_id
            .as_deref()
            .is_some_and(|running| running != source_id)
        {
            return;
        }
        self.library.similarity_prep.running = false;
        self.library.similarity_prep.running_source_id = None;
    }

    fn queue_pending_similarity_prep_source(&mut self, source_id: &str) {
        if self.library.similarity_prep.running_source_id.as_deref() == Some(source_id)
            || self
                .library
                .similarity_prep
                .pending_source_ids
                .iter()
                .any(|pending| pending == source_id)
        {
            return;
        }
        self.library
            .similarity_prep
            .pending_source_ids
            .push_back(source_id.to_string());
    }

    fn start_next_pending_similarity_prep(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if self.library.similarity_prep.running {
            return;
        }
        while let Some(source_id) = self.library.similarity_prep.pending_source_ids.pop_front() {
            let Some(source) = self.similarity_prep_source_for_id(&source_id) else {
                continue;
            };
            self.queue_similarity_prep_for_source(
                source,
                SimilarityPrepTrigger::Automatic,
                context,
            );
            break;
        }
    }

    fn selected_similarity_prep_source(&self) -> Option<SimilarityPrepSource> {
        let source_id = self.library.folder_browser.selected_source_id().to_string();
        self.similarity_prep_source_for_id(&source_id)
    }

    fn similarity_prep_source_for_id(&self, source_id: &str) -> Option<SimilarityPrepSource> {
        let source = self
            .library
            .folder_browser
            .sources()
            .iter()
            .find(|source| source.id == source_id)?
            .as_sample_source();
        Some(SimilarityPrepSource { source })
    }
}

fn starmap_layout_may_have_changed(summary: &SimilarityPrepEnqueueSummary) -> bool {
    summary.has_work() || summary.status == NativeSimilarityPrepStatus::UpToDate
}

impl NativeSimilarityPrepStatus {
    pub(in crate::native_app) fn summary(&self) -> &'static str {
        match self {
            Self::UpToDate => "Similarity ready",
            Self::Outdated => "Similarity prep is out of date",
            Self::Blocked { .. } => "Similarity prep blocked",
            Self::MissingArtifacts { .. } => "Similarity prep needed",
        }
    }
}

impl SimilarityPrepEnqueueSummary {
    fn has_work(&self) -> bool {
        self.finalized
            || self.analysis_inserted > 0
            || self.embedding_inserted > 0
            || self.jobs_processed > 0
    }

    fn message(&self) -> String {
        if let NativeSimilarityPrepStatus::Blocked { failed_count, .. } = self.status {
            return format!(
                "Similarity prep blocked: {failed_count} job{} failed",
                if failed_count == 1 { "" } else { "s" }
            );
        }
        if self.jobs_failed > 0 {
            return format!(
                "Similarity prep blocked: {} job{} failed",
                self.jobs_failed,
                if self.jobs_failed == 1 { "" } else { "s" }
            );
        }
        if self.finalized {
            return String::from("Similarity ready");
        }
        if self.jobs_processed > 0 {
            return format!(
                "Similarity prep processed {} job{}",
                self.jobs_processed,
                if self.jobs_processed == 1 { "" } else { "s" }
            );
        }
        let queued = self.analysis_inserted + self.embedding_inserted;
        if queued == 0 {
            String::from("Similarity prep refreshed")
        } else {
            format!(
                "Similarity prep queued {queued} job{}",
                if queued == 1 { "" } else { "s" }
            )
        }
    }

    fn footer_message(&self) -> Option<String> {
        if self.finalized && self.status == NativeSimilarityPrepStatus::UpToDate {
            return None;
        }
        Some(self.message())
    }

    fn should_refresh_anchor_scores(&self) -> bool {
        self.has_work() || self.status == NativeSimilarityPrepStatus::UpToDate
    }
}

fn resolve_status_result(source: SimilarityPrepSource) -> SimilarityPrepStatusResult {
    let source_id = source.id().as_str().to_string();
    SimilarityPrepStatusResult {
        source_id,
        status: resolve_similarity_prep_status(&source.sample_source()),
    }
}

fn enqueue_similarity_prep(
    source: SimilarityPrepSource,
    trigger: SimilarityPrepTrigger,
    cancel: Option<&std::sync::atomic::AtomicBool>,
) -> SimilarityPrepEnqueueResult {
    let source_id = source.id().as_str().to_string();
    let sample_source = source.sample_source();
    SimilarityPrepEnqueueResult {
        source_id,
        trigger,
        result: enqueue_similarity_prep_inner_with_cancel(
            &sample_source,
            trigger == SimilarityPrepTrigger::Automatic,
            cancel,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(status: NativeSimilarityPrepStatus) -> SimilarityPrepEnqueueSummary {
        SimilarityPrepEnqueueSummary {
            analysis_inserted: 0,
            embedding_inserted: 0,
            jobs_processed: 0,
            jobs_failed: 0,
            finalized: false,
            status,
        }
    }

    #[test]
    fn starmap_layout_refreshes_after_success_or_work() {
        let ready = summary(NativeSimilarityPrepStatus::UpToDate);
        let mut with_work = summary(NativeSimilarityPrepStatus::MissingArtifacts {
            missing_embeddings: true,
            missing_aspects: false,
            missing_layout: true,
        });
        with_work.jobs_processed = 1;

        assert!(starmap_layout_may_have_changed(&ready));
        assert!(starmap_layout_may_have_changed(&with_work));
    }

    #[test]
    fn blocked_no_work_similarity_prep_keeps_starmap_request_suppressed() {
        let blocked = summary(NativeSimilarityPrepStatus::Blocked {
            failed_count: 1,
            unsupported_count: 0,
        });

        assert!(!starmap_layout_may_have_changed(&blocked));
    }
}
