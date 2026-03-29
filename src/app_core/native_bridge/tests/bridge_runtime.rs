use super::*;
use crate::app::state::BrowserDuplicateCleanupState;
use crate::app_core::state::{InlineFolderEdit, InlineFolderEditKind};

fn browser_row_bucket_label(
    model: &crate::app_core::actions::NativeAppModel,
    row_label: &str,
) -> Option<String> {
    model
        .browser
        .rows
        .iter()
        .find(|row| row.label == row_label)
        .and_then(|row| row.bucket_label.clone())
}

/// Async revision-bus updates must invalidate the retained projection key on first pull.
#[test]
fn bridge_reprojects_after_async_loaded_wav_revision_change() {
    let mut bridge = test_bridge(32);

    let first = bridge.project_model();
    assert!(first.waveform.loaded_label.is_none());

    bridge
        .controller
        .set_ui_loaded_wav(Some(PathBuf::from("fresh_take.wav")));
    let second = bridge.project_model();
    assert_eq!(second.waveform.loaded_label.as_deref(), Some("fresh_take"));
}

/// Initial full projection should bump all static segment revisions.
#[test]
fn pull_model_bumps_segment_revisions_on_first_projection() {
    let mut bridge = test_bridge(16);
    let _ = bridge.pull_model();
    let revisions = bridge.take_segment_revisions();

    assert!(revisions.has_static_revisions());
    assert!(revisions.status_bar > 0);
    assert!(revisions.browser_frame > 0);
    assert!(revisions.browser_rows_window > 0);
    assert!(revisions.map_panel > 0);
    assert!(revisions.waveform_overlay > 0);
    assert!(revisions.global_static > 0);
}

/// Repeated idle motion pulls should stay on the motion-only path without
/// forcing a fallback full-model refresh.
#[test]
fn idle_motion_projection_stays_available_across_repeated_pulls() {
    let mut bridge = test_bridge(16);

    let first = bridge.project_motion_model_snapshot();
    let second = bridge.project_motion_model_snapshot();
    let third = bridge.project_motion_model_snapshot();

    assert!(first.is_some());
    assert!(second.is_some());
    assert!(third.is_some());
    assert!(!bridge.controller.has_dirty_derived_nodes());
}

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

/// Toggling random navigation should refresh browser chrome in the same action cycle.
#[test]
fn random_navigation_toggle_updates_projected_browser_actions_immediately() {
    let mut bridge = test_bridge(16);

    let initial = bridge.project_model();
    assert!(!initial.browser_actions.random_navigation_enabled);

    bridge.on_action(NativeUiAction::ToggleRandomNavigationMode);

    let updated = bridge.project_model();
    assert!(updated.browser_actions.random_navigation_enabled);
}

/// Duplicate cleanup should project active toolbar state plus transient row badges.
#[test]
fn duplicate_cleanup_projects_active_browser_state_and_row_badges() {
    let bundle = crate::app_core::controller::build_named_gui_fixture_controller(
        WaveformRenderer::new(16, 16),
        "waveform",
    )
    .expect("waveform fixture");
    let _sandbox_guards = bundle.sandbox_guards;
    let mut controller = bundle.controller;
    let source_id = controller.current_source().expect("selected source").id;
    let anchor_path = controller
        .wav_entry(0)
        .expect("anchor entry")
        .relative_path
        .clone();
    let keep_path = controller
        .wav_entry(1)
        .expect("keep entry")
        .relative_path
        .clone();
    let anchor_label = crate::app::view_model::sample_display_label(&anchor_path);
    let keep_label = crate::app::view_model::sample_display_label(&keep_path);
    controller.focus_browser_row_only(0);
    controller.ui.browser.duplicate_cleanup = Some(BrowserDuplicateCleanupState::new(
        source_id,
        String::from("sample-id"),
        anchor_path,
        String::from("Duplicates"),
        vec![0, 1],
        vec![1.0, 0.999],
        0,
    ));
    controller
        .ui
        .browser
        .duplicate_cleanup
        .as_mut()
        .expect("duplicate cleanup should exist")
        .kept_indices
        .insert(1);
    let mut bridge = crate::app_core::native_bridge::new_native_bridge_with_controller(controller);

    let projected = bridge.project_model();

    assert!(projected.browser.duplicate_cleanup_active);
    assert!(projected.browser_actions.duplicate_cleanup_active);
    assert_eq!(
        browser_row_bucket_label(&projected, &anchor_label),
        Some(String::from("ANCHOR"))
    );
    assert_eq!(
        browser_row_bucket_label(&projected, &keep_label),
        Some(String::from("KEEP"))
    );
}

