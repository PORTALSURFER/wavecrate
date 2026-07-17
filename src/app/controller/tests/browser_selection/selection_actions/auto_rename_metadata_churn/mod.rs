use super::*;
use std::io;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

mod loop_recovery;
mod multi_step_churn;
mod provenance_logging;
mod stale_selection;

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

fn assert_sidebar_loop_state(
    controller: &mut AppController,
    expected: crate::app_core::actions::NativeBrowserTagState,
) {
    let model = crate::app_core::ui_projection::project_browser_tag_sidebar_model(controller);
    assert_eq!(model.exclusive_pills[0].state, expected);
}

fn assert_sidebar_one_shot_state(
    controller: &mut AppController,
    expected: crate::app_core::actions::NativeBrowserTagState,
) {
    let model = crate::app_core::ui_projection::project_browser_tag_sidebar_model(controller);
    assert_eq!(model.exclusive_pills[1].state, expected);
}

fn assert_renamed_loop_surfaces(
    controller: &mut AppController,
    source: &SampleSource,
    renamed: &Path,
) {
    let entry_index = controller
        .wav_index_for_path(renamed)
        .expect("renamed entry should stay cached");
    assert!(
        controller.wav_entry(entry_index).unwrap().looped,
        "cached WavEntry should remain Loop after auto-rename"
    );
    let projected = crate::app_core::ui_projection::project_browser_model(controller);
    let row = projected
        .rows
        .iter()
        .find(|row| row.label.as_ref() == "portal_loop")
        .expect("renamed row should be visible in the browser projection");
    assert_eq!(
        row.bucket_label.as_deref(),
        Some("LOOP"),
        "visible browser row should still project Loop"
    );
    assert_sidebar_loop_state(
        controller,
        crate::app_core::actions::NativeBrowserTagState::On,
    );
    assert_sidebar_one_shot_state(
        controller,
        crate::app_core::actions::NativeBrowserTagState::Off,
    );
    assert_eq!(
        controller
            .database_for(source)
            .unwrap()
            .looped_for_path(renamed)
            .unwrap(),
        Some(true),
        "source DB row for the renamed path should persist Loop"
    );
}

fn register_entry_metadata(
    controller: &mut AppController,
    source: &SampleSource,
    entry: &crate::sample_sources::WavEntry,
) {
    let metadata = std::fs::metadata(source.root.join(&entry.relative_path)).unwrap();
    let db = controller.database_for(source).unwrap();
    db.upsert_file(&entry.relative_path, metadata.len(), 0)
        .unwrap();
    db.set_tag(&entry.relative_path, entry.tag).unwrap();
    db.set_looped(&entry.relative_path, entry.looped).unwrap();
    db.set_locked(&entry.relative_path, entry.locked).unwrap();
    db.set_sound_type(&entry.relative_path, entry.sound_type)
        .unwrap();
    db.set_user_tag(&entry.relative_path, entry.user_tag.as_deref())
        .unwrap();
}

fn lock_source_db_until_released(
    source_root: &Path,
) -> (std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>) {
    let (lock_release_tx, lock_release_rx) = std::sync::mpsc::channel();
    let (lock_done_tx, lock_done_rx) = std::sync::mpsc::channel();
    let (locked_tx, locked_rx) = std::sync::mpsc::channel();
    let db_file = source_root.join(crate::sample_sources::db::DB_FILE_NAME);
    std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(db_file).expect("open sqlite lock connection");
        conn.execute_batch("BEGIN IMMEDIATE")
            .expect("start immediate transaction");
        let _ = locked_tx.send(());
        let _ = lock_release_rx.recv();
        let _ = conn.execute_batch("COMMIT");
        drop(conn);
        let _ = lock_done_tx.send(());
    });
    locked_rx.recv().expect("wait for sqlite lock");
    (lock_release_tx, lock_done_rx)
}

fn release_source_db_lock(
    lock_release_tx: std::sync::mpsc::Sender<()>,
    lock_done_rx: std::sync::mpsc::Receiver<()>,
) {
    let _ = lock_release_tx.send(());
    lock_done_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("wait for sqlite lock release");
}
