use super::*;

#[test]
fn tag_sidebar_auto_rename_logs_metadata_provenance() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);
    let entry = sample_entry("raw.wav", crate::sample_sources::Rating::NEUTRAL);
    register_entry_metadata(&mut controller, &source, &entry);
    controller.set_wav_entries_for_tests(vec![entry]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.ui.browser.tag_sidebar_open = true;
    controller.ui.browser.tag_sidebar_auto_rename = true;
    controller.focus_browser_row_only(0);

    let captured = capture_info_logs(|| {
        controller
            .apply_browser_tag_sidebar_looped(true)
            .expect("loop click should auto rename");
    });

    assert!(
        captured.contains("auto rename: request metadata provenance")
            && captured.contains("raw.wav -> portal_loop.wav looped=true"),
        "tag-sidebar auto-rename should log requested loop provenance: {captured}"
    );
    assert!(
        captured.contains("auto rename: persisted loop metadata provenance")
            && captured.contains("old_path=raw.wav")
            && captured.contains("new_path=portal_loop.wav")
            && captured.contains("request_looped=true")
            && captured.contains("db_looped=Some(true)")
            && captured.contains("final_looped=true"),
        "tag-sidebar auto-rename should log DB and final loop provenance: {captured}"
    );
    assert!(
        captured.contains("source metadata mutation: source ops resolved")
            && captured.contains("SetLooped raw.wav")
            && captured.contains("result=\"ok\""),
        "tag-sidebar metadata write should log operation names and result: {captured}"
    );
}
