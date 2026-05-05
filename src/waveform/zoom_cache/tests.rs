use super::*;
use std::{
    sync::{Arc, Barrier, mpsc},
    thread,
    time::Duration,
};

fn first_mono_column(columns: &CachedColumns) -> (f32, f32) {
    match columns {
        CachedColumns::Mono(cols) => cols[0],
        CachedColumns::SplitStereo { .. } => panic!("expected mono columns"),
    }
}

#[test]
fn cache_token_prevents_stale_hits_when_memory_is_reused() {
    let cache = WaveformZoomCache::new();
    let mut samples = vec![0.0_f32, 1.0, 0.0, 1.0];

    let initial = cache.get_or_compute(1, &samples, 1, WaveformChannelView::Mono, 1);
    samples.fill(1.0);
    let changed = cache.get_or_compute(2, &samples, 1, WaveformChannelView::Mono, 1);

    assert_ne!(first_mono_column(&initial), first_mono_column(&changed));
}

#[test]
/// Shard routing should remain deterministic for repeated key lookups.
fn shard_index_is_stable_for_identical_keys() {
    let samples = vec![0.0_f32, 1.0, 0.0, 1.0];
    let key = CacheKey::new(42, &samples, 1, WaveformChannelView::Mono, 64);
    assert_eq!(shard_index_for_key(key), shard_index_for_key(key));
}

#[test]
fn cache_order_stays_bounded_for_repeated_touch() {
    let mut inner = CacheInner::new(1);
    inner.max_entries = 1;
    let samples = vec![0.0_f32, 1.0];
    let key = CacheKey::new(1, &samples, 1, WaveformChannelView::Mono, 10);
    let value = CachedColumns::Mono(std::sync::Arc::from([(0.0, 1.0)]));

    inner.insert(key, value);
    for _ in 0..128 {
        inner.touch(key);
    }

    assert_eq!(inner.map.len(), 1);
    assert!(inner.order.len() <= 8);
}

#[test]
fn get_or_compute_allows_parallel_requests() {
    let cache = Arc::new(WaveformZoomCache::new());
    let samples = Arc::new(vec![0.0_f32, 1.0, 0.0, 1.0]);
    let threads = 8;
    let barrier = Arc::new(Barrier::new(threads));
    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::with_capacity(threads);

    for _ in 0..threads {
        let cache = Arc::clone(&cache);
        let samples = Arc::clone(&samples);
        let barrier = Arc::clone(&barrier);
        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            let columns = cache.get_or_compute(1, &samples, 1, WaveformChannelView::Mono, 32);
            tx.send(first_mono_column(&columns))
                .expect("send waveform column");
        }));
    }
    drop(tx);

    let mut results = Vec::with_capacity(threads);
    for _ in 0..threads {
        results.push(
            rx.recv_timeout(Duration::from_secs(2))
                .expect("receive waveform column"),
        );
    }
    for handle in handles {
        handle.join().expect("join waveform thread");
    }

    for result in results.iter().skip(1) {
        assert_eq!(*result, results[0]);
    }
}

#[test]
fn get_or_compute_recovers_after_poisoned_lock() {
    let cache = WaveformZoomCache::new();
    let samples = vec![0.0_f32, 1.0];

    let result = std::panic::catch_unwind(|| {
        let _guard = match cache.shards[0].lock() {
            Ok(guard) => guard,
            Err(err) => panic!("poison cache lock: {err}"),
        };
        panic!("poison cache lock for test");
    });
    assert!(result.is_err());

    let columns = cache.get_or_compute(1, &samples, 1, WaveformChannelView::Mono, 1);
    assert!(matches!(columns, CachedColumns::Mono(_)));
}
