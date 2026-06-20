use crate::native_app::ui::ids as widget_ids;

pub(in crate::native_app) const SAMPLE_BROWSER_LIST_ID: u64 = widget_ids::SAMPLE_BROWSER_LIST_ID;
pub(in crate::native_app) const SAMPLE_BROWSER_ROW_HEIGHT: f32 = 22.0;
pub(in crate::native_app) const SAMPLE_BROWSER_EDGE_CONTEXT_ROWS: usize = 2;
pub(in crate::native_app) const SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS: usize =
    SAMPLE_BROWSER_EDGE_CONTEXT_ROWS + 1;
pub(in crate::native_app) const SAMPLE_BROWSER_OVERSCAN_ROWS: usize = 24;
pub(in crate::native_app) const SAMPLE_BROWSER_PROJECTED_VIEWPORT_ROWS: usize = 96;
