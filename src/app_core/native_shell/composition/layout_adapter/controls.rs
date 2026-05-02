//! Slotized helpers for native-shell action-button rows and toolbar partitions.

#[path = "controls/browser_toolbar.rs"]
mod browser_toolbar;
#[path = "controls/shared.rs"]
mod shared;
#[cfg(test)]
#[path = "controls/sidebar_buttons.rs"]
mod sidebar_buttons;
#[path = "controls/update_buttons.rs"]
mod update_buttons;

pub(crate) use browser_toolbar::compute_browser_toolbar_sections;
#[cfg(test)]
pub(crate) use sidebar_buttons::compute_sidebar_action_button_rects;
pub(crate) use update_buttons::compute_update_action_button_rects;

#[cfg(test)]
#[path = "controls_tests.rs"]
mod controls_tests;
