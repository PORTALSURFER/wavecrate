use super::{WaveformChannelView, WaveformColumnView, WaveformRenderer};
use std::{
    collections::{HashMap, VecDeque},
    hash::{Hash, Hasher},
    mem::size_of,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tracing::warn;

/// Cache of precomputed waveform columns keyed by token, view, and width.
pub(super) struct WaveformZoomCache {
    inner: Mutex<CacheInner>,
}

const HOTPATH_TELEMETRY_ENV: &str = "SEMPAL_HOTPATH_TELEMETRY";
const ZOOM_CACHE_TELEMETRY_LOG_EVERY: u64 = 1_024;
static ZOOM_CACHE_TELEMETRY_ENABLED: OnceLock<bool> = OnceLock::new();
static ZOOM_CACHE_LOCK_ACQUIRE_COUNT: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_LOCK_WAIT_NS: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_LOCK_POISON_RECOVERY_COUNT: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_HIT_COUNT: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_MISS_COUNT: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_INSERT_COUNT: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_EVICT_COUNT: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_COMPACT_COUNT: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_RESIDENT_BYTES: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_PEAK_RESIDENT_BYTES: AtomicU64 = AtomicU64::new(0);

fn zoom_cache_telemetry_enabled() -> bool {
    *ZOOM_CACHE_TELEMETRY_ENABLED
        .get_or_init(|| crate::env_flags::env_var_truthy(HOTPATH_TELEMETRY_ENV))
}

fn saturating_add_duration_ns(counter: &AtomicU64, duration: Duration) {
    let dur_ns = duration.as_nanos().min(u64::MAX as u128) as u64;
    counter.fetch_add(dur_ns, Ordering::Relaxed);
}

fn record_zoom_cache_lock_wait(duration: Duration) {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = ZOOM_CACHE_LOCK_ACQUIRE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    saturating_add_duration_ns(&ZOOM_CACHE_LOCK_WAIT_NS, duration);
    maybe_emit_zoom_cache_telemetry(sample_tick);
}

fn record_zoom_cache_hit() {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = ZOOM_CACHE_HIT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_zoom_cache_telemetry(sample_tick);
}

fn record_zoom_cache_miss() {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = ZOOM_CACHE_MISS_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_zoom_cache_telemetry(sample_tick);
}

fn record_zoom_cache_insert() {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = ZOOM_CACHE_INSERT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_zoom_cache_telemetry(sample_tick);
}

fn record_zoom_cache_evict() {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = ZOOM_CACHE_EVICT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_zoom_cache_telemetry(sample_tick);
}

fn record_zoom_cache_compaction() {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = ZOOM_CACHE_COMPACT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_zoom_cache_telemetry(sample_tick);
}

fn update_zoom_cache_resident_bytes(resident_bytes: usize) {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let resident = resident_bytes.min(u64::MAX as usize) as u64;
    ZOOM_CACHE_RESIDENT_BYTES.store(resident, Ordering::Relaxed);
    ZOOM_CACHE_PEAK_RESIDENT_BYTES.fetch_max(resident, Ordering::Relaxed);
}

fn maybe_emit_zoom_cache_telemetry(sample_tick: u64) {
    if !zoom_cache_telemetry_enabled()
        || sample_tick == 0
        || !sample_tick.is_multiple_of(ZOOM_CACHE_TELEMETRY_LOG_EVERY)
    {
        return;
    }

    let lock_acquires = ZOOM_CACHE_LOCK_ACQUIRE_COUNT.load(Ordering::Relaxed);
    let lock_wait_ns = ZOOM_CACHE_LOCK_WAIT_NS.load(Ordering::Relaxed);
    let lock_poison_recoveries = ZOOM_CACHE_LOCK_POISON_RECOVERY_COUNT.load(Ordering::Relaxed);
    let hits = ZOOM_CACHE_HIT_COUNT.load(Ordering::Relaxed);
    let misses = ZOOM_CACHE_MISS_COUNT.load(Ordering::Relaxed);
    let inserts = ZOOM_CACHE_INSERT_COUNT.load(Ordering::Relaxed);
    let evicts = ZOOM_CACHE_EVICT_COUNT.load(Ordering::Relaxed);
    let compactions = ZOOM_CACHE_COMPACT_COUNT.load(Ordering::Relaxed);
    let resident_bytes = ZOOM_CACHE_RESIDENT_BYTES.load(Ordering::Relaxed);
    let peak_resident_bytes = ZOOM_CACHE_PEAK_RESIDENT_BYTES.load(Ordering::Relaxed);
    let avg_lock_wait_us = if lock_acquires == 0 {
        0.0
    } else {
        lock_wait_ns as f64 / lock_acquires as f64 / 1_000.0
    };

    tracing::info!(
        target: "perf::hotpath",
        module = "waveform_zoom_cache",
        lock_acquires,
        avg_lock_wait_us,
        lock_poison_recoveries,
        hits,
        misses,
        inserts,
        evicts,
        compactions,
        resident_bytes,
        peak_resident_bytes,
        "Waveform zoom cache telemetry snapshot"
    );
}

fn cached_columns_bytes(columns: &CachedColumns) -> usize {
    let pair_size = size_of::<(f32, f32)>();
    match columns {
        CachedColumns::Mono(cols) => cols.len().saturating_mul(pair_size),
        CachedColumns::SplitStereo { left, right } => left
            .len()
            .saturating_add(right.len())
            .saturating_mul(pair_size),
    }
}

impl WaveformZoomCache {
    /// Create an empty cache with a small, bounded entry budget.
    pub(super) fn new() -> Self {
        Self {
            inner: Mutex::new(CacheInner::new()),
        }
    }

    /// Return cached columns for the request or compute and store them on miss.
    ///
    /// This keeps the render path fast while allowing cache invalidation via the token.
    pub(super) fn get_or_compute(
        &self,
        cache_token: u64,
        samples: &[f32],
        channels: usize,
        view: WaveformChannelView,
        width: u32,
    ) -> CachedColumns {
        let key = CacheKey::new(cache_token, samples, channels, view, width);
        {
            let mut inner = self.lock_inner();
            if let Some(hit) = inner.get(key) {
                return hit;
            }
        }

        let computed =
            match WaveformRenderer::sample_columns_for_width(samples, channels, width, view) {
                WaveformColumnView::Mono(cols) => CachedColumns::Mono(cols.into()),
                WaveformColumnView::SplitStereo { left, right } => CachedColumns::SplitStereo {
                    left: left.into(),
                    right: right.into(),
                },
            };
        let mut inner = self.lock_inner();
        if let Some(hit) = inner.get(key) {
            return hit;
        }
        inner.insert(key, computed.clone());
        computed
    }

    fn lock_inner(&self) -> std::sync::MutexGuard<'_, CacheInner> {
        let lock_start = zoom_cache_telemetry_enabled().then(Instant::now);
        let guard = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Waveform zoom cache mutex poisoned; recovering with cleared cache.");
                let mut inner = poisoned.into_inner();
                inner.map.clear();
                inner.order.clear();
                inner.next_stamp = 1;
                inner.resident_bytes = 0;
                if zoom_cache_telemetry_enabled() {
                    ZOOM_CACHE_LOCK_POISON_RECOVERY_COUNT.fetch_add(1, Ordering::Relaxed);
                    update_zoom_cache_resident_bytes(0);
                }
                inner
            }
        };
        if let Some(start) = lock_start {
            record_zoom_cache_lock_wait(start.elapsed());
        }
        guard
    }
}

