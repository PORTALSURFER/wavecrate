mod format;
mod types;

pub(crate) use types::{HotkeyAction, HotkeyGesture, HotkeyScope, KeyPress};

use crate::app::state::FocusContext;
use crate::app_core::actions::NativeUiAction;

fn map_focus_context(focus: radiant::app::FocusContextModel) -> FocusContext {
    match focus {
        radiant::app::FocusContextModel::None => FocusContext::None,
        radiant::app::FocusContextModel::Waveform => FocusContext::Waveform,
        radiant::app::FocusContextModel::SampleBrowser => FocusContext::SampleBrowser,
        radiant::app::FocusContextModel::SourceFolders => FocusContext::SourceFolders,
        radiant::app::FocusContextModel::SourcesList => FocusContext::SourcesList,
    }
}

fn map_scope(scope: radiant::app::HotkeyScope) -> HotkeyScope {
    match scope {
        radiant::app::HotkeyScope::Global => HotkeyScope::Global,
        radiant::app::HotkeyScope::Focus(focus) => HotkeyScope::Focus(map_focus_context(focus)),
    }
}

fn map_keypress(press: radiant::app::KeyPress) -> KeyPress {
    KeyPress {
        key: press.key,
        command: press.command,
        shift: press.shift,
        alt: press.alt,
    }
}

fn map_action(binding: &radiant::app::HotkeyBinding) -> HotkeyAction {
    HotkeyAction {
        id: binding.id,
        label: binding.label,
        gesture: HotkeyGesture {
            first: map_keypress(binding.gesture.first),
            chord: binding.gesture.chord.map(map_keypress),
        },
        scope: map_scope(binding.scope),
        action: binding.action.clone(),
    }
}

pub(crate) fn iter_actions() -> impl Iterator<Item = HotkeyAction> {
    radiant::app::iter_hotkey_bindings().map(map_action)
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
