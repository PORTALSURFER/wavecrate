use std::path::PathBuf;

/// Sidebar list of configured drop targets.
#[derive(Clone, Debug, Default)]
pub struct DropTargetsUiState {
    /// Render rows for drop targets.
    pub rows: Vec<DropTargetRowView>,
    /// Currently selected row index.
    pub selected: Option<usize>,
    /// Row index with an open context menu.
    pub menu_row: Option<usize>,
    /// Row index to scroll into view.
    pub scroll_to: Option<usize>,
    /// User-defined height for the drop targets section, in points.
    pub height_override: Option<f32>,
    /// Cached height at the start of a resize drag for stable deltas.
    pub resize_origin_height: Option<f32>,
    /// Cached header height for the drop targets section.
    pub header_height: f32,
}

/// Display data for a single drop target row.
#[derive(Clone, Debug)]
pub struct DropTargetRowView {
    /// Drop target path.
    pub path: PathBuf,
    /// Display name.
    pub name: String,
    /// Cached label used for drag payloads.
    pub drag_label: String,
    /// Cached display path used in tooltips.
    pub tooltip_path: String,
    /// Whether the drop target path is missing.
    pub missing: bool,
    /// Optional drop target color.
    pub color: Option<crate::sample_sources::config::DropTargetColor>,
}
