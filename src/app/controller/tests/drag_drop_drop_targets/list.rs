use super::super::super::*;
use crate::app::state::{DragPayload, DragSource, DragTarget};
use crate::app_dirs::ConfigBaseGuard;
use crate::sample_sources::config::DropTargetConfig;
use crate::sample_sources::SampleSource;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn drop_target_panel_accepts_folder_drag() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root = temp.path().join("source");
    let target = root.join("targets");
    std::fs::create_dir_all(&target).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());

    controller.ui.drag.payload = Some(DragPayload::Folder {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("targets"),
    });
    controller
        .ui
        .drag
        .set_target(DragSource::DropTargets, DragTarget::DropTargetsPanel);
    controller.finish_active_drag();

    assert_eq!(controller.settings.drop_targets.len(), 1);
    assert_eq!(
        controller.settings.drop_targets[0].path,
        root.join("targets")
    );
}

#[test]
fn drop_target_drag_reorders_list() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root = temp.path().join("source");
    let a = root.join("a");
    let b = root.join("b");
    let c = root.join("c");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    std::fs::create_dir_all(&c).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source);
    controller.settings.drop_targets = vec![
        DropTargetConfig::new(a.clone()),
        DropTargetConfig::new(b.clone()),
        DropTargetConfig::new(c.clone()),
    ];
    controller.refresh_drop_targets_ui();

    controller.ui.drag.payload = Some(DragPayload::DropTargetReorder { path: a.clone() });
    controller.ui.drag.set_target(
        DragSource::DropTargets,
        DragTarget::DropTarget { path: c.clone() },
    );
    controller.finish_active_drag();

    assert_eq!(controller.settings.drop_targets[0].path, b);
    assert_eq!(controller.settings.drop_targets[1].path, a);
    assert_eq!(controller.settings.drop_targets[2].path, c);

    controller.ui.drag.payload = Some(DragPayload::DropTargetReorder { path: a.clone() });
    controller
        .ui
        .drag
        .set_target(DragSource::DropTargets, DragTarget::DropTargetsPanel);
    controller.finish_active_drag();

    assert_eq!(controller.settings.drop_targets[0].path, b);
    assert_eq!(controller.settings.drop_targets[1].path, c);
    assert_eq!(controller.settings.drop_targets[2].path, a);
}
