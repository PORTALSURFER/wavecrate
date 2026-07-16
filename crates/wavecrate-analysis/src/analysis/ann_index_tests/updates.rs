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

#[test]
fn stale_generation_rejects_ann_container_and_metadata_publication() {
    with_ann_test_db(|conn| {
        let dim = similarity::SIMILARITY_DIM;
        insert_embeddings(conn, dim, &basic_samples(dim));
        ann_index::rebuild_index(conn).expect("ANN rebuild");

        let fresh = normalize(unit_vec(dim, 4));
        insert_embeddings(conn, dim, &[("s4", fresh.clone())]);
        ann_index::upsert_embedding(conn, "s4", &fresh).expect("stage ANN insert");

        let published =
            ann_index::flush_pending_inserts_with_publication_fence(conn, &|_| Ok(false))
                .expect("reject stale ANN publication");

        assert!(!published);
        assert_eq!(load_disk_state(conn).id_map.len(), 3);
    });
}

#[test]
fn stale_generation_rejects_lazy_ann_build_before_it_publishes() {
    with_ann_test_db(|conn| {
        let dim = similarity::SIMILARITY_DIM;
        insert_embeddings(conn, dim, &basic_samples(dim));
        let index_path = ann_storage::default_index_path(conn).expect("ANN index path");

        let published =
            ann_index::flush_pending_inserts_with_publication_fence(conn, &|_| Ok(false))
                .expect("reject stale lazy ANN build");

        assert!(!published);
        assert!(!index_path.exists());
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM ann_index_meta", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count ANN metadata"),
            0
        );
    });
}

#[test]
fn ann_index_batch_validation_failure_leaves_state_unchanged() {
    with_ann_test_db(|conn| {
        let dim = similarity::SIMILARITY_DIM;
        insert_embeddings(conn, dim, &basic_samples(dim));
        let params = ann_index::state::default_params();
        let index_path = ann_storage::default_index_path(conn).expect("ANN index path");
        let mut state =
            ann_build::build_index_from_db(conn, params, index_path).expect("ANN in-memory state");
        state.dirty_inserts = 7;

        let id_map_before = state.id_map.clone();
        let id_lookup_before = state.id_lookup.clone();
        let point_count_before = state.hnsw.get_nb_point();
        let dirty_inserts_before = state.dirty_inserts;
        let last_flush_before = state.last_flush;
        let valid = normalize(unit_vec(dim, 4));
        let invalid = vec![0.0; dim - 1];

        let error = ann_update::upsert_embeddings_batch(
            conn,
            &mut state,
            [
                ("s1", invalid.as_slice()),
                ("s4", valid.as_slice()),
                ("s4", invalid.as_slice()),
                ("s5", invalid.as_slice()),
            ],
        )
        .expect_err("later invalid dimension must reject the complete batch");

        assert!(error.contains("Embedding dim mismatch"));
        assert_eq!(state.id_map, id_map_before);
        assert_eq!(state.id_lookup, id_lookup_before);
        assert_eq!(state.hnsw.get_nb_point(), point_count_before);
        assert_eq!(state.dirty_inserts, dirty_inserts_before);
        assert_eq!(state.last_flush, last_flush_before);
    });
}

#[test]
fn ann_index_batch_preserves_duplicate_handling_with_staged_validation() {
    with_ann_test_db(|conn| {
        let dim = similarity::SIMILARITY_DIM;
        insert_embeddings(conn, dim, &basic_samples(dim));
        let params = ann_index::state::default_params();
        let index_path = ann_storage::default_index_path(conn).expect("ANN index path");
        let mut state =
            ann_build::build_index_from_db(conn, params, index_path).expect("ANN in-memory state");
        let fresh = normalize(unit_vec(dim, 4));
        let invalid_duplicate = vec![0.0; dim - 1];

        ann_update::upsert_embeddings_batch(
            conn,
            &mut state,
            [
                ("s1", invalid_duplicate.as_slice()),
                ("s4", fresh.as_slice()),
                ("s4", invalid_duplicate.as_slice()),
            ],
        )
        .expect("existing and repeated IDs remain skipped");

        assert_eq!(state.id_map.len(), 4);
        assert_eq!(state.hnsw.get_nb_point(), 4);
        assert_eq!(state.id_map.iter().filter(|id| *id == "s4").count(), 1);
        assert_eq!(state.id_lookup.get("s4"), Some(&3));
        assert_eq!(state.dirty_inserts, 1);
    });
}
