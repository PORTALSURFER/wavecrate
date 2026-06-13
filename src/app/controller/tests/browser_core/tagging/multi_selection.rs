use super::*;
#[test]
fn browser_tag_sidebar_multi_selection_commits_full_comma_token_set() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.set_browser_selected_paths(vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]);
    controller.set_browser_tag_sidebar_input(String::from("kick, hard"));

    controller
        .commit_browser_tag_sidebar_input()
        .expect("multi-selection should receive every token");

    let db = controller.database_for(&source).unwrap();
    let mut first_labels = tag_labels(db.tags_for_path(Path::new("one.wav")).unwrap());
    first_labels.sort();
    let mut second_labels = tag_labels(db.tags_for_path(Path::new("two.wav")).unwrap());
    second_labels.sort();
    assert_eq!(first_labels, vec!["hard", "kick"]);
    assert_eq!(second_labels, vec!["hard", "kick"]);
}

#[test]
fn browser_tag_sidebar_multi_selection_tracks_mixed_and_removes_normal_tags() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller
        .database_for(&source)
        .unwrap()
        .assign_tag_to_path(Path::new("one.wav"), "kick")
        .unwrap();
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    let paths = vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")];

    assert_eq!(
        controller
            .normal_tag_state_for_source(&source, &paths, "kick")
            .unwrap(),
        crate::app_core::actions::NativeBrowserTagState::Mixed
    );

    controller
        .apply_browser_tag_sidebar_normal_tag("kick")
        .expect("assignment should apply to every selected path");
    assert_eq!(
        controller
            .normal_tag_state_for_source(&source, &paths, "kick")
            .unwrap(),
        crate::app_core::actions::NativeBrowserTagState::On
    );
    controller
        .remove_browser_tag_sidebar_normal_tag("kick")
        .expect("removal should apply to every selected path");

    let db = controller.database_for(&source).unwrap();
    assert!(db.tags_for_path(Path::new("one.wav")).unwrap().is_empty());
    assert!(db.tags_for_path(Path::new("two.wav")).unwrap().is_empty());
}

#[test]
fn browser_tag_sidebar_multi_selection_queues_one_normal_tag_metadata_batch() {
    crate::app::controller::batch_latency::clear();
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    let paths = vec![
        PathBuf::from("one.wav"),
        PathBuf::from("two.wav"),
        PathBuf::from("three.wav"),
    ];
    controller.set_browser_selected_paths(paths.clone());

    controller
        .apply_browser_tag_sidebar_normal_tag("Vintage FX")
        .expect("assignment should batch selected paths");

    let samples = crate::app::controller::batch_latency::snapshot();
    let queue_samples = samples
        .iter()
        .filter(|sample| {
            sample.phase
                == crate::app::controller::batch_latency::BatchLatencyPhase::MetadataMutationQueue
        })
        .collect::<Vec<_>>();
    assert_eq!(queue_samples.len(), 1, "{samples:#?}");
    assert_eq!(queue_samples[0].item_count, paths.len());
    assert_eq!(queue_samples[0].detail_count, paths.len());
    for path in &paths {
        let index = controller.wav_index_for_path(path).unwrap();
        assert_eq!(
            controller.wav_entry(index).unwrap().normal_tags,
            vec![String::from("Vintage FX")]
        );
    }
}

#[test]
fn browser_tag_sidebar_multi_selection_queues_one_looped_metadata_batch() {
    crate::app::controller::batch_latency::clear();
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    let paths = vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")];
    controller.set_browser_selected_paths(paths.clone());

    controller
        .apply_browser_tag_sidebar_looped(true)
        .expect("loop marker should batch selected paths");

    let samples = crate::app::controller::batch_latency::snapshot();
    let queue_samples = samples
        .iter()
        .filter(|sample| {
            sample.phase
                == crate::app::controller::batch_latency::BatchLatencyPhase::MetadataMutationQueue
        })
        .collect::<Vec<_>>();
    assert_eq!(queue_samples.len(), 1, "{samples:#?}");
    assert_eq!(queue_samples[0].item_count, paths.len());
    assert_eq!(queue_samples[0].detail_count, paths.len());
    for path in &paths {
        let index = controller.wav_index_for_path(path).unwrap();
        assert!(controller.wav_entry(index).unwrap().looped);
    }
}
