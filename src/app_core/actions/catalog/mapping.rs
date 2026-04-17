//! GUI action kind/sample mapping derived from the shared catalog rows.
//!
//! This module owns payload-to-kind matching and representative sample payload
//! generation. It must not redefine catalog metadata or policy tables.

use super::super::NativeUiAction;
use super::GuiActionKind;
use super::data::gui_action_rows;

macro_rules! build_action_mapping {
    ($(
        $kind:ident $pattern:tt => {
            id: $id:literal,
            surface: $surface:ident,
            effect: $effect:ident,
            coverage: [$($coverage:ident),+ $(,)?],
            fixtures: [$($fixture:literal),* $(,)?],
            sample: $sample:expr
        }
    ),+ $(,)?) => {
        /// Return the payload-free kind for one concrete native UI action.
        pub fn action_kind(action: &NativeUiAction) -> GuiActionKind {
            match action {
                $(build_action_mapping!(@match $kind $pattern) => GuiActionKind::$kind,)+
            }
        }

        /// Return a representative action payload for the provided kind.
        pub fn representative_action_for_kind(kind: GuiActionKind) -> NativeUiAction {
            match kind {
                $(GuiActionKind::$kind => $sample,)+
            }
        }
    };
    (@match $kind:ident {}) => {
        NativeUiAction::$kind
    };
    (@match $kind:ident { $($field:ident),+ }) => {
        NativeUiAction::$kind { $($field: _,)+ .. }
    };
}

gui_action_rows!(build_action_mapping);
