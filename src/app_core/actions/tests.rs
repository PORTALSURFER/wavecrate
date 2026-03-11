use super::{
    GUI_ACTION_CATALOG, GuiActionKind, action_catalog_entry_by_id, action_kind,
    representative_action_for_kind,
};
use std::collections::BTreeSet;

#[test]
fn catalog_contains_every_action_kind_exactly_once() {
    assert_eq!(GUI_ACTION_CATALOG.len(), GuiActionKind::ALL.len());
    let mut seen = BTreeSet::new();
    for kind in GuiActionKind::ALL {
        assert!(
            seen.insert(kind),
            "duplicate GuiActionKind declaration in ALL: {kind:?}"
        );
        assert!(
            GUI_ACTION_CATALOG.iter().any(|entry| entry.kind == kind),
            "missing catalog entry for {kind:?}"
        );
    }
}

#[test]
fn catalog_action_ids_are_unique_and_resolvable() {
    let mut ids = BTreeSet::new();
    for entry in GUI_ACTION_CATALOG {
        assert!(
            ids.insert(entry.action_id),
            "duplicate action id {}",
            entry.action_id
        );
        assert_eq!(action_catalog_entry_by_id(entry.action_id), Some(entry));
    }
}

#[test]
fn every_catalog_entry_declares_required_coverage() {
    for entry in GUI_ACTION_CATALOG {
        assert!(
            !entry.coverage_layers.is_empty(),
            "catalog entry {} is missing coverage layers",
            entry.action_id
        );
        let mut layers = BTreeSet::new();
        for layer in entry.coverage_layers {
            assert!(
                layers.insert(layer),
                "catalog entry {} repeats coverage layer {:?}",
                entry.action_id,
                layer
            );
        }
    }
}

#[test]
fn representative_actions_round_trip_through_kind_matcher() {
    for kind in GuiActionKind::ALL {
        let action = representative_action_for_kind(kind);
        assert_eq!(action_kind(&action), kind);
    }
}
