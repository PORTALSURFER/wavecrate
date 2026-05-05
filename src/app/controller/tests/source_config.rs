use super::super::{AppController, WaveformRenderer};
use crate::app::state::FolderPaneId;
use crate::sample_sources::SampleSource;

#[test]
/// Legacy upper/lower persisted pane assignments collapse to one active source.
fn legacy_dual_pane_startup_config_collapses_to_one_active_source() {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempfile::tempdir().expect("create source root tempdir");
    let source_a_root = dir.path().join("user-project").join("source-a");
    let source_b_root = dir.path().join("user-project").join("source-b");
    std::fs::create_dir_all(&source_a_root).expect("create source-a root");
    std::fs::create_dir_all(&source_b_root).expect("create source-b root");
    let source_a = SampleSource::new(source_a_root);
    let source_b = SampleSource::new(source_b_root);
    let mut cfg = crate::sample_sources::config::AppConfig::default();
    cfg.sources = vec![source_a.clone(), source_b.clone()];
    cfg.core.upper_folder_pane_source = Some(source_a.id.clone());
    cfg.core.lower_folder_pane_source = Some(source_b.id.clone());
    cfg.core.active_folder_pane = Some(String::from("lower"));

    controller
        .apply_configuration(cfg)
        .expect("legacy dual-pane config should load");

    assert_eq!(controller.active_folder_pane(), FolderPaneId::Upper);
    assert_eq!(controller.selected_source_id(), Some(source_b.id.clone()));
    assert_eq!(
        controller.folder_pane_source(FolderPaneId::Upper),
        Some(source_b.id.clone())
    );
    assert_eq!(
        controller.folder_pane_source(FolderPaneId::Lower),
        Some(source_b.id)
    );
}
