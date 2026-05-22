use super::*;
use crate::app::controller::batch_latency::{
    BatchLatencyPhase, LARGE_BROWSER_BATCH_CONTROLLER_BUDGET, clear as clear_batch_latency,
    snapshot as batch_latency_snapshot,
};
use crate::app::controller::test_support::{dummy_controller, sample_entry, write_test_wav};
use crate::sample_sources::db::DB_FILE_NAME;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    mpsc::{Receiver, Sender},
};
use std::time::{Duration, Instant};
use tracing_subscriber::fmt::MakeWriter;

const LARGE_BACKGROUND_FILE_OP_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Default)]
struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

impl SharedBuffer {
    fn captured(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl<'a> MakeWriter<'a> for SharedBuffer {
    type Writer = SharedBufferWriter;

    fn make_writer(&'a self) -> Self::Writer {
        SharedBufferWriter(self.0.clone())
    }
}

struct SharedBufferWriter(Arc<Mutex<Vec<u8>>>);

impl io::Write for SharedBufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn capture_info_logs<F>(run: F) -> String
where
    F: FnOnce(),
{
    let buffer = SharedBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .with_writer(buffer.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, run);
    buffer.captured()
}

#[test]
fn auto_rename_request_preflight_stays_prompt_under_source_db_write_contention() {
    let (mut controller, source) = dummy_controller();
    controller.settings.default_identifier = String::from("Artist Name");
    controller.ui.options_panel.default_identifier = String::from("Artist Name");
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("kick.wav"), &[0.0]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "kick.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    let (lock_release_tx, lock_done_rx) = lock_db_until_released(&source.root);
    let started_at = Instant::now();
    let requests = BrowserController::new(&mut controller)
        .prepare_auto_rename_requests(&source, &[PathBuf::from("kick.wav")])
        .expect("preflight should succeed while writer holds BEGIN IMMEDIATE");
    let elapsed = started_at.elapsed();
    release_db_lock(lock_release_tx, lock_done_rx);

    assert!(
        elapsed < Duration::from_secs(1),
        "auto-rename controller preflight waited {elapsed:?} under DB contention"
    );
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].new_relative,
        PathBuf::from("artistname_SS_kick.wav")
    );
    assert_eq!(
        requests[0].sound_type,
        Some(crate::sample_sources::SampleSoundType::Kick)
    );
}

#[test]
fn prepare_auto_rename_requests_prefers_live_sidebar_metadata() {
    let (mut controller, source) = dummy_controller();
    controller.settings.default_identifier = String::from("Artist Name");
    controller.ui.options_panel.default_identifier = String::from("Artist Name");
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);

    let mut entry = sample_entry("raw.wav", crate::sample_sources::Rating::NEUTRAL);
    entry.sound_type = Some(crate::sample_sources::SampleSoundType::Hat);
    entry.user_tag = Some(String::from("Live Tag"));
    controller.set_wav_entries_for_tests(vec![entry]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    let db = controller.database_for(&source).unwrap();
    db.set_sound_type(
        Path::new("raw.wav"),
        Some(crate::sample_sources::SampleSoundType::Kick),
    )
    .unwrap();
    db.set_user_tag(Path::new("raw.wav"), Some("DB Tag"))
        .unwrap();
    controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source.id.clone())
        .or_default()
        .insert(PathBuf::from("raw.wav"), Some(128.0));

    let request = BrowserController::new(&mut controller)
        .prepare_auto_rename_requests(&source, &[PathBuf::from("raw.wav")])
        .expect("request preparation should succeed")
        .into_iter()
        .next()
        .expect("request should exist");

    assert_eq!(
        request.sound_type,
        Some(crate::sample_sources::SampleSoundType::Hat)
    );
    assert_eq!(
        request.new_relative,
        PathBuf::from("artistname_SS_hat_livetag_128.wav")
    );
}

