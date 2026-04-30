use super::*;

use crate::app::controller::jobs::SampleAutoRenameProgress;
use crate::app::controller::state::runtime::{
    AutoRenameBatchRowSnapshot, AutoRenameBatchRowState, BrowserRenameIntentKey,
};
use crate::app_core::native_shell::project_waveform_model;

/// Seed a loaded waveform with a reusable native projection signature.
fn seed_stable_waveform_projection(
    controller: &mut crate::app::controller::AppController,
    source: &crate::sample_sources::SampleSource,
    relative_path: &Path,
) {
    controller
        .load_waveform_for_selection(source, relative_path)
        .expect("waveform should load");
    controller.ui.waveform.image = Some(crate::waveform::WaveformImage {
        size: [2, 1],
        pixels: vec![
            crate::waveform::WaveformRgba::from_rgba_unmultiplied(10, 20, 30, 255),
            crate::waveform::WaveformRgba::from_rgba_unmultiplied(40, 50, 60, 255),
        ],
    });
    controller.ui.waveform.waveform_image_signature = Some(77);
    controller.set_waveform_render_meta_for_tests(Some(
        crate::app::controller::WaveformRenderMeta {
            view_start: controller.ui.waveform.view.start,
            view_end: controller.ui.waveform.view.end,
            size: controller.sample_view.waveform.size,
            samples_len: 2,
            texture_width: 2,
            channel_view: crate::waveform::WaveformChannelView::Mono,
            channels: 1,
            edit_fade: None,
            transient_visual_token: None,
        },
    ));
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.375;
}

/// Build one successful auto-rename result row for controller application tests.
fn auto_rename_success(
    old_relative: &str,
    new_relative: &str,
) -> crate::app::controller::jobs::SampleAutoRenameSuccess {
    crate::app::controller::jobs::SampleAutoRenameSuccess {
        old_relative: PathBuf::from(old_relative),
        new_relative: PathBuf::from(new_relative),
        entry: sample_entry(new_relative, crate::sample_sources::Rating::NEUTRAL),
        resume_playback: false,
        resume_looped: false,
        resume_start_override: None,
    }
}

/// Apply a completed auto-rename batch directly through the file-op result path.
fn apply_auto_rename_successes(
    controller: &mut crate::app::controller::AppController,
    source_id: crate::sample_sources::SourceId,
    requested_paths: Vec<PathBuf>,
    renamed: Vec<crate::app::controller::jobs::SampleAutoRenameSuccess>,
) {
    controller.apply_file_op_result(
        crate::app::controller::jobs::FileOpResult::SampleAutoRename(
            crate::app::controller::jobs::SampleAutoRenameResult {
                source_id,
                requested_paths,
                renamed,
                skipped: Vec::new(),
                errors: Vec::new(),
            },
        ),
    );
}

#[test]
fn auto_rename_uses_primary_row_plus_hidden_selection() {
    let (mut controller, source) = dummy_controller();
    controller.settings.default_identifier = String::from("Artist Name");
    controller.ui.options_panel.default_identifier = String::from("Artist Name");
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    for name in ["kick.wav", "bass.wav"] {
        write_test_wav(&source.root.join(name), &[0.0]);
    }
    controller.set_wav_entries_for_tests(vec![
        sample_entry("kick.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("bass.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    let db = controller.database_for(&source).unwrap();
    db.set_sound_type(
        Path::new("kick.wav"),
        Some(crate::sample_sources::SampleSoundType::Kick),
    )
    .unwrap();
    db.set_sound_type(
        Path::new("bass.wav"),
        Some(crate::sample_sources::SampleSoundType::Bass),
    )
    .unwrap();
    controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source.id.clone())
        .or_default()
        .insert(PathBuf::from("kick.wav"), Some(130.0));
    controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source.id.clone())
        .or_default()
        .insert(PathBuf::from("bass.wav"), Some(131.0));

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.set_browser_search(String::from("kick"));
    controller.auto_rename_browser_selection_action(Some(0));

    assert!(source.root.join("artistname_SS_kick_130.wav").exists());
    assert!(source.root.join("artistname_SS_bass_131.wav").exists());
}

#[test]
fn repeated_auto_rename_for_active_target_collapses_without_warning_churn() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "raw.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);

    controller
        .runtime
        .source_lane
        .mutations
        .begin_browser_rename_intent(
            crate::app::controller::state::runtime::BrowserRenameIntentKey::new(
                source.id.clone(),
                vec![(PathBuf::from("raw.wav"), PathBuf::from("portal_SS.wav"))],
            ),
        );
    let (_file_op_tx, file_op_rx) =
        std::sync::mpsc::channel::<crate::app::controller::jobs::FileOpMessage>();
    controller.runtime.jobs.start_file_ops(
        file_op_rx,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    );

    controller.auto_rename_browser_selection_action(Some(0));
    controller.auto_rename_browser_selection_action(Some(0));

    assert_eq!(
        controller.ui.status.text,
        "Auto rename already in progress..."
    );
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Busy
    );
    assert!(source.root.join("raw.wav").exists());
    assert!(!source.root.join("portal_SS.wav").exists());
}

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

