//! Shared native radiant UI helpers for companion apps.
//!
//! These helpers keep the installer and updater-helper aligned on the same
//! launch sizing, icon-decoding policy, and shell-model scaffolding without
//! forcing a heavy abstraction over their distinct workflow state machines.

use crate::{
    app_core::actions::{
        NativeAppModel as AppModel, NativeBrowserActionsModel as BrowserActionsModel,
        NativeBrowserChromeModel as BrowserChromeModel,
        NativeBrowserPanelModel as BrowserPanelModel, NativeBrowserRowModel as BrowserRowModel,
        NativeSourceRowModel as SourceRowModel, NativeStatusBarModel as StatusBarModel,
        NativeUpdatePanelModel as UpdatePanelModel,
    },
    gui_runtime::{NativeRunOptions, WindowIconRgba},
};

/// Build the standard native radiant window options for companion apps.
///
/// The installer and updater helper intentionally share the same default
/// window geometry and frame pacing so they behave consistently on first open.
pub fn standard_window_options(
    title: impl Into<String>,
    icon: Option<WindowIconRgba>,
) -> NativeRunOptions {
    NativeRunOptions {
        title: title.into(),
        inner_size: Some([860.0, 620.0]),
        min_inner_size: Some([640.0, 420.0]),
        maximized: false,
        decorations: true,
        target_fps: 120,
        icon,
    }
}

/// Decode the first usable window icon from a list of bundled assets.
///
/// Candidates are tried in order so callers can prefer `.ico` payloads and
/// fall back to PNG assets when platform-specific decoding fails.
pub fn decode_first_window_icon(candidates: &[&[u8]]) -> Option<WindowIconRgba> {
    candidates
        .iter()
        .find_map(|bytes| decode_window_icon(bytes))
}

/// Shared inputs for a companion browser panel.
pub struct CompanionBrowserPanelConfig {
    /// Visible selection within the current rows, if any.
    pub selected_visible_row: Option<usize>,
    /// Count of selected paths represented in the helper UI.
    pub selected_path_count: usize,
    /// Context label shown in the shared browser search field.
    pub search_query: String,
    /// Placeholder text for the search field.
    pub search_placeholder: Option<String>,
    /// Busy-state flag for long-running work.
    pub busy: bool,
    /// Visible browser tab label.
    pub active_tab_label: Option<String>,
    /// Focused row label surfaced beside the browser.
    pub focused_sample_label: Option<String>,
    /// Rows to render in the helper browser panel.
    pub rows: Vec<BrowserRowModel>,
    /// Sort label shown in the shared browser chrome.
    pub sort_label: Option<String>,
}

/// Shared inputs for a companion browser chrome section.
pub struct CompanionBrowserChromeConfig {
    /// Primary browser-tab label.
    pub samples_tab_label: String,
    /// Secondary browser-tab label.
    pub map_tab_label: String,
    /// Prefix shown before the shared search context label.
    pub search_prefix_label: String,
    /// Search-field placeholder text.
    pub search_placeholder: String,
    /// Idle activity label.
    pub activity_ready_label: String,
    /// Busy activity label.
    pub activity_busy_label: String,
    /// Prefix shown before the sort-order label.
    pub sort_prefix_label: String,
    /// Sort-order label.
    pub sort_order_label: String,
    /// Visible row count shown in the chrome footer.
    pub item_count: usize,
}

/// Shared inputs for the companion root app shell.
pub struct CompanionAppModelConfig {
    /// Window/application title.
    pub title: String,
    /// Backend context label shown in the top bar.
    pub backend_label: String,
    /// Source panel title.
    pub sources_label: String,
    /// Shared status bar model.
    pub status: StatusBarModel,
    /// Shared browser panel model.
    pub browser: BrowserPanelModel,
    /// Shared browser chrome model.
    pub browser_chrome: BrowserChromeModel,
    /// Source rows rendered in the sidebar.
    pub source_rows: Vec<SourceRowModel>,
    /// Update panel content rendered in the top-right action area.
    pub update: UpdatePanelModel,
}

/// Build the shared browser panel shell used by the installer and updater helper.
pub fn standard_browser_panel(config: CompanionBrowserPanelConfig) -> BrowserPanelModel {
    BrowserPanelModel {
        visible_count: config.rows.len(),
        selected_visible_row: config.selected_visible_row,
        autoscroll: true,
        view_start_row: 0,
        selected_path_count: config.selected_path_count,
        search_query: config.search_query,
        active_rating_filters: [false; 8],
        active_playback_age_filters: [false; 3],
        marked_filter_active: false,
        search_placeholder: config.search_placeholder,
        busy: config.busy,
        similarity_filtered: false,
        duplicate_cleanup_active: false,
        sort_label: config.sort_label,
        active_tab_label: config.active_tab_label,
        focused_sample_label: config.focused_sample_label,
        anchor_visible_row: None,
        rows: config.rows,
    }
}

/// Build the shared browser chrome shell used by the installer and updater helper.
pub fn standard_browser_chrome(config: CompanionBrowserChromeConfig) -> BrowserChromeModel {
    BrowserChromeModel {
        samples_tab_label: config.samples_tab_label,
        map_tab_label: config.map_tab_label,
        search_prefix_label: config.search_prefix_label,
        search_placeholder: config.search_placeholder,
        activity_ready_label: config.activity_ready_label,
        activity_busy_label: config.activity_busy_label,
        sort_prefix_label: config.sort_prefix_label,
        sort_order_label: config.sort_order_label,
        similarity_toggle_label: String::from("n/a"),
        item_count_label: format!("{} rows", config.item_count),
    }
}

/// Build the shared native helper app shell around app-specific content.
pub fn standard_app_model(config: CompanionAppModelConfig) -> AppModel {
    let mut model = AppModel {
        title: config.title,
        backend_label: config.backend_label,
        sources_label: config.sources_label,
        status_text: String::new(),
        status: config.status,
        transport_running: true,
        ..AppModel::default()
    };
    model.browser_actions = BrowserActionsModel::default();
    model.browser = config.browser;
    model.browser_chrome = config.browser_chrome;
    model.sources.rows = config.source_rows;
    model.update = config.update;
    model
}

fn decode_window_icon(bytes: &[u8]) -> Option<WindowIconRgba> {
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    Some(WindowIconRgba {
        rgba: image.into_raw(),
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_window_options_use_companion_defaults() {
        let options = standard_window_options("Test", None);
        assert_eq!(options.inner_size, Some([860.0, 620.0]));
        assert_eq!(options.min_inner_size, Some([640.0, 420.0]));
        assert_eq!(options.target_fps, 120);
        assert!(!options.maximized);
        assert!(options.decorations);
    }

    #[test]
    fn standard_browser_panel_keeps_shared_defaults() {
        let panel = standard_browser_panel(CompanionBrowserPanelConfig {
            selected_visible_row: Some(0),
            selected_path_count: 1,
            search_query: String::from("flow"),
            search_placeholder: Some(String::from("placeholder")),
            busy: true,
            active_tab_label: Some(String::from("Flow")),
            focused_sample_label: Some(String::from("row")),
            rows: vec![BrowserRowModel::new(0, "row", 1, true, true)],
            sort_label: Some(String::from("custom")),
        });
        assert!(panel.autoscroll);
        assert_eq!(panel.view_start_row, 0);
        assert_eq!(panel.active_rating_filters, [false; 8]);
        assert!(!panel.similarity_filtered);
        assert_eq!(panel.sort_label.as_deref(), Some("custom"));
    }
}
