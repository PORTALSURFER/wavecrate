use super::*;

#[test]
fn metadata_tag_input_persists_tag_assignments_and_removals_to_source_database() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("persistent-tag.wav");
    fs::write(&sample_path, []).expect("sample file");
    let source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("metadata-tags-persist-test"),
        source_root.path().to_path_buf(),
    );
    let source_id = source.id.as_str().to_string();
    wavecrate::sample_sources::config::save(&crate::native_app::test_support::AppConfig {
        sources: vec![source.clone()],
        core: crate::native_app::test_support::AppSettingsCore::default(),
    })
    .expect("seed config");
    let selected_file = sample_path.display().to_string();
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[source]);
    state
        .library
        .folder_browser
        .select_file(selected_file.clone());

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Submitted {
                value: String::from("Deep Kick, Warm Tone"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );
    assert_eq!(
        state.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("deep-kick"), String::from("warm-tone")])
    );

    crate::native_app::metadata::persist_metadata_tag_additions_for_tests(
        sample_path.clone(),
        source_root.path().to_path_buf(),
        PathBuf::from("persistent-tag.wav"),
        vec![String::from("deep-kick"), String::from("warm-tone")],
    )
    .expect("persist tags");

    let db = wavecrate::sample_sources::SourceDatabase::open(source_root.path())
        .expect("open source db");
    assert_eq!(
        db.tag_labels_for_path(std::path::Path::new("persistent-tag.wav"))
            .expect("tag labels"),
        vec![String::from("deep-kick"), String::from("warm-tone")]
    );

    crate::native_app::metadata::persist_metadata_tag_removals_for_tests(
        sample_path.clone(),
        source_root.path().to_path_buf(),
        PathBuf::from("persistent-tag.wav"),
        vec![String::from("deep-kick")],
    )
    .expect("persist tag removal");

    assert_eq!(
        db.tag_labels_for_path(std::path::Path::new("persistent-tag.wav"))
            .expect("tag labels after removal"),
        vec![String::from("warm-tone")]
    );

    let mut reloaded = NativeAppState::load_default().expect("default state reloads");
    reloaded.refresh_persisted_metadata_tags_for_source(&source_id);
    assert_eq!(
        reloaded.metadata.tags_by_file.get(&selected_file),
        Some(&vec![String::from("warm-tone")])
    );
}

#[test]
fn metadata_tag_input_keeps_delimiters_while_editing() {
    let mut state = gui_state_for_span_tests();

    state.apply_message(
        crate::native_app::test_support::GuiMessage::MetadataTagInput(
            radiant::widgets::TextInputMessage::Changed {
                value: String::from("kick, warm tone"),
            },
        ),
        &mut ui::UpdateContext::default(),
    );

    assert!(state.metadata.tags_by_file.is_empty());
    assert_eq!(state.metadata.tag_draft, "kick, warm tone");
}
