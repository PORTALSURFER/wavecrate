use crate::app_core::actions::NativeBrowserChromeModel as BrowserChromeModel;
use crate::app_core::state::{SampleBrowserSort, SampleBrowserTab, UiState};

/// Project browser toolbar/tab/footer labels.
pub(crate) fn project_browser_chrome_model(
    ui: &UiState,
    visible_count: usize,
) -> BrowserChromeModel {
    let duplicate_cleanup_active = ui.browser.duplicate_cleanup.is_some();
    BrowserChromeModel {
        samples_tab_label: String::from("Samples"),
        sample_column_label: String::from("Sample"),
        map_tab_label: String::from("Starmap"),
        tag_editor_label: String::from("Tags"),
        search_prefix_label: String::from("Search"),
        search_placeholder: browser_search_placeholder(),
        activity_ready_label: String::from("Ready"),
        activity_busy_label: String::from("Filtering"),
        sort_prefix_label: String::from("Sort"),
        sort_order_label: if duplicate_cleanup_active {
            String::from("Duplicate cleanup")
        } else {
            browser_sort_label(SampleBrowserSort::from(ui.browser.search.sort)).to_owned()
        },
        similarity_toggle_label: if duplicate_cleanup_active {
            String::from("anchor kept • right-click keep • Enter trashes rest")
        } else if ui.browser.search.similarity_sort_follow_loaded {
            String::from("follow loaded")
        } else {
            String::from("manual anchor")
        },
        item_count_label: format!("{visible_count} items"),
    }
}

/// Format one browser sort label for chrome projection.
pub(super) fn browser_sort_label(sort: SampleBrowserSort) -> &'static str {
    match sort {
        SampleBrowserSort::ListOrder => "List order",
        SampleBrowserSort::Similarity => "Similarity",
        SampleBrowserSort::PlaybackAgeAsc => "Playback age ↑",
        SampleBrowserSort::PlaybackAgeDesc => "Playback age ↓",
    }
}

/// Format one browser tab label for chrome projection.
pub(super) fn browser_tab_label(tab: SampleBrowserTab) -> &'static str {
    match tab {
        SampleBrowserTab::List => "Samples",
        SampleBrowserTab::Map => "Starmap",
    }
}

/// Resolve search placeholder text.
pub(super) fn browser_search_placeholder() -> String {
    String::from("Search samples")
}