#[test]
fn prepare_auto_rename_requests_logs_looped_provenance() {
    let (mut controller, source) = dummy_controller();
    controller.settings.default_identifier = String::from("Artist Name");
    controller.ui.options_panel.default_identifier = String::from("Artist Name");
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);

    let mut entry = sample_entry("raw.wav", crate::sample_sources::Rating::NEUTRAL);
    entry.looped = true;
    controller.set_wav_entries_for_tests(vec![entry]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    let captured = capture_info_logs(|| {
        let requests = BrowserController::new(&mut controller)
            .prepare_auto_rename_requests(&source, &[PathBuf::from("raw.wav")])
            .expect("request preparation should succeed");
        assert_eq!(requests.len(), 1);
    });

    assert!(
        captured.contains("auto rename: request metadata provenance"),
        "request preparation should log metadata provenance: {captured}"
    );
    assert!(
        captured.contains("lane=\"controller\"")
            && captured.contains("request_count=1")
            && captured.contains("raw.wav -> artistname_loop.wav looped=true"),
        "log should include old path, new path, and requested loop value: {captured}"
    );
}

#[test]
fn resolve_auto_rename_target_skips_existing_and_reserved_names() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    write_test_wav(&source.root.join("artistname_SS_kick.wav"), &[0.0]);
    write_test_wav(&source.root.join("artistname_SS_kick_001.wav"), &[0.0]);

    let browser = BrowserController::new(&mut controller);
    let mut reserved_targets = HashSet::from([PathBuf::from("artistname_SS_kick_002.wav")]);
    let resolved = browser
        .resolve_auto_rename_target(
            &source.root,
            Path::new("raw.wav"),
            Some("artistname_SS_kick"),
            "artistname",
            &mut reserved_targets,
        )
        .expect("target resolution should succeed");

    assert_eq!(resolved, PathBuf::from("artistname_SS_kick_003.wav"));
    assert!(reserved_targets.contains(&resolved));
}

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

#[test]
fn large_auto_rename_background_dispatch_registers_file_ops_before_planning_finishes() {
    /// Large enough to exercise batch dispatch without making the regression test slow.
    const SAMPLE_COUNT: usize = 24;
    clear_batch_latency();
    let (mut controller, source, paths) = large_auto_rename_fixture(SAMPLE_COUNT);

    let started_at = Instant::now();
    BrowserController::new(&mut controller)
        .auto_rename_browser_sample_paths_background_for_tests(&paths)
        .expect("background auto rename dispatch should start");
    let elapsed = started_at.elapsed();

    assert!(
        elapsed <= LARGE_BROWSER_BATCH_CONTROLLER_BUDGET,
        "background auto-rename dispatch exceeded {:?}: {elapsed:?}",
        LARGE_BROWSER_BATCH_CONTROLLER_BUDGET
    );
    assert_eq!(
        controller.ui.progress.task,
        Some(crate::app::state::ProgressTaskKind::FileOps)
    );
    assert_eq!(controller.ui.progress.title, "Preparing auto rename");
    assert!(controller.ui.progress.cancelable);
    assert_eq!(controller.ui.progress.total, SAMPLE_COUNT);

    wait_for_background_jobs(&mut controller, LARGE_BACKGROUND_FILE_OP_TIMEOUT);
    assert!(source.root.join("artistname_SS.wav").exists());
}

#[test]
fn large_background_auto_rename_reuses_source_db_for_batch_execution() {
    /// Large enough to catch per-item database opens in the auto-rename worker.
    const SAMPLE_COUNT: usize = 24;
    let (mut controller, source, paths) = large_auto_rename_fixture(SAMPLE_COUNT);

    crate::sample_sources::db::test_reset_source_db_open_total_count(&source.root);
    BrowserController::new(&mut controller)
        .auto_rename_browser_sample_paths_background_for_tests(&paths)
        .expect("background auto rename should start");
    wait_for_background_jobs(&mut controller, LARGE_BACKGROUND_FILE_OP_TIMEOUT);

    let open_count = crate::sample_sources::db::test_source_db_open_total_count(&source.root);
    assert!(
        open_count <= 3,
        "24-item background auto-rename should reuse source DB access instead of opening per item; observed {open_count}"
    );
    assert!(source.root.join("artistname_SS.wav").exists());
    assert!(source.root.join("artistname_SS_023.wav").exists());
}

