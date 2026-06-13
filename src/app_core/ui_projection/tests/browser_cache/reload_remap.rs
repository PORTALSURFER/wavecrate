use super::*;
#[test]
/// Browser row projection should refresh retained labels after a same-length page-0 reload.
fn browser_rows_projection_refreshes_labels_after_same_length_reload() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let source = crate::sample_sources::SampleSource::new(root);
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.register_source_for_tests(source.clone());
    controller.select_browser_source_for_tests(source.id.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![
        crate::sample_sources::WavEntry {
            relative_path: std::path::PathBuf::from("alpha.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("alpha-hash")),
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
            relative_path: std::path::PathBuf::from("beta.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: Some(String::from("beta-hash")),
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
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.ui.browser.viewport.visible = projection_fixtures::visible_rows_all(2);

    let projected = project_browser_model(&mut controller);
    assert_eq!(projected.rows[0].label.as_ref(), "alpha");
    assert_eq!(projected.rows[1].label.as_ref(), "beta");

    controller.apply_wav_entries_with_params(crate::app::controller::ApplyWavEntriesParams {
        entries: vec![
            crate::sample_sources::WavEntry {
                relative_path: std::path::PathBuf::from("beta.wav"),
                file_size: 0,
                modified_ns: 0,
                content_hash: Some(String::from("beta-hash")),
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
                relative_path: std::path::PathBuf::from("alpha.wav"),
                file_size: 0,
                modified_ns: 0,
                content_hash: Some(String::from("alpha-hash")),
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
        ],
        total: 2,
        page_size: 2,
        page_index: 0,
        from_cache: false,
        source_id: Some(source.id),
        elapsed: None,
    });

    let projected = project_browser_model(&mut controller);
    assert_eq!(projected.rows[0].label.as_ref(), "beta");
    assert_eq!(projected.rows[1].label.as_ref(), "alpha");
}

#[test]
/// Same-folder sample renames should refresh retained row labels without another click.
fn browser_rows_projection_refreshes_label_after_cached_rename() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let source = crate::sample_sources::SampleSource::new(root);
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.register_source_for_tests(source.clone());
    controller.select_browser_source_for_tests(source.id.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("raw.wav"),
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
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.ui.browser.viewport.visible = projection_fixtures::visible_rows_all(1);

    let projected = project_browser_model(&mut controller);
    assert_eq!(projected.rows[0].label.as_ref(), "raw");
    assert!(controller.projected_browser_rows.contains_key(&0));

    let mut updated = crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from("renamed.wav"),
        file_size: 10,
        modified_ns: 20,
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
    };
    updated.looped = true;
    controller.update_cached_entry(&source, std::path::Path::new("raw.wav"), updated);
    let _ = controller.refresh_projection_revision_bus();

    let projected = project_browser_model(&mut controller);
    assert_eq!(projected.rows[0].label.as_ref(), "renamed");
    assert_eq!(
        controller
            .browser_projection_entry(0)
            .map(|entry| entry.relative_path),
        Some(std::path::Path::new("renamed.wav"))
    );
}

#[test]
/// Auto-rename row processing states should project through path remaps.
fn browser_rows_projection_tracks_auto_rename_processing_states_and_remaps() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let source = crate::sample_sources::SampleSource::new(root);
    let mut controller = AppController::new(crate::waveform::WaveformRenderer::new(16, 16), None);
    controller.register_source_for_tests(source.clone());
    controller.select_browser_source_for_tests(source.id.clone());
    controller.set_wav_entries_for_tests(vec![
        test_wav_entry("alpha.wav"),
        test_wav_entry("beta.wav"),
        test_wav_entry("gamma.wav"),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.ui.browser.viewport.visible = projection_fixtures::visible_rows_all(3);
    controller.begin_auto_rename_batch_for_tests(
        source.id.clone(),
        vec![
            std::path::PathBuf::from("alpha.wav"),
            std::path::PathBuf::from("beta.wav"),
        ],
    );

    let projected = project_browser_model(&mut controller);
    assert_eq!(
        projected.rows[0].processing_state,
        crate::app_core::actions::NativeBrowserRowProcessingState::Queued
    );
    assert_eq!(
        projected.rows[1].processing_state,
        crate::app_core::actions::NativeBrowserRowProcessingState::Queued
    );
    assert_eq!(
        projected.rows[2].processing_state,
        crate::app_core::actions::NativeBrowserRowProcessingState::None
    );

    controller.apply_auto_rename_progress_for_tests(
        crate::app::controller::jobs::SampleAutoRenameProgress::Active {
            old_relative: std::path::PathBuf::from("alpha.wav"),
        },
    );
    let projected = project_browser_model(&mut controller);
    assert_eq!(
        projected.rows[0].processing_state,
        crate::app_core::actions::NativeBrowserRowProcessingState::Active
    );

    controller.apply_auto_rename_progress_for_tests(
        crate::app::controller::jobs::SampleAutoRenameProgress::Completed {
            old_relative: std::path::PathBuf::from("alpha.wav"),
            new_relative: std::path::PathBuf::from("alpha_renamed.wav"),
        },
    );
    controller.update_cached_entry(
        &source,
        std::path::Path::new("alpha.wav"),
        test_wav_entry("alpha_renamed.wav"),
    );
    controller.apply_auto_rename_progress_for_tests(
        crate::app::controller::jobs::SampleAutoRenameProgress::Failed {
            old_relative: std::path::PathBuf::from("beta.wav"),
            error: String::from("Disk error"),
        },
    );

    let projected = project_browser_model(&mut controller);
    assert_eq!(projected.rows[0].label.as_ref(), "alpha_renamed");
    assert_eq!(
        projected.rows[0].processing_state,
        crate::app_core::actions::NativeBrowserRowProcessingState::Completed
    );
    assert_eq!(
        projected.rows[1].processing_state,
        crate::app_core::actions::NativeBrowserRowProcessingState::Failed
    );
}

fn test_wav_entry(path: &str) -> crate::sample_sources::WavEntry {
    crate::sample_sources::WavEntry {
        relative_path: std::path::PathBuf::from(path),
        file_size: 0,
        modified_ns: 0,
        content_hash: Some(format!("hash-{path}")),
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        sound_type: None,
        locked: false,
        missing: false,
        last_played_at: None,
        user_tag: None,
        tag_named: false,
        normal_tags: Vec::new(),
    }
}
