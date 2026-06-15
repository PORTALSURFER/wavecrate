use crate::native_app::app::{GuiMessage, NativeAppState};
pub(in crate::native_app) use crate::native_app::app_chrome::library_browser::sample_browser_view::SampleFileHitTarget;
use crate::native_app::app_chrome::library_browser::sample_browser_view::sample_browser_from_state;
use crate::native_app::app_chrome::view_models::sample_browser::{
    SampleBrowserViewModel, SampleBrowserViewProjection,
    prepare_sample_browser_view as prepare_chrome_sample_browser_view,
};
use radiant::prelude as ui;

pub(in crate::native_app) use crate::native_app::sample_library::folder_browser::view_contract::{
    DEFAULT_FOLDER_WIDTH, MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH,
};
pub(in crate::native_app) use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_EDGE_CONTEXT_ROWS, SAMPLE_BROWSER_ROW_HEIGHT,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct SampleBrowserWindowProjection {
    pub(in crate::native_app) total_items: usize,
    pub(in crate::native_app) total_count: usize,
    pub(in crate::native_app) visible_rows: usize,
    pub(in crate::native_app) window_len: usize,
    pub(in crate::native_app) first_stems: Vec<String>,
}

pub(in crate::native_app) fn sample_browser(state: &NativeAppState) -> ui::View<GuiMessage> {
    sample_browser_from_state(state)
}

pub(in crate::native_app) fn prepare_sample_browser_view(state: &mut NativeAppState) {
    prepare_chrome_sample_browser_view(state);
}

pub(in crate::native_app) fn sample_file_hit_target(
    path: String,
    selected: bool,
    drag_active: bool,
    drag_source: bool,
    cached: bool,
) -> SampleFileHitTarget {
    SampleFileHitTarget::new(path, selected, drag_active, drag_source, cached)
}

pub(in crate::native_app) fn sample_browser_window_projection(
    state: &NativeAppState,
    take: usize,
) -> SampleBrowserWindowProjection {
    let model = SampleBrowserViewModel::from_projection(
        SampleBrowserViewProjection::from_prepared_app_state(state),
    );
    SampleBrowserWindowProjection {
        total_items: model.visible_samples.window.total_items,
        total_count: model.visible_samples.total_count,
        visible_rows: model.visible_samples.rows.len(),
        window_len: model.visible_samples.window.window_len(),
        first_stems: model
            .visible_samples
            .rows
            .iter()
            .take(take)
            .filter_map(|row| row.as_ref().map(|row| row.file.stem.clone()))
            .collect(),
    }
}