#[test]
fn large_tag_sidebar_background_auto_rename_streams_file_ops_progress_and_refreshes_rows() {
    /// Large enough to exercise progress streaming and final browser-row refresh behavior.
    const SAMPLE_COUNT: usize = 24;
    clear_batch_latency();
    let (mut controller, source, paths) = large_auto_rename_fixture(SAMPLE_COUNT);
    controller.set_browser_selected_paths(paths.clone());
    let visible_before = controller.visible_browser_len();

    controller
        .apply_browser_tag_sidebar_normal_tag("Vintage FX")
        .expect("large tag mutation should update selected paths");
    controller.ui.browser.tag_sidebar_auto_rename = true;
    BrowserController::new(&mut controller)
        .auto_rename_browser_sample_paths_background_for_tests(&paths)
        .expect("background auto rename should start after tag mutation");

    assert_eq!(
        controller.ui.progress.task,
        Some(crate::app::state::ProgressTaskKind::FileOps)
    );
    assert_eq!(controller.ui.progress.title, "Preparing auto rename");
    assert_eq!(controller.ui.progress.total, SAMPLE_COUNT);
    assert!(controller.ui.progress.cancelable);

    wait_for_file_ops_detail(&mut controller, Duration::from_secs(15), |detail| {
        detail.starts_with("Planning sample_")
    });
    assert_eq!(
        controller.ui.progress.task,
        Some(crate::app::state::ProgressTaskKind::FileOps)
    );
    assert_eq!(controller.ui.progress.completed, 0);
    assert!(
        controller
            .ui
            .progress
            .has_task(crate::app::state::ProgressTaskKind::FileOps)
    );

    wait_for_background_jobs(&mut controller, LARGE_BACKGROUND_FILE_OP_TIMEOUT);

    assert_eq!(controller.visible_browser_len(), visible_before);
    assert!(source.root.join("artistname_SS_vintagefx.wav").exists());
    assert!(source.root.join("artistname_SS_vintagefx_023.wav").exists());
    assert!(!source.root.join("sample_000.wav").exists());
    assert!(!controller.ui.progress.visible);
    assert_eq!(
        controller.ui.status.text,
        "Auto Rename: renamed 24, skipped 0, failed 0"
    );

    let _ = controller.refresh_projection_revision_bus();
    let projected = crate::app_core::native_shell::project_browser_model(&mut controller);
    assert_eq!(projected.rows[0].label.as_ref(), "artistname_SS_vintagefx");
    assert_eq!(
        controller
            .browser_projection_entry(0)
            .map(|entry| entry.relative_path),
        Some(Path::new("artistname_SS_vintagefx.wav"))
    );
}

