use super::*;

#[test]
fn apply_ui_toggle_browser_sidebar_normal_tag_assigns_and_removes_candidate() {
    let (mut controller, source) = controller_with_source_entries(vec![wav_entry("one.wav")]);
    controller.focus_browser_row_only(0);

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::ToggleBrowserSidebarNormalTag {
            label: String::from("Texture"),
        },
    ));

    assert_eq!(
        tag_labels(
            controller
                .database_for(&source)
                .unwrap()
                .tags_for_path(Path::new("one.wav"))
                .unwrap()
        ),
        vec!["Texture"]
    );

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::ToggleBrowserSidebarNormalTag {
            label: String::from("Texture"),
        },
    ));

    assert!(
        controller
            .database_for(&source)
            .unwrap()
            .tags_for_path(Path::new("one.wav"))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn apply_ui_commit_browser_tag_sidebar_input_creates_normal_tag() {
    let (mut controller, source) = controller_with_source_entries(vec![wav_entry("one.wav")]);
    controller.focus_browser_row_only(0);
    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::SetBrowserTagSidebarInput {
            value: String::from("  Vinyl   Crackle "),
        },
    ));

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::CommitBrowserTagSidebarInput,
    ));

    assert_eq!(
        tag_labels(
            controller
                .database_for(&source)
                .unwrap()
                .tags_for_path(Path::new("one.wav"))
                .unwrap()
        ),
        vec!["Vinyl Crackle"]
    );
    assert_eq!(controller.ui.browser.tag_sidebar_input, "");
}

#[test]
fn apply_ui_commit_browser_tag_sidebar_input_tokenizes_comma_separated_tags() {
    let (mut controller, source) = controller_with_source_entries(vec![wav_entry("one.wav")]);
    controller.focus_browser_row_only(0);
    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::SetBrowserTagSidebarInput {
            value: String::from("kick, hard, one shot"),
        },
    ));

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::CommitBrowserTagSidebarInput,
    ));

    let mut labels = tag_labels(
        controller
            .database_for(&source)
            .unwrap()
            .tags_for_path(Path::new("one.wav"))
            .unwrap(),
    );
    labels.sort();
    assert_eq!(labels, vec!["hard", "kick", "one shot"]);
    assert_eq!(controller.ui.browser.tag_sidebar_input, "");
}

fn controller_with_source_entries(
    entries: Vec<WavEntry>,
) -> (AppController, crate::sample_sources::SampleSource) {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let temp = tempfile::tempdir().unwrap();
    let root = temp.keep().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let source = crate::sample_sources::SampleSource::new(root);
    controller.register_source_for_tests(source.clone());
    controller.select_browser_source_for_tests(source.id.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(entries);
    controller.rebuild_browser_lists();
    (controller, source)
}

fn wav_entry(name: &str) -> WavEntry {
    WavEntry {
        relative_path: Path::new(name).to_path_buf(),
        file_size: 0,
        modified_ns: 0,
        content_hash: None,
        tag: Rating::NEUTRAL,
        looped: false,
        sound_type: None,
        locked: false,
        missing: false,
        last_played_at: None,
        last_curated_at: None,
        user_tag: None,
        tag_named: false,
        normal_tags: Vec::new(),
    }
}

fn tag_labels(tags: Vec<crate::sample_sources::db::SourceTag>) -> Vec<String> {
    tags.into_iter().map(|tag| tag.display_label).collect()
}
