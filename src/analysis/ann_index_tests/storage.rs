use super::*;

#[test]
fn ann_index_container_round_trip_loads() {
    with_ann_test_db(|conn| {
        let dim = similarity::SIMILARITY_DIM;
        insert_embeddings(conn, dim, &basic_samples(dim));

        let params = crate::analysis::ann_index::state::default_params();
        let index_path = ann_storage::default_index_path(conn).unwrap();
        let mut state = ann_build::build_index_from_db(conn, params.clone(), index_path).unwrap();
        ann_update::flush_index(conn, &mut state).unwrap();

        let meta = ann_storage::read_meta(conn, &params.model_id)
            .unwrap()
            .expect("ann meta");
        let outcome = ann_build::load_index_from_disk(conn, &meta)
            .unwrap()
            .expect("ann load");
        assert!(!outcome.needs_migration);
        assert_eq!(state.id_map, outcome.state.id_map);
    });
}

#[test]
fn ann_index_legacy_migrates_to_container() {
    with_ann_test_db(|conn| {
        let dim = similarity::SIMILARITY_DIM;
        insert_embeddings(conn, dim, &basic_samples(dim));

        let params = crate::analysis::ann_index::state::default_params();
        let legacy_path = ann_storage::legacy_index_path(conn).unwrap();
        let state =
            ann_build::build_index_from_db(conn, params.clone(), legacy_path.clone()).unwrap();
        write_legacy_ann_files(&state, &legacy_path);
        ann_storage::upsert_meta(conn, &state).unwrap();

        let meta = ann_storage::read_meta(conn, &params.model_id)
            .unwrap()
            .expect("ann meta");
        let outcome = ann_build::load_index_from_disk(conn, &meta)
            .unwrap()
            .expect("ann load");
        assert!(outcome.needs_migration);

        let mut migrated = outcome.state;
        ann_update::flush_index(conn, &mut migrated).unwrap();
        let container_path = ann_storage::default_index_path(conn).unwrap();
        assert!(container_path.is_file());
    });
}
