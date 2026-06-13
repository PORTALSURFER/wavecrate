use super::*;
use crate::app::controller::batch_latency::{
    LARGE_BROWSER_BATCH_CONTROLLER_BUDGET, clear as clear_batch_latency,
};

mod dispatch;
mod partial_failure;
mod progress_streaming;

const LARGE_BACKGROUND_FILE_OP_TIMEOUT: Duration = Duration::from_secs(180);

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
    panic!(
        "background file-op did not finish within {timeout:?}; progress: {:?}",
        controller.ui.progress
    );
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
