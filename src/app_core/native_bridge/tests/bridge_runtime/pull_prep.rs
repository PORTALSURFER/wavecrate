use super::*;

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
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
        gui_test_recorder: None,
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
