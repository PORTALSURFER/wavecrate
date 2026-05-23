use super::*;

/// Build automation nodes for the sidebar browser filters.
pub(super) fn filters_group(rect: Rect, model: &AppModel) -> AutomationNodeSnapshot {
    let rows = ["format", "bit_depth", "channels", "bpm", "key", "rating"];
    let mut children: Vec<_> = rows
        .into_iter()
        .map(|name| {
            let (value, actions) = match name {
                "format" => (
                    sidebar_option_summary(model.sidebar_filters.formats.len(), "WAV"),
                    vec![String::from("toggle_browser_sidebar_filter")],
                ),
                "bit_depth" => (
                    sidebar_option_summary(model.sidebar_filters.bit_depths.len(), "Unavailable"),
                    vec![String::from("toggle_browser_sidebar_filter")],
                ),
                "channels" => (
                    sidebar_option_summary(model.sidebar_filters.channels.len(), "Unavailable"),
                    vec![String::from("toggle_browser_sidebar_filter")],
                ),
                "bpm" => (
                    sidebar_bpm_summary(model),
                    vec![
                        String::from("toggle_browser_sidebar_filter"),
                        String::from("clear_browser_sidebar_filter"),
                    ],
                ),
                "key" => (
                    sidebar_option_summary(model.sidebar_filters.keys.len(), "Unknown"),
                    vec![String::from("toggle_browser_sidebar_filter")],
                ),
                "rating" => (
                    rating_filter_summary(model),
                    vec![String::from("toggle_browser_rating_filter")],
                ),
                _ => (String::new(), Vec::new()),
            };
            simple_node(
                format!("sources.filters.{name}"),
                AutomationRole::Button,
                Some(name.replace('_', " ")),
                rect,
                Some(value),
                true,
                false,
                actions,
            )
        })
        .collect();
    children.push(simple_node(
        "sources.filters.marked",
        AutomationRole::Button,
        Some(String::from("Marked")),
        rect,
        Some(bool_text(model.browser.marked_filter_active).to_string()),
        true,
        model.browser.marked_filter_active,
        vec![String::from("toggle_browser_marked_filter")],
    ));
    for (slug, active) in [
        ("never", model.browser.active_recency_filters[0]),
        ("month", model.browser.active_recency_filters[1]),
        ("week", model.browser.active_recency_filters[2]),
    ] {
        children.push(simple_node(
            format!("sources.filters.playback_age.{slug}"),
            AutomationRole::Button,
            Some(format!("Playback age {slug}")),
            rect,
            Some(bool_text(active).to_string()),
            true,
            active,
            vec![String::from("toggle_browser_playback_age_filter")],
        ));
    }
    children.push(simple_node(
        "sources.filters.tag_named",
        AutomationRole::Button,
        Some(String::from("Tag-derived names")),
        rect,
        Some(if model.browser.derived_label_filter_negated {
            String::from("not tag-derived")
        } else if model.browser.derived_label_filter_active {
            String::from("tag-derived")
        } else {
            String::from("any")
        }),
        true,
        model.browser.derived_label_filter_active,
        vec![String::from("toggle_browser_tag_named_filter")],
    ));
    AutomationNodeSnapshot {
        id: node_id("sources.filters"),
        role: AutomationRole::Group,
        label: Some(String::from("Filters")),
        bounds: bounds(rect),
        value: None,
        enabled: true,
        selected: false,
        available_actions: vec![String::from("focus_browser_panel")],
        metadata: metadata(&[("placement", "left_sidebar")]),
        children,
    }
}

/// Summarize single-option sidebar facets for automation.
fn sidebar_option_summary(active_count: usize, label: &str) -> String {
    if active_count == 0 {
        String::from("Any")
    } else {
        label.to_string()
    }
}

/// Summarize active BPM sidebar facets for automation.
fn sidebar_bpm_summary(model: &AppModel) -> String {
    if model.sidebar_filters.bpms.is_empty() {
        String::from("Any")
    } else {
        model
            .sidebar_filters
            .bpms
            .iter()
            .map(|facet| format!("{facet:?}"))
            .collect::<Vec<_>>()
            .join("|")
    }
}

/// Summarize the active rating filters for automation.
fn rating_filter_summary(model: &AppModel) -> String {
    let active = model
        .browser
        .active_rating_filters
        .iter()
        .filter(|active| **active)
        .count();
    if active == 0 {
        String::from("Any")
    } else {
        format!("{active} active")
    }
}
