use super::*;

#[test]
fn find_similar_for_visible_row_does_not_enqueue_analysis_jobs() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("anchor.wav", Rating::NEUTRAL),
        sample_entry("near.wav", Rating::NEUTRAL),
    ]);
    controller.set_similarity_prep_fast_mode_enabled(true);
    let fast_sample_rate = controller.similarity_prep_fast_sample_rate();
    insert_similarity_embedding(&source, "anchor.wav", 1.0, 0.0);
    insert_similarity_embedding(&source, "near.wav", 0.95, 0.05);
    let sample_id = set_fast_similarity_metadata(&source, "anchor.wav", fast_sample_rate);

    controller.find_similar_for_visible_row(0).unwrap();

    assert_eq!(count_analysis_jobs(&source, &sample_id), 0);
    let query = controller
        .ui
        .browser
        .search
        .similar_query
        .as_ref()
        .expect("similar query");
    assert_eq!(query.sample_id, sample_id);
    assert_eq!(query.anchor_index, Some(0));
}
