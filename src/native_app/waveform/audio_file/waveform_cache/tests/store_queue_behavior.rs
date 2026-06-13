use super::*;

#[test]
fn background_store_queue_coalesces_duplicate_cache_paths() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("same-cache.wav");
    fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
    let first = waveform_file_from_mono_samples(
        path.clone(),
        Arc::from([1_u8, 2, 3, 4]),
        48_000,
        1,
        vec![0.0, 0.25],
    );
    let replacement = waveform_file_from_mono_samples(
        path,
        Arc::from([1_u8, 2, 3, 4]),
        96_000,
        1,
        vec![0.0, 0.5],
    );
    let queue = test_store_queue(4);

    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&first).expect("first job")),
        StoreEnqueueOutcome::Enqueued
    );
    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&replacement).expect("replacement job")),
        StoreEnqueueOutcome::Coalesced
    );

    let queued = queue.pop_next_for_test().expect("queued job");
    assert_eq!(queued.file.sample_rate, 96_000);
    queue.finish_job(&queued.cache_path);
    assert_eq!(queue.pending_for_test(), 0);
}

#[test]
fn background_store_queue_reports_queue_full_without_blocking() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let first_path = dir.path().join("first.wav");
    let second_path = dir.path().join("second.wav");
    fs::write(&first_path, [1_u8, 2, 3, 4]).expect("write first sample");
    fs::write(&second_path, [1_u8, 2, 3, 4, 5]).expect("write second sample");
    let first = waveform_file_from_mono_samples(
        first_path,
        Arc::from([1_u8, 2, 3, 4]),
        48_000,
        1,
        vec![0.0, 0.25],
    );
    let second = waveform_file_from_mono_samples(
        second_path,
        Arc::from([1_u8, 2, 3, 4, 5]),
        48_000,
        1,
        vec![0.0, 0.25],
    );
    let queue = test_store_queue(1);

    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&first).expect("first job")),
        StoreEnqueueOutcome::Enqueued
    );
    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&second).expect("second job")),
        StoreEnqueueOutcome::QueueFull
    );
    assert_eq!(queue.pending_for_test(), 1);
}

#[test]
fn shutdown_flush_waits_for_background_store_queue_drain() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("shutdown-cache.wav");
    fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
    let file = waveform_file_from_mono_samples(
        path,
        Arc::from([1_u8, 2, 3, 4]),
        48_000,
        1,
        vec![0.0, 0.25],
    );
    let queue = test_store_queue(2);
    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&file).expect("job")),
        StoreEnqueueOutcome::Enqueued
    );
    let queued = queue.pop_next_for_test().expect("queued job");

    let started_at = Instant::now();
    thread::scope(|scope| {
        let waiter = scope.spawn(|| queue.flush_for_shutdown(Duration::from_millis(200)));
        thread::sleep(Duration::from_millis(20));
        queue.finish_job(&queued.cache_path);
        waiter.join().expect("shutdown flush completes");
    });
    let elapsed = started_at.elapsed();
    assert!(
        elapsed >= Duration::from_millis(15) && elapsed < Duration::from_millis(150),
        "shutdown flush should wait for active cache persistence and wake after completion"
    );
    assert_eq!(queue.pending_for_test(), 0);
}
