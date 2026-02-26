use super::{WaveformChannelView, WaveformColumnView, WaveformRenderer};
use std::{
    collections::{HashMap, VecDeque},
    hash::{Hash, Hasher},
    sync::Mutex,
};
use tracing::warn;

/// Cache of precomputed waveform columns keyed by token, view, and width.
pub(super) struct WaveformZoomCache {
    inner: Mutex<CacheInner>,
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
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Waveform zoom cache mutex poisoned; recovering with cleared cache.");
                let mut inner = poisoned.into_inner();
                inner.map.clear();
                inner.order.clear();
                inner.next_stamp = 1;
                inner
            }
        }
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
}

impl CacheInner {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            max_entries: 12,
            next_stamp: 1,
        }
    }

    fn get(&mut self, key: CacheKey) -> Option<CachedColumns> {
        let stamp = self.next_stamp();
        let entry = self.map.get_mut(&key)?;
        entry.stamp = stamp;
        self.order.push_back(TouchEntry { key, stamp });
        self.compact_order_if_needed();
        Some(entry.columns.clone())
    }

    #[cfg(test)]
    fn touch(&mut self, key: CacheKey) {
        if let Some(entry) = self.map.get_mut(&key) {
            let stamp = self.next_stamp();
            entry.stamp = stamp;
            self.order.push_back(TouchEntry { key, stamp });
            self.compact_order_if_needed();
        }
    }

    fn insert(&mut self, key: CacheKey, value: CachedColumns) {
        let stamp = self.next_stamp();
        self.map.insert(
            key,
            CacheEntry {
                columns: value,
                stamp,
            },
        );
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
            if is_current {
                self.map.remove(&touch.key);
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
