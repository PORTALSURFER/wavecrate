use super::super::test_support::{dummy_controller, sample_entry, write_test_wav};
use std::path::{Path, PathBuf};

#[test]
fn map_focus_queues_async_load_without_sync_preview_decode() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    write_test_wav(&source.root.join("map.wav"), &[0.0, 0.3, -0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "map.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    let sample_id = crate::app::controller::library::analysis_jobs::build_sample_id(
        source.id.as_str(),
        Path::new("map.wav"),
    );
    controller.focus_map_sample_and_preview(&sample_id);

    let pending = controller
        .runtime
        .jobs
        .pending_audio
        .as_ref()
        .expect("map focus should keep async load queued");
    assert_eq!(pending.relative_path, PathBuf::from("map.wav"));
    assert_eq!(
        controller.ui.browser.active_tab,
        crate::app::state::SampleBrowserTab::Map
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("map.wav"))
    );
    assert!(controller.runtime.jobs.pending_playback.is_some());
}
