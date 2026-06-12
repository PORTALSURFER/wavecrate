//! Domain-oriented catalog views derived from the canonical action table.
//!
//! The catalog rows remain a single source of truth in `data.rs`; this module
//! provides the domain ownership index used by tests and future catalog edits.

use super::{GUI_ACTION_CATALOG, GuiActionCatalogEntry, representative_action_for_kind};
use crate::app_core::actions::NativeUiActionDomain;

/// Stable domain order for catalog ownership checks.
pub const GUI_ACTION_CATALOG_DOMAINS: &[NativeUiActionDomain] = &[
    NativeUiActionDomain::ColumnTriage,
    NativeUiActionDomain::Transport,
    NativeUiActionDomain::Shell,
    NativeUiActionDomain::SourcesAndFolders,
    NativeUiActionDomain::Browser,
    NativeUiActionDomain::PromptsAndEdits,
    NativeUiActionDomain::Options,
    NativeUiActionDomain::Waveform,
    NativeUiActionDomain::HistoryAndUpdates,
];

/// Iterate catalog entries owned by one UI action domain.
pub fn action_catalog_entries_by_domain(
    domain: NativeUiActionDomain,
) -> impl Iterator<Item = &'static GuiActionCatalogEntry> {
    GUI_ACTION_CATALOG
        .iter()
        .filter(move |entry| representative_action_for_kind(entry.kind).domain() == domain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::actions::{GUI_ACTION_CATALOG, GuiActionKind};
    use std::collections::BTreeSet;

    #[test]
    fn every_catalog_domain_owns_entries() {
        for domain in GUI_ACTION_CATALOG_DOMAINS {
            let entries = action_catalog_entries_by_domain(*domain).collect::<Vec<_>>();
            assert!(!entries.is_empty(), "empty catalog domain: {domain:?}");
        }
    }

    #[test]
    fn catalog_domain_views_partition_catalog_entries() {
        let mut seen = BTreeSet::new();
        for domain in GUI_ACTION_CATALOG_DOMAINS {
            for entry in action_catalog_entries_by_domain(*domain) {
                assert!(
                    seen.insert(entry.kind),
                    "catalog entry appears in multiple domains: {:?}",
                    entry.kind
                );
            }
        }

        let all = GUI_ACTION_CATALOG
            .iter()
            .map(|entry| entry.kind)
            .collect::<BTreeSet<_>>();
        assert_eq!(seen, all);
    }

    #[test]
    fn catalog_domain_views_match_representative_action_domains() {
        for domain in GUI_ACTION_CATALOG_DOMAINS {
            for entry in action_catalog_entries_by_domain(*domain) {
                assert_eq!(
                    representative_action_for_kind(entry.kind).domain(),
                    *domain,
                    "catalog entry {} is in the wrong domain",
                    entry.action_id
                );
            }
        }
    }

    #[test]
    fn adding_actions_has_obvious_domain_locations() {
        let cases = [
            (
                NativeUiActionDomain::ColumnTriage,
                GuiActionKind::SelectColumn,
            ),
            (
                NativeUiActionDomain::Transport,
                GuiActionKind::ToggleTransport,
            ),
            (
                NativeUiActionDomain::Options,
                GuiActionKind::OpenOptionsMenu,
            ),
            (
                NativeUiActionDomain::SourcesAndFolders,
                GuiActionKind::FocusSourceRow,
            ),
            (
                NativeUiActionDomain::Browser,
                GuiActionKind::MoveBrowserFocus,
            ),
            (
                NativeUiActionDomain::PromptsAndEdits,
                GuiActionKind::ConfirmPrompt,
            ),
            (
                NativeUiActionDomain::Options,
                GuiActionKind::SetInputMonitoringEnabled,
            ),
            (
                NativeUiActionDomain::Waveform,
                GuiActionKind::SeekWaveformPrecise,
            ),
            (
                NativeUiActionDomain::HistoryAndUpdates,
                GuiActionKind::CheckForUpdates,
            ),
        ];

        for (domain, expected_kind) in cases {
            assert!(
                action_catalog_entries_by_domain(domain).any(|entry| entry.kind == expected_kind),
                "{expected_kind:?} should be discoverable in {domain:?}"
            );
        }
    }
}
