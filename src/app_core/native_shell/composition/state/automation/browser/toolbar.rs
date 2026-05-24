//! Browser toolbar automation nodes.

use super::*;
use crate::app_core::native_shell::runtime_contract::{AutomationRole, PlaybackAgeFilterChip};

pub(super) fn toolbar_control_nodes(
    toolbar: &BrowserToolbarLayout,
    model: &AppModel,
) -> Vec<AutomationNodeSnapshot> {
    let mut children = Vec::new();
    if toolbar.search_field.width() > 1.0 {
        children.push(simple_node(
            "browser.search_field",
            AutomationRole::SearchField,
            Some(String::from("Browser search")),
            toolbar.search_field,
            Some(model.browser.search_query.clone()),
            true,
            false,
            vec![
                String::from("focus_browser_search"),
                String::from("set_browser_search"),
            ],
        ));
    }
    children.extend(rating_filter_nodes(toolbar, model));
    children.extend(playback_age_filter_nodes(toolbar, model));
    children.extend(toggle_filter_nodes(toolbar, model));

    children
}

fn rating_filter_nodes(
    toolbar: &BrowserToolbarLayout,
    model: &AppModel,
) -> Vec<AutomationNodeSnapshot> {
    let mut children = Vec::new();
    for (index, chip) in toolbar.rating_filter_chips.iter().copied().enumerate() {
        if chip.width() <= 1.0 {
            continue;
        }
        let level = super::super::super::BROWSER_RATING_FILTER_LEVELS[index];
        children.push(simple_node(
            format!("browser.rating_filter.{level}"),
            AutomationRole::Button,
            Some(format!("Rating filter {level}")),
            chip,
            None,
            true,
            model.browser.active_rating_filters[index],
            vec![String::from("toggle_browser_rating_filter")],
        ));
    }
    children
}

fn playback_age_filter_nodes(
    toolbar: &BrowserToolbarLayout,
    model: &AppModel,
) -> Vec<AutomationNodeSnapshot> {
    let mut children = Vec::new();
    for (index, chip) in toolbar
        .playback_age_filter_chips
        .iter()
        .copied()
        .enumerate()
    {
        if chip.width() <= 1.0 {
            continue;
        }
        let bucket = super::super::super::BROWSER_PLAYBACK_AGE_FILTER_CHIPS[index];
        let (slug, label) = playback_age_filter_label(bucket);
        children.push(simple_node(
            format!("browser.playback_age_filter.{slug}"),
            AutomationRole::Button,
            Some(String::from(label)),
            chip,
            None,
            true,
            model.browser.active_recency_filters[index],
            vec![String::from("toggle_browser_playback_age_filter")],
        ));
    }
    children
}

fn playback_age_filter_label(bucket: PlaybackAgeFilterChip) -> (&'static str, &'static str) {
    match bucket {
        PlaybackAgeFilterChip::NeverPlayed => ("never", "Never played"),
        PlaybackAgeFilterChip::OlderThanMonth => ("month", "Older than month"),
        PlaybackAgeFilterChip::OlderThanWeek => ("week", "Older than week"),
    }
}

fn toggle_filter_nodes(
    toolbar: &BrowserToolbarLayout,
    model: &AppModel,
) -> Vec<AutomationNodeSnapshot> {
    let mut children = Vec::new();
    if toolbar.marked_filter_chip.width() > 1.0 {
        children.push(simple_node(
            "browser.marked_filter",
            AutomationRole::Button,
            Some(String::from("Marked filter")),
            toolbar.marked_filter_chip,
            None,
            true,
            model.browser.marked_filter_active,
            vec![String::from("toggle_browser_marked_filter")],
        ));
    }
    if toolbar.derived_label_filter_chip.width() > 1.0 {
        children.push(simple_node(
            "browser.derived_label_filter",
            AutomationRole::Button,
            Some(String::from("Derived-label filter")),
            toolbar.derived_label_filter_chip,
            None,
            true,
            model.browser.derived_label_filter_active,
            vec![String::from("toggle_browser_derived_label_filter")],
        ));
    }
    children
}

pub(super) fn browser_action_nodes(buttons: Vec<ActionButton>) -> Vec<AutomationNodeSnapshot> {
    buttons
        .into_iter()
        .map(|button| {
            simple_node(
                format!("browser.action.{}", slug(button.label)),
                AutomationRole::Button,
                Some(String::from(button.label)),
                button.rect,
                None,
                button.enabled,
                button.active,
                vec![action_slug(&button.action)],
            )
        })
        .collect()
}
