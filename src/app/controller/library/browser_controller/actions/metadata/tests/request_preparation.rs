use super::*;

#[test]
fn prepare_auto_rename_requests_prefers_live_sidebar_metadata() {
    let (mut controller, source) = dummy_controller();
    controller.settings.default_identifier = String::from("Artist Name");
    controller.ui.options_panel.default_identifier = String::from("Artist Name");
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);

    let mut entry = sample_entry("raw.wav", crate::sample_sources::Rating::NEUTRAL);
    entry.sound_type = Some(crate::sample_sources::SampleSoundType::Hat);
    entry.user_tag = Some(String::from("Live Tag"));
    controller.set_wav_entries_for_tests(vec![entry]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    let db = controller.database_for(&source).unwrap();
    db.set_sound_type(
        Path::new("raw.wav"),
        Some(crate::sample_sources::SampleSoundType::Kick),
    )
    .unwrap();
    db.set_user_tag(Path::new("raw.wav"), Some("DB Tag"))
        .unwrap();
    controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source.id.clone())
        .or_default()
        .insert(PathBuf::from("raw.wav"), Some(128.0));

    let request = BrowserController::new(&mut controller)
        .prepare_auto_rename_requests(&source, &[PathBuf::from("raw.wav")])
        .expect("request preparation should succeed")
        .into_iter()
        .next()
        .expect("request should exist");

    assert_eq!(
        request.sound_type,
        Some(crate::sample_sources::SampleSoundType::Hat)
    );
    assert_eq!(
        request.new_relative,
        PathBuf::from("artistname_SS_hat_livetag_128.wav")
    );
}

#[test]
fn prepare_auto_rename_requests_logs_looped_provenance() {
    let (mut controller, source) = dummy_controller();
    controller.settings.default_identifier = String::from("Artist Name");
    controller.ui.options_panel.default_identifier = String::from("Artist Name");
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);

    let mut entry = sample_entry("raw.wav", crate::sample_sources::Rating::NEUTRAL);
    entry.looped = true;
    controller.set_wav_entries_for_tests(vec![entry]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    let captured = capture_info_logs(|| {
        let requests = BrowserController::new(&mut controller)
            .prepare_auto_rename_requests(&source, &[PathBuf::from("raw.wav")])
            .expect("request preparation should succeed");
        assert_eq!(requests.len(), 1);
    });

    assert!(
        captured.contains("auto rename: request metadata provenance"),
        "request preparation should log metadata provenance: {captured}"
    );
    assert!(
        captured.contains("lane=\"controller\"")
            && captured.contains("request_count=1")
            && captured.contains("raw.wav -> artistname_loop.wav looped=true"),
        "log should include old path, new path, and requested loop value: {captured}"
    );
}
