use super::super::{AppController, WaveformRenderer};
use crate::app::state::FolderPaneId;
use crate::sample_sources::{SampleSource, SourceId};

/// Build a controller config fixture with two durable user-like source roots.
fn config_with_two_sources() -> (tempfile::TempDir, crate::sample_sources::config::AppConfig) {
    let dir = tempfile::tempdir().expect("create source root tempdir");
    let source_a_root = dir.path().join("user-project").join("source-a");
    let source_b_root = dir.path().join("user-project").join("source-b");
    std::fs::create_dir_all(&source_a_root).expect("create source-a root");
    std::fs::create_dir_all(&source_b_root).expect("create source-b root");
    let source_a = SampleSource::new(source_a_root);
    let source_b = SampleSource::new(source_b_root);
    let cfg = crate::sample_sources::config::AppConfig {
        sources: vec![source_a, source_b],
        ..Default::default()
    };
    (dir, cfg)
}

/// Apply a config fixture and assert both compatibility panes collapse to one source.
fn assert_config_collapses_to_source(
    cfg: crate::sample_sources::config::AppConfig,
    expected_source_id: SourceId,
) {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);

    controller
        .apply_configuration(cfg)
        .expect("legacy source config should load");

    assert_eq!(controller.active_folder_pane(), FolderPaneId::Upper);
    assert_eq!(
        controller.selected_source_id(),
        Some(expected_source_id.clone())
    );
    assert_eq!(
        controller.folder_pane_source(FolderPaneId::Upper),
        Some(expected_source_id.clone())
    );
    assert_eq!(
        controller.folder_pane_source(FolderPaneId::Lower),
        Some(expected_source_id)
    );
}

#[test]
/// Legacy upper/lower persisted pane assignments collapse to one active source.
fn legacy_dual_pane_startup_config_collapses_to_one_active_source() {
    let (_dir, mut cfg) = config_with_two_sources();
    let source_a_id = cfg.sources[0].id.clone();
    let source_b_id = cfg.sources[1].id.clone();
    cfg.core.upper_folder_pane_source = Some(source_a_id);
    cfg.core.lower_folder_pane_source = Some(source_b_id.clone());
    cfg.core.active_folder_pane = Some(String::from("lower"));

    assert_config_collapses_to_source(cfg, source_b_id);
}

#[test]
/// Upper-only persisted pane state chooses the upper source.
fn legacy_upper_only_startup_config_collapses_to_upper_source() {
    let (_dir, mut cfg) = config_with_two_sources();
    let source_a_id = cfg.sources[0].id.clone();
    cfg.core.upper_folder_pane_source = Some(source_a_id.clone());
    cfg.core.active_folder_pane = Some(String::from("lower"));

    assert_config_collapses_to_source(cfg, source_a_id);
}

#[test]
/// Lower-only persisted pane state chooses the lower source.
fn legacy_lower_only_startup_config_collapses_to_lower_source() {
    let (_dir, mut cfg) = config_with_two_sources();
    let source_b_id = cfg.sources[1].id.clone();
    cfg.core.lower_folder_pane_source = Some(source_b_id.clone());
    cfg.core.active_folder_pane = Some(String::from("upper"));

    assert_config_collapses_to_source(cfg, source_b_id);
}

#[test]
/// When both panes are set, the persisted active pane wins.
fn legacy_both_panes_startup_config_prefers_active_pane_source() {
    let (_dir, mut cfg) = config_with_two_sources();
    let source_a_id = cfg.sources[0].id.clone();
    let source_b_id = cfg.sources[1].id.clone();
    cfg.core.upper_folder_pane_source = Some(source_a_id.clone());
    cfg.core.lower_folder_pane_source = Some(source_b_id);
    cfg.core.active_folder_pane = Some(String::from("upper"));
    cfg.core.last_selected_source = Some(cfg.sources[1].id.clone());

    assert_config_collapses_to_source(cfg, source_a_id);
}

#[test]
/// With no pane assignments, persisted selected source is the migration fallback.
fn legacy_no_pane_startup_config_uses_last_selected_source() {
    let (_dir, mut cfg) = config_with_two_sources();
    let source_b_id = cfg.sources[1].id.clone();
    cfg.core.last_selected_source = Some(source_b_id.clone());

    assert_config_collapses_to_source(cfg, source_b_id);
}

#[test]
/// With no persisted pane or selection, migration falls back to the first source.
fn legacy_empty_pane_startup_config_uses_first_available_source() {
    let (_dir, cfg) = config_with_two_sources();
    let source_a_id = cfg.sources[0].id.clone();

    assert_config_collapses_to_source(cfg, source_a_id);
}
