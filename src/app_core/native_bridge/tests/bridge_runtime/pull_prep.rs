use super::*;
use crate::app_core::controller::NativeFramePreparationPlan;

/// No-op immediate focus movement should keep projection cache keys intact.
#[test]
fn apply_browser_focus_delta_immediate_noop_keeps_projection_cache_key() {
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let key = build_projection_cache_key(&controller);
    let cache = NativeProjectionCache {
        app_key: Some(key.clone()),
        ..NativeProjectionCache::default()
    };

    let mut bridge = SempalNativeBridge {
        controller,
        projection_cache: cache,
        projection_key_snapshot: None,
        derived_projection_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
        gui_test_recorder: None,
        last_action_handled: None,
        runtime_exit_emitted: false,
    };
    bridge.apply_browser_focus_delta_immediately(1);
    assert_eq!(bridge.projection_cache.app_key, Some(key));
}

/// Local-only focus actions should avoid dirtying derived state and arm the one-shot fast path.
#[test]
fn local_focus_actions_arm_local_model_pull_fast_path() {
    let mut bridge = test_bridge(16);

    bridge.on_action(NativeUiAction::FocusBrowserSearch);

    assert_eq!(
        bridge.pending_model_pull_preparation,
        PendingModelPullPreparation::LocalOnly
    );
    assert!(!bridge.controller.has_dirty_derived_nodes());
}

/// Folder-panel search edits should stay on the local-only pull path.
#[test]
fn folder_search_actions_arm_local_model_pull_fast_path() {
    let mut bridge = test_bridge(16);

    bridge.on_action(NativeUiAction::SetFolderSearch {
        pane: None,
        query: String::from("drums"),
    });

    assert_eq!(
        bridge.pending_model_pull_preparation,
        PendingModelPullPreparation::LocalOnly
    );
    assert!(!bridge.controller.has_dirty_derived_nodes());
}

/// Browser search mutations still require the full pull-preparation path.
#[test]
fn search_query_actions_stay_on_full_model_pull_preparation() {
    let mut bridge = test_bridge(16);

    bridge.on_action(NativeUiAction::SetBrowserSearch {
        query: String::from("kick"),
    });

    assert_eq!(
        bridge.pending_model_pull_preparation,
        PendingModelPullPreparation::Full
    );
    assert!(bridge.controller.has_dirty_derived_nodes());
}

/// Browser-only dirty state should use the narrower retained-pull preparation lane.
#[test]
fn browser_dirty_state_uses_browser_retained_pull_plan() {
    let mut bridge = test_bridge(16);
    bridge
        .controller
        .mark_derived_source_dirty(DerivedNodeId::BrowserState, DirtyReason::BrowserAction);

    assert_eq!(
        bridge.model_pull_preparation_plan_for_tests(),
        NativeFramePreparationPlan::BrowserRetainedPull
    );
}

/// Retained browser pulls should still flush derived dirt before projection.
#[test]
fn browser_retained_pull_clears_dirty_nodes_after_projection() {
    let mut bridge = test_bridge(16);
    bridge
        .controller
        .mark_derived_source_dirty(DerivedNodeId::BrowserState, DirtyReason::BrowserAction);

    let _ = bridge.pull_model_arc_snapshot();

    assert!(!bridge.controller.has_dirty_derived_nodes());
}

/// Startup or non-browser dirty work should keep the bridge on the full preparation lane.
#[test]
fn startup_work_uses_startup_retained_pull_plan() {
    let mut bridge = test_bridge(16);
    bridge
        .controller
        .apply_configuration(crate::sample_sources::config::AppConfig::default())
        .expect("apply startup config");

    assert_eq!(
        bridge.model_pull_preparation_plan_for_tests(),
        NativeFramePreparationPlan::StartupRetainedPull
    );
}

/// Deferred metadata persistence should use the metadata-retained preparation lane.
#[test]
fn metadata_work_uses_metadata_retained_pull_plan() {
    let bundle = crate::app_core::controller::build_named_gui_fixture_controller(
        WaveformRenderer::new(16, 16),
        "waveform",
    )
    .expect("build waveform fixture");
    let mut bridge = SempalNativeBridge {
        controller: bundle.controller,
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        derived_projection_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
        gui_test_recorder: None,
        last_action_handled: None,
        runtime_exit_emitted: false,
    };
    let dirty_nodes = bridge.controller.dirty_derived_nodes_in_topo_order();
    for node in dirty_nodes {
        bridge.controller.clear_derived_dirty_node(node);
    }
    assert!(
        bridge
            .controller
            .has_pending_loaded_duration_metadata_write()
    );

    assert_eq!(
        bridge.model_pull_preparation_plan_for_tests(),
        NativeFramePreparationPlan::MetadataRetainedPull
    );
}

/// Deferred volume persistence should use the transport-retained preparation lane.
#[test]
fn transport_work_uses_transport_retained_pull_plan() {
    let mut bridge = test_bridge(16);
    bridge.controller.set_volume_live(0.75);

    assert_eq!(
        bridge.model_pull_preparation_plan_for_tests(),
        NativeFramePreparationPlan::TransportRetainedPull
    );
}

/// Mixed retained work should still fall back to the full preparation lane.
#[test]
fn mixed_retained_work_keeps_full_model_pull_plan() {
    let mut bridge = test_bridge(16);
    bridge
        .controller
        .apply_configuration(crate::sample_sources::config::AppConfig::default())
        .expect("apply startup config");
    bridge.controller.set_volume_live(0.75);

    assert_eq!(
        bridge.model_pull_preparation_plan_for_tests(),
        NativeFramePreparationPlan::Full
    );
}

/// Waveform dirt should keep the bridge on the full preparation lane.
#[test]
fn waveform_dirty_state_keeps_full_model_pull_plan() {
    let mut bridge = test_bridge(16);
    bridge.controller.mark_derived_source_dirty(
        DerivedNodeId::WaveformState,
        DirtyReason::WaveformViewAction,
    );

    assert_eq!(
        bridge.model_pull_preparation_plan_for_tests(),
        NativeFramePreparationPlan::Full
    );
}
