use super::*;

/// Project browser toolbar/tab/footer labels.
pub(crate) fn project_browser_chrome_model(
    ui: &UiState,
    visible_count: usize,
) -> BrowserChromeModel {
    let search_focused = ui.browser.search_focus_requested;
    BrowserChromeModel {
        samples_tab_label: String::from("Samples"),
        map_tab_label: String::from("Similarity map"),
        search_prefix_label: if search_focused {
            String::from("Search • focused")
        } else {
            String::from("Search")
        },
        search_placeholder: browser_search_placeholder(search_focused),
        activity_ready_label: String::from("Ready"),
        activity_busy_label: String::from("Filtering"),
        sort_prefix_label: String::from("Sort"),
        sort_order_label: browser_sort_label(SampleBrowserSort::from(ui.browser.sort)).to_owned(),
        similarity_toggle_label: if ui.browser.similarity_sort_follow_loaded {
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
        SampleBrowserTab::Map => "Similarity map",
    }
}

/// Resolve search placeholder text, including a focused caret hint when active.
pub(super) fn browser_search_placeholder(search_focused: bool) -> String {
    if search_focused {
        String::from("▌")
    } else {
        String::from("Search samples (Ctrl+F)")
    }
}
