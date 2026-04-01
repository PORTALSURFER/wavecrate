//! Canonical GUI action catalog used by host-side tests, tools, and automation metadata.

mod coverage;
mod entries;
mod kinds;

pub use self::coverage::{
    GuiActionCatalogEntry, GuiCoverageLayer, GuiDispatchPolicy, GuiEffectClass,
    GuiHistoryPolicy, GuiSurface,
};
pub use self::entries::{
    GUI_ACTION_CATALOG, action_catalog_entry, action_catalog_entry_by_id, action_kind,
    representative_action_for_kind,
};
pub use self::kinds::GuiActionKind;
