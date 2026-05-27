use super::*;
use crate::app::controller::jobs::{
    ActiveRetainedDeleteResolution, ClipboardPasteOutcome, ClipboardPasteResult, FileOpMessage,
    FileOpResult, RetainedDeleteResolutionMode, RetainedDeleteResolutionResult,
    SampleAutoRenameProgress, SampleAutoRenameResult, SampleAutoRenameSuccess,
};
use crate::app::controller::state::runtime::BrowserRenameIntentKey;
use crate::app::controller::test_support::{dummy_controller, sample_entry, write_test_wav};
use crate::app::state::ProgressTaskKind;
use crate::app_core::actions::NativeBrowserRowProcessingState;
use crate::app_core::ui_projection::{project_browser_model, project_waveform_model};
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool, mpsc::channel};

#[test]
fn file_ops_messages_update_progress_and_clear_active_overlay_on_finish() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.show_status_progress(ProgressTaskKind::FileOps, "Copying files", 5, true);
    let (tx, rx) = channel();
    controller
        .runtime
        .jobs
        .start_file_ops(rx, Arc::new(AtomicBool::new(false)));
    drop(tx);

    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Progress {
        completed: 2,
        detail: Some("Copying kick.wav".into()),
        item: None,
    }));

    assert_eq!(controller.ui.progress.completed, 2);
    assert_eq!(
        controller.ui.progress.detail.as_deref(),
        Some("Copying kick.wav")
    );
    assert!(controller.runtime.jobs.file_ops_in_progress());

    let result = FileOpResult::ClipboardPaste(ClipboardPasteResult {
        outcome: ClipboardPasteOutcome::Source {
            source_id: source.id,
            added: Vec::new(),
        },
        skipped: 0,
        errors: Vec::new(),
        cancelled: true,
        target_label: "Source".into(),
        action_past_tense: "Pasted",
    });
    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Finished(result)));

    assert!(!controller.runtime.jobs.file_ops_in_progress());
    assert!(!controller.ui.progress.visible);
    assert_eq!(controller.ui.progress.task, None);
}

#[test]
fn retained_delete_resolution_result_clears_busy_scope_and_progress() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.show_status_progress(
        ProgressTaskKind::FileOps,
        "Restoring retained deletes",
        1,
        false,
    );
    controller.runtime.active_retained_delete_resolution = Some(ActiveRetainedDeleteResolution {
        entries: Vec::new(),
    });
    let (tx, rx) = channel();
    controller
        .runtime
        .jobs
        .start_file_ops(rx, Arc::new(AtomicBool::new(false)));
    drop(tx);

    let result = FileOpResult::RetainedDeleteResolution(RetainedDeleteResolutionResult {
        mode: RetainedDeleteResolutionMode::Restore,
        resolved: 1,
        affected_sources: vec![source.id],
        scan_sources: Vec::new(),
        failures: Vec::new(),
        recovery_report:
            crate::app::controller::library::source_folders::delete_recovery::DeleteRecoveryReport {
                entries: Vec::new(),
                retained_entries: Vec::new(),
                scan_sources: Vec::new(),
                errors: Vec::new(),
            },
    });
    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Finished(result)));

    assert!(
        controller
            .runtime
            .active_retained_delete_resolution
            .is_none()
    );
    assert!(!controller.runtime.jobs.file_ops_in_progress());
    assert!(!controller.ui.progress.visible);
}

