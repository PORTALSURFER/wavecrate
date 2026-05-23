use super::*;
use crate::app_core::native_shell::runtime_contract::BrowserPillState;

pub(super) fn build_browser_pill_editor_automation(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> Option<AutomationNodeSnapshot> {
    let rect = browser_pill_editor_panel_rect(layout.browser_rows, style.sizing, model)?;
    let sidebar = &model.browser.pill_editor;
    let option_pill_labels = sidebar
        .option_pills
        .iter()
        .map(|pill| pill.label.as_str())
        .collect::<Vec<_>>()
        .join("|");
    let mut children = vec![
        simple_node(
            "browser.pill_editor.input",
            AutomationRole::SearchField,
            Some(sidebar.input_placeholder.clone()),
            rect,
            Some(sidebar.input_value.clone()),
            true,
            false,
            vec![
                String::from("focus_browser_pill_editor_input"),
                String::from("set_browser_pill_editor_input"),
                String::from("commit_browser_pill_editor_input"),
            ],
        ),
        browser_pill_editor_pill_node(
            "browser.pill_editor.exclusive.0",
            &sidebar.exclusive_pills[0],
            rect,
            vec![String::from("set_browser_sidebar_looped")],
        ),
        browser_pill_editor_pill_node(
            "browser.pill_editor.exclusive.1",
            &sidebar.exclusive_pills[1],
            rect,
            vec![String::from("set_browser_sidebar_looped")],
        ),
    ];
    children.extend(sidebar.option_pills.iter().map(|pill| {
        browser_pill_editor_pill_node(
            format!("browser.pill_editor.option.{}", slug(&pill.label)),
            pill,
            rect,
            vec![String::from("toggle_browser_pill_option")],
        )
    }));
    if let Some(pill) = sidebar.create_pill.as_ref() {
        children.push(browser_pill_editor_pill_node(
            format!("browser.pill_editor.create.{}", slug(&pill.id)),
            pill,
            rect,
            vec![String::from("toggle_browser_pill_option")],
        ));
    }
    let mut node = simple_node(
        "browser.pill_editor",
        AutomationRole::Panel,
        Some(String::from("Pill editor")),
        rect,
        Some(sidebar.header_label.clone()),
        true,
        false,
        Vec::new(),
    );
    node.metadata = metadata(&[
        ("selected_count", &sidebar.selected_count.to_string()),
        (
            "primary_action_enabled",
            bool_text(sidebar.primary_action_enabled),
        ),
        ("option_pill_labels", &option_pill_labels),
    ]);
    node.children = children;
    Some(node)
}

fn browser_pill_editor_pill_node(
    id: impl Into<String>,
    pill: &crate::app_core::native_shell::runtime_contract::BrowserPillModel,
    rect: Rect,
    available_actions: Vec<String>,
) -> AutomationNodeSnapshot {
    let state = match pill.state {
        BrowserPillState::Off => "off",
        BrowserPillState::On => "on",
        BrowserPillState::Mixed => "mixed",
    };
    let mut node = simple_node(
        id,
        AutomationRole::Button,
        Some(pill.label.clone()),
        rect,
        Some(state.to_string()),
        true,
        pill.state == BrowserPillState::On,
        available_actions,
    );
    node.metadata = metadata(&[("pill_state", state), ("pill_id", pill.id.as_str())]);
    node
}
