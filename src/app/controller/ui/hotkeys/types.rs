use crate::app::state::FocusContext;
use crate::app_core::actions::NativeUiAction;
use radiant::gui::input::KeyCode;

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

impl KeyPress {
    pub(crate) const fn new(key: KeyCode) -> Self {
        Self {
            key,
            command: false,
            shift: false,
            alt: false,
        }
    }

    pub(crate) const fn with_command(key: KeyCode) -> Self {
        Self {
            key,
            command: true,
            shift: false,
            alt: false,
        }
    }

    pub(crate) const fn with_shift(key: KeyCode) -> Self {
        Self {
            key,
            command: false,
            shift: true,
            alt: false,
        }
    }

    pub(crate) const fn with_alt(key: KeyCode) -> Self {
        Self {
            key,
            command: false,
            shift: false,
            alt: true,
        }
    }
}

/// Keyboard gesture used to trigger a hotkey binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HotkeyGesture {
    pub(crate) first: KeyPress,
    pub(crate) chord: Option<KeyPress>,
}

impl HotkeyGesture {
    pub(crate) const fn new(key: KeyCode) -> Self {
        Self {
            first: KeyPress::new(key),
            chord: None,
        }
    }

    pub(crate) const fn with_command(key: KeyCode) -> Self {
        Self {
            first: KeyPress::with_command(key),
            chord: None,
        }
    }

    pub(crate) const fn with_shift(key: KeyCode) -> Self {
        Self {
            first: KeyPress::with_shift(key),
            chord: None,
        }
    }

    pub(crate) const fn with_alt(key: KeyCode) -> Self {
        Self {
            first: KeyPress::with_alt(key),
            chord: None,
        }
    }

    pub(crate) const fn with_chord(first: KeyPress, second: KeyPress) -> Self {
        Self {
            first,
            chord: Some(second),
        }
    }
}

/// Wavecrate-owned hotkey metadata for UI runtime and help surfaces.
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

/// Result of resolving one keypress against the Wavecrate-owned hotkey catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HotkeyResolution {
    pub(crate) action: Option<NativeUiAction>,
    pub(crate) handled: bool,
    pub(crate) pending_chord: Option<KeyPress>,
}