#[test]
fn different_auto_rename_request_queues_one_follow_up_after_active_rename_finishes() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    for name in ["raw.wav", "kick.wav"] {
        write_test_wav(&source.root.join(name), &[0.0]);
    }
    let mut kick = sample_entry("kick.wav", crate::sample_sources::Rating::NEUTRAL);
    kick.sound_type = Some(crate::sample_sources::SampleSoundType::Kick);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("raw.wav", crate::sample_sources::Rating::NEUTRAL),
        kick,
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller
        .runtime
        .source_lane
        .mutations
        .begin_browser_rename_intent(
            crate::app::controller::state::runtime::BrowserRenameIntentKey::new(
                source.id.clone(),
                vec![(PathBuf::from("raw.wav"), PathBuf::from("portal_SS.wav"))],
            ),
        );
    let (_file_op_tx, file_op_rx) =
        std::sync::mpsc::channel::<crate::app::controller::jobs::FileOpMessage>();
    controller.runtime.jobs.start_file_ops(
        file_op_rx,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    );

    controller.focus_browser_row_only(1);
    controller.auto_rename_browser_selection_action(Some(1));
    assert_eq!(
        controller.ui.status.text,
        "Auto rename queued after current rename..."
    );

    controller.runtime.jobs.clear_file_ops();
    controller.apply_file_op_result(
        crate::app::controller::jobs::FileOpResult::SampleAutoRename(
            crate::app::controller::jobs::SampleAutoRenameResult {
                source_id: source.id.clone(),
                requested_paths: vec![PathBuf::from("raw.wav")],
                renamed: Vec::new(),
                skipped: Vec::new(),
                errors: vec![(PathBuf::from("raw.wav"), String::from("Rename cancelled"))],
            },
        ),
    );

    assert!(source.root.join("raw.wav").exists());
    assert!(!source.root.join("kick.wav").exists());
    assert!(source.root.join("portal_SS_kick.wav").exists());
}

#[test]
fn queued_auto_rename_replays_against_active_rename_success_path() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "raw.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);

    controller
        .runtime
        .source_lane
        .mutations
        .begin_browser_rename_intent(
            crate::app::controller::state::runtime::BrowserRenameIntentKey::new(
                source.id.clone(),
                vec![(PathBuf::from("raw.wav"), PathBuf::from("portal_SS.wav"))],
            ),
        );
    let (_file_op_tx, file_op_rx) =
        std::sync::mpsc::channel::<crate::app::controller::jobs::FileOpMessage>();
    controller.runtime.jobs.start_file_ops(
        file_op_rx,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    );

    controller
        .apply_browser_tag_sidebar_sound_type(Some(crate::sample_sources::SampleSoundType::Kick))
        .expect("sound type should apply");
    controller.auto_rename_browser_selection_action(Some(0));

    let first_relative = Path::new("portal_SS.wav");
    std::fs::rename(
        source.root.join("raw.wav"),
        source.root.join(first_relative),
    )
    .unwrap();
    let db = controller.database_for(&source).unwrap();
    db.remove_file(Path::new("raw.wav")).unwrap();
    db.upsert_file(first_relative, 0, 0).unwrap();
    db.set_tag(first_relative, crate::sample_sources::Rating::NEUTRAL)
        .unwrap();
    db.set_sound_type(
        first_relative,
        Some(crate::sample_sources::SampleSoundType::Kick),
    )
    .unwrap();
    let mut entry = sample_entry("portal_SS.wav", crate::sample_sources::Rating::NEUTRAL);
    entry.sound_type = Some(crate::sample_sources::SampleSoundType::Kick);

    controller.runtime.jobs.clear_file_ops();
    controller.apply_file_op_result(
        crate::app::controller::jobs::FileOpResult::SampleAutoRename(
            crate::app::controller::jobs::SampleAutoRenameResult {
                source_id: source.id.clone(),
                requested_paths: vec![PathBuf::from("raw.wav")],
                renamed: vec![crate::app::controller::jobs::SampleAutoRenameSuccess {
                    old_relative: PathBuf::from("raw.wav"),
                    new_relative: PathBuf::from("portal_SS.wav"),
                    entry,
                    resume_playback: false,
                    resume_looped: false,
                    resume_start_override: None,
                }],
                skipped: Vec::new(),
                errors: Vec::new(),
            },
        ),
    );

    assert!(!source.root.join("raw.wav").exists());
    assert!(!source.root.join("portal_SS.wav").exists());
    assert!(source.root.join("portal_SS_kick.wav").exists());
}

