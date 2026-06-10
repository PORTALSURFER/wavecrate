use radiant::prelude as ui;
use std::collections::{HashMap, HashSet};

use crate::native_app::app::{NativeAppState, SampleNameViewMode};
use crate::native_app::sample_library::folder_browser::{
    FileColumn, FileColumnDragFeedback, FolderBrowserState,
};
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, SAMPLE_BROWSER_OVERSCAN_ROWS,
    SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS,
};

pub(in crate::native_app) struct SampleBrowserViewModel<'a> {
    pub(in crate::native_app) folder_browser: &'a FolderBrowserState,
    pub(in crate::native_app) audio_count: usize,
    pub(in crate::native_app) columns: Vec<&'a FileColumn>,
    pub(in crate::native_app) window: ui::VirtualListWindow,
    pub(in crate::native_app) name_view_mode: SampleNameViewMode,
    pub(in crate::native_app) metadata_tags_by_file: &'a HashMap<String, Vec<String>>,
    pub(in crate::native_app) cached_sample_paths: &'a HashSet<String>,
    pub(in crate::native_app) similarity_mode_active: bool,
    pub(in crate::native_app) file_drag_active: bool,
    pub(in crate::native_app) extracted_file_drag_active: bool,
    pub(in crate::native_app) hovered_folder_drop_target: bool,
    pub(in crate::native_app) drag_feedback: Option<FileColumnDragFeedback>,
}

impl<'a> SampleBrowserViewModel<'a> {
    pub(in crate::native_app) fn from_app_state(state: &'a mut NativeAppState) -> Self {
        let window = state
            .library
            .folder_browser
            .follow_selected_file_view_matching_tags(
                SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS,
                SAMPLE_BROWSER_OVERSCAN_ROWS,
                SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
                &state.metadata.tags_by_file,
            );
        let audio_count = state
            .library
            .folder_browser
            .selected_audio_file_count_matching_tags(&state.metadata.tags_by_file);
        let columns = state.library.folder_browser.visible_file_columns();
        let file_drag_active = state.library.folder_browser.file_drag_active();
        let extracted_file_drag_active = state.library.folder_browser.extracted_file_drag_active();
        let hovered_folder_drop_target = state
            .library
            .folder_browser
            .hovered_drop_target_folder_id()
            .is_some();
        let drag_feedback = state.library.folder_browser.file_column_drag_feedback();
        let similarity_mode_active = state.library.folder_browser.similarity_mode_active();

        Self {
            folder_browser: &state.library.folder_browser,
            audio_count,
            columns,
            window,
            name_view_mode: state.metadata.sample_name_view_mode,
            metadata_tags_by_file: &state.metadata.tags_by_file,
            cached_sample_paths: &state.waveform.cache.cached_sample_paths,
            similarity_mode_active,
            file_drag_active,
            extracted_file_drag_active,
            hovered_folder_drop_target,
            drag_feedback,
        }
    }
}