#[test]
fn auto_rename_file_op_progress_drives_footer_rows_and_keeps_waveform_stable() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    for name in ["loaded.wav", "alpha.wav", "beta.wav", "gamma.wav"] {
        write_test_wav(&source.root.join(name), &[0.0, 0.1, -0.1]);
    }
    controller.set_wav_entries_for_tests(vec![
        sample_entry("loaded.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("alpha.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("beta.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("gamma.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    seed_stable_waveform_projection(&mut controller, &source, Path::new("loaded.wav"));

    let requested_paths = vec![
        PathBuf::from("alpha.wav"),
        PathBuf::from("beta.wav"),
        PathBuf::from("gamma.wav"),
    ];
    controller
        .runtime
        .source_lane
        .mutations
        .begin_browser_rename_intent(BrowserRenameIntentKey::new(
            source.id.clone(),
            requested_paths
                .iter()
                .cloned()
                .map(|path| (path.clone(), path))
                .collect(),
        ));
    controller
        .runtime
        .source_lane
        .mutations
        .begin_auto_rename_batch(source.id.clone(), requested_paths.clone());
    controller.begin_pending_file_mutation(&source.id, requested_paths.clone());
    controller.show_status_progress(
        ProgressTaskKind::FileOps,
        "Preparing auto rename",
        requested_paths.len(),
        true,
    );
    let (tx, rx) = channel();
    controller
        .runtime
        .jobs
        .start_file_ops(rx, Arc::new(AtomicBool::new(false)));
    drop(tx);

    let before_waveform = project_waveform_model(&mut controller);
    let projected = project_browser_model(&mut controller);
    assert_eq!(
        projected.rows[0].processing_state,
        NativeBrowserRowProcessingState::None
    );
    assert_eq!(
        projected.rows[1].processing_state,
        NativeBrowserRowProcessingState::Queued
    );
    assert_eq!(
        projected.rows[2].processing_state,
        NativeBrowserRowProcessingState::Queued
    );
    assert_eq!(
        projected.rows[3].processing_state,
        NativeBrowserRowProcessingState::Queued
    );

    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Progress {
        completed: 0,
        detail: None,
        item: Some(SampleAutoRenameProgress::Active {
            old_relative: PathBuf::from("alpha.wav"),
        }),
    }));
    assert_file_op_footer(&controller, 0, 3, None);
    let projected = project_browser_model(&mut controller);
    assert_eq!(
        projected.rows[1].processing_state,
        NativeBrowserRowProcessingState::Active
    );
    assert_waveform_projection_stable(&before_waveform, &project_waveform_model(&mut controller));

    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Progress {
        completed: 1,
        detail: Some(String::from("Renamed alpha.wav")),
        item: Some(SampleAutoRenameProgress::Completed {
            old_relative: PathBuf::from("alpha.wav"),
            new_relative: PathBuf::from("alpha_renamed.wav"),
        }),
    }));
    assert_file_op_footer(&controller, 1, 3, Some("Renamed alpha.wav"));
    let projected = project_browser_model(&mut controller);
    assert_eq!(projected.rows[1].label.as_ref(), "alpha");
    assert_eq!(
        projected.rows[1].processing_state,
        NativeBrowserRowProcessingState::Completed
    );

    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Progress {
        completed: 2,
        detail: Some(String::from("Skipped beta.wav")),
        item: Some(SampleAutoRenameProgress::Skipped {
            old_relative: PathBuf::from("beta.wav"),
            reason: String::from("Already named"),
        }),
    }));
    assert_file_op_footer(&controller, 2, 3, Some("Skipped beta.wav"));
    let projected = project_browser_model(&mut controller);
    assert_eq!(
        projected.rows[2].processing_state,
        NativeBrowserRowProcessingState::Skipped
    );

    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Progress {
        completed: 3,
        detail: Some(String::from("Failed gamma.wav")),
        item: Some(SampleAutoRenameProgress::Failed {
            old_relative: PathBuf::from("gamma.wav"),
            error: String::from("Disk error"),
        }),
    }));
    assert_file_op_footer(&controller, 3, 3, Some("Failed gamma.wav"));
    let projected = project_browser_model(&mut controller);
    assert_eq!(
        projected.rows[3].processing_state,
        NativeBrowserRowProcessingState::Failed
    );
    assert_waveform_projection_stable(&before_waveform, &project_waveform_model(&mut controller));

    let result = FileOpResult::SampleAutoRename(SampleAutoRenameResult {
        source_id: source.id.clone(),
        requested_paths: requested_paths.clone(),
        renamed: vec![auto_rename_success("alpha.wav", "alpha_renamed.wav")],
        skipped: vec![(PathBuf::from("beta.wav"), String::from("Already named"))],
        errors: vec![(PathBuf::from("gamma.wav"), String::from("Disk error"))],
    });
    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Finished(result)));

    let projected = project_browser_model(&mut controller);
    assert_eq!(projected.rows[1].label.as_ref(), "alpha_renamed");
    assert!(
        projected
            .rows
            .iter()
            .all(|row| row.processing_state == NativeBrowserRowProcessingState::None)
    );
    assert!(!controller.runtime.jobs.file_ops_in_progress());
    assert!(!controller.source_has_pending_file_mutations(&source.id));
    assert!(
        controller
            .runtime
            .source_lane
            .mutations
            .active_auto_rename_batch_snapshot()
            .is_none()
    );
    assert!(!controller.ui.progress.visible);
    assert_eq!(controller.ui.progress.task, None);
    assert_waveform_projection_stable(&before_waveform, &project_waveform_model(&mut controller));
}

#[test]
fn selection_export_progress_message_updates_status_bar_progress() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.runtime.jobs.set_pending_slice_batch_export(Some(
        crate::app::controller::jobs::PendingSliceBatchExport {
            request_id: 23,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("clip.wav"),
        },
    ));

    controller.handle_background_job_message(JobMessage::SelectionExport(
        SelectionExportMessage::Progress {
            request_id: 23,
            total: 4,
            completed: 2,
            detail: Some("Saving clip_slice002.wav".into()),
        },
    ));

    assert!(controller.ui.progress.visible);
    assert!(!controller.ui.progress.modal);
    assert_eq!(
        controller.ui.progress.task,
        Some(ProgressTaskKind::SelectionExport)
    );
    assert_eq!(controller.ui.progress.total, 4);
    assert_eq!(controller.ui.progress.completed, 2);
    assert_eq!(
        controller.ui.progress.detail.as_deref(),
        Some("Saving clip_slice002.wav")
    );
}

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

fn auto_rename_success(old_relative: &str, new_relative: &str) -> SampleAutoRenameSuccess {
    SampleAutoRenameSuccess {
        old_relative: PathBuf::from(old_relative),
        new_relative: PathBuf::from(new_relative),
        entry: sample_entry(new_relative, crate::sample_sources::Rating::NEUTRAL),
        resume_playback: false,
        resume_looped: false,
        resume_start_override: None,
    }
}

fn assert_file_op_footer(
    controller: &crate::app::controller::AppController,
    completed: usize,
    total: usize,
    detail: Option<&str>,
) {
    assert_eq!(controller.ui.progress.task, Some(ProgressTaskKind::FileOps));
    assert_eq!(controller.ui.progress.title, "Preparing auto rename");
    assert_eq!(controller.ui.progress.completed, completed);
    assert_eq!(controller.ui.progress.total, total);
    assert_eq!(controller.ui.progress.detail.as_deref(), detail);
}

fn assert_waveform_projection_stable(
    before: &crate::app_core::actions::NativeWaveformPanelModel,
    after: &crate::app_core::actions::NativeWaveformPanelModel,
) {
    assert_eq!(
        after.waveform_image_signature,
        before.waveform_image_signature
    );
    assert_eq!(after.loading, before.loading);
    assert_eq!(after.playhead_micros, before.playhead_micros);
    assert_eq!(after.loaded_label, before.loaded_label);
}
