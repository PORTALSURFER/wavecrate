use std::path::Path;

use radiant::prelude as ui;
use wavecrate::sample_sources::{StarmapLayoutLoadResult, load_starmap_layout};

use crate::native_app::app::{GuiMessage, NativeAppState, SampleBrowserDisplayMode};

mod worker;

pub(in crate::native_app) use worker::{
    SimilarityPublicationFence, finalize_similarity_artifacts_if_ready,
    native_similarity_artifact_version, reset_interrupted_readiness_jobs,
    run_internal_similarity_finalizer_from_args,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct SimilarityArtifactRefreshState {
    pub(in crate::native_app) readiness_score_refresh_running: bool,
    pub(in crate::native_app) readiness_score_refresh_pending: bool,
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

    pub(in crate::native_app) fn prepare_similarity_for_anchor_path(
        &mut self,
        file_id: &str,
        _context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some((source, relative_path)) = self
            .library
            .folder_browser
            .sample_source_for_file_path(Path::new(file_id))
        else {
            return;
        };
        self.background.source_processing.prioritize_path(
            source.id.as_str(),
            &relative_path.to_string_lossy(),
            true,
        );
    }
}
