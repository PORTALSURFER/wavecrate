use super::*;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

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

#[test]
fn repeated_disk_loads_release_loader_backing_on_drop() {
    with_ann_test_db(|conn| {
        let path = write_loader_backing_fixture(conn);
        let dir = path.parent().expect("index parent");
        let basename = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("index basename");
        let live = Arc::new(AtomicUsize::new(0));

        for _ in 0..8 {
            let loaded = crate::analysis::ann_index::state::LoadedAnnHnsw::load_with_live_probe(
                dir,
                basename,
                Arc::clone(&live),
            )
            .expect("load index");
            assert_eq!(live.load(Ordering::Acquire), 1);
            assert_eq!(loaded.get_nb_point(), 3);
            drop(loaded);
            assert_eq!(live.load(Ordering::Acquire), 0);
        }
    });
}

#[test]
fn concurrent_cache_population_keeps_only_winner_loader_backing() {
    with_ann_test_db(|conn| {
        let path = write_loader_backing_fixture(conn);
        let dir = path.parent().expect("index parent").to_path_buf();
        let basename = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("index basename")
            .to_owned();
        let live = Arc::new(AtomicUsize::new(0));
        let winner = Arc::new(Mutex::new(None));
        let threads = (0..8)
            .map(|_| {
                let dir = dir.clone();
                let basename = basename.clone();
                let live = Arc::clone(&live);
                let winner = Arc::clone(&winner);
                thread::spawn(move || {
                    let loaded =
                        crate::analysis::ann_index::state::LoadedAnnHnsw::load_with_live_probe(
                            &dir, &basename, live,
                        )
                        .expect("load racing index");
                    let mut winner = winner.lock().expect("winner lock");
                    if winner.is_none() {
                        *winner = Some(loaded);
                    }
                })
            })
            .collect::<Vec<_>>();

        for handle in threads {
            handle.join().expect("loader thread");
        }
        assert_eq!(live.load(Ordering::Acquire), 1);
        drop(winner.lock().expect("winner lock").take());
        assert_eq!(live.load(Ordering::Acquire), 0);
    });
}

fn write_loader_backing_fixture(conn: &Connection) -> std::path::PathBuf {
    let dim = similarity::SIMILARITY_DIM;
    insert_embeddings(conn, dim, &basic_samples(dim));
    let params = crate::analysis::ann_index::state::default_params();
    let path = ann_storage::legacy_index_path(conn).expect("legacy path");
    let state = ann_build::build_index_from_db(conn, params, path.clone()).expect("build index");
    write_legacy_ann_files(&state, &path);
    path
}
