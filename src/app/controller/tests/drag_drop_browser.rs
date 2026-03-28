use super::super::test_support::{sample_entry, write_test_wav};
use super::super::*;
use crate::app::state::{DragPayload, DragSource, DragTarget, UiPoint};
use crate::app_dirs::ConfigBaseGuard;
use std::path::{Path, PathBuf};
use tempfile::{TempDir, tempdir};

struct BrowserDragSetup {
    _temp: TempDir,
    _guard: ConfigBaseGuard,
    controller: AppController,
    source: SampleSource,
}

fn browser_drag_setup(paths: &[&str]) -> BrowserDragSetup {
    let temp = tempdir().unwrap();
    let guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    for path in paths {
        let absolute = root.join(path);
        if let Some(parent) = absolute.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        write_test_wav(&absolute, &[0.1, 0.2]);
    }

    controller.set_wav_entries_for_tests(
        paths
            .iter()
            .map(|path| sample_entry(path, crate::sample_sources::Rating::NEUTRAL))
            .collect(),
    );
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    BrowserDragSetup {
        _temp: temp,
        _guard: guard,
        controller,
        source,
    }
}

#[test]
fn start_browser_sample_drag_uses_only_unselected_pressed_row() {
    let mut setup = browser_drag_setup(&["one.wav", "two.wav"]);
    setup
        .controller
        .set_browser_selected_paths(vec![PathBuf::from("one.wav")]);

    setup
        .controller
        .start_browser_sample_drag_action(1, UiPoint::new(12.0, 18.0));

    assert_eq!(
        setup.controller.ui.drag.payload,
        Some(DragPayload::Sample {
            source_id: setup.source.id,
            relative_path: PathBuf::from("two.wav"),
        })
    );
    assert_eq!(setup.controller.ui.drag.label, "two");
}

#[test]
fn start_browser_sample_drag_uses_selected_set_for_pressed_row() {
    let mut setup = browser_drag_setup(&["one.wav", "two.wav", "three.wav"]);
    setup
        .controller
        .set_browser_selected_paths(vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]);

    setup
        .controller
        .start_browser_sample_drag_action(0, UiPoint::new(12.0, 18.0));

    assert_eq!(
        setup.controller.ui.drag.payload,
        Some(DragPayload::Samples {
            samples: vec![
                crate::app::state::DragSample {
                    source_id: setup.source.id.clone(),
                    relative_path: PathBuf::from("one.wav"),
                },
                crate::app::state::DragSample {
                    source_id: setup.source.id,
                    relative_path: PathBuf::from("two.wav"),
                },
            ],
        })
    );
    assert_eq!(setup.controller.ui.drag.label, "2 samples");
}

#[test]
fn finish_browser_drag_to_folder_moves_single_sample() {
    let mut setup = browser_drag_setup(&["one.wav"]);
    std::fs::create_dir_all(setup.source.root.join("dest")).unwrap();

    setup
        .controller
        .start_browser_sample_drag_action(0, UiPoint::new(12.0, 18.0));
    setup.controller.update_active_drag(
        UiPoint::new(40.0, 80.0),
        DragSource::Browser,
        DragTarget::FolderPanel {
            folder: Some(PathBuf::from("dest")),
        },
        false,
        false,
    );
    setup.controller.finish_active_drag();

    assert!(!setup.source.root.join("one.wav").exists());
    assert!(setup.source.root.join("dest/one.wav").is_file());
    assert!(
        setup
            .controller
            .wav_index_for_path(Path::new("dest/one.wav"))
            .is_some()
    );
}

#[test]
fn finish_browser_drag_to_folder_moves_selected_samples_and_remaps_selection() {
    let mut setup = browser_drag_setup(&["one.wav", "two.wav", "three.wav"]);
    std::fs::create_dir_all(setup.source.root.join("dest")).unwrap();
    setup
        .controller
        .set_browser_selected_paths(vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]);

    setup
        .controller
        .start_browser_sample_drag_action(0, UiPoint::new(12.0, 18.0));
    setup.controller.update_active_drag(
        UiPoint::new(40.0, 80.0),
        DragSource::Browser,
        DragTarget::FolderPanel {
            folder: Some(PathBuf::from("dest")),
        },
        false,
        false,
    );
    setup.controller.finish_active_drag();

    assert!(setup.source.root.join("dest/one.wav").is_file());
    assert!(setup.source.root.join("dest/two.wav").is_file());
    assert_eq!(
        setup.controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("dest/one.wav"), PathBuf::from("dest/two.wav")]
    );
}
