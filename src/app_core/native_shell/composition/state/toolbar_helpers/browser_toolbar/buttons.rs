//! Browser toolbar action-button helpers.

use super::super::super::*;

pub(in crate::gui::native_shell::state) fn browser_action_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    toolbar: &BrowserToolbarLayout,
) -> Vec<ActionButton> {
    let _ = layout;
    if toolbar.action_slots.iter().all(|rect| rect.width() <= 1.0) {
        return Vec::new();
    }
    let mut buttons = Vec::new();
    if toolbar.action_slots[0].width() > 1.0 {
        buttons.push(ActionButton {
            rect: toolbar.action_slots[0],
            label: "Random",
            icon: Some(WaveformToolbarIcon::Dice),
            enabled: true,
            active: model.browser_actions.random_navigation_enabled,
            action: UiAction::ToggleRandomNavigationMode,
            text_color: if model.browser_actions.random_navigation_enabled {
                style.highlight_cyan
            } else {
                style.text_primary
            },
        });
    }
    if toolbar.action_slots[1].width() > 1.0 {
        buttons.push(ActionButton {
            rect: toolbar.action_slots[1],
            label: "Cleanup",
            icon: Some(WaveformToolbarIcon::Filter),
            enabled: true,
            active: model.browser_actions.duplicate_cleanup_active,
            action: UiAction::ToggleBrowserDuplicateCleanupMode,
            text_color: if model.browser_actions.duplicate_cleanup_active {
                style.highlight_orange
            } else {
                style.text_primary
            },
        });
    }
    if toolbar.action_slots[2].width() > 1.0 {
        buttons.push(ActionButton {
            rect: toolbar.action_slots[2],
            label: "Tags",
            icon: None,
            enabled: true,
            active: model.browser_actions.tag_sidebar_open,
            action: UiAction::ToggleBrowserTagSidebar,
            text_color: if model.browser_actions.tag_sidebar_open {
                style.highlight_cyan
            } else {
                style.text_primary
            },
        });
    }
    buttons
}
