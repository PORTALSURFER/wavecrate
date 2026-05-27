use super::*;
use crate::app_core::native_shell::runtime_contract::BrowserPillState;

/// Build automation nodes for the sidebar tag editor.
pub(super) fn tags_group(rect: Rect, model: &AppModel) -> AutomationNodeSnapshot {
    let sidebar = model.browser.pill_editor();
    let mut children = Vec::new();
    children.push(simple_node(
        "sources.tags.expand",
        AutomationRole::Button,
        Some(String::from("Tag library")),
        sidebar_tag_expand_button_rect_for_automation(rect),
        Some(if model.browser_actions.pill_editor_open() {
            String::from("open")
        } else {
            String::from("closed")
        }),
        true,
        model.browser_actions.pill_editor_open(),
        vec![String::from("toggle_browser_pill_editor")],
    ));
    children.push(simple_node(
        "sources.tags.input",
        AutomationRole::SearchField,
        Some(String::from("Add tag")),
        sidebar_tag_input_rect_for_automation(rect),
        Some(sidebar.input_value.clone()),
        true,
        false,
        vec![
            String::from("focus_browser_pill_editor_input"),
            String::from("set_browser_pill_editor_input"),
            String::from("commit_browser_pill_editor_input"),
        ],
    ));
    children.extend(
        sidebar
            .accepted_pills
            .iter()
            .enumerate()
            .map(|(index, pill)| AutomationNodeSnapshot {
                id: node_id(format!("sources.tags.accepted.{index}.{}", slug(&pill.id))),
                role: AutomationRole::Button,
                label: Some(pill.label.clone()),
                bounds: bounds(rect),
                value: Some(format!("{:?}", pill.state)),
                enabled: true,
                selected: true,
                available_actions: vec![String::from("toggle_browser_pill_option")],
                metadata: metadata(&[("pill_id", pill.id.as_str()), ("chip_kind", "accepted")]),
                children: Vec::new(),
            }),
    );
    children.extend(
        sidebar
            .option_pills
            .iter()
            .enumerate()
            .map(|(index, pill)| AutomationNodeSnapshot {
                id: node_id(format!("sources.tags.suggestion.{index}")),
                role: AutomationRole::Button,
                label: Some(pill.label.clone()),
                bounds: bounds(rect),
                value: Some(format!("{:?}", pill.state)),
                enabled: true,
                selected: !matches!(pill.state, BrowserPillState::Off),
                available_actions: vec![String::from("toggle_browser_pill_option")],
                metadata: metadata(&[("pill_id", pill.id.as_str())]),
                children: Vec::new(),
            }),
    );
    if let Some(pill) = sidebar.create_pill.as_ref() {
        children.push(AutomationNodeSnapshot {
            id: node_id(format!("sources.tags.create_tag.{}", slug(&pill.id))),
            role: AutomationRole::Button,
            label: Some(pill.label.clone()),
            bounds: bounds(rect),
            value: Some(format!("{:?}", pill.state)),
            enabled: true,
            selected: false,
            available_actions: vec![String::from("commit_browser_pill_editor_input")],
            metadata: metadata(&[("pill_id", pill.id.as_str())]),
            children: Vec::new(),
        });
    }
    let option_pill_labels = sidebar
        .option_pills
        .iter()
        .map(|pill| pill.label.as_str())
        .collect::<Vec<_>>()
        .join("|");
    let accepted_pill_labels = sidebar
        .accepted_pills
        .iter()
        .map(|pill| pill.label.as_str())
        .collect::<Vec<_>>()
        .join("|");
    AutomationNodeSnapshot {
        id: node_id("sources.tags"),
        role: AutomationRole::Group,
        label: Some(String::from("Tags")),
        bounds: bounds(rect),
        value: Some(sidebar.header_label.clone()),
        enabled: true,
        selected: false,
        available_actions: vec![String::from("focus_browser_pill_editor_input")],
        metadata: metadata(&[
            ("selected_count", &sidebar.selected_count.to_string()),
            ("accepted_tag_labels", accepted_pill_labels.as_str()),
            ("normal_tag_labels", option_pill_labels.as_str()),
        ]),
        children,
    }
}

fn sidebar_tag_expand_button_rect_for_automation(rect: Rect) -> Rect {
    let pad = 6.0;
    let side = 14.0;
    Rect::from_min_max(
        Point::new(rect.max.x - pad - side, rect.min.y + 3.0),
        Point::new(rect.max.x - pad, rect.min.y + 3.0 + side),
    )
}

/// Return the sidebar tag input bounds used by automation snapshots.
fn sidebar_tag_input_rect_for_automation(rect: Rect) -> Rect {
    let pad = 6.0;
    let height = 18.0;
    Rect::from_min_max(
        Point::new(
            rect.min.x + pad,
            (rect.max.y - pad - height).max(rect.min.y + pad),
        ),
        Point::new(rect.max.x - pad, rect.max.y - pad),
    )
}
