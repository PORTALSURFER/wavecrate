use super::{WaveformChannelView, WaveformColumnView, WaveformRenderer};
use crate::waveform::lru_cache::BoundedLruCache;
use std::{
    collections::hash_map::DefaultHasher,
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
                let stale_bytes = inner.clear();
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
    entries: BoundedLruCache<CacheKey, CachedColumns>,
}

impl CacheInner {
    /// Create one bounded LRU shard with `max_entries` capacity.
    fn new(max_entries: usize) -> Self {
        Self {
            entries: BoundedLruCache::new(max_entries),
        }
    }

    fn get(&mut self, key: CacheKey) -> Option<CachedColumns> {
        let result = self.entries.get(&key);
        if result.compacted {
            telemetry::record_zoom_cache_compaction();
        }
        let Some(columns) = result.value else {
            telemetry::record_zoom_cache_miss();
            return None;
        };
        telemetry::record_zoom_cache_hit();
        Some(columns)
    }

    #[cfg(test)]
    fn touch(&mut self, key: CacheKey) {
        let result = self.entries.get(&key);
        if result.compacted {
            telemetry::record_zoom_cache_compaction();
        }
    }

    fn insert(&mut self, key: CacheKey, value: CachedColumns) {
        let bytes_estimate = cached_columns_bytes(&value);
        let outcome = self.entries.insert(key, value, bytes_estimate);
        if outcome.replaced_bytes > 0 {
            telemetry::subtract_zoom_cache_resident_bytes(outcome.replaced_bytes);
        }
        telemetry::add_zoom_cache_resident_bytes(outcome.inserted_bytes);
        if outcome.evicted_bytes > 0 {
            telemetry::subtract_zoom_cache_resident_bytes(outcome.evicted_bytes);
        }
        telemetry::record_zoom_cache_insert();
        if outcome.compacted {
            telemetry::record_zoom_cache_compaction();
        }
        for _ in 0..outcome.evicted_count {
            telemetry::record_zoom_cache_evict();
        }
    }

    fn clear(&mut self) -> usize {
        self.entries.clear()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    fn order_len(&self) -> usize {
        self.entries.order_len()
    }
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
