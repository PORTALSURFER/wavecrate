use super::super::super::test_support::{
    prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::analysis::vector::encode_f32_le_blob;
use crate::app::controller::jobs::{
    ActiveRetainedDeleteResolution, RetainedDeleteBusyEntry, RetainedDeleteResolutionMode,
};
use crate::app::controller::library::analysis_jobs;
use crate::app::controller::state::audio::{AudioLoadIntent, PendingAudio};
use crate::app::controller::ui::hotkeys;
use crate::app::state::FocusContext;
use crate::sample_sources::Rating;
use rusqlite::params;
use std::path::{Path, PathBuf};

fn normalize_embedding(values: &mut [f32]) {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in values {
            *value /= norm;
        }
    }
}

fn insert_similarity_embedding(
    source: &crate::sample_sources::SampleSource,
    relative_path: &str,
    x: f32,
    y: f32,
) {
    let conn = crate::sample_sources::SourceDatabase::open_connection(&source.root)
        .expect("open source DB");
    let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), Path::new(relative_path));
    let mut embedding = vec![0.0_f32; crate::analysis::similarity::SIMILARITY_DIM];
    embedding[0] = x;
    embedding[1] = y;
    normalize_embedding(&mut embedding);
    let blob = encode_f32_le_blob(&embedding);
    conn.execute(
        "DELETE FROM embeddings WHERE sample_id = ?1 AND model_id = ?2",
        params![sample_id, crate::analysis::similarity::SIMILARITY_MODEL_ID,],
    )
    .expect("clear old embedding");
    conn.execute(
        "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, ?2, ?3, 'f32', 1, ?4, 0)",
        params![
            sample_id,
            crate::analysis::similarity::SIMILARITY_MODEL_ID,
            crate::analysis::similarity::SIMILARITY_DIM as i64,
            blob,
        ],
    )
    .expect("insert embedding");
    crate::analysis::rebuild_ann_index(&conn).expect("rebuild ann index");
}

fn visible_browser_paths(controller: &mut crate::app::controller::AppController) -> Vec<PathBuf> {
    (0..controller.visible_browser_len())
        .filter_map(|row| controller.browser_path_for_visible(row))
        .collect()
}

#[test]
fn hotkey_tagging_applies_to_all_selected_rows() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.tag_selected_left();

    assert_eq!(controller.wav_entry(0).unwrap().tag, Rating::TRASH_3);
    assert_eq!(controller.wav_entry(1).unwrap().tag, Rating::TRASH_3);
}

#[test]
fn x_key_toggle_respects_focus() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);

    controller.focus_browser_row(0);
    controller.toggle_focused_selection();
    assert!(controller.ui.browser.selection.selected_paths.is_empty());
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));

    controller.toggle_focused_selection();
    assert!(
        controller
            .ui
            .browser
            .selection
            .selected_paths
            .iter()
            .any(|path| path == &PathBuf::from("one.wav"))
    );
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
}

#[test]
fn action_rows_include_selection_and_primary() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
    controller.set_browser_selected_indices(vec![0, 2]);

    let rows = controller.action_rows_from_primary(1);

    assert_eq!(rows, vec![0, 1, 2]);
}

#[test]
fn tag_actions_apply_to_all_selected_rows() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);

    controller.focus_browser_row(0);
    controller.toggle_browser_row_selection(1);
    let rows = controller.action_rows_from_primary(0);

    controller
        .tag_browser_samples(&rows, Rating::KEEP_1, 0)
        .unwrap();

    assert_eq!(controller.wav_entry(0).unwrap().tag, Rating::KEEP_1);
    assert_eq!(controller.wav_entry(1).unwrap().tag, Rating::KEEP_1);
}

#[test]
fn delete_actions_apply_to_all_selected_rows() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.toggle_browser_row_selection(2);
    let rows = controller.action_rows_from_primary(0);

    controller.delete_browser_samples(&rows).unwrap();

    assert_eq!(controller.wav_entries_len(), 0);
    assert!(!source.root.join("one.wav").exists());
    assert!(!source.root.join("two.wav").exists());
    assert!(!source.root.join("three.wav").exists());
}