#[test]
fn large_background_auto_rename_reports_partial_failure_through_file_ops_progress() {
    /// Large enough to prove partial failures advance inside a real batch.
    const SAMPLE_COUNT: usize = 24;
    clear_batch_latency();
    let (mut controller, source, paths) = large_auto_rename_fixture(SAMPLE_COUNT);
    std::fs::remove_file(source.root.join("sample_010.wav"))
        .expect("remove one fixture file after browser snapshot is loaded");

    BrowserController::new(&mut controller)
        .auto_rename_browser_sample_paths_background_for_tests(&paths)
        .expect("background auto rename should start");
    wait_for_file_ops_detail(&mut controller, Duration::from_secs(60), |detail| {
        detail == "Failed sample_010.wav"
    });

    assert_eq!(
        controller.ui.progress.task,
        Some(crate::app::state::ProgressTaskKind::FileOps)
    );
    assert!(
        (11..=SAMPLE_COUNT).contains(&controller.ui.progress.completed),
        "progress should have reached the failed item without exceeding the batch: {:?}",
        controller.ui.progress
    );
    assert_eq!(controller.ui.progress.total, SAMPLE_COUNT);

    wait_for_background_jobs(&mut controller, LARGE_BACKGROUND_FILE_OP_TIMEOUT);

    assert_eq!(
        controller.ui.status.text,
        "Auto Rename: renamed 23, skipped 0, failed 1"
    );
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Warning
    );
    assert!(source.root.join("artistname_SS.wav").exists());
    assert!(source.root.join("artistname_SS_023.wav").exists());
    assert!(!source.root.join("artistname_SS_010.wav").exists());
    assert!(!controller.ui.progress.visible);
}

fn large_auto_rename_fixture(
    sample_count: usize,
) -> (
    crate::app::controller::AppController,
    crate::sample_sources::SampleSource,
    Vec<PathBuf>,
) {
    let (mut controller, source) = dummy_controller();
    controller.settings.default_identifier = String::from("Artist Name");
    controller.ui.options_panel.default_identifier = String::from("Artist Name");
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();

    let mut entries = Vec::with_capacity(sample_count);
    let mut paths = Vec::with_capacity(sample_count);
    let db = controller.database_for(&source).unwrap();
    for index in 0..sample_count {
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
    (controller, source, paths)
}

/// Return a captured phase sample and require it to cover the expected item count.
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

/// Filter captured latency samples to one phase.
fn phase_samples(
    samples: &[crate::app::controller::batch_latency::BatchLatencySample],
    phase: BatchLatencyPhase,
) -> Vec<&crate::app::controller::batch_latency::BatchLatencySample> {
    samples
        .iter()
        .filter(|sample| sample.phase == phase)
        .collect()
}

fn lock_db_until_released(source_root: &Path) -> (Sender<()>, Receiver<()>) {
    let (lock_release_tx, lock_release_rx) = std::sync::mpsc::channel();
    let (lock_done_tx, lock_done_rx) = std::sync::mpsc::channel();
    let (locked_tx, locked_rx) = std::sync::mpsc::channel();
    let db_file = source_root.join(DB_FILE_NAME);
    std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(db_file).expect("open sqlite lock connection");
        conn.execute_batch("BEGIN IMMEDIATE")
            .expect("start immediate transaction");
        let _ = locked_tx.send(());
        let _ = lock_release_rx.recv();
        let _ = conn.execute_batch("COMMIT");
        let _ = lock_done_tx.send(());
    });
    locked_rx.recv().expect("wait for sqlite lock");
    (lock_release_tx, lock_done_rx)
}

fn release_db_lock(lock_release_tx: Sender<()>, lock_done_rx: Receiver<()>) {
    let _ = lock_release_tx.send(());
    lock_done_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("wait for sqlite lock release");
}

fn wait_for_background_jobs(
    controller: &mut crate::app::controller::AppController,
    timeout: Duration,
) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        controller.poll_background_jobs();
        if !controller
            .ui
            .progress
            .has_task(crate::app::state::ProgressTaskKind::FileOps)
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("background file-op did not finish within {timeout:?}");
}

fn wait_for_file_ops_detail(
    controller: &mut crate::app::controller::AppController,
    timeout: Duration,
    matches_detail: impl Fn(&str) -> bool,
) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        controller.poll_background_jobs();
        if controller
            .ui
            .progress
            .task_detail(crate::app::state::ProgressTaskKind::FileOps)
            .is_some_and(&matches_detail)
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!(
        "file-op progress detail did not match before {timeout:?}; last detail: {:?}",
        controller
            .ui
            .progress
            .task_detail(crate::app::state::ProgressTaskKind::FileOps)
    );
}
