use super::*;
#[test]
/// Reusing the projection buffer should preserve the existing allocation.
fn browser_rows_projection_reuses_provided_buffer_capacity() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("snare.wav"),
        file_size: 0,
        modified_ns: 0,
        content_hash: Some(String::from("hash")),
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        sound_type: None,
        locked: false,
        missing: false,
        last_played_at: None,
        user_tag: None,
        tag_named: false,
        normal_tags: Vec::new(),
    }]);
    controller.ui.browser.viewport.visible = projection_fixtures::visible_rows_list(vec![0usize]);
    let mut rows = crate::app_core::actions::NativeRetainedVec::new();

    project_browser_rows_model_into(&mut controller, 1, Some(0), None, &mut rows);
    let first_capacity = rows.make_mut().capacity();
    let first_ptr = rows.as_slice().as_ptr();

    project_browser_rows_model_into(&mut controller, 1, Some(0), None, &mut rows);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows.make_mut().capacity(), first_capacity);
    assert_eq!(rows.as_slice().as_ptr(), first_ptr);
}

#[test]
/// Row-state patching should update focused and selected flags without touching labels.
fn browser_rows_state_patch_updates_flags_without_rebuilding_labels() {
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.set_wav_entries_for_tests(vec![
        crate::sample_sources::WavEntry {
            relative_path: std::path::PathBuf::from("kick.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("kick-hash")),
            tag: crate::sample_sources::Rating::NEUTRAL,
            looped: false,
            sound_type: None,
            locked: false,
            missing: false,
            last_played_at: None,
            user_tag: None,
            tag_named: false,
            normal_tags: Vec::new(),
        },
        crate::sample_sources::WavEntry {
            relative_path: std::path::PathBuf::from("snare.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("snare-hash")),
            tag: crate::sample_sources::Rating::NEUTRAL,
            looped: false,
            sound_type: None,
            locked: false,
            missing: false,
            last_played_at: None,
            user_tag: None,
            tag_named: false,
            normal_tags: Vec::new(),
        },
    ]);
    controller.ui.browser.viewport.visible =
        projection_fixtures::visible_rows_list(vec![0usize, 1usize]);
    controller.ui.browser.selection.selected_paths = vec![std::path::PathBuf::from("snare.wav")];
    controller.mark_browser_selected_paths_changed();

    let mut rows = vec![
        crate::app_core::actions::NativeBrowserRowModel::new(0, "Kick", 1, true, true),
        crate::app_core::actions::NativeBrowserRowModel::new(1, "Snare", 1, false, false),
    ];

    patch_browser_rows_state(&mut controller, Some(1), &mut rows);

    assert_eq!(rows[0].label.as_ref(), "Kick");
    assert_eq!(rows[1].label.as_ref(), "Snare");
    assert!(!rows[0].selected);
    assert!(!rows[0].focused);
    assert!(rows[1].selected);
    assert!(rows[1].focused);
}
