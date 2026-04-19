use super::*;
use crate::app_core::state::StatusTone;
use crate::app_dirs::{ConfigBaseGuard, PersistenceProfileGuard, set_app_root_override};
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

#[test]
fn startup_repair_removes_only_seeded_transient_test_sources_from_library_db() {
    let config_base = tempdir().expect("config base tempdir");
    let _config_guard = ConfigBaseGuard::set(config_base.path().to_path_buf());
    let _profile_guard = PersistenceProfileGuard::live();
    crate::sample_sources::config::save(&crate::sample_sources::config::AppConfig::default())
        .expect("seed startup settings file");

    let retained_root = std::env::current_dir()
        .expect("resolve workspace root")
        .join("tmp")
        .join(format!(
            "opt59-retained-source-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time after epoch")
                .as_nanos()
        ))
        .join("source");
    std::fs::create_dir_all(&retained_root).expect("create retained source root");

    let transient_dir = tempdir().expect("transient source tempdir");
    let transient_root = transient_dir.path().join("source_a");
    std::fs::create_dir_all(&transient_root).expect("create transient source root");

    library::save(&LibraryState {
        sources: vec![
            SampleSource::new(transient_root.clone()),
            SampleSource::new(retained_root.clone()),
            SampleSource::new(std::env::temp_dir().join("browser-source")),
        ],
    })
    .expect("seed polluted library");

    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller
        .load_configuration()
        .expect("load configuration with startup repair");

    let repaired_roots = library::load()
        .expect("reload repaired library")
        .sources
        .into_iter()
        .map(|source| source.root)
        .collect::<Vec<_>>();

    assert_eq!(repaired_roots, vec![retained_root.clone()]);
    assert_eq!(
        controller.ui.status.text,
        "Removed 2 transient test sources from startup config"
    );
    assert_eq!(controller.ui.status.status_tone, StatusTone::Info);
}
