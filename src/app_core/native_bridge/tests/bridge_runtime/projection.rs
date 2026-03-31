use super::*;

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

/// Async waveform-image completion must invalidate the retained waveform segment.
#[test]
fn bridge_reprojects_after_async_waveform_image_arrival() {
    let mut bridge = test_bridge(32);

    bridge
        .controller
        .set_ui_loaded_wav(Some(PathBuf::from("fresh_take.wav")));
    let initial = bridge.project_model();
    assert_eq!(initial.waveform.loaded_label.as_deref(), Some("fresh_take"));
    assert!(initial.waveform.waveform_image.is_none());

    bridge.controller.ui.waveform.image = Some(crate::waveform::WaveformImage::new(
        [1, 1],
        vec![crate::waveform::WaveformRgba::from_rgb(12, 34, 56)],
    ));
    bridge.controller.ui.waveform.waveform_image_signature = Some(1);
    bridge.controller.projected_waveform_image = None;
    bridge.controller.projected_waveform_image_signature = None;
    bridge.controller.mark_waveform_projection_dirty();

    let updated = bridge.project_model();
    assert_eq!(updated.waveform.loaded_label.as_deref(), Some("fresh_take"));
    let waveform_image = updated
        .waveform
        .waveform_image
        .as_ref()
        .expect("async waveform image should be projected");
    assert_eq!(waveform_image.width, 1);
    assert_eq!(waveform_image.height, 1);
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
