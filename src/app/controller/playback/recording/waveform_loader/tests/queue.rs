use super::*;

#[test]
fn recording_waveform_queue_replaces_pending() {
    let queue = RecordingWaveformJobQueue::new();
    let job = RecordingWaveformJob {
        request_id: 1,
        source_id: SourceId::from_string("source"),
        relative_path: PathBuf::from("one.wav"),
        absolute_path: PathBuf::from("/tmp/one.wav"),
        last_file_len: 0,
        loaded_once: false,
        sample_rate: 48_000,
        channels: 1,
    };
    let newer = RecordingWaveformJob {
        request_id: 2,
        source_id: SourceId::from_string("source"),
        relative_path: PathBuf::from("two.wav"),
        absolute_path: PathBuf::from("/tmp/two.wav"),
        last_file_len: 0,
        loaded_once: false,
        sample_rate: 48_000,
        channels: 1,
    };
    queue.send(job);
    queue.send(newer.clone());
    let pending = queue.try_take().expect("expected pending job");
    assert_eq!(pending.request_id, newer.request_id);
    assert_eq!(pending.relative_path, newer.relative_path);
}

#[test]
fn recording_waveform_queue_shutdown_unblocks() {
    let queue = Arc::new(RecordingWaveformJobQueue::new());
    let (tx, rx) = std::sync::mpsc::channel();
    let queue_worker = Arc::clone(&queue);
    let handle = thread::spawn(move || {
        let result = queue_worker.take_blocking();
        tx.send(result.is_none()).expect("send result");
    });
    queue.shutdown();
    let shutdown = rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("shutdown signal");
    assert!(shutdown);
    handle.join().expect("worker thread panicked");
}

#[test]
fn recording_waveform_queue_recovers_after_poisoned_lock() {
    let queue = RecordingWaveformJobQueue::new();
    let job = RecordingWaveformJob {
        request_id: 42,
        source_id: SourceId::from_string("source"),
        relative_path: PathBuf::from("recover.wav"),
        absolute_path: PathBuf::from("/tmp/recover.wav"),
        last_file_len: 0,
        loaded_once: false,
        sample_rate: 48_000,
        channels: 1,
    };
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = queue.state.lock().expect("poison queue lock");
        panic!("poison queue lock for test");
    }));
    queue.send(job.clone());
    let pending = queue.try_take().expect("expected pending job");
    assert_eq!(pending.request_id, job.request_id);
    assert_eq!(pending.relative_path, job.relative_path);
}
