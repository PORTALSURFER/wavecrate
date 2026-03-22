use crate::app::state::FocusContext;
use crate::app_core::actions::NativeUiAction;
use crate::gui::input::KeyCode;

/// Identifies the section that owns a hotkey binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HotkeyScope {
    Global,
    Focus(FocusContext),
}

impl HotkeyScope {
    pub(crate) fn matches(self, focus: FocusContext) -> bool {
        match self {
            Self::Global => true,
            Self::Focus(target) => target == focus,
        }
    }
}

/// A single keypress plus modifier state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KeyPress {
    pub(crate) key: KeyCode,
    pub(crate) command: bool,
    pub(crate) shift: bool,
    pub(crate) alt: bool,
}

/// Keyboard gesture used to trigger a hotkey binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HotkeyGesture {
    pub(crate) first: KeyPress,
    pub(crate) chord: Option<KeyPress>,
}

/// Hotkey metadata mirrored from the shared `radiant` catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HotkeyAction {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) gesture: HotkeyGesture,
    pub(crate) scope: HotkeyScope,
    pub(crate) action: NativeUiAction,
}

impl HotkeyAction {
    pub(crate) fn is_active(&self, focus: FocusContext) -> bool {
        self.scope.matches(focus)
    }

    pub(crate) fn is_global(&self) -> bool {
        matches!(self.scope, HotkeyScope::Global)
    }
}
