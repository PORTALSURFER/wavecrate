use std::path::PathBuf;

use radiant::prelude as ui;
use wavecrate::sample_sources::{SampleSource, SourceId};

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};

mod worker;
use worker::{enqueue_similarity_prep_inner, resolve_similarity_prep_status};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct NativeSimilarityPrepState {
    pub(in crate::native_app) status: Option<NativeSimilarityPrepStatus>,
    pub(in crate::native_app) running: bool,
    pub(in crate::native_app) summary: Option<String>,
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
    pub(in crate::native_app) result: Result<SimilarityPrepEnqueueSummary, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct SimilarityPrepEnqueueSummary {
    pub(in crate::native_app) analysis_inserted: usize,
    pub(in crate::native_app) embedding_inserted: usize,
    pub(in crate::native_app) finalized: bool,
    pub(in crate::native_app) status: NativeSimilarityPrepStatus,
}

#[derive(Clone, Debug)]
struct SimilarityPrepSource {
    id: SourceId,
    root: PathBuf,
}

impl SimilarityPrepSource {
    fn sample_source(&self) -> SampleSource {
        SampleSource::new_with_id(self.id.clone(), self.root.clone())
    }
}

impl NativeAppState {
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

    pub(in crate::native_app) fn prepare_similarity_for_selected_source(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if self.library.similarity_prep.running {
            self.ui.status.sample = String::from("Similarity prep already running");
            return;
        }
        let Some(source) = self.selected_similarity_prep_source() else {
            self.ui.status.sample = String::from("Select a source before preparing similarity");
            return;
        };
        self.library.similarity_prep.running = true;
        self.library.similarity_prep.summary = Some(String::from("Similarity prep queued"));
        context.business().background("gui-similarity-prep").run(
            move |_| enqueue_similarity_prep(source),
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
        if result.source_id != self.library.folder_browser.selected_source_id() {
            return;
        }
        self.library.similarity_prep.running = false;
        match result.result {
            Ok(summary) => {
                let message = summary.message();
                self.library.similarity_prep.status = Some(summary.status);
                self.library.similarity_prep.summary = Some(message.clone());
                self.ui.status.sample = message;
                self.refresh_selected_similarity_prep_status(context);
            }
            Err(error) => {
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

    fn selected_similarity_prep_source(&self) -> Option<SimilarityPrepSource> {
        let source_id = self.library.folder_browser.selected_source_id().to_string();
        let root = self.library.folder_browser.source_root_path(&source_id)?;
        Some(SimilarityPrepSource {
            id: SourceId::from_string(source_id),
            root,
        })
    }
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

    pub(in crate::native_app) fn can_prepare(&self) -> bool {
        !matches!(self, Self::UpToDate)
    }
}

impl SimilarityPrepEnqueueSummary {
    fn message(&self) -> String {
        if self.finalized {
            return String::from("Similarity ready");
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
}

fn resolve_status_result(source: SimilarityPrepSource) -> SimilarityPrepStatusResult {
    let source_id = source.id.as_str().to_string();
    SimilarityPrepStatusResult {
        source_id,
        status: resolve_similarity_prep_status(&source.sample_source()),
    }
}

fn enqueue_similarity_prep(source: SimilarityPrepSource) -> SimilarityPrepEnqueueResult {
    let source_id = source.id.as_str().to_string();
    SimilarityPrepEnqueueResult {
        source_id,
        result: enqueue_similarity_prep_inner(&source.sample_source()),
    }
}
