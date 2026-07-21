use super::*;

#[test]
fn waveform_cache_prune_removes_old_payloads_and_stale_temps() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let old_path = dir.path().join("old.wfc");
    let newer_path = dir.path().join("newer.wfc");
    let pinned_path = dir.path().join("pinned.wfc");
    let temp_path = dir.path().join("stale.tmp");
    let old_sidecar = playback_sidecar_path(&old_path);
    let old_source_ready = source_warm_marker_path(&old_path);
    let pinned_source_ready = source_warm_marker_path(&pinned_path);
    fs::write(&old_path, [0_u8; 4]).expect("write old cache");
    fs::write(&old_sidecar, [9_u8; 8]).expect("write old sidecar");
    fs::write(&old_source_ready, []).expect("write old source-ready marker");
    fs::write(&newer_path, [1_u8; 4]).expect("write newer cache");
    fs::write(&pinned_path, [2_u8; 4]).expect("write pinned cache");
    fs::write(&pinned_source_ready, []).expect("write pinned source-ready marker");
    fs::write(&temp_path, [3_u8; 4]).expect("write temp cache");

    set_file_modified_seconds(&old_path, 10);
    set_file_modified_seconds(&newer_path, 20);
    set_file_modified_seconds(&pinned_path, 30);

    let prune = prune_waveform_cache_dir(dir.path(), Some(&pinned_path), 8);

    assert!(!old_path.exists());
    assert!(!old_sidecar.exists());
    assert!(!old_source_ready.exists());
    assert!(!temp_path.exists());
    assert!(newer_path.exists());
    assert!(pinned_path.exists());
    assert!(pinned_source_ready.exists());
    assert_eq!(prune.stale_temp_removed, 1);
    assert_eq!(prune.directory_scans, 1);
    assert!(prune.entries_examined >= 4);
    assert!(prune.metadata_probes >= 9);
    assert_eq!(prune.cache_removed, 1);
    assert_eq!(prune.companion_remove_failed, 0);
    assert_eq!(prune.bytes_after, 8);
}

#[test]
fn waveform_cache_prune_counts_directory_and_metadata_work() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    for name in ["one.wfc", "two.wfc", "three.wfc"] {
        fs::write(dir.path().join(name), [0_u8; 4]).expect("write cache entry");
    }

    let prune = prune_waveform_cache_dir(dir.path(), None, u64::MAX);

    assert_eq!(prune.directory_scans, 1);
    assert_eq!(prune.entries_examined, 3);
    assert_eq!(prune.metadata_probes, 9);
}

#[test]
fn waveform_cache_store_persists_summary_without_decoded_playback_sidecar() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("missing-sidecar.wav");
    fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
    let audio_bytes: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4]);
    let mut file = waveform_file_from_mono_samples(
        path.clone(),
        Arc::clone(&audio_bytes),
        48_000,
        1,
        vec![0.0, 0.5, -0.5, 0.25],
    );
    file.playback_samples = Some(Arc::from(vec![0.0_f32, 0.5, -0.5, 0.25]));

    store_cached_waveform_file(&file);
    let identity = CacheIdentity::for_path(&path).expect("identity");
    let cache_path = cache_path_for_identity(&path, &identity).expect("cache path");

    assert!(
        cache_path.is_file(),
        "compact waveform summary should persist"
    );
    assert!(
        !playback_sidecar_path(&cache_path).exists(),
        "decoded PCM must not be persisted"
    );
    assert!(!cached_waveform_file_playback_ready_exists(&path));
    assert!(load_cached_waveform_file_for_playback(path).is_none());
}

#[test]
#[ignore = "filesystem performance benchmark"]
fn benchmark_batched_waveform_cache_pruning_reduces_directory_metadata_work() {
    const POPULATED_ENTRIES: usize = 1_024;
    const BURST_WRITES: usize = 128;
    const PAYLOAD_BYTES: usize = 1_024;

    let _guard = waveform_cache_test_guard();
    let naive_dir = tempfile::tempdir().expect("naive tempdir");
    let batched_dir = tempfile::tempdir().expect("batched tempdir");
    seed_benchmark_cache(naive_dir.path(), POPULATED_ENTRIES, PAYLOAD_BYTES);
    seed_benchmark_cache(batched_dir.path(), POPULATED_ENTRIES, PAYLOAD_BYTES);

    let naive_started = Instant::now();
    let mut naive_scans = 0;
    let mut naive_metadata_probes = 0;
    for index in 0..BURST_WRITES {
        let path = naive_dir.path().join(format!("burst-{index:04}.wfc"));
        fs::write(&path, vec![0_u8; PAYLOAD_BYTES]).expect("write naive burst entry");
        let outcome = prune_waveform_cache_dir(naive_dir.path(), Some(&path), u64::MAX);
        naive_scans += outcome.directory_scans;
        naive_metadata_probes += outcome.metadata_probes;
    }
    let naive_elapsed = naive_started.elapsed();

    let batched_started = Instant::now();
    let startup = prune_waveform_cache_dir(batched_dir.path(), None, u64::MAX);
    let mut batched_scans = startup.directory_scans;
    let mut batched_metadata_probes = startup.metadata_probes;
    let mut schedule = CachePruneSchedule::default();
    for index in 0..BURST_WRITES {
        let path = batched_dir.path().join(format!("burst-{index:04}.wfc"));
        fs::write(&path, vec![0_u8; PAYLOAD_BYTES]).expect("write batched burst entry");
        schedule.record_success(&path, Some(PAYLOAD_BYTES as u64));
        if schedule.immediate_prune_due() {
            let outcome =
                prune_waveform_cache_dir(batched_dir.path(), schedule.pinned_path(), u64::MAX);
            batched_scans += outcome.directory_scans;
            batched_metadata_probes += outcome.metadata_probes;
            schedule.reset();
        }
    }
    if schedule.successful_writes() > 0 {
        let outcome =
            prune_waveform_cache_dir(batched_dir.path(), schedule.pinned_path(), u64::MAX);
        batched_scans += outcome.directory_scans;
        batched_metadata_probes += outcome.metadata_probes;
    }
    let batched_elapsed = batched_started.elapsed();

    println!(
        "{{\"populated_entries\":{POPULATED_ENTRIES},\"burst_writes\":{BURST_WRITES},\"baseline_scans\":{naive_scans},\"batched_scans\":{batched_scans},\"baseline_metadata_probes\":{naive_metadata_probes},\"batched_metadata_probes\":{batched_metadata_probes},\"baseline_wall_ms\":{},\"batched_wall_ms\":{}}}",
        naive_elapsed.as_millis(),
        batched_elapsed.as_millis()
    );
    assert_eq!(naive_scans, BURST_WRITES);
    assert!(batched_scans * 10 < naive_scans);
    assert!(batched_metadata_probes * 10 < naive_metadata_probes);
    assert!(batched_elapsed < naive_elapsed);
}

fn seed_benchmark_cache(dir: &Path, entries: usize, payload_bytes: usize) {
    for index in 0..entries {
        fs::write(
            dir.join(format!("seed-{index:04}.wfc")),
            vec![0_u8; payload_bytes],
        )
        .expect("seed waveform cache benchmark");
    }
}
