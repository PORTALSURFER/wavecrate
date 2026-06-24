use std::collections::{BTreeMap, BTreeSet};

use super::*;

#[test]
fn registered_native_app_widget_ids_are_unique() {
    let mut values = BTreeMap::new();
    let mut constants = BTreeSet::new();
    let mut stable_names = BTreeSet::new();

    for id in REGISTERED_WIDGET_IDS {
        assert_ne!(
            id.value, 0,
            "{} must not use the zero widget id",
            id.constant
        );
        assert!(
            constants.insert(id.constant),
            "{} is registered more than once",
            id.constant
        );
        assert!(
            stable_names.insert(id.stable_name),
            "{} is registered more than once",
            id.stable_name
        );
        if let Some(previous) = values.insert(id.value, id) {
            panic!(
                "duplicate native app widget id {:#x}: {} and {}",
                id.value, previous.constant, id.constant
            );
        }
    }
}

#[test]
fn registered_native_app_widget_ids_stay_inside_owner_namespaces() {
    for id in REGISTERED_WIDGET_IDS {
        assert!(
            id.owner.namespace().contains(id.value),
            "{} ({}) must stay inside the {:?} widget id namespace",
            id.constant,
            id.stable_name,
            id.owner
        );
    }
}
