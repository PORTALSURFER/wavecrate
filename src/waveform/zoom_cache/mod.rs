use super::{WaveformChannelView, WaveformColumnView, WaveformRenderer};
use std::{
    collections::hash_map::DefaultHasher,
    collections::{HashMap, VecDeque},
    hash::{Hash, Hasher},
    mem::size_of,
    sync::Mutex,
    time::Instant,
};
use tracing::warn;

mod telemetry;

#[cfg(test)]
mod tests;

/// Cache of precomputed waveform columns keyed by token, view, and width.
pub(super) struct WaveformZoomCache {
    shards: Vec<Mutex<CacheInner>>,
}

/// Global zoom-cache entry budget shared across all shards.
///
/// The budget is intentionally modest, but large enough to retain a few adjacent
/// zoom-width families per waveform token without immediately evicting them.
const ZOOM_CACHE_TOTAL_MAX_ENTRIES: usize = 32;
/// Number of independent mutex shards used to reduce lock contention.
const ZOOM_CACHE_SHARD_COUNT: usize = 4;

impl WaveformZoomCache {
    /// Create an empty cache with a small, bounded entry budget.
    pub(super) fn new() -> Self {
        let per_shard_max_entries = (ZOOM_CACHE_TOTAL_MAX_ENTRIES / ZOOM_CACHE_SHARD_COUNT).max(1);
        let mut shards = Vec::with_capacity(ZOOM_CACHE_SHARD_COUNT);
        for _ in 0..ZOOM_CACHE_SHARD_COUNT {
            shards.push(Mutex::new(CacheInner::new(per_shard_max_entries)));
        }
        Self { shards }
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
        let shard_index = shard_index_for_key(key);
        {
            let mut inner = self.lock_shard(shard_index);
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
        let mut inner = self.lock_shard(shard_index);
        if let Some(hit) = inner.get(key) {
            return hit;
        }
        inner.insert(key, computed.clone());
        computed
    }

    /// Lock one cache shard and recover from poison by resetting shard-local state.
    fn lock_shard(&self, shard_index: usize) -> std::sync::MutexGuard<'_, CacheInner> {
        let lock_start = telemetry::zoom_cache_telemetry_enabled().then(Instant::now);
        let guard = match self.shards[shard_index].lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Waveform zoom cache mutex poisoned; recovering with cleared cache.");
                let mut inner = poisoned.into_inner();
                let stale_bytes = inner.resident_bytes;
                inner.map.clear();
                inner.order.clear();
                inner.next_stamp = 1;
                inner.resident_bytes = 0;
                if telemetry::zoom_cache_telemetry_enabled() {
                    telemetry::record_zoom_cache_poison_recovery(stale_bytes);
                }
                inner
            }
        };
        if let Some(start) = lock_start {
            telemetry::record_zoom_cache_lock_wait(start.elapsed());
        }
        guard
    }
}

/// Resolve the deterministic mutex-shard index for one cache key.
fn shard_index_for_key(key: CacheKey) -> usize {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    (hasher.finish() as usize) % ZOOM_CACHE_SHARD_COUNT
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
    /// Create one bounded LRU shard with `max_entries` capacity.
    fn new(max_entries: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            max_entries: max_entries.max(1),
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
                telemetry::record_zoom_cache_miss();
                return None;
            }
        };
        self.order.push_back(TouchEntry { key, stamp });
        self.compact_order_if_needed();
        telemetry::record_zoom_cache_hit();
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
            telemetry::subtract_zoom_cache_resident_bytes(replaced.bytes_estimate);
        }
        self.resident_bytes = self.resident_bytes.saturating_add(bytes_estimate);
        telemetry::add_zoom_cache_resident_bytes(bytes_estimate);
        telemetry::record_zoom_cache_insert();
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
                telemetry::subtract_zoom_cache_resident_bytes(removed.bytes_estimate);
                telemetry::record_zoom_cache_evict();
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
        telemetry::record_zoom_cache_compaction();
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