#[derive(Clone)]
/// Cached waveform columns stored in shared arcs for inexpensive cloning.
pub(super) enum CachedColumns {
    Mono(std::sync::Arc<[(f32, f32)]>),
    SplitStereo {
        left: std::sync::Arc<[(f32, f32)]>,
        right: std::sync::Arc<[(f32, f32)]>,
    },
}

#[derive(Clone, Copy, Debug, Eq)]
struct CacheKey {
    cache_token: u64,
    samples_len: usize,
    channels: u16,
    view: WaveformChannelView,
    width: u32,
}

impl CacheKey {
    fn new(
        cache_token: u64,
        samples: &[f32],
        channels: usize,
        view: WaveformChannelView,
        width: u32,
    ) -> Self {
        Self {
            cache_token,
            samples_len: samples.len(),
            channels: channels.min(u16::MAX as usize) as u16,
            view,
            width,
        }
    }
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.cache_token == other.cache_token
            && self.samples_len == other.samples_len
            && self.channels == other.channels
            && self.view == other.view
            && self.width == other.width
    }
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cache_token.hash(state);
        self.samples_len.hash(state);
        self.channels.hash(state);
        self.view.hash(state);
        self.width.hash(state);
    }
}

struct CacheInner {
    map: HashMap<CacheKey, CacheEntry>,
    order: VecDeque<TouchEntry>,
    max_entries: usize,
    next_stamp: u64,
    resident_bytes: usize,
}

