use super::*;

#[test]
fn ann_index_matches_bruteforce_neighbors_on_fixture() {
    with_ann_test_db(|conn| {
        let dim = similarity::SIMILARITY_DIM;
        let samples = vec![
            ("s1", normalize(unit_vec(dim, 0))),
            ("s2", normalize(blend_unit(dim, 0, 1, 0.08))),
            ("s3", normalize(unit_vec(dim, 1))),
            ("s4", normalize(unit_vec(dim, 2))),
        ];
        insert_embeddings(conn, dim, &samples);

        ann_index::rebuild_index(conn).expect("ANN rebuild");
        let results = ann_index::find_similar(conn, "s1", 2).expect("ANN search");
        let expected = brute_force_neighbors("s1", &samples, 2);
        let result_ids: Vec<_> = results
            .iter()
            .map(|entry| entry.sample_id.as_str())
            .collect();
        assert_eq!(result_ids.first().copied(), expected.first().copied());
        assert_results_within_top_k("s1", &samples, 2, &result_ids);
    });
}

#[test]
fn ann_index_incremental_update_matches_full_rebuild() {
    with_ann_test_db(|conn| {
        let dim = similarity::SIMILARITY_DIM;
        let base_samples = vec![
            ("s1", normalize(unit_vec(dim, 0))),
            ("s2", normalize(unit_vec(dim, 1))),
            ("s3", normalize(unit_vec(dim, 2))),
        ];
        let extra_samples = vec![("s4", normalize(blend_unit(dim, 0, 1, 0.12)))];
        let mut all_samples = base_samples.clone();
        all_samples.extend(extra_samples.clone());
        insert_embeddings(conn, dim, &base_samples);

        ann_index::rebuild_index(conn).expect("ANN rebuild");

        insert_embeddings(conn, dim, &extra_samples);
        for (sample_id, vec) in &extra_samples {
            ann_index::upsert_embedding(conn, sample_id, vec).expect("ANN upsert");
        }
        let incremental = ann_index::find_similar(conn, "s1", 2).expect("ANN search");
        let incremental_ids: Vec<_> = incremental
            .iter()
            .map(|entry| entry.sample_id.as_str())
            .collect();

        ann_index::rebuild_index(conn).expect("ANN rebuild");
        let rebuilt = ann_index::find_similar(conn, "s1", 2).expect("ANN search");
        let rebuilt_ids: Vec<_> = rebuilt
            .iter()
            .map(|entry| entry.sample_id.as_str())
            .collect();

        assert_eq!(incremental_ids.len(), 2);
        assert_eq!(rebuilt_ids.len(), 2);
        assert_results_within_top_k("s1", &all_samples, 2, &incremental_ids);
        assert_results_within_top_k("s1", &all_samples, 2, &rebuilt_ids);
    });
}

#[test]
fn ann_index_find_similar_backfills_missing_query_id_from_embeddings_table() {
    with_ann_test_db(|conn| {
        let dim = similarity::SIMILARITY_DIM;
        let base_samples = basic_samples(dim);
        insert_embeddings(conn, dim, &base_samples);
        ann_index::rebuild_index(conn).expect("ANN rebuild");

        let new_sample = normalize(blend_unit(dim, 0, 1, 0.12));
        insert_embeddings(conn, dim, &[("s4", new_sample.clone())]);

        let results = ann_index::find_similar(conn, "s4", 1).expect("ANN search");
        assert_eq!(results.len(), 1);
        assert_ne!(results[0].sample_id, "s4");

        ann_index::flush_pending_inserts(conn).expect("ANN flush");
        let state = load_disk_state(conn);
        assert!(state.id_lookup.contains_key("s4"));

        let result_ids: Vec<_> = ann_index::find_similar(conn, "s1", 3)
            .expect("ANN search")
            .into_iter()
            .map(|entry| entry.sample_id)
            .collect();
        assert!(result_ids.iter().any(|id| id == "s4"));
    });
}