/// Loop-toggle metadata writes should refresh the visible browser badge in the
/// same projection cycle for the loaded waveform sample.
#[test]
fn toggle_loop_playback_refreshes_loaded_sample_loop_badge_immediately() {
    let bundle = crate::app_core::controller::build_named_gui_fixture_controller(
        WaveformRenderer::new(16, 16),
        "waveform",
    )
    .expect("waveform fixture");
    let _sandbox_guards = bundle.sandbox_guards;
    let mut bridge =
        crate::app_core::native_bridge::new_native_bridge_with_controller(bundle.controller);

    let initial = bridge.project_model();
    assert_eq!(browser_row_bucket_label(&initial, "kick_one"), None);

    bridge.on_action(NativeUiAction::ToggleLoopPlayback);

    let enabled = bridge.project_model();
    let enabled_label = browser_row_bucket_label(&enabled, "kick_one")
        .expect("loop toggle should project a browser-row badge");
    assert!(
        enabled_label.contains("LOOP"),
        "expected LOOP in projected badge, got {enabled_label:?}"
    );

    bridge.on_action(NativeUiAction::ToggleLoopPlayback);

    let disabled = bridge.project_model();
    let disabled_label = browser_row_bucket_label(&disabled, "kick_one");
    assert!(
        disabled_label
            .as_deref()
            .is_none_or(|label| !label.contains("LOOP")),
        "expected LOOP to be removed, got {disabled_label:?}"
    );
}

/// Enabling loop should project persisted BPM metadata alongside the new loop badge.
#[test]
fn toggle_loop_playback_refreshes_loaded_sample_bpm_and_loop_badges_immediately() {
    let bundle = crate::app_core::controller::build_named_gui_fixture_controller(
        WaveformRenderer::new(16, 16),
        "waveform",
    )
    .expect("waveform fixture");
    let _sandbox_guards = bundle.sandbox_guards;
    let mut controller = bundle.controller;
    controller.ui.waveform.bpm_value = Some(128.0);
    controller.ui.waveform.bpm_input = String::from("128");
    let mut bridge = crate::app_core::native_bridge::new_native_bridge_with_controller(controller);

    bridge.on_action(NativeUiAction::ToggleLoopPlayback);

    let projected = bridge.project_model();
    assert_eq!(
        browser_row_bucket_label(&projected, "kick_one"),
        Some(String::from("128 BPM · LOOP"))
    );
}

/// Manual browser viewport actions must refresh the projected row window
/// immediately so wheel/scrollbar input updates both the semantic snapshot and
/// the rendered browser list in the same interaction.
#[test]
fn set_browser_view_start_action_refreshes_projected_model_immediately() {
    let mut bridge = test_bridge(16);
    bridge.controller.ui.browser.viewport.visible =
        crate::app_core::state::VisibleRows::All { total: 40 };

    let initial = bridge.project_model();
    assert_eq!(initial.browser.view_start_row, 0);

    bridge.on_action(NativeUiAction::SetBrowserViewStart { visible_row: 1 });

    let updated = bridge.project_model();
    assert_eq!(updated.browser.view_start_row, 1);
}

/// Folder-create input updates must refresh the projected draft text immediately.
#[test]
fn set_folder_create_input_action_refreshes_projected_model_immediately() {
    let mut bridge = test_bridge(16);
    bridge.controller.ui.sources.folders.inline_edit = Some(InlineFolderEdit {
        kind: InlineFolderEditKind::Create {
            parent: PathBuf::new(),
        },
        name: String::new(),
        focus_requested: true,
        select_all_on_focus_requested: false,
    });
    bridge
        .controller
        .ui
        .sources
        .folders
        .rows
        .push(crate::app::state::FolderRowView {
            path: PathBuf::new(),
            name: String::from("Root"),
            depth: 0,
            has_children: true,
            expanded: true,
            selected: false,
            negated: false,
            hotkey: None,
            is_root: true,
            file_scope_mode: Some(crate::app::state::FolderFileScopeMode::AllDescendants),
        });

    let initial = bridge.project_model();
    let initial_draft = initial
        .sources
        .folder_rows
        .iter()
        .find(|row| row.kind == crate::app_core::actions::NativeFolderRowKind::CreateDraft)
        .expect("folder create draft should be projected");
    assert_eq!(initial_draft.input_value.as_deref(), Some(""));

    bridge.on_action(NativeUiAction::SetFolderCreateInput {
        value: String::from("drums"),
    });

    let updated = bridge.project_model();
    let updated_draft = updated
        .sources
        .folder_rows
        .iter()
        .find(|row| row.kind == crate::app_core::actions::NativeFolderRowKind::CreateDraft)
        .expect("folder create draft should still be projected");
    assert_eq!(updated_draft.input_value.as_deref(), Some("drums"));
}

