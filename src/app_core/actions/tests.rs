use super::{
    GUI_ACTION_CATALOG, GuiCoverageLayer, GuiDispatchPolicy, GuiEffectClass, GuiSurface,
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
use std::fs;
use std::path::Path;

#[test]
fn catalog_contains_every_action_kind_exactly_once() {
    let mut seen = BTreeSet::new();
    for kind in GUI_ACTION_CATALOG.iter().map(|entry| entry.kind) {
        assert!(seen.insert(kind), "duplicate catalog action kind: {kind:?}");
    }
    assert_eq!(GUI_ACTION_CATALOG.len(), seen.len());
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
    for kind in GUI_ACTION_CATALOG.iter().map(|entry| entry.kind) {
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
fn targeted_invalidation_catalog_entries_stay_on_sidebar_or_browser_surfaces() {
    for entry in GUI_ACTION_CATALOG {
        if !catalog_prefers_targeted_invalidation(entry.kind) {
            continue;
        }
        assert!(
            matches!(entry.surface, GuiSurface::Browser | GuiSurface::Sources),
            "targeted invalidation action {} should stay on browser/sidebar surfaces",
            entry.action_id
        );
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

#[test]
fn runtime_internal_waveform_shift_actions_are_not_public_dispatch() {
    for action_id in [
        "begin_waveform_selection_shift",
        "begin_waveform_edit_selection_shift",
    ] {
        let entry = action_catalog_entry_by_id(action_id).expect("catalog entry");
        assert_eq!(entry.dispatch_policy, GuiDispatchPolicy::RuntimeInternal);
    }
}

#[test]
fn native_action_exports_are_owned_in_app_core() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let actions_mod =
        fs::read_to_string(manifest_dir.join("src/app_core/actions/mod.rs")).expect("actions mod");

    assert!(
        !actions_mod.contains("pub type NativeUiAction = radiant::compat::legacy_shell"),
        "NativeUiAction must stay Sempal-owned, with compatibility conversion at the runtime boundary"
    );
    assert!(
        !actions_mod.contains("pub type NativeAppModel = radiant::compat::legacy_shell"),
        "NativeAppModel must stay Sempal-owned, with compatibility conversion at the runtime boundary"
    );
    assert!(
        !actions_mod.contains("pub type NativeDirtySegments = radiant::compat::legacy_shell"),
        "NativeDirtySegments must stay Sempal-owned, with compatibility conversion at the runtime boundary"
    );
    assert!(
        actions_mod.contains("mod native_shell_actions;")
            && actions_mod.contains("mod native_shell_bridge;")
            && actions_mod.contains("mod native_shell_dtos;"),
        "Sempal-owned action, bridge, and projection DTO modules must remain explicit"
    );
    assert!(
        !actions_mod.contains("pub use radiant::compat::legacy_shell::NativeAppBridge"),
        "NativeAppBridge must stay Sempal-owned, with compatibility conversion at the runtime boundary"
    );

    let radiant_app_sources = [
        "actions/mod.rs",
        "dirty_segments.rs",
        "motion.rs",
        "shell.rs",
    ]
    .into_iter()
    .map(|file| manifest_dir.join("vendor/radiant/src/app").join(file));
    let forbidden_native_exports = [
        "pub type NativeUiAction",
        "pub enum NativeUiAction",
        "pub type NativeAppModel",
        "pub struct NativeAppModel",
        "pub type NativeDirtySegments",
        "pub struct NativeDirtySegments",
    ];
    for source_path in radiant_app_sources {
        let source = fs::read_to_string(&source_path).expect("radiant app source");
        for forbidden in forbidden_native_exports {
            assert!(
                !source.contains(forbidden),
                "{} must not define Sempal-owned {forbidden}",
                source_path.display()
            );
        }
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
