mod actions;
mod format;
mod types;

pub(crate) use types::{HotkeyAction, HotkeyScope};

use crate::app::state::FocusContext;
use crate::app_core::actions::NativeUiAction;

pub(crate) fn iter_actions() -> impl Iterator<Item = HotkeyAction> {
    actions::HOTKEY_ACTIONS.iter().cloned()
}

pub(crate) fn focused_actions(focus: FocusContext) -> Vec<HotkeyAction> {
    iter_actions()
        .filter(|action| matches!(action.scope, HotkeyScope::Focus(_)) && action.is_active(focus))
        .collect()
}

pub(crate) fn global_actions() -> Vec<HotkeyAction> {
    iter_actions()
        .filter(|action| matches!(action.scope, HotkeyScope::Global))
        .collect()
}

pub(crate) fn find_action(predicate: impl Fn(&NativeUiAction) -> bool) -> Option<HotkeyAction> {
    iter_actions().find(|action| predicate(&action.action))
}