/// Canceling folder-create should remove the draft from the next projected model immediately.
#[test]
fn cancel_folder_create_action_refreshes_projected_model_immediately() {
    let mut bridge = test_bridge(16);
    bridge.controller.ui.sources.folders.inline_edit = Some(InlineFolderEdit {
        kind: InlineFolderEditKind::Create {
            parent: PathBuf::new(),
        },
        name: String::from("drums"),
        focus_requested: true,
        select_all_on_focus_requested: false,
    });
    bridge
        .controller
        .ui
        .sources
        .folders
        .rows
        .push(crate::app::state::FolderRowView {
            path: PathBuf::new(),
            name: String::from("Root"),
            depth: 0,
            has_children: true,
            expanded: true,
            selected: false,
            negated: false,
            hotkey: None,
            is_root: true,
            file_scope_mode: Some(crate::app::state::FolderFileScopeMode::AllDescendants),
        });

    let initial = bridge.project_model();
    assert!(
        initial
            .sources
            .folder_rows
            .iter()
            .any(|row| row.kind == crate::app_core::actions::NativeFolderRowKind::CreateDraft)
    );

    bridge.on_action(NativeUiAction::CancelFolderCreate);

    let updated = bridge.project_model();
    assert!(
        updated
            .sources
            .folder_rows
            .iter()
            .all(|row| row.kind != crate::app_core::actions::NativeFolderRowKind::CreateDraft)
    );
}

/// Starting folder rename should immediately project an inline rename row.
#[test]
fn start_folder_rename_action_refreshes_projected_model_immediately() {
    let mut bridge = test_bridge(16);
    bridge
        .controller
        .ui
        .sources
        .folders
        .rows
        .push(crate::app::state::FolderRowView {
            path: PathBuf::new(),
            name: String::from("Root"),
            depth: 0,
            has_children: true,
            expanded: true,
            selected: false,
            negated: false,
            hotkey: None,
            is_root: true,
            file_scope_mode: Some(crate::app::state::FolderFileScopeMode::AllDescendants),
        });
    bridge
        .controller
        .ui
        .sources
        .folders
        .rows
        .push(crate::app::state::FolderRowView {
            path: PathBuf::from("drums"),
            name: String::from("drums"),
            depth: 1,
            has_children: false,
            expanded: false,
            selected: true,
            negated: false,
            hotkey: None,
            is_root: false,
            file_scope_mode: None,
        });
    bridge.controller.ui.sources.folders.focused = Some(1);

    bridge.on_action(NativeUiAction::StartFolderRename);

    let updated = bridge.project_model();
    let draft = updated
        .sources
        .folder_rows
        .iter()
        .find(|row| row.kind == crate::app_core::actions::NativeFolderRowKind::RenameDraft)
        .expect("folder rename draft should be projected");
    assert_eq!(draft.input_value.as_deref(), Some("drums"));
    assert!(draft.select_all_on_focus);
}

/// Focus-only browser actions should preserve the current manual viewport start
/// so native guard-band autoscroll can continue from the rows already on
/// screen instead of snapping back to the retained host slice start.
#[test]
fn focus_browser_row_preserves_manual_viewport_start_in_projected_model() {
    let mut bridge = test_bridge(16);
    bridge.controller.ui.browser.viewport.visible =
        crate::app_core::state::VisibleRows::All { total: 40 };

    bridge.on_action(NativeUiAction::SetBrowserViewStart { visible_row: 7 });
    let scrolled = bridge.project_model();
    assert_eq!(scrolled.browser.view_start_row, 7);

    bridge.on_action(NativeUiAction::FocusBrowserRow { visible_row: 18 });
    let refocused = bridge.project_model();
    assert_eq!(refocused.browser.view_start_row, 7);
}

/// Waveform preview-class actions should bypass queueing for immediate feedback.
#[test]
fn on_action_applies_waveform_preview_actions_immediately() {
    let mut bridge = test_bridge(16);

    bridge.on_action(NativeUiAction::SetWaveformCursor {
        position_milli: 420,
    });

    assert_eq!(bridge.pending_waveform_actions.cursor_nanos, None);
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::WaveformState)
    );
}

/// Seek actions should remain coalesced in the queue to cap apply-stage cost.
#[test]
fn on_action_keeps_seek_actions_queued() {
    let mut bridge = test_bridge(16);

    bridge.on_action(NativeUiAction::SeekWaveform {
        position_milli: 333,
    });

    assert_eq!(
        bridge.pending_waveform_actions.seek_nanos,
        Some(333_000_000)
    );
}

/// Flushing queued waveform actions should clear queue state and mark waveform dirties.
#[test]
fn flush_pending_waveform_actions_clears_queue_and_marks_waveform_dirty() {
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let cache = NativeProjectionCache {
        app_key: Some(build_projection_cache_key(&controller)),
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
    };

    assert!(
        bridge.enqueue_waveform_action(&NativeUiAction::SetWaveformCursor {
            position_milli: 500,
        })
    );
    bridge.flush_pending_waveform_actions();

    assert!(!bridge.pending_waveform_actions.has_pending());
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::WaveformState)
    );
    assert!(bridge.projection_cache.app_key.is_some());
}

