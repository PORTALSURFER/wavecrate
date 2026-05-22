//! Stable payload-free GUI action identities used across host tooling.

use super::data::gui_action_rows;
use serde::Serialize;

macro_rules! build_gui_action_kinds {
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
        /// Stable payload-free identity for one GUI action variant.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
        #[serde(rename_all = "snake_case")]
        pub enum GuiActionKind {
            $(
                #[doc = concat!("Stable payload-free identity for `", $id, "`.")]
                $kind,
            )+
        }
    };
}

gui_action_rows!(build_gui_action_kinds);
