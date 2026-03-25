use super::{
    GUI_ACTION_CATALOG, GuiActionKind, GuiCoverageLayer, GuiEffectClass, GuiSurface,
    action_catalog_entry_by_id, action_kind, representative_action_for_kind,
};
use crate::app_core::app_api::controller_state::DerivedNodeId;
use crate::app_core::native_bridge::{
    InteractionActionClass, catalog_dirty_source, catalog_interaction_class,
    catalog_is_immediate_waveform_preview_action, catalog_prefers_targeted_invalidation,
    catalog_uses_local_model_pull_fast_path,
};
use crate::gui_test::{GuiAivAssertion, GuiAivStep, gui_aiv_suite_manifest};
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
fn desktop_aiv_catalog_claims_are_backed_by_manifest_cases() {
    let mut covered_action_ids = BTreeSet::new();
    for pack_name in ["desktop-smoke", "desktop-regression"] {
        let manifest = gui_aiv_suite_manifest(pack_name).expect("desktop AIV manifest");
        collect_desktop_aiv_action_ids(&manifest.cases, &mut covered_action_ids);
    }
    let claimed_action_ids = GUI_ACTION_CATALOG
        .iter()
        .filter(|entry| {
            entry
                .coverage_layers
                .contains(&GuiCoverageLayer::DesktopAiv)
        })
        .map(|entry| entry.action_id.to_string())
        .collect::<BTreeSet<_>>();
    let missing_claims = claimed_action_ids
        .difference(&covered_action_ids)
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        missing_claims.is_empty(),
        "catalog DesktopAiv coverage has no matching desktop-AIV case assertions: {}",
        missing_claims.join(", ")
    );
}

#[test]
fn representative_actions_round_trip_through_kind_matcher() {
    for kind in GuiActionKind::ALL {
        let action = representative_action_for_kind(kind);
        assert_eq!(action_kind(&action), kind);
    }
}

#[test]
fn every_history_enabled_catalog_entry_has_a_transaction_handler() {
    for entry in GUI_ACTION_CATALOG {
        assert!(
            crate::app::controller::catalog_history_handler_supported(
                entry.kind,
                entry.history_policy,
            ),
            "catalog history policy {:?} for {} has no controller transaction handler",
            entry.history_policy,
            entry.action_id
        );
    }
}

#[test]
fn profiled_interaction_catalog_entries_match_expected_surfaces_and_dirty_sources() {
    for entry in GUI_ACTION_CATALOG {
        let Some(class) = catalog_interaction_class(entry.kind) else {
            continue;
        };
        match class {
            InteractionActionClass::Wheel => {
                assert_eq!(entry.surface, GuiSurface::Browser);
                assert_eq!(
                    catalog_dirty_source(entry.kind).map(|(node, _)| node),
                    Some(DerivedNodeId::BrowserState),
                );
            }
            InteractionActionClass::MapPanProxy => {
                assert_eq!(entry.surface, GuiSurface::Map);
                assert_eq!(
                    catalog_dirty_source(entry.kind).map(|(node, _)| node),
                    Some(DerivedNodeId::MapState),
                );
            }
            InteractionActionClass::Waveform => {
                assert!(
                    matches!(entry.surface, GuiSurface::Waveform | GuiSurface::Transport),
                    "waveform-profiled action {} should stay on waveform/transport surfaces",
                    entry.action_id
                );
                assert!(
                    matches!(
                        catalog_dirty_source(entry.kind).map(|(node, _)| node),
                        Some(DerivedNodeId::WaveformState)
                            | Some(DerivedNodeId::TransportState)
                            | None
                    ),
                    "waveform-profiled action {} should keep waveform/transport/queued dirty semantics",
                    entry.action_id
                );
            }
            InteractionActionClass::Volume => {
                assert_eq!(entry.surface, GuiSurface::Transport);
                assert_eq!(
                    catalog_dirty_source(entry.kind).map(|(node, _)| node),
                    Some(DerivedNodeId::TransportState),
                );
            }
        }
    }
}

#[test]
fn targeted_invalidation_catalog_entries_stay_on_browser_surface() {
    for entry in GUI_ACTION_CATALOG {
        if !catalog_prefers_targeted_invalidation(entry.kind) {
            continue;
        }
        assert_eq!(entry.surface, GuiSurface::Browser);
        assert_eq!(
            catalog_dirty_source(entry.kind).map(|(node, _)| node),
            Some(DerivedNodeId::BrowserState),
        );
    }
}

#[test]
fn immediate_waveform_preview_catalog_entries_stay_on_waveform_surface() {
    for entry in GUI_ACTION_CATALOG {
        if !catalog_is_immediate_waveform_preview_action(entry.kind) {
            continue;
        }
        assert_eq!(entry.surface, GuiSurface::Waveform);
        if let Some((node, _)) = catalog_dirty_source(entry.kind) {
            assert_eq!(node, DerivedNodeId::WaveformState);
        }
    }
}

#[test]
fn local_model_pull_fast_path_catalog_entries_remain_ui_only_actions() {
    for entry in GUI_ACTION_CATALOG {
        if !catalog_uses_local_model_pull_fast_path(entry.kind) {
            continue;
        }
        assert!(matches!(
            entry.effect_class,
            GuiEffectClass::Projection | GuiEffectClass::StateOnly
        ));
    }
}

fn collect_desktop_aiv_action_ids(
    cases: &[crate::gui_test::GuiAivCase],
    out: &mut BTreeSet<String>,
) {
    for case in cases {
        collect_desktop_aiv_step_action_ids(&case.steps, out);
        collect_desktop_aiv_assertion_action_ids(&case.expected_assertions, out);
    }
}

fn collect_desktop_aiv_step_action_ids(steps: &[GuiAivStep], out: &mut BTreeSet<String>) {
    for step in steps {
        let GuiAivStep::Assert { assertion } = step else {
            continue;
        };
        collect_desktop_aiv_assertion_action_ids(std::slice::from_ref(assertion), out);
    }
}

fn collect_desktop_aiv_assertion_action_ids(
    assertions: &[GuiAivAssertion],
    out: &mut BTreeSet<String>,
) {
    for assertion in assertions {
        let GuiAivAssertion::AssertActionRecorded { action_id } = assertion else {
            continue;
        };
        out.insert(action_id.clone());
    }
}
