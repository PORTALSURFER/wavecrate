use super::*;

#[test]
fn browser_tag_named_filter_supports_positive_and_negated_views() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    let mut tag_named = sample_entry("portal_SS_kick.wav", crate::sample_sources::Rating::NEUTRAL);
    tag_named.tag_named = true;
    controller.set_wav_entries_for_tests(vec![
        sample_entry("raw.wav", crate::sample_sources::Rating::NEUTRAL),
        tag_named,
        sample_entry("untouched.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.toggle_browser_tag_named_filter(false);
    assert_eq!(visible_indices(&controller), vec![1]);

    controller.toggle_browser_tag_named_filter(true);
    assert_eq!(visible_indices(&controller), vec![0, 2]);

    controller.toggle_browser_tag_named_filter(true);
    assert_eq!(visible_indices(&controller), vec![0, 1, 2]);
}
