use super::*;
use std::time::{Duration, Instant};
use wavecrate_analysis::aspects::SimilarityAspect;

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
    assert!(active_frame.paint_plan.contains_text("Weight"));
}

#[test]
fn sample_browser_similarity_controls_emit_control_messages() {
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
        crate::native_app::test_support::state::FolderBrowserMessage::ToggleSimilarityAnchor(
            anchor.display().to_string(),
        ),
    );
    prepare_sample_browser_view(&mut state);

    let surface =
        crate::native_app::test_support::sample_browser::sample_browser(&state).into_surface();
    assert_eq!(
        surface.dispatch_widget_output(
            crate::native_app::ui::ids::SAMPLE_SIMILARITY_WEIGHTING_TOGGLE_ID,
            radiant::widgets::WidgetOutput::typed(radiant::widgets::ToggleMessage::ValueChanged {
                checked: true
            },),
        ),
        Some(
            crate::native_app::test_support::state::GuiMessage::SetSimilarityAspectWeightingEnabled(
                true
            )
        )
    );
    assert_eq!(
        surface.dispatch_widget_output(
            radiant::widgets::stable_widget_id(
                crate::native_app::ui::ids::SAMPLE_SIMILARITY_ASPECT_TOGGLE_SCOPE,
                "spectrum",
            ),
            radiant::widgets::WidgetOutput::typed(radiant::widgets::ToggleMessage::ValueChanged {
                checked: false
            },),
        ),
        Some(
            crate::native_app::test_support::state::GuiMessage::SetSimilarityAspectEnabled {
                aspect: SimilarityAspect::Spectrum,
                enabled: false,
            },
        )
    );
    assert_eq!(
        surface.dispatch_widget_output(
            radiant::widgets::stable_widget_id(
                crate::native_app::ui::ids::SAMPLE_SIMILARITY_ASPECT_WEIGHT_SCOPE,
                "spectrum",
            ),
            radiant::widgets::WidgetOutput::typed(radiant::widgets::SliderMessage::ValueChanged {
                value: 0.25
            },),
        ),
        Some(
            crate::native_app::test_support::state::GuiMessage::SetSimilarityAspectWeight {
                aspect: SimilarityAspect::Spectrum,
                weight: 0.25,
            },
        )
    );
}

#[test]
fn sample_browser_similarity_controls_reorder_and_mute_disabled_aspects() {
    let mut state = crate::native_app::tests::gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let anchor = drums.join("anchor.wav");
    let raw_near = drums.join("raw_near.wav");
    let spectrum_near = drums.join("spectrum_near.wav");
    for path in [&anchor, &raw_near, &spectrum_near] {
        fs::write(path, []).expect("write sample");
    }
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
    let raw_near_id = raw_near.display().to_string();
    let spectrum_near_id = spectrum_near.display().to_string();
    let mut raw_near_aspects =
        crate::native_app::sample_library::folder_browser::model::EMPTY_SIMILARITY_ASPECT_STRENGTHS;
    raw_near_aspects[SimilarityAspect::Spectrum.index()] = Some(0.1);
    raw_near_aspects[SimilarityAspect::Timbre.index()] = Some(0.9);
    let mut spectrum_near_aspects =
        crate::native_app::sample_library::folder_browser::model::EMPTY_SIMILARITY_ASPECT_STRENGTHS;
    spectrum_near_aspects[SimilarityAspect::Spectrum.index()] = Some(0.95);
    spectrum_near_aspects[SimilarityAspect::Timbre.index()] = Some(0.9);
    state
        .library
        .folder_browser
        .set_similarity_scores_with_aspects(
            anchor_id.clone(),
            [(raw_near_id.clone(), 0.9), (spectrum_near_id.clone(), 0.2)]
                .into_iter()
                .collect(),
            [
                (raw_near_id.clone(), raw_near_aspects),
                (spectrum_near_id.clone(), spectrum_near_aspects),
            ]
            .into_iter()
            .collect(),
        );
    assert_eq!(
        state
            .library
            .folder_browser
            .selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>(),
        vec![
            anchor_id.clone(),
            raw_near_id.clone(),
            spectrum_near_id.clone()
        ]
    );

    let mut context = radiant::prelude::UiUpdateContext::default();
    for message in [
        crate::native_app::test_support::state::GuiMessage::SetSimilarityAspectWeightingEnabled(
            true,
        ),
        crate::native_app::test_support::state::GuiMessage::SetSimilarityAspectEnabled {
            aspect: SimilarityAspect::Overall,
            enabled: false,
        },
        crate::native_app::test_support::state::GuiMessage::SetSimilarityAspectEnabled {
            aspect: SimilarityAspect::Timbre,
            enabled: false,
        },
        crate::native_app::test_support::state::GuiMessage::SetSimilarityAspectEnabled {
            aspect: SimilarityAspect::Pitch,
            enabled: false,
        },
        crate::native_app::test_support::state::GuiMessage::SetSimilarityAspectEnabled {
            aspect: SimilarityAspect::Amplitude,
            enabled: false,
        },
    ] {
        state.apply_message(message, &mut context);
    }

    assert_eq!(
        state
            .library
            .folder_browser
            .selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>(),
        vec![anchor_id, spectrum_near_id.clone(), raw_near_id]
    );
    let strengths = state
        .library
        .folder_browser
        .similarity_aspect_display_strengths_for_file(spectrum_near_id.as_str());
    assert!(strengths[SimilarityAspect::Spectrum.index()].is_some());
    assert_eq!(strengths[SimilarityAspect::Timbre.index()], None);
}