#[test]
fn deleting_similarity_result_recomputes_filter_from_same_anchor() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("anchor.wav", Rating::NEUTRAL),
        sample_entry("close.wav", Rating::NEUTRAL),
        sample_entry("far.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("anchor.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("close.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("far.wav"), &[0.0, 0.1]);
    insert_similarity_embedding(&source, "anchor.wav", 1.0, 0.0);
    insert_similarity_embedding(&source, "close.wav", 0.9, 0.1);
    insert_similarity_embedding(&source, "far.wav", 0.7, 0.3);

    controller.find_similar_for_visible_row(0).unwrap();

    controller.delete_browser_samples(&[1]).unwrap();

    let query = controller
        .ui
        .browser
        .search
        .similar_query
        .as_ref()
        .expect("recomputed similarity query");
    assert_eq!(
        query.sample_id,
        analysis_jobs::build_sample_id(source.id.as_str(), Path::new("anchor.wav"))
    );
    assert_eq!(
        visible_browser_paths(&mut controller),
        vec![PathBuf::from("anchor.wav"), PathBuf::from("far.wav")]
    );
}

#[test]
fn deleting_similarity_anchor_promotes_next_best_survivor() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("anchor.wav", Rating::NEUTRAL),
        sample_entry("close.wav", Rating::NEUTRAL),
        sample_entry("far.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("anchor.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("close.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("far.wav"), &[0.0, 0.1]);
    insert_similarity_embedding(&source, "anchor.wav", 1.0, 0.0);
    insert_similarity_embedding(&source, "close.wav", 0.9, 0.1);
    insert_similarity_embedding(&source, "far.wav", 0.7, 0.3);

    controller.find_similar_for_visible_row(0).unwrap();

    controller.delete_browser_samples(&[0]).unwrap();

    let query = controller
        .ui
        .browser
        .search
        .similar_query
        .as_ref()
        .expect("recomputed similarity query");
    assert_eq!(
        query.sample_id,
        analysis_jobs::build_sample_id(source.id.as_str(), Path::new("close.wav"))
    );
    assert_eq!(
        visible_browser_paths(&mut controller),
        vec![PathBuf::from("close.wav"), PathBuf::from("far.wav")]
    );
    assert_eq!(
        controller.focused_browser_path().as_deref(),
        Some(Path::new("close.wav"))
    );
}

#[test]
fn delete_hotkey_applies_to_all_selected_rows() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.toggle_browser_row_selection(2);
    let action = hotkeys::iter_actions()
        .find(|action| action.id == "delete-browser")
        .expect("delete-browser hotkey");

    controller.handle_hotkey(action, FocusContext::SampleBrowser);

    assert_eq!(controller.wav_entries_len(), 0);
    assert!(!source.root.join("one.wav").exists());
    assert!(!source.root.join("two.wav").exists());
    assert!(!source.root.join("three.wav").exists());
}

#[test]
fn delete_hotkey_keeps_focus_when_file_delete_fails() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a.wav", Rating::NEUTRAL),
        sample_entry("b.wav", Rating::NEUTRAL),
        sample_entry("c.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("c.wav"), &[0.0, 0.1]);
    controller.focus_browser_row_only(1);
    let action = hotkeys::iter_actions()
        .find(|action| action.id == "delete-browser")
        .expect("delete-browser hotkey");

    controller.handle_hotkey(action, FocusContext::SampleBrowser);

    assert_eq!(
        controller.focused_browser_path().as_deref(),
        Some(Path::new("b.wav"))
    );
    assert_eq!(visible_browser_paths(&mut controller).len(), 3);
    assert_eq!(controller.ui.status.status_tone, crate::app::state::StatusTone::Error);
    assert!(controller.ui.status.text.contains("Failed to delete file"));
}

#[test]
fn delete_hotkey_waits_for_loading_sample() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("one.wav", Rating::NEUTRAL)]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    controller.focus_browser_row_only(0);
    controller.ui.waveform.loading = Some(PathBuf::from("one.wav"));
    controller.runtime.jobs.set_pending_audio(Some(PendingAudio {
        request_id: 1,
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("one.wav"),
        intent: AudioLoadIntent::Selection,
    }));
    let action = hotkeys::iter_actions()
        .find(|action| action.id == "delete-browser")
        .expect("delete-browser hotkey");

    controller.handle_hotkey(action, FocusContext::SampleBrowser);

    assert!(source.root.join("one.wav").exists());
    assert_eq!(
        controller.focused_browser_path().as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(controller.wav_entries_len(), 1);
    assert_eq!(controller.ui.status.status_tone, crate::app::state::StatusTone::Info);
    assert_eq!(
        controller.ui.status.text,
        "Wait for sample load to finish before deleting one.wav"
    );
}