#[test]
fn active_auto_rename_batch_tracks_progress_remaps_and_clears_on_finish() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.show_status_progress(
        crate::app::state::ProgressTaskKind::FileOps,
        "Preparing auto rename",
        2,
        true,
    );
    controller
        .runtime
        .source_lane
        .mutations
        .begin_browser_rename_intent(BrowserRenameIntentKey::new(
            source.id.clone(),
            vec![
                (PathBuf::from("alpha.wav"), PathBuf::from("alpha.wav")),
                (PathBuf::from("beta.wav"), PathBuf::from("beta.wav")),
            ],
        ));
    controller
        .runtime
        .source_lane
        .mutations
        .begin_auto_rename_batch(
            source.id.clone(),
            vec![PathBuf::from("alpha.wav"), PathBuf::from("beta.wav")],
        );
    let (_tx, rx) = std::sync::mpsc::channel();
    controller.runtime.jobs.start_file_ops(
        rx,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    );

    assert_auto_rename_row(
        &controller
            .runtime
            .source_lane
            .mutations
            .active_auto_rename_batch_snapshot()
            .expect("active batch")
            .rows[0],
        "alpha.wav",
        "alpha.wav",
        AutoRenameBatchRowState::Queued,
    );

    controller
        .runtime
        .source_lane
        .mutations
        .apply_auto_rename_progress(SampleAutoRenameProgress::Active {
            old_relative: PathBuf::from("alpha.wav"),
        });
    let snapshot = controller
        .runtime
        .source_lane
        .mutations
        .active_auto_rename_batch_snapshot()
        .expect("active batch after start");
    assert_eq!(snapshot.current_path, Some(PathBuf::from("alpha.wav")));
    assert_auto_rename_row(
        &snapshot.rows[0],
        "alpha.wav",
        "alpha.wav",
        AutoRenameBatchRowState::Active,
    );

    controller
        .runtime
        .source_lane
        .mutations
        .apply_auto_rename_progress(SampleAutoRenameProgress::Completed {
            old_relative: PathBuf::from("alpha.wav"),
            new_relative: PathBuf::from("alpha_renamed.wav"),
        });
    controller
        .runtime
        .source_lane
        .mutations
        .apply_auto_rename_progress(SampleAutoRenameProgress::Active {
            old_relative: PathBuf::from("beta.wav"),
        });
    controller
        .runtime
        .source_lane
        .mutations
        .apply_auto_rename_progress(SampleAutoRenameProgress::Failed {
            old_relative: PathBuf::from("beta.wav"),
            error: String::from("Disk error"),
        });

    let snapshot = controller
        .runtime
        .source_lane
        .mutations
        .active_auto_rename_batch_snapshot()
        .expect("active batch after item progress");
    assert_eq!(snapshot.current_path, None);
    assert_eq!(
        snapshot.remaps,
        vec![(
            PathBuf::from("alpha.wav"),
            PathBuf::from("alpha_renamed.wav")
        )]
    );
    assert_auto_rename_row(
        &snapshot.rows[0],
        "alpha.wav",
        "alpha_renamed.wav",
        AutoRenameBatchRowState::Completed,
    );
    assert_auto_rename_row(
        &snapshot.rows[1],
        "beta.wav",
        "beta.wav",
        AutoRenameBatchRowState::Failed,
    );

    controller.apply_file_op_result(
        crate::app::controller::jobs::FileOpResult::SampleAutoRename(
            crate::app::controller::jobs::SampleAutoRenameResult {
                source_id: source.id,
                requested_paths: vec![PathBuf::from("alpha.wav"), PathBuf::from("beta.wav")],
                renamed: Vec::new(),
                skipped: Vec::new(),
                errors: vec![(PathBuf::from("beta.wav"), String::from("Disk error"))],
            },
        ),
    );

    assert!(
        controller
            .runtime
            .source_lane
            .mutations
            .active_auto_rename_batch_snapshot()
            .is_none()
    );
}

#[test]
fn active_auto_rename_batch_clears_when_selected_source_changes() {
    let (mut controller, first) = dummy_controller();
    let second_temp = tempfile::tempdir().unwrap();
    let second = crate::sample_sources::SampleSource::new(second_temp.path().to_path_buf());
    controller.library.sources.push(first.clone());
    controller.library.sources.push(second.clone());
    controller.select_source_by_index(0);
    controller
        .runtime
        .source_lane
        .mutations
        .begin_auto_rename_batch(first.id.clone(), vec![PathBuf::from("alpha.wav")]);

    assert!(
        controller
            .runtime
            .source_lane
            .mutations
            .active_auto_rename_batch_snapshot()
            .is_some()
    );

    controller.select_source_by_index(1);

    assert!(
        controller
            .runtime
            .source_lane
            .mutations
            .active_auto_rename_batch_snapshot()
            .is_none()
    );
}

fn assert_auto_rename_row(
    row: &AutoRenameBatchRowSnapshot,
    requested: &str,
    current: &str,
    state: AutoRenameBatchRowState,
) {
    assert_eq!(row.requested_path, PathBuf::from(requested));
    assert_eq!(row.current_path, PathBuf::from(current));
    assert_eq!(row.state, state);
}
