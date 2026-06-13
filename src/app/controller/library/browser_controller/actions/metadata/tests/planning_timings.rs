use super::*;
use crate::app::controller::batch_latency::{
    BatchLatencyPhase, LARGE_BROWSER_BATCH_CONTROLLER_BUDGET, clear as clear_batch_latency,
    snapshot as batch_latency_snapshot,
};

#[test]
/// Exercise the large tag-sidebar plus auto-rename path and assert phase timing evidence.
fn large_tag_sidebar_auto_rename_batch_reports_controller_phase_timings() {
    /// Large enough to cover multi-path behavior while keeping the test focused.
    const SAMPLE_COUNT: usize = 64;
    clear_batch_latency();
    let (mut controller, source) = dummy_controller();
    controller.settings.default_identifier = String::from("Artist Name");
    controller.ui.options_panel.default_identifier = String::from("Artist Name");
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();

    let mut entries = Vec::with_capacity(SAMPLE_COUNT);
    let mut paths = Vec::with_capacity(SAMPLE_COUNT);
    let db = controller.database_for(&source).unwrap();
    for index in 0..SAMPLE_COUNT {
        let name = format!("sample_{index:03}.wav");
        write_test_wav(&source.root.join(&name), &[0.0, 0.1]);
        db.upsert_file(Path::new(&name), 0, 0).unwrap();
        db.set_tag(Path::new(&name), crate::sample_sources::Rating::NEUTRAL)
            .unwrap();
        entries.push(sample_entry(&name, crate::sample_sources::Rating::NEUTRAL));
        paths.push(PathBuf::from(name));
    }
    controller.set_wav_entries_for_tests(entries);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.set_browser_selected_paths(paths.clone());
    controller.ui.browser.tag_sidebar_auto_rename = true;

    controller
        .apply_browser_tag_sidebar_normal_tag("Vintage FX")
        .expect("large tag plus auto-rename batch should complete");

    let samples = batch_latency_snapshot();
    assert_phase_count_at_least(
        &samples,
        BatchLatencyPhase::TagSidebarTargetResolution,
        SAMPLE_COUNT,
    );
    assert_eq!(
        phase_samples(&samples, BatchLatencyPhase::TagSidebarOptimisticTag).len(),
        1,
        "expected one optimistic tag batch for selected paths: {samples:#?}"
    );
    assert_phase_count_at_least(
        &samples,
        BatchLatencyPhase::TagSidebarOptimisticTag,
        SAMPLE_COUNT,
    );
    assert_eq!(
        phase_samples(&samples, BatchLatencyPhase::MetadataMutationQueue).len(),
        1,
        "expected one queued metadata mutation for the tag batch: {samples:#?}"
    );
    assert_phase_count_at_least(&samples, BatchLatencyPhase::BpmPreload, SAMPLE_COUNT);
    let prepare =
        assert_phase_count_at_least(&samples, BatchLatencyPhase::AutoRenamePrepare, SAMPLE_COUNT);
    let dispatch = assert_phase_count_at_least(
        &samples,
        BatchLatencyPhase::AutoRenameDispatch,
        SAMPLE_COUNT,
    );
    let worker =
        assert_phase_count_at_least(&samples, BatchLatencyPhase::AutoRenameWorker, SAMPLE_COUNT);

    assert!(
        prepare.elapsed <= LARGE_BROWSER_BATCH_CONTROLLER_BUDGET,
        "auto-rename controller preparation exceeded {:?}: {samples:#?}",
        LARGE_BROWSER_BATCH_CONTROLLER_BUDGET
    );
    assert!(
        dispatch.elapsed <= LARGE_BROWSER_BATCH_CONTROLLER_BUDGET,
        "auto-rename controller dispatch exceeded {:?}: {samples:#?}",
        LARGE_BROWSER_BATCH_CONTROLLER_BUDGET
    );
    assert_eq!(worker.detail_count, SAMPLE_COUNT);
    assert!(
        phase_samples(&samples, BatchLatencyPhase::MetadataMutationQueue)
            .iter()
            .all(|sample| sample.detail_count == SAMPLE_COUNT),
        "queue evidence should capture the full OPT-229 tag batch: {samples:#?}"
    );
}

fn assert_phase_count_at_least(
    samples: &[crate::app::controller::batch_latency::BatchLatencySample],
    phase: BatchLatencyPhase,
    item_count: usize,
) -> crate::app::controller::batch_latency::BatchLatencySample {
    let sample = phase_samples(samples, phase)
        .into_iter()
        .max_by_key(|sample| sample.item_count)
        .unwrap_or_else(|| panic!("missing phase {phase:?}: {samples:#?}"));
    assert!(
        sample.item_count >= item_count,
        "phase {phase:?} reported {} items, expected at least {item_count}: {samples:#?}",
        sample.item_count
    );
    sample.clone()
}

fn phase_samples(
    samples: &[crate::app::controller::batch_latency::BatchLatencySample],
    phase: BatchLatencyPhase,
) -> Vec<&crate::app::controller::batch_latency::BatchLatencySample> {
    samples
        .iter()
        .filter(|sample| sample.phase == phase)
        .collect()
}
