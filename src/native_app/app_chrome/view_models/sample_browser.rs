use std::collections::HashMap;

use crate::native_app::app::{NativeAppState, SampleNameViewMode};
use crate::native_app::sample_library::folder_browser::projection::{
    FileColumnDragFeedback, VisibleSampleList, VisibleSampleQuery, VisibleSampleWindowPolicy,
};
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, SAMPLE_BROWSER_OVERSCAN_ROWS,
    SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS,
};
use crate::native_app::sample_library::similarity_prep::NativeSimilarityPrepState;

pub(in crate::native_app) struct SampleBrowserViewModel<'a> {
    pub(in crate::native_app) visible_samples: VisibleSampleList<'a>,
    pub(in crate::native_app) name_view_mode: SampleNameViewMode,
    pub(in crate::native_app) random_navigation_enabled: bool,
    pub(in crate::native_app) metadata_tags_by_file: &'a HashMap<String, Vec<String>>,
    pub(in crate::native_app) file_drag_active: bool,
    pub(in crate::native_app) extracted_file_drag_active: bool,
    pub(in crate::native_app) hovered_folder_drop_target: bool,
    pub(in crate::native_app) drag_feedback: Option<FileColumnDragFeedback>,
    pub(in crate::native_app) similarity_prep: &'a NativeSimilarityPrepState,
}

pub(in crate::native_app) struct SampleBrowserViewProjection<'a> {
    visible_samples: VisibleSampleList<'a>,
    name_view_mode: SampleNameViewMode,
    random_navigation_enabled: bool,
    metadata_tags_by_file: &'a HashMap<String, Vec<String>>,
    file_drag_active: bool,
    extracted_file_drag_active: bool,
    hovered_folder_drop_target: bool,
    drag_feedback: Option<FileColumnDragFeedback>,
    similarity_prep: &'a NativeSimilarityPrepState,
}

impl<'a> SampleBrowserViewProjection<'a> {
    pub(in crate::native_app) fn from_prepared_app_state(state: &'a NativeAppState) -> Self {
        let file_drag_active = state.library.folder_browser.file_drag_active();
        let extracted_file_drag_active = state.library.folder_browser.extracted_file_drag_active();
        let hovered_folder_drop_target = state
            .library
            .folder_browser
            .hovered_drop_target_folder_id()
            .is_some();
        let drag_feedback = state.library.folder_browser.file_column_drag_feedback();
        let visible_samples = state
            .library
            .folder_browser
            .visible_samples(VisibleSampleQuery {
                tags_by_file: &state.metadata.tags_by_file,
                cached_sample_paths: &state.waveform.cache.cached_sample_paths,
            });

        Self {
            visible_samples,
            name_view_mode: state.metadata.sample_name_view_mode,
            random_navigation_enabled: state.library.folder_browser.random_navigation_enabled(),
            metadata_tags_by_file: &state.metadata.tags_by_file,
            file_drag_active,
            extracted_file_drag_active,
            hovered_folder_drop_target,
            drag_feedback,
            similarity_prep: &state.library.similarity_prep,
        }
    }
}

impl<'a> SampleBrowserViewModel<'a> {
    pub(in crate::native_app) fn from_projection(
        projection: SampleBrowserViewProjection<'a>,
    ) -> Self {
        Self {
            visible_samples: projection.visible_samples,
            name_view_mode: projection.name_view_mode,
            random_navigation_enabled: projection.random_navigation_enabled,
            metadata_tags_by_file: projection.metadata_tags_by_file,
            file_drag_active: projection.file_drag_active,
            extracted_file_drag_active: projection.extracted_file_drag_active,
            hovered_folder_drop_target: projection.hovered_folder_drop_target,
            drag_feedback: projection.drag_feedback,
            similarity_prep: projection.similarity_prep,
        }
    }
}

pub(in crate::native_app) fn prepare_sample_browser_view(state: &mut NativeAppState) {
    state
        .library
        .folder_browser
        .prepare_visible_sample_window(VisibleSampleWindowPolicy {
            tags_by_file: &state.metadata.tags_by_file,
            viewport_rows: SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS,
            overscan_rows: SAMPLE_BROWSER_OVERSCAN_ROWS,
            guard_rows: SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
        });
}
