use super::*;

#[test]
fn ann_index_upsert_embedding_skips_existing_ids() {
    with_ann_test_db(|conn| {
        let dim = similarity::SIMILARITY_DIM;
        insert_embeddings(conn, dim, &basic_samples(dim));

        ann_index::rebuild_index(conn).expect("ANN rebuild");
        let replacement = normalize(unit_vec(dim, 7));

        ann_index::upsert_embedding(conn, "s1", &replacement).expect("ANN upsert");
        ann_index::flush_pending_inserts(conn).expect("ANN flush");

        let state = load_disk_state(conn);
        assert_eq!(state.id_map.len(), 3);
        assert_eq!(
            state.id_map.iter().filter(|id| id.as_str() == "s1").count(),
            1
        );
    });
}

#[test]
fn ann_index_upsert_embeddings_batch_only_appends_new_ids() {
    with_ann_test_db(|conn| {
        let dim = similarity::SIMILARITY_DIM;
        insert_embeddings(conn, dim, &basic_samples(dim));

        ann_index::rebuild_index(conn).expect("ANN rebuild");
        let duplicate = normalize(unit_vec(dim, 5));
        let fresh = normalize(blend_unit(dim, 0, 1, 0.2));

        ann_index::upsert_embeddings_batch(
            conn,
            [("s1", duplicate.as_slice()), ("s4", fresh.as_slice())],
        )
        .expect("ANN batch upsert");
        ann_index::flush_pending_inserts(conn).expect("ANN flush");

        let state = load_disk_state(conn);
        assert_eq!(state.id_map.len(), 4);
        assert_eq!(
            state.id_map.iter().filter(|id| id.as_str() == "s1").count(),
            1
        );
        assert!(state.id_lookup.contains_key("s4"));
    });
}