#[test]
fn delete_browser_samples_reports_partial_failure_and_refocuses_survivor() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a.wav", Rating::NEUTRAL),
        sample_entry("b.wav", Rating::NEUTRAL),
        sample_entry("c.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("c.wav"), &[0.0, 0.1]);
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    let rows = controller.action_rows_from_primary(0);

    let err = controller
        .delete_browser_samples(&rows)
        .expect_err("partial delete should report failure");

    assert!(!source.root.join("a.wav").exists());
    assert_eq!(
        visible_browser_paths(&mut controller),
        vec![PathBuf::from("b.wav"), PathBuf::from("c.wav")]
    );
    assert_eq!(
        controller.focused_browser_path().as_deref(),
        Some(Path::new("c.wav"))
    );
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Warning
    );
    assert!(controller.ui.status.text.contains("Deleted 1 sample with 1 error"));
    assert_eq!(controller.ui.status.text, err);
}

#[test]
fn normalize_actions_apply_to_all_selected_rows() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    let rows = controller.action_rows_from_primary(0);

    controller.normalize_browser_samples(&rows).unwrap();

    let entries = controller.wav_entries.pages.get(&0).expect("entries");
    assert!(entries.iter().all(|entry| entry.modified_ns > 0));
    assert!(entries.iter().all(|entry| entry.file_size > 0));
}

#[test]
fn delete_actions_warn_when_retained_recovery_is_processing_selected_rows() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("busy/one.wav", Rating::NEUTRAL),
        sample_entry("busy/two.wav", Rating::NEUTRAL),
    ]);
    std::fs::create_dir_all(source.root.join("busy")).unwrap();
    write_test_wav(&source.root.join("busy/one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("busy/two.wav"), &[0.0, 0.1]);
    controller.runtime.active_retained_delete_resolution = Some(ActiveRetainedDeleteResolution {
        entries: vec![RetainedDeleteBusyEntry {
            mode: RetainedDeleteResolutionMode::Restore,
            source_id: source.id.clone(),
            source_label: "source".into(),
            relative_path: PathBuf::from("busy"),
        }],
    });

    controller.delete_browser_samples(&[0, 1]).unwrap();

    assert_eq!(controller.wav_entries_len(), 2);
    assert!(source.root.join("busy/one.wav").exists());
    assert!(source.root.join("busy/two.wav").exists());
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Warning
    );
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Recovery is still restoring")
    );
}

#[test]
fn normalize_actions_warn_when_retained_recovery_is_processing_selected_rows() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("busy/one.wav", Rating::NEUTRAL),
        sample_entry("busy/two.wav", Rating::NEUTRAL),
    ]);
    std::fs::create_dir_all(source.root.join("busy")).unwrap();
    write_test_wav(&source.root.join("busy/one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("busy/two.wav"), &[0.0, 0.1]);
    controller.runtime.active_retained_delete_resolution = Some(ActiveRetainedDeleteResolution {
        entries: vec![RetainedDeleteBusyEntry {
            mode: RetainedDeleteResolutionMode::Restore,
            source_id: source.id.clone(),
            source_label: "source".into(),
            relative_path: PathBuf::from("busy"),
        }],
    });

    controller.normalize_browser_samples(&[0, 1]).unwrap();

    let entries = controller.wav_entries.pages.get(&0).expect("entries");
    assert!(entries.iter().all(|entry| entry.modified_ns == 0));
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Warning
    );
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Recovery is still restoring")
    );
}

#[test]
fn selection_persists_when_nudging_focus() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);

    controller.focus_browser_row(0);
    controller.toggle_browser_row_selection(1);
    controller.nudge_selection(1);

    let selected = &controller.ui.browser.selection.selected_paths;
    assert!(selected.contains(&PathBuf::from("one.wav")));
    assert!(selected.contains(&PathBuf::from("two.wav")));
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));
}

#[test]
fn focused_row_actions_work_without_explicit_selection() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);

    controller.settings.controls.advance_after_rating = false;
    controller.nudge_selection(0);
    assert!(controller.ui.browser.selection.selected_paths.is_empty());

    controller.tag_selected_left();

    assert_eq!(controller.wav_entry(0).unwrap().tag, Rating::TRASH_3);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));
}

#[test]
fn nudge_selection_uses_random_mode_pool_without_repeating_current_row() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.focus_browser_row_only(0);
    controller.toggle_random_navigation_mode();
    controller
        .history
        .random_history
        .mark_played(&source.id, Path::new("two.wav"));

    controller.nudge_selection(1);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));
}