#[test]
fn sample_browser_similarity_controls_persist_after_debounce() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let mut state = crate::native_app::tests::gui_state_for_span_tests();
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SetSimilarityAspectWeightingEnabled(
            true,
        ),
        &mut context,
    );

    let loaded = wavecrate::sample_sources::config::load_or_default().expect("load config");
    assert!(!loaded.core.similarity.weighting_enabled);
    assert!(state.ui.settings.similarity_persist_deadline.is_some());

    state.ui.settings.similarity_persist_deadline = Some(Instant::now() - Duration::from_millis(1));
    state.advance_frame(&mut context);
    let radiant::runtime::Command::Perform { priority, work, .. } = context.into_command() else {
        panic!("expected similarity settings persist background command");
    };
    assert_eq!(priority, radiant::prelude::TaskPriority::BlockingIo);
    state.apply_message(work(), &mut radiant::prelude::UiUpdateContext::default());

    let loaded = wavecrate::sample_sources::config::load_or_default().expect("reload config");
    assert!(loaded.core.similarity.weighting_enabled);
    assert!(!state.ui.settings.similarity_persist_inflight);
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
            .similarity_aspect_display_strengths_for_file(near_id.as_str())
            [wavecrate_analysis::aspects::SimilarityAspect::Spectrum.index()]
        .is_some_and(|strength| (strength - 1.0).abs() < 1e-5)
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

#[test]
fn source_prep_trigger_queues_cache_warm_and_similarity_jobs() {
    let mut state = crate::native_app::tests::gui_state_for_span_tests();
    let source_root = tempfile::tempdir().expect("source root");
    let drums = source_root.path().join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let first = drums.join("first.wav");
    let second = drums.join("second.wav");
    fs::write(&first, []).expect("write first sample");
    fs::write(&second, []).expect("write second sample");
    seed_source_scan_row(source_root.path(), "drums/first.wav");
    seed_source_scan_row(source_root.path(), "drums/second.wav");

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
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.queue_selected_source_prep(
        crate::native_app::sample_library::source_prep::SourcePrepTrigger::SourceSelected,
        &mut context,
    );

    assert_eq!(state.waveform.cache.active_folder_warm_pending.len(), 2);
    assert!(
        state
            .waveform
            .cache
            .active_folder_warm_delay_task
            .active()
            .is_some()
    );

    super::super::run_command_for_tests(&mut state, context.into_command());

    assert_eq!(
        pending_source_jobs(source_root.path(), "wav_metadata_v1"),
        2
    );
    assert_eq!(
        pending_source_jobs(source_root.path(), "embedding_backfill_v1"),
        1
    );
    assert!(!state.library.similarity_prep.running);
}

