use super::*;

#[test]
/// Auto-renaming rows outside the loaded waveform must not invalidate native waveform projection.
fn auto_rename_unrelated_rows_keeps_loaded_waveform_projection_stable() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    for name in ["loaded.wav", "queued.wav", "other.wav"] {
        write_test_wav(&source.root.join(name), &[0.0, 0.1, -0.1]);
    }
    controller.set_wav_entries_for_tests(vec![
        sample_entry("loaded.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("queued.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("other.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    seed_stable_waveform_projection(&mut controller, &source, Path::new("loaded.wav"));
    controller.ui.waveform.loop_enabled = true;

    let before = project_waveform_model(&mut controller);
    apply_auto_rename_successes(
        &mut controller,
        source.id.clone(),
        vec![PathBuf::from("queued.wav"), PathBuf::from("other.wav")],
        vec![
            auto_rename_success("queued.wav", "renamed_queued.wav"),
            auto_rename_success("other.wav", "renamed_other.wav"),
        ],
    );
    let after = project_waveform_model(&mut controller);

    assert_eq!(before.waveform_image_signature, Some(77));
    assert_eq!(
        after.waveform_image_signature,
        before.waveform_image_signature
    );
    assert_eq!(after.loading, before.loading);
    assert_eq!(after.playhead_micros, before.playhead_micros);
    assert_eq!(controller.ui.waveform.loading, None);
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("loaded.wav"))
    );
    assert_eq!(
        controller.ui.loaded_wav.as_deref(),
        Some(Path::new("loaded.wav"))
    );
    assert!(controller.runtime.jobs.pending_audio.is_none());
    assert!(controller.sample_view.waveform.decoded.is_some());
    assert!(controller.ui.waveform.loop_enabled);
}

#[test]
/// Auto-renaming the loaded row should remap waveform identity without publishing a blank frame.
fn auto_rename_loaded_row_remaps_waveform_identity_without_blank_frame() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    for name in ["loaded.wav", "other.wav"] {
        write_test_wav(&source.root.join(name), &[0.0, 0.1, -0.1]);
    }
    controller.set_wav_entries_for_tests(vec![
        sample_entry("loaded.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("other.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    seed_stable_waveform_projection(&mut controller, &source, Path::new("loaded.wav"));
    controller.set_browser_selected_paths(vec![PathBuf::from("loaded.wav")]);

    let before = project_waveform_model(&mut controller);
    apply_auto_rename_successes(
        &mut controller,
        source.id.clone(),
        vec![PathBuf::from("loaded.wav"), PathBuf::from("other.wav")],
        vec![
            auto_rename_success("loaded.wav", "renamed_loaded.wav"),
            auto_rename_success("other.wav", "renamed_other.wav"),
        ],
    );
    let after = project_waveform_model(&mut controller);

    assert_eq!(
        after.waveform_image_signature,
        before.waveform_image_signature
    );
    assert_eq!(after.loading, before.loading);
    assert_eq!(after.playhead_micros, before.playhead_micros);
    assert_eq!(controller.ui.waveform.loading, None);
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("renamed_loaded.wav"))
    );
    assert_eq!(
        controller.ui.loaded_wav.as_deref(),
        Some(Path::new("renamed_loaded.wav"))
    );
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| audio.relative_path.as_path()),
        Some(Path::new("renamed_loaded.wav"))
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("renamed_loaded.wav")]
    );
    assert!(controller.runtime.jobs.pending_audio.is_none());
    assert!(controller.sample_view.waveform.decoded.is_some());
}