/// No-op queued waveform actions should not dirty the derived graph.
#[test]
fn flush_pending_waveform_actions_noop_skips_dirty_marking() {
    let mut bridge = test_bridge(16);

    assert!(
        bridge.enqueue_waveform_action(&NativeUiAction::SetWaveformCursor {
            position_milli: 500,
        })
    );
    bridge.flush_pending_waveform_actions();
    let Some(first_snapshot) = bridge.projection_key_snapshot.as_ref().cloned() else {
        panic!("waveform flush should retain a projection key snapshot");
    };
    bridge.flush_derived_updates_before_pull(false);
    assert!(!bridge.controller.has_dirty_derived_nodes());

    assert!(
        bridge.enqueue_waveform_action(&NativeUiAction::SetWaveformCursor {
            position_milli: 500,
        })
    );
    bridge.flush_pending_waveform_actions();

    assert!(
        !bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::WaveformState)
    );
    assert_eq!(
        bridge.projection_key_snapshot.as_ref(),
        Some(&first_snapshot)
    );
}

/// Action classification should mark waveform source and projection-key nodes dirty.
#[test]
fn mark_dirty_for_waveform_action_marks_graph_nodes() {
    let mut bridge = test_bridge(16);

    bridge.mark_dirty_for_action(&NativeUiAction::SeekWaveform {
        position_milli: 250,
    });

    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::WaveformState)
    );
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::NativeAppProjectionKey)
    );
}

/// Exact-position waveform click-play should still dirty the transport graph.
#[test]
fn mark_dirty_for_waveform_click_play_marks_transport_state_dirty() {
    let mut bridge = test_bridge(16);

    bridge.mark_dirty_for_action(&NativeUiAction::PlayWaveformAtPrecise {
        position_nanos: 420_000_000,
    });

    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::TransportState)
    );
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::NativeAppProjectionKey)
    );
}

/// High-frequency browser focus actions should avoid broad invalidation fan-out.
#[test]
fn mark_dirty_for_browser_focus_action_stays_targeted() {
    let mut bridge = test_bridge(16);

    bridge.mark_dirty_for_action(&NativeUiAction::MoveBrowserFocus { delta: 1 });

    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::BrowserState)
    );
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::NativeAppProjectionKey)
    );
    assert!(
        !bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::MapState)
    );
    assert!(
        !bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::TransportState)
    );
    assert!(
        !bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::StatusState)
    );
}

/// Non-targeted actions should keep broad invalidation as the conservative fallback.
#[test]
fn mark_dirty_for_unclassified_action_keeps_broad_invalidation() {
    let mut bridge = test_bridge(16);

    bridge.mark_dirty_for_action(&NativeUiAction::OpenSourceFolderRow { index: 0 });

    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::BrowserState)
    );
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::MapState)
    );
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::TransportState)
    );
    assert!(
        bridge
            .controller
            .is_derived_node_dirty_for_test(DerivedNodeId::StatusState)
    );
}

/// Flushing derived updates should clear graph dirties and invalidate projection cache key.
#[test]
fn flush_derived_updates_clears_nodes_and_invalidates_key() {
    let controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let cache = NativeProjectionCache {
        app_key: Some(build_projection_cache_key(&controller)),
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
    };
    let _ = bridge.projection_key_snapshot();
    assert!(bridge.projection_key_snapshot.is_some());

    bridge.mark_dirty_for_action(&NativeUiAction::SetBrowserSearch {
        query: String::from("kick"),
    });
    bridge.flush_derived_updates_before_pull(false);

    assert!(!bridge.controller.has_dirty_derived_nodes());
    assert!(bridge.projection_cache.app_key.is_none());
    assert!(bridge.projection_key_snapshot.is_none());
}

/// Repeated no-op pulls should preserve snapshot/cache reuse and avoid full reprojection.
#[test]
fn pull_model_snapshot_noop_pull_reuses_cached_projection() {
    let mut bridge = test_bridge(16);

    let first_model = bridge.pull_model_arc_snapshot();
    let Some(first_snapshot) = bridge.projection_key_snapshot.as_ref().cloned() else {
        panic!("pull should populate projection key snapshot");
    };
    let Some(first_cache_key) = bridge.projection_cache.app_key.as_ref().cloned() else {
        panic!("pull should populate projection cache key");
    };
    assert_eq!(first_snapshot, first_cache_key);

    let second_model = bridge.pull_model_arc_snapshot();
    assert!(Arc::ptr_eq(&first_model, &second_model));
    assert_eq!(
        bridge.projection_key_snapshot.as_ref(),
        Some(&first_snapshot)
    );
}