#[test]
fn source_prep_retains_similarity_trigger_while_another_source_is_running() {
    let mut state = crate::native_app::tests::gui_state_for_span_tests();
    let first_root = tempfile::tempdir().expect("first root");
    let second_root = tempfile::tempdir().expect("second root");
    fs::write(first_root.path().join("first.wav"), []).expect("write first sample");
    fs::write(second_root.path().join("second.wav"), []).expect("write second sample");
    let first_source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("first-source"),
        first_root.path().to_path_buf(),
    );
    let second_source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("second-source"),
        second_root.path().to_path_buf(),
    );
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            first_source,
            second_source,
        ]);
    let mut context = radiant::prelude::UiUpdateContext::default();

    state.queue_source_prep(
        String::from("first-source"),
        crate::native_app::sample_library::source_prep::SourcePrepTrigger::SourceSelected,
        &mut context,
    );
    state.queue_source_prep(
        String::from("second-source"),
        crate::native_app::sample_library::source_prep::SourcePrepTrigger::FilesystemChanged,
        &mut context,
    );

    assert_eq!(
        state.library.similarity_prep.running_source_id.as_deref(),
        Some("first-source")
    );
    assert_eq!(
        state
            .library
            .similarity_prep
            .pending_source_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["second-source"]
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
    seed_similarity_aspects(source_root.path(), "drums/anchor.wav");
    seed_similarity_aspects(source_root.path(), "drums/near.wav");
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

fn seed_source_scan_row(source_root: &std::path::Path, relative_path: &str) {
    let db = wavecrate::sample_sources::SourceDatabase::open(source_root).expect("source db");
    db.upsert_file(std::path::Path::new(relative_path), 0, 10)
        .expect("file row");
    db.set_metadata(
        wavecrate::sample_sources::db::META_LAST_SCAN_COMPLETED_AT,
        "20",
    )
    .expect("scan timestamp");
}

fn pending_source_jobs(source_root: &std::path::Path, job_type: &str) -> i64 {
    let conn = wavecrate::sample_sources::SourceDatabase::open_connection_with_role(
        source_root,
        wavecrate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("source db connection");
    conn.query_row(
        "SELECT COUNT(*) FROM analysis_jobs WHERE job_type = ?1 AND status = 'pending'",
        [job_type],
        |row| row.get(0),
    )
    .expect("pending job count")
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

fn seed_similarity_aspects(source_root: &std::path::Path, relative_path: &str) {
    let _db = wavecrate::sample_sources::SourceDatabase::open(source_root).expect("source db");
    let conn = wavecrate::sample_sources::SourceDatabase::open_connection_with_role(
        source_root,
        wavecrate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("source db connection");
    let sample_id = format!("{SIMILARITY_TEST_SOURCE_ID}::{relative_path}");
    let mut features = vec![0.0_f32; wavecrate_analysis::FEATURE_VECTOR_LEN_V1];
    for (index, value) in features.iter_mut().enumerate() {
        *value = index as f32 + 1.0;
    }
    let descriptors =
        wavecrate_analysis::aspects::AspectDescriptorSet::from_feature_vector_v1(&features)
            .expect("aspect descriptors");
    let blob = wavecrate_analysis::vector::encode_f32_le_blob(descriptors.packed());
    conn.execute(
        "INSERT OR IGNORE INTO samples (sample_id, content_hash, size, mtime_ns)
         VALUES (?1, 'test-hash', 0, 0)",
        rusqlite::params![sample_id],
    )
    .expect("insert sample row");
    conn.execute(
        "INSERT OR REPLACE INTO similarity_aspect_descriptors
         (sample_id, model_id, dim, dtype, l2_normed, valid_mask, vec, created_at)
         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, 0)",
        rusqlite::params![
            sample_id,
            wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
            wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
            wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
            descriptors.valid_mask() as i64,
            blob
        ],
    )
    .expect("insert aspect descriptors");
}
