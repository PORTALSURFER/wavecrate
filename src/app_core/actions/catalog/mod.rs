//! Canonical GUI action catalog used by host-side tests, tools, and automation metadata.

mod catalog_table;
mod coverage;
mod data;
mod domain;
mod kinds;
mod mapping;
mod policy;

pub use self::catalog_table::{
    GUI_ACTION_CATALOG, action_catalog_entry, action_catalog_entry_by_id,
};
pub use self::coverage::{
    GuiActionCatalogEntry, GuiCoverageLayer, GuiDispatchPolicy, GuiEffectClass, GuiHistoryPolicy,
    GuiSurface,
};
pub use self::domain::{GUI_ACTION_CATALOG_DOMAINS, action_catalog_entries_by_domain};
pub use self::kinds::GuiActionKind;
pub use self::mapping::{action_kind, representative_action_for_kind};
