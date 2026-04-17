//! Public GUI action catalog table and lookup helpers.
//!
//! This module owns the metadata table exposed to tests, tools, and automation.
//! It derives that table from the shared row definitions and must not redefine
//! action rows independently.

use super::super::NativeUiAction;
use super::data::gui_action_rows;
use super::policy::{gui_dispatch_policy, gui_history_policy};
use super::{GuiActionCatalogEntry, GuiActionKind, GuiCoverageLayer, GuiEffectClass, GuiSurface};

macro_rules! build_gui_action_catalog {
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
        /// Canonical host-side GUI action catalog.
        pub const GUI_ACTION_CATALOG: &[GuiActionCatalogEntry] = &[
            $(
                GuiActionCatalogEntry {
                    kind: GuiActionKind::$kind,
                    action_id: $id,
                    surface: GuiSurface::$surface,
                    effect_class: GuiEffectClass::$effect,
                    dispatch_policy: gui_dispatch_policy(GuiActionKind::$kind),
                    history_policy: gui_history_policy(GuiActionKind::$kind),
                    coverage_layers: &[$(GuiCoverageLayer::$coverage),+],
                    default_fixture_tags: &[$($fixture),*],
                },
            )+
        ];
    };
}

gui_action_rows!(build_gui_action_catalog);

/// Return the catalog entry for one concrete native action.
pub fn action_catalog_entry(action: &NativeUiAction) -> &'static GuiActionCatalogEntry {
    action_catalog_entry_by_kind(super::action_kind(action))
}

/// Resolve one catalog entry by stable action identifier.
pub fn action_catalog_entry_by_id(action_id: &str) -> Option<&'static GuiActionCatalogEntry> {
    GUI_ACTION_CATALOG
        .iter()
        .find(|entry| entry.action_id == action_id)
}

/// Resolve one catalog entry by action kind.
pub(super) fn action_catalog_entry_by_kind(kind: GuiActionKind) -> &'static GuiActionCatalogEntry {
    GUI_ACTION_CATALOG
        .iter()
        .find(|entry| entry.kind == kind)
        .unwrap_or_else(|| panic!("missing GUI action catalog entry for {kind:?}"))
}

#[cfg(test)]
mod tests {
    use super::action_catalog_entry_by_kind;
    use crate::app_core::actions::GuiActionKind;

    #[test]
    fn action_catalog_entry_by_kind_resolves_every_kind() {
        for kind in GuiActionKind::ALL {
            assert_eq!(action_catalog_entry_by_kind(kind).kind, kind);
        }
    }
}
