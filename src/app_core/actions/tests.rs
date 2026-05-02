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

    let native_dtos =
        fs::read_to_string(manifest_dir.join("src/app_core/actions/native_shell_dtos.rs"))
            .expect("native shell dtos");
    let native_actions =
        fs::read_to_string(manifest_dir.join("src/app_core/actions/native_shell_actions.rs"))
            .expect("native shell actions");
    let native_hit_testing = fs::read_to_string(
        manifest_dir.join("src/app_core/native_shell/composition/state/hit_testing/browser.rs"),
    )
    .expect("native browser hit testing");
    assert!(
        !native_dtos.contains("pub struct RetainedVec"),
        "retained snapshot storage should use the Radiant-owned generic primitive"
    );
    assert!(
        native_dtos.contains("pub type RetainedVec<T> = retained::RetainedVec<T>;"),
        "Sempal native DTOs should alias the generic Radiant retained storage primitive"
    );
    assert!(
        !native_dtos.contains("pub struct AutomationNodeId"),
        "automation node IDs should use the Radiant-owned generic primitive"
    );
    assert!(
        !native_dtos.contains("pub struct AutomationBounds"),
        "automation bounds should use the Radiant-owned generic primitive"
    );
    assert!(
        native_dtos.contains("pub type AutomationNodeId = automation::AutomationNodeId;")
            && native_dtos.contains("pub type AutomationBounds = automation::AutomationBounds;"),
        "Sempal native automation DTOs should alias generic Radiant automation primitives"
    );
    assert!(
        native_dtos.contains("pub enum AutomationRole")
            && native_dtos.contains("WaveformRegion")
            && native_dtos.contains("MapCanvas")
            && native_dtos.contains("MapPoint"),
        "Sempal should own product automation role names at the app boundary"
    );
    assert!(
        native_dtos.contains("compat::AutomationRole::TimelineRegion => Self::WaveformRegion")
            && native_dtos.contains("compat::AutomationRole::SpatialCanvas => Self::MapCanvas")
            && native_dtos.contains("compat::AutomationRole::SpatialPoint => Self::MapPoint")
            && native_dtos.contains("AutomationRole::WaveformRegion => Self::TimelineRegion")
            && native_dtos.contains("AutomationRole::MapCanvas => Self::SpatialCanvas")
            && native_dtos.contains("AutomationRole::MapPoint => Self::SpatialPoint"),
        "Sempal automation DTO conversion should map product role names onto generic Radiant roles"
    );
    assert!(
        !native_dtos.contains("pub struct FrameBuildResult"),
        "frame feedback should use the Radiant-owned generic primitive"
    );
    assert!(
        native_dtos.contains("pub type FrameBuildResult = frame::FrameBuildResult;"),
        "Sempal native frame feedback should alias the generic Radiant frame primitive"
    );
    assert!(
        !native_dtos.contains("pub struct NormalizedRangeModel"),
        "normalized ranges should use the Radiant-owned generic primitive"
    );
    assert!(
        native_dtos.contains("pub type NormalizedRangeModel = range::NormalizedRange;"),
        "Sempal native range fields should alias the generic Radiant range primitive"
    );
    assert!(
        !native_dtos.contains("pub struct StatusBarModel"),
        "status chrome should use the Radiant-owned generic primitive"
    );
    assert!(
        native_dtos.contains("pub type StatusBarModel = chrome::StatusSegments;"),
        "Sempal status chrome should alias the generic Radiant chrome primitive"
    );
    assert!(
        !native_dtos.contains("pub struct ProgressOverlayModel")
            && !native_dtos.contains("pub struct DragOverlayModel"),
        "feedback overlays should use Radiant-owned generic primitives"
    );
    assert!(
        native_dtos.contains("pub type ProgressOverlayModel = feedback::ProgressOverlay;")
            && native_dtos.contains("pub type DragOverlayModel = feedback::DragOverlay;"),
        "Sempal feedback overlays should alias generic Radiant feedback primitives"
    );
    assert!(
        !native_dtos.contains("pub struct ColumnModel")
            && !native_dtos.contains("pub enum FolderRowKind")
            && !native_dtos.contains("pub struct FolderRowModel")
            && !native_dtos.contains("pub struct FolderPaneModel")
            && !native_dtos.contains("pub struct FolderActionsModel"),
        "generic list column, editable-row kind, editable-tree row/pane, and editable-tree actions should use Radiant-owned primitives"
    );
    assert!(
        native_dtos.contains("pub type ColumnModel = list::ColumnSummary;")
            && native_dtos.contains("pub type FolderPaneIdModel = panel::SplitPaneSlot;")
            && native_dtos
                .contains("pub type FolderPaneModel = panel::SplitPaneTreePanel<FolderRowModel>;")
            && native_dtos.contains("pub type SourceRowModel = panel::SplitPaneAssignedRow;")
            && native_dtos.contains("pub type FolderRowKind = list::EditableRowKind;")
            && native_dtos.contains("pub type FolderRowModel = list::EditableTreeRow;")
            && native_dtos.contains("pub type FolderActionsModel = list::EditableTreeActions;"),
        "Sempal source/sidebar DTOs should alias generic Radiant list primitives"
    );
    assert!(
        !native_dtos.contains("pub enum BrowserRowProcessingState")
            && !native_dtos.contains("pub struct BrowserRowModel")
            && !native_dtos.contains("pub enum PlaybackAgeFilterChip")
            && !native_dtos.contains("pub enum PlaybackAgeBucket")
            && !native_dtos.contains("pub enum BrowserTagState")
            && !native_dtos.contains("pub struct BrowserTagPillModel")
            && !native_dtos.contains("pub struct BrowserTagSidebarModel")
            && !native_dtos.contains("pub enum MapRenderModeModel")
            && !native_dtos.contains("pub struct MapPanelModel")
            && !native_dtos.contains("pub struct WaveformSlicePreviewModel")
            && !native_dtos.contains("pub enum UpdateStatusModel"),
        "generic row state, pill state/panel, map mode/panel, and update status DTOs should use Radiant-owned primitives"
    );
    assert!(
        native_dtos.contains("pub type BrowserRowProcessingState = list::RowProcessingState;")
            && native_dtos.contains("pub type BrowserRowModel = list::ContentListRow;")
            && native_dtos.contains("pub type PlaybackAgeFilterChip = list::RecencyFilterChip;")
            && native_dtos.contains("pub type PlaybackAgeBucket = list::RecencyBucket;")
            && native_dtos.contains("pub type BrowserTagState = selection::TriState;")
            && native_dtos
                .contains("pub type BrowserTagPillModel = badge::SelectablePill<BrowserTagState>;")
            && native_dtos.contains(
                "pub type BrowserTagSidebarModel = badge::PillEditorPanel<BrowserTagState>;"
            )
            && native_dtos
                .contains("pub type MapRenderModeModel = visualization::PointRenderMode;")
            && native_dtos.contains("pub type MapPanelModel = visualization::SpatialPanel;")
            && native_dtos
                .contains("pub type WaveformChannelViewModel = visualization::ChannelViewMode;")
            && native_dtos.contains(
                "pub type WaveformSlicePreviewModel = visualization::TimelineMarkerPreview;"
            )
            && native_dtos.contains("pub type MapPointModel = visualization::SpatialPoint;")
            && native_dtos.contains("pub type UpdateStatusModel = feedback::UpdateStatus;"),
        "Sempal native DTOs should alias generic Radiant state primitives"
    );
    assert!(
        !native_dtos.contains("pub struct MapPointModel"),
        "map point geometry should use the Radiant-owned generic spatial point primitive"
    );
    assert!(
        !native_dtos.contains("pub enum WaveformChannelViewModel"),
        "waveform channel view should use the Radiant-owned generic channel-view primitive"
    );
    assert!(
        !native_dtos.contains("pub enum FolderPaneIdModel"),
        "split pane identity should use the Radiant-owned generic panel primitive"
    );
    assert!(
        native_dtos.contains("pub enum FocusContextModel")
            && native_dtos.contains("SampleBrowser")
            && native_dtos.contains("SourceFolders"),
        "Sempal should own focus-context names for product surfaces"
    );
    assert!(
        !native_dtos.contains("pub struct SourceRowModel"),
        "split pane row projection should use the Radiant-owned generic panel primitive"
    );
    assert!(
        !native_dtos.contains("pub struct UpdatePanelModel"),
        "update panels should use the Radiant-owned generic feedback primitive"
    );
    assert!(
        native_dtos.contains("pub type UpdatePanelModel = feedback::UpdatePanel;"),
        "Sempal native update panels should alias the generic Radiant feedback primitive"
    );
    assert!(
        !native_dtos.contains("pub struct ConfirmPromptModel"),
        "confirm prompts should use the Radiant-owned generic prompt primitive with a Sempal prompt kind"
    );
    assert!(
        native_dtos
            .contains("pub type ConfirmPromptModel = feedback::ConfirmPrompt<ConfirmPromptKind>;"),
        "Sempal native confirm prompts should alias the generic Radiant prompt primitive"
    );
    assert!(
        native_dtos.contains("pub enum ConfirmPromptKind")
            && native_dtos.contains("DestructiveEdit")
            && native_dtos.contains("BrowserRename")
            && native_dtos.contains("FolderRename")
            && native_dtos.contains("OptionsDefaultIdentifier"),
        "Sempal should own product prompt-kind names at the app boundary"
    );
    assert!(
        native_dtos
            .contains("compat::ConfirmPromptKind::DestructiveOperation => Self::DestructiveEdit")
            && native_dtos
                .contains("compat::ConfirmPromptKind::RenameContent => Self::BrowserRename")
            && native_dtos
                .contains("compat::ConfirmPromptKind::RenameNavigationItem => Self::FolderRename")
            && native_dtos
                .contains("compat::ConfirmPromptKind::CreateNavigationItem => Self::FolderCreate")
            && native_dtos
                .contains("ConfirmPromptKind::DestructiveEdit => Self::DestructiveOperation")
            && native_dtos.contains("ConfirmPromptKind::BrowserRename => Self::RenameContent")
            && native_dtos
                .contains("ConfirmPromptKind::FolderRename => Self::RenameNavigationItem")
            && native_dtos
                .contains("ConfirmPromptKind::FolderCreate => Self::CreateNavigationItem"),
        "Sempal prompt DTO conversion should map product prompt names onto generic Radiant intents"
    );
    assert!(
        !native_dtos.contains("pub enum AudioEngineChipStateModel"),
        "audio chip health should use the Radiant-owned generic health-state primitive"
    );
    assert!(
        native_dtos.contains("pub type AudioEngineChipStateModel = feedback::HealthState;"),
        "Sempal native audio chip state should alias the generic Radiant health primitive"
    );
    assert!(
        !native_dtos.contains("pub struct AudioOptionItemModel")
            && !native_dtos.contains("pub struct AudioFieldModel"),
        "audio picker item and summary field containers should use Radiant-owned generic form primitives"
    );
    assert!(
        native_dtos
            .contains("pub type AudioOptionItemModel = form::OptionItem<AudioOptionValueModel>;")
            && native_dtos.contains("pub type AudioFieldModel = form::SummaryField;"),
        "Sempal native audio picker DTOs should alias generic Radiant form primitives"
    );
    assert!(
        native_actions.contains("pub enum BrowserTagTarget")
            && native_actions.contains("Trash")
            && native_actions.contains("Keep"),
        "Sempal should own browser triage target names"
    );
    assert!(
        native_actions.contains(
            "compat::UiAction::MoveDiscardedItemsToFolder => Self::MoveTrashedSamplesToFolder"
        ) && native_actions
            .contains("UiAction::MoveTrashedSamplesToFolder => Self::MoveDiscardedItemsToFolder"),
        "Sempal action conversion should map the product trash action onto Radiant's generic discard action"
    );
    assert!(
        native_actions.contains(
            "compat::UiAction::FocusLoadedContentInList => Self::FocusLoadedSampleInBrowser"
        ) && native_actions
            .contains("UiAction::FocusLoadedSampleInBrowser => Self::FocusLoadedContentInList"),
        "Sempal action conversion should map the product loaded-sample focus action onto Radiant's generic loaded-content focus action"
    );
    assert!(
        native_actions.contains("compat::UiAction::SetCompareAnchorFromFocusedContent")
            && native_actions.contains("Self::SetCompareAnchorFromFocusedBrowserSample")
            && native_actions.contains("UiAction::SetCompareAnchorFromFocusedBrowserSample")
            && native_actions.contains("Self::SetCompareAnchorFromFocusedContent"),
        "Sempal action conversion should map the product compare-anchor action onto Radiant's generic focused-content action"
    );
    assert!(
        native_actions.contains("compat::UiAction::ToggleContentMark")
            && native_actions.contains("Self::ToggleBrowserSampleMark")
            && native_actions.contains("UiAction::ToggleBrowserSampleMark")
            && native_actions.contains("Self::ToggleContentMark"),
        "Sempal action conversion should map the product browser mark action onto Radiant's generic content mark action"
    );
    assert!(
        native_actions.contains("compat::UiAction::ToggleFindSimilarFocusedContent")
            && native_actions.contains("Self::ToggleFindSimilarFocusedSample")
            && native_actions.contains("UiAction::ToggleFindSimilarFocusedSample")
            && native_actions.contains("Self::ToggleFindSimilarFocusedContent"),
        "Sempal action conversion should map the product find-similar action onto Radiant's generic focused-content action"
    );
    assert!(
        native_actions.contains(
            "compat::UiAction::NormalizeFocusedContentItem => Self::NormalizeFocusedBrowserSample"
        ) && native_actions.contains(
            "UiAction::NormalizeFocusedBrowserSample => Self::NormalizeFocusedContentItem"
        ),
        "Sempal action conversion should map the product normalize action onto Radiant's generic focused-content normalize action"
    );
    assert!(
        native_actions
            .contains("compat::UiAction::PlayRandomContentItem => Self::PlayRandomSample")
            && native_actions.contains(
                "compat::UiAction::PlayPreviousRandomContentItem => Self::PlayPreviousRandomSample"
            )
            && native_actions.contains("UiAction::PlayRandomSample => Self::PlayRandomContentItem")
            && native_actions.contains(
                "UiAction::PlayPreviousRandomSample => Self::PlayPreviousRandomContentItem"
            ),
        "Sempal action conversion should map product random-sample actions onto Radiant's generic random-content actions"
    );
    assert!(
        native_actions.contains("compat::UiAction::FocusSpatialContentItem { content_id }")
            && native_actions.contains("Self::FocusMapSample")
            && native_actions.contains("sample_id: content_id")
            && native_actions.contains("UiAction::FocusMapSample { sample_id }")
            && native_actions.contains("Self::FocusSpatialContentItem")
            && native_actions.contains("content_id: sample_id"),
        "Sempal action conversion should map product map-sample focus onto Radiant's generic spatial-content focus action"
    );
    assert!(
        native_hit_testing.contains("fn map_focus_action(content_id: String) -> UiAction")
            && native_hit_testing.contains("UiAction::FocusSpatialContentItem")
            && native_hit_testing.contains("UiAction::FocusMapSample"),
        "shared map hit-testing should emit Radiant's generic spatial-content action in the legacy-shell build and Sempal's product action in the app build"
    );
    assert!(
        native_dtos
            .contains("\"focus_spatial_content_item\" => String::from(\"focus_map_sample\")")
            && native_dtos
                .contains("\"focus_map_sample\" => String::from(\"focus_spatial_content_item\")"),
        "Sempal automation DTO conversion should map Radiant's generic spatial-content action id onto the product map-sample action id"
    );
    assert!(
        native_hit_testing.contains("fn focused_similarity_action() -> UiAction")
            && native_hit_testing.contains("UiAction::ToggleFindSimilarFocusedContent")
            && native_hit_testing.contains("UiAction::ToggleFindSimilarFocusedSample"),
        "shared browser hit-testing should emit Radiant's generic focused-content action in the legacy-shell build and Sempal's product action in the app build"
    );
    assert!(
        !native_dtos.contains("pub struct FolderRecoveryModel"),
        "folder recovery counters should use the Radiant-owned generic recovery summary primitive"
    );
    assert!(
        native_dtos.contains("pub type FolderRecoveryModel = feedback::RecoverySummary;"),
        "Sempal native folder recovery DTOs should alias the generic Radiant feedback primitive"
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
