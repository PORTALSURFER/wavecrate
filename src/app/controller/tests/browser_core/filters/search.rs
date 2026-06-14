use super::*;

#[test]
fn browser_search_limits_visible_rows() {
    let (mut controller, _source) = browser_rating_filter_fixture(false);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("kick.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("snare.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("hat.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.set_browser_search("snr");

    assert_eq!(visible_indices(&controller), vec![1]);
}

#[test]
fn browser_search_orders_results_by_score_then_index() {
    let (mut controller, _source) = browser_rating_filter_fixture(false);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("abc.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("abc_extra.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("abdc.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.set_browser_search("abc");

    assert_eq!(visible_indices(&controller), vec![0, 1, 2]);
}
