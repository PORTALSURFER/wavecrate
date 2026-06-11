use std::collections::HashMap;

use crate::native_app::app::{NativeAppState, SampleNameViewMode};
use crate::native_app::sample_library::folder_browser::{
    FileColumnDragFeedback, VisibleSampleList, VisibleSampleQuery,
};
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, SAMPLE_BROWSER_OVERSCAN_ROWS,
    SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS,
};

pub(in crate::native_app) struct SampleBrowserViewModel<'a> {
    pub(in crate::native_app) visible_samples: VisibleSampleList<'a>,
    pub(in crate::native_app) name_view_mode: SampleNameViewMode,
    pub(in crate::native_app) metadata_tags_by_file: &'a HashMap<String, Vec<String>>,
    pub(in crate::native_app) file_drag_active: bool,
    pub(in crate::native_app) extracted_file_drag_active: bool,
    pub(in crate::native_app) hovered_folder_drop_target: bool,
    pub(in crate::native_app) drag_feedback: Option<FileColumnDragFeedback>,
}

impl<'a> SampleBrowserViewModel<'a> {
    pub(in crate::native_app) fn from_app_state(state: &'a mut NativeAppState) -> Self {
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
                viewport_rows: SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS,
                overscan_rows: SAMPLE_BROWSER_OVERSCAN_ROWS,
                guard_rows: SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
            });

        Self {
            visible_samples,
            name_view_mode: state.metadata.sample_name_view_mode,
            metadata_tags_by_file: &state.metadata.tags_by_file,
            file_drag_active,
            extracted_file_drag_active,
            hovered_folder_drop_target,
            drag_feedback,
        }
    }
}
