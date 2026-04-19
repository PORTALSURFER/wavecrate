use super::*;
use crate::app_dirs::{PersistenceProfileGuard, set_app_root_override};
use crate::sample_sources::{LibraryState, SampleSource, library};

#[test]
fn controller_test_runs_do_not_mutate_live_library_state() {
    let live_base = tempdir().expect("live base tempdir");
    let live_root = live_base.path().join(".sempal-live");
    let live_source_root = live_base.path().join("real-source");
    std::fs::create_dir_all(&live_source_root).expect("create live source root");

    {
        let _live_profile_guard = PersistenceProfileGuard::live();
        set_app_root_override(live_root.clone()).expect("set live app root override");
        library::save(&LibraryState {
            sources: vec![SampleSource::new(live_source_root.clone())],
        })
        .expect("seed live library");
    }

    let before_roots = {
        let _live_profile_guard = PersistenceProfileGuard::live();
        set_app_root_override(live_root.clone()).expect("restore live app root override");
        library::load()
            .expect("load live library before test run")
            .sources
            .into_iter()
            .map(|source| source.root)
            .collect::<Vec<_>>()
    };

    let isolated_source_dir = tempdir().expect("isolated source tempdir");
    let isolated_source_root = isolated_source_dir.path().join("source");
    std::fs::create_dir_all(&isolated_source_root).expect("create isolated source root");
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller
        .add_source_from_path(isolated_source_root)
        .expect("add isolated source through persisted controller flow");

    let after_roots = {
        let _live_profile_guard = PersistenceProfileGuard::live();
        set_app_root_override(live_root).expect("restore live app root override");
        library::load()
            .expect("load live library after test run")
            .sources
            .into_iter()
            .map(|source| source.root)
            .collect::<Vec<_>>()
    };

    assert_eq!(after_roots, before_roots);
}
