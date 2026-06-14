use super::*;

const SIMILARITY_TEST_SOURCE_ID: &str = "native-similarity-test";

#[test]
fn sample_row_selection_still_works_in_similarity_mode() {
    let mut state = crate::native_app::tests::gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let anchor = drums.join("anchor.wav");
    let near = drums.join("near.wav");
    fs::write(&anchor, []).expect("write anchor");
    fs::write(&near, []).expect("write near");
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
            Default::default(),
        ),
    );
    let anchor_id = anchor.display().to_string();
    let near_id = near.display().to_string();
    state
        .library
        .folder_browser
        .set_similarity_scores_for_tests(anchor_id, [(near_id.clone(), 0.9)].into_iter().collect());
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame_with_default_theme();

    runtime.dispatch_primary_click(text_center(&frame, "near"));

    assert_eq!(
        runtime
            .bridge()
            .state()
            .library
            .folder_browser
            .selected_file_id(),
        Some(near_id.as_str())
    );
}

#[test]
fn sample_browser_renders_similarity_header_only_in_similarity_mode() {
    let mut state = crate::native_app::tests::gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let anchor = drums.join("anchor.wav");
    fs::write(&anchor, []).expect("write anchor");
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
            Default::default(),
        ),
    );

    prepare_sample_browser_view(&mut state);
    let inactive_frame = crate::native_app::test_support::sample_browser::sample_browser(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 360.0));
    assert!(!inactive_frame.paint_plan.contains_text("Sim"));

    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ToggleSimilarityAnchor(
            anchor.display().to_string(),
        ),
    );
    prepare_sample_browser_view(&mut state);
    let active_frame = crate::native_app::test_support::sample_browser::sample_browser(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(720.0, 360.0));
    assert!(active_frame.paint_plan.contains_text("Sim"));
}

#[test]
fn sample_browser_similarity_anchor_resolves_production_scores() {
    let (mut state, _source_root, anchor_id, near_id, far_id, missing_id) =
        similarity_state_with_embeddings();
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ToggleSimilarityAnchor(
            anchor_id.clone(),
        ),
    );
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.queue_similarity_score_resolution(anchor_id.clone(), &mut context);
    super::super::run_command_for_tests(&mut state, context.into_command());

    let ordered_ids = state
        .library
        .folder_browser
        .selected_audio_files()
        .into_iter()
        .map(|file| file.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        ordered_ids,
        vec![
            anchor_id.clone(),
            near_id.clone(),
            far_id,
            missing_id.clone()
        ]
    );
    assert_eq!(
        state
            .library
            .folder_browser
            .similarity_display_strength_for_file(anchor_id.as_str()),
        Some(1.0)
    );
    assert!(
        state
            .library
            .folder_browser
            .similarity_display_strength_for_file(near_id.as_str())
            .is_some()
    );
    assert!(
        state
            .library
            .folder_browser
            .similarity_display_strength_for_file(missing_id.as_str())
            .is_none()
    );
}

#[test]
fn sample_browser_similarity_ignores_stale_score_results() {
    let (mut state, _source_root, anchor_id, near_id, _far_id, _missing_id) =
        similarity_state_with_embeddings();
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ToggleSimilarityAnchor(
            anchor_id.clone(),
        ),
    );
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.queue_similarity_score_resolution(anchor_id, &mut context);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ToggleSimilarityAnchor(
            near_id.clone(),
        ),
    );
    super::super::run_command_for_tests(&mut state, context.into_command());

    assert_eq!(
        state.library.folder_browser.similarity_anchor_id(),
        Some(near_id.as_str())
    );
    assert_eq!(
        state
            .library
            .folder_browser
            .similarity_display_strength_for_file(near_id.as_str()),
        Some(1.0)
    );
}

fn similarity_state_with_embeddings() -> (
    crate::native_app::test_support::state::NativeAppState,
    tempfile::TempDir,
    String,
    String,
    String,
    String,
) {
    let mut state = crate::native_app::tests::gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let anchor = drums.join("anchor.wav");
    let near = drums.join("near.wav");
    let far = drums.join("far.wav");
    let missing = drums.join("missing.wav");
    for path in [&anchor, &near, &far, &missing] {
        fs::write(path, []).expect("write sample");
    }
    seed_similarity_embedding(source_root.path(), "drums/anchor.wav", &[1.0, 0.0]);
    seed_similarity_embedding(source_root.path(), "drums/near.wav", &[0.8, 0.6]);
    seed_similarity_embedding(source_root.path(), "drums/far.wav", &[0.0, 1.0]);
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new_with_id(
                wavecrate::sample_sources::SourceId::from_string(SIMILARITY_TEST_SOURCE_ID),
                source_root.path().to_path_buf(),
            ),
        ]);
    state.library.folder_browser.apply_message(
        crate::native_app::test_support::state::FolderBrowserMessage::ActivateFolder(
            drums.display().to_string(),
            Default::default(),
        ),
    );
    (
        state,
        source_root,
        anchor.display().to_string(),
        near.display().to_string(),
        far.display().to_string(),
        missing.display().to_string(),
    )
}

fn seed_similarity_embedding(source_root: &std::path::Path, relative_path: &str, values: &[f32]) {
    let _db = wavecrate::sample_sources::SourceDatabase::open(source_root).expect("source db");
    let conn = wavecrate::sample_sources::SourceDatabase::open_connection_with_role(
        source_root,
        wavecrate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("source db connection");
    let sample_id = format!("{SIMILARITY_TEST_SOURCE_ID}::{relative_path}");
    let blob = wavecrate_analysis::vector::encode_f32_le_blob(values);
    conn.execute(
        "INSERT OR REPLACE INTO embeddings
         (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, ?2, ?3, 'f32', 1, ?4, 0)",
        rusqlite::params![
            sample_id,
            wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
            values.len() as i64,
            blob
        ],
    )
    .expect("insert embedding");
}
