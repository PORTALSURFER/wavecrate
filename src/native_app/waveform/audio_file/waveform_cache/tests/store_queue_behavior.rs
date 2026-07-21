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
    let queue = BackgroundStoreQueue::new(4, true, false);

    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&first).expect("first job")),
        StoreEnqueueOutcome::Enqueued
    );
    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&replacement).expect("replacement job")),
        StoreEnqueueOutcome::ReplacedQueued
    );

    let queued = queue.pop_next_for_test().expect("queued job");
    assert_eq!(queued.file.sample_rate, 96_000);
    queue.finish_job(&queued.cache_path, false);
    assert_eq!(queue.pending_for_test(), 0);
}

#[test]
fn background_store_queue_retains_latest_successor_for_active_cache_path() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("active-successor.wav");
    fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
    let first = waveform_file_from_mono_samples(
        path.clone(),
        Arc::from([1_u8, 2, 3, 4]),
        48_000,
        1,
        vec![0.0, 0.25],
    );
    let replacement = waveform_file_from_mono_samples(
        path.clone(),
        Arc::from([1_u8, 2, 3, 4]),
        96_000,
        1,
        vec![0.0, 0.5],
    );
    let latest_replacement = waveform_file_from_mono_samples(
        path.clone(),
        Arc::from([1_u8, 2, 3, 4]),
        192_000,
        1,
        vec![0.0, 0.75],
    );
    let queue = BackgroundStoreQueue::new(4, true, false);

    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&first).expect("first job")),
        StoreEnqueueOutcome::Enqueued
    );
    let active = queue.pop_next_for_test().expect("active job");
    invalidate_persisted_waveform_cache_path(&path);
    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&replacement).expect("replacement job")),
        StoreEnqueueOutcome::DeferredForActive
    );
    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&latest_replacement).expect("latest job")),
        StoreEnqueueOutcome::DeferredForActive
    );
    assert_eq!(
        queue.pending_for_test(),
        2,
        "active writes retain only the latest successor per cache path"
    );
    assert_eq!(
        store_cached_waveform_file_now(active.clone()),
        StoreWriteOutcome::StaleInput(Default::default())
    );

    queue.finish_job(&active.cache_path, false);
    let successor = queue.pop_next_for_test().expect("successor job");
    assert_eq!(successor.file.sample_rate, 192_000);
    assert!(matches!(
        store_cached_waveform_file_now(successor.clone()),
        StoreWriteOutcome::Completed(_)
    ));
    queue.finish_job(&successor.cache_path, false);
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
    let queue = BackgroundStoreQueue::new(1, true, false);

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
    let queue = BackgroundStoreQueue::new(2, true, false);
    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&file).expect("job")),
        StoreEnqueueOutcome::Enqueued
    );
    let queued = queue.pop_next_for_test().expect("queued job");

    let started_at = Instant::now();
    thread::scope(|scope| {
        let waiter = scope.spawn(|| queue.flush_for_shutdown(Duration::from_millis(200)));
        thread::sleep(Duration::from_millis(20));
        queue.finish_job(&queued.cache_path, false);
        waiter.join().expect("shutdown flush completes");
    });
    let elapsed = started_at.elapsed();
    assert!(
        elapsed >= Duration::from_millis(15) && elapsed < Duration::from_millis(150),
        "shutdown flush should wait for active cache persistence and wake after completion"
    );
    assert_eq!(queue.pending_for_test(), 0);
}

#[test]
fn shutdown_flush_waits_for_pending_cache_prune() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("shutdown-prune.wav");
    fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
    let file = waveform_file_from_mono_samples(
        path,
        Arc::from([1_u8, 2, 3, 4]),
        48_000,
        1,
        vec![0.0, 0.25],
    );
    let queue = BackgroundStoreQueue::new(2, true, false);
    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&file).expect("job")),
        StoreEnqueueOutcome::Enqueued
    );
    let queued = queue.pop_next_for_test().expect("queued job");
    queue.finish_job(&queued.cache_path, true);

    let started_at = Instant::now();
    thread::scope(|scope| {
        let waiter = scope.spawn(|| queue.flush_for_shutdown(Duration::from_millis(200)));
        thread::sleep(Duration::from_millis(20));
        queue.finish_prune();
        waiter.join().expect("shutdown flush completes");
    });
    let elapsed = started_at.elapsed();
    assert!(
        elapsed >= Duration::from_millis(15) && elapsed < Duration::from_millis(150),
        "shutdown flush should include pending cache-limit maintenance"
    );
}
