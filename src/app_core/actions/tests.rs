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
fn native_app_default_keeps_sempal_product_labels_owned_in_app_core() {
    let model = crate::app_core::actions::NativeAppModel::default();

    assert_eq!(model.columns[1].title, "Samples");
    assert_eq!(model.browser_chrome.samples_tab_label, "Samples");
    assert_eq!(model.browser_chrome.map_tab_label, "Similarity map");
    assert_eq!(
        model.browser_chrome.search_placeholder,
        "Search samples (Ctrl+F)"
    );
}

#[test]
fn native_runtime_contract_round_trips_representative_actions() {
    for entry in GUI_ACTION_CATALOG {
        let action = representative_action_for_kind(entry.kind);
        let runtime_action: crate::app_core::native_shell::runtime_contract::UiAction =
            action.clone().into();
        let product_round_trip =
            crate::app_core::actions::NativeUiAction::from(runtime_action.clone());
        assert_eq!(
            product_round_trip, action,
            "native -> runtime -> native conversion changed {}",
            entry.action_id
        );

        let runtime_round_trip: crate::app_core::native_shell::runtime_contract::UiAction =
            product_round_trip.into();
        assert_eq!(
            runtime_round_trip, runtime_action,
            "runtime conversion is not stable for {}",
            entry.action_id
        );
    }
}

#[test]
fn native_runtime_contract_preserves_intentional_semantic_translations() {
    use crate::app_core::actions::{NativeBrowserTagTarget, NativeUiAction};
    use crate::app_core::native_shell::runtime_contract;

    let action_cases = [
        (
            NativeUiAction::FocusLoadedSampleInBrowser,
            runtime_contract::UiAction::FocusLoadedContentInList,
        ),
        (
            NativeUiAction::SetCompareAnchorFromFocusedBrowserSample,
            runtime_contract::UiAction::SetCompareAnchorFromFocusedContent,
        ),
        (
            NativeUiAction::ToggleBrowserSampleMark,
            runtime_contract::UiAction::ToggleContentMark,
        ),
        (
            NativeUiAction::ToggleFindSimilarFocusedSample,
            runtime_contract::UiAction::ToggleFindSimilarFocusedContent,
        ),
        (
            NativeUiAction::NormalizeFocusedBrowserSample,
            runtime_contract::UiAction::NormalizeFocusedContentItem,
        ),
        (
            NativeUiAction::PlayRandomSample,
            runtime_contract::UiAction::PlayRandomContentItem,
        ),
        (
            NativeUiAction::PlayPreviousRandomSample,
            runtime_contract::UiAction::PlayPreviousRandomContentItem,
        ),
        (
            NativeUiAction::MoveTrashedSamplesToFolder,
            runtime_contract::UiAction::MoveDiscardedItemsToFolder,
        ),
    ];

    for (product_action, runtime_action) in action_cases {
        let converted_runtime: runtime_contract::UiAction = product_action.clone().into();
        assert_eq!(converted_runtime, runtime_action);
        let converted_product = NativeUiAction::from(runtime_action.clone());
        assert_eq!(converted_product, product_action);
    }

    assert_eq!(
        runtime_contract::BrowserTriageTarget::from(NativeBrowserTagTarget::Trash),
        runtime_contract::BrowserTriageTarget::Negative
    );
    assert_eq!(
        NativeBrowserTagTarget::from(runtime_contract::BrowserTriageTarget::Positive),
        NativeBrowserTagTarget::Keep
    );
}

#[test]
fn post_cutover_compatibility_facades_are_absent() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    assert!(
        !manifest_dir.join("src/compat_app_contract.rs").exists(),
        "the temporary local runtime-contract facade should be deleted"
    );
    assert!(
        !manifest_dir
            .join("vendor/radiant/src/compat/legacy_shell")
            .exists(),
        "Radiant's removed legacy compatibility tree must stay absent"
    );
}

#[test]
fn native_projection_dtos_keep_product_defaults_and_generic_primitives() {
    let model = crate::app_core::actions::NativeAppModel::default();

    assert_eq!(model.columns[1].title, "Samples");
    assert_eq!(model.browser_chrome.samples_tab_label, "Samples");
    assert_eq!(model.browser_chrome.map_tab_label, "Similarity map");
    assert_eq!(
        model.browser_chrome.search_placeholder,
        "Search samples (Ctrl+F)"
    );
    assert_eq!(
        model
            .sources
            .folder_pane(crate::app_core::actions::NativeFolderPaneIdModel::Upper),
        &model.sources.upper_folder_pane
    );
    assert_eq!(
        model
            .sources
            .folder_pane(crate::app_core::actions::NativeFolderPaneIdModel::Lower),
        &model.sources.lower_folder_pane
    );

    let preferences = model.options_panel.preference_state();
    assert_eq!(preferences.toggles.len(), 4);
    assert_eq!(model.waveform.timeline_surface().markers.len(), 0);
    assert!(model.waveform_chrome.signal_tools().markers_visible);
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