impl CacheInner {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            max_entries: 12,
            next_stamp: 1,
            resident_bytes: 0,
        }
    }

    fn get(&mut self, key: CacheKey) -> Option<CachedColumns> {
        let stamp = self.next_stamp();
        let columns = match self.map.get_mut(&key) {
            Some(entry) => {
                entry.stamp = stamp;
                entry.columns.clone()
            }
            None => {
                record_zoom_cache_miss();
                return None;
            }
        };
        self.order.push_back(TouchEntry { key, stamp });
        self.compact_order_if_needed();
        record_zoom_cache_hit();
        Some(columns)
    }

    #[cfg(test)]
    fn touch(&mut self, key: CacheKey) {
        let stamp = self.next_stamp();
        if let Some(entry) = self.map.get_mut(&key) {
            entry.stamp = stamp;
            self.order.push_back(TouchEntry { key, stamp });
            self.compact_order_if_needed();
        }
    }

    fn insert(&mut self, key: CacheKey, value: CachedColumns) {
        let stamp = self.next_stamp();
        let bytes_estimate = cached_columns_bytes(&value);
        if let Some(replaced) = self.map.insert(
            key,
            CacheEntry {
                columns: value,
                stamp,
                bytes_estimate,
            },
        ) {
            self.resident_bytes = self.resident_bytes.saturating_sub(replaced.bytes_estimate);
        }
        self.resident_bytes = self.resident_bytes.saturating_add(bytes_estimate);
        update_zoom_cache_resident_bytes(self.resident_bytes);
        record_zoom_cache_insert();
        self.order.push_back(TouchEntry { key, stamp });
        self.compact_order_if_needed();
        self.evict();
    }

    fn evict(&mut self) {
        while self.map.len() > self.max_entries {
            let Some(touch) = self.order.pop_front() else {
                break;
            };
            let is_current = self
                .map
                .get(&touch.key)
                .is_some_and(|entry| entry.stamp == touch.stamp);
            if is_current && let Some(removed) = self.map.remove(&touch.key) {
                self.resident_bytes = self.resident_bytes.saturating_sub(removed.bytes_estimate);
                update_zoom_cache_resident_bytes(self.resident_bytes);
                record_zoom_cache_evict();
            }
        }
    }

    fn compact_order_if_needed(&mut self) {
        let compact_threshold = self.max_entries.saturating_mul(8).max(self.max_entries + 1);
        if self.order.len() <= compact_threshold {
            return;
        }

        let mut active: Vec<_> = self
            .map
            .iter()
            .map(|(key, entry)| TouchEntry {
                key: *key,
                stamp: entry.stamp,
            })
            .collect();
        active.sort_by_key(|entry| entry.stamp);
        self.order = active.into_iter().collect();
        record_zoom_cache_compaction();
    }

    fn next_stamp(&mut self) -> u64 {
        let stamp = self.next_stamp;
        self.next_stamp = self.next_stamp.wrapping_add(1);
        if self.next_stamp == 0 {
            self.next_stamp = 1;
        }
        stamp
    }
}

#[derive(Clone)]
struct CacheEntry {
    columns: CachedColumns,
    stamp: u64,
    bytes_estimate: usize,
}

#[derive(Clone, Copy)]
struct TouchEntry {
    key: CacheKey,
    stamp: u64,
}

#[cfg(test)]
mod tests {
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
    fn cache_order_stays_bounded_for_repeated_touch() {
        let mut inner = CacheInner::new();
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
            let _guard = cache.inner.lock().expect("poison cache lock");
            panic!("poison cache lock for test");
        });
        assert!(result.is_err());

        let columns = cache.get_or_compute(1, &samples, 1, WaveformChannelView::Mono, 1);
        assert!(matches!(columns, CachedColumns::Mono(_)));
    }
}
