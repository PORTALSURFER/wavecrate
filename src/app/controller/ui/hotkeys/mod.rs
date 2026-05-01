mod actions;
mod format;
mod types;

pub(crate) use types::{HotkeyAction, HotkeyResolution, HotkeyScope, KeyPress};

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

pub(crate) fn resolve_hotkey_press(
    pending_chord: Option<KeyPress>,
    press: KeyPress,
    focus: FocusContext,
) -> HotkeyResolution {
    if let Some(first) = pending_chord {
        if let Some(action) = actions::HOTKEY_ACTIONS.iter().find(|action| {
            action.is_active(focus)
                && action.gesture.first == first
                && action.gesture.chord == Some(press)
        }) {
            return HotkeyResolution {
                action: Some(action.action.clone()),
                handled: true,
                pending_chord: None,
            };
        }

        if actions::HOTKEY_ACTIONS
            .iter()
            .any(|action| action.gesture.first == press && action.gesture.chord.is_some())
        {
            return HotkeyResolution {
                action: None,
                handled: true,
                pending_chord: Some(press),
            };
        }

        return HotkeyResolution {
            action: None,
            handled: true,
            pending_chord: None,
        };
    }

    if let Some(action) = actions::HOTKEY_ACTIONS
        .iter()
        .find(|action| action.is_active(focus) && action.gesture.first == press)
        && action.gesture.chord.is_none()
    {
        return HotkeyResolution {
            action: Some(action.action.clone()),
            handled: true,
            pending_chord: None,
        };
    }

    if actions::HOTKEY_ACTIONS.iter().any(|action| {
        action.is_active(focus) && action.gesture.first == press && action.gesture.chord.is_some()
    }) {
        return HotkeyResolution {
            action: None,
            handled: true,
            pending_chord: Some(press),
        };
    }

    HotkeyResolution {
        action: None,
        handled: false,
        pending_chord: None,
    }
}
