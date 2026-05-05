use std::collections::{HashMap, VecDeque};
use std::mem::size_of;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

use crate::hotpath_telemetry;
use crate::waveform::DecodedWaveform;

/// LRU cache of decoded waveform payloads used by [`WaveformRenderer`].
///
/// Cache keys are derived from input bytes and entries are kept in insertion/access
/// order with bounded eviction.
pub(crate) struct DecodeCache {
    entries: HashMap<DecodeCacheKey, CacheEntry>,
    order: VecDeque<TouchEntry>,
    max_entries: usize,
    next_stamp: u64,
    resident_bytes: usize,
}

/// Compact fixed-size decode cache key derived from source bytes.
pub(super) type DecodeCacheKey = blake3::Hash;

const DECODE_CACHE_TELEMETRY_LOG_EVERY: u64 = 1_024;
static DECODE_CACHE_TELEMETRY_ENABLED: OnceLock<bool> = OnceLock::new();
static DECODE_CACHE_LOCK_ACQUIRE_COUNT: AtomicU64 = AtomicU64::new(0);
static DECODE_CACHE_LOCK_WAIT_NS: AtomicU64 = AtomicU64::new(0);
static DECODE_CACHE_LOCK_POISON_COUNT: AtomicU64 = AtomicU64::new(0);
static DECODE_CACHE_HIT_COUNT: AtomicU64 = AtomicU64::new(0);
static DECODE_CACHE_MISS_COUNT: AtomicU64 = AtomicU64::new(0);
static DECODE_CACHE_INSERT_COUNT: AtomicU64 = AtomicU64::new(0);
static DECODE_CACHE_EVICT_COUNT: AtomicU64 = AtomicU64::new(0);
static DECODE_CACHE_COMPACT_COUNT: AtomicU64 = AtomicU64::new(0);
static DECODE_CACHE_RESIDENT_BYTES: AtomicU64 = AtomicU64::new(0);
static DECODE_CACHE_PEAK_RESIDENT_BYTES: AtomicU64 = AtomicU64::new(0);

fn decode_cache_telemetry_enabled() -> bool {
    hotpath_telemetry::enabled(&DECODE_CACHE_TELEMETRY_ENABLED)
}

/// Record lock wait observed while entering the decode cache critical section.
pub(super) fn record_decode_cache_lock_wait(duration: Duration) {
    if !decode_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = DECODE_CACHE_LOCK_ACQUIRE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    hotpath_telemetry::add_duration_ns(&DECODE_CACHE_LOCK_WAIT_NS, duration);
    maybe_emit_decode_cache_telemetry(sample_tick);
}

/// Record a poisoned decode cache lock recovery/fallback event.
pub(super) fn record_decode_cache_lock_poison() {
    if !decode_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = DECODE_CACHE_LOCK_POISON_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_decode_cache_telemetry(sample_tick);
}

fn record_decode_cache_hit() {
    if !decode_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = DECODE_CACHE_HIT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_decode_cache_telemetry(sample_tick);
}

fn record_decode_cache_miss() {
    if !decode_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = DECODE_CACHE_MISS_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_decode_cache_telemetry(sample_tick);
}

fn record_decode_cache_insert() {
    if !decode_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = DECODE_CACHE_INSERT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_decode_cache_telemetry(sample_tick);
}

fn record_decode_cache_evict() {
    if !decode_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = DECODE_CACHE_EVICT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_decode_cache_telemetry(sample_tick);
}

fn record_decode_cache_compaction() {
    if !decode_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = DECODE_CACHE_COMPACT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_decode_cache_telemetry(sample_tick);
}

fn update_decode_cache_resident_bytes(resident_bytes: usize) {
    if !decode_cache_telemetry_enabled() {
        return;
    }
    hotpath_telemetry::store_resident_and_peak(
        &DECODE_CACHE_RESIDENT_BYTES,
        &DECODE_CACHE_PEAK_RESIDENT_BYTES,
        resident_bytes,
    );
}

fn maybe_emit_decode_cache_telemetry(sample_tick: u64) {
    if !decode_cache_telemetry_enabled()
        || !hotpath_telemetry::should_emit(sample_tick, DECODE_CACHE_TELEMETRY_LOG_EVERY)
    {
        return;
    }

    let lock_acquires = DECODE_CACHE_LOCK_ACQUIRE_COUNT.load(Ordering::Relaxed);
    let lock_wait_ns = DECODE_CACHE_LOCK_WAIT_NS.load(Ordering::Relaxed);
    let lock_poison = DECODE_CACHE_LOCK_POISON_COUNT.load(Ordering::Relaxed);
    let hits = DECODE_CACHE_HIT_COUNT.load(Ordering::Relaxed);
    let misses = DECODE_CACHE_MISS_COUNT.load(Ordering::Relaxed);
    let inserts = DECODE_CACHE_INSERT_COUNT.load(Ordering::Relaxed);
    let evicts = DECODE_CACHE_EVICT_COUNT.load(Ordering::Relaxed);
    let compactions = DECODE_CACHE_COMPACT_COUNT.load(Ordering::Relaxed);
    let resident_bytes = DECODE_CACHE_RESIDENT_BYTES.load(Ordering::Relaxed);
    let peak_resident_bytes = DECODE_CACHE_PEAK_RESIDENT_BYTES.load(Ordering::Relaxed);
    let avg_lock_wait_us = if lock_acquires == 0 {
        0.0
    } else {
        lock_wait_ns as f64 / lock_acquires as f64 / 1_000.0
    };

    tracing::info!(
        target: "perf::hotpath",
        module = "waveform_decode_cache",
        lock_acquires,
        avg_lock_wait_us,
        lock_poison,
        hits,
        misses,
        inserts,
        evicts,
        compactions,
        resident_bytes,
        peak_resident_bytes,
        "Waveform decode cache telemetry snapshot"
    );
}

fn decoded_waveform_bytes_estimate(decoded: &DecodedWaveform) -> usize {
    let sample_bytes = decoded.samples.len().saturating_mul(size_of::<f32>());
    let analysis_bytes = decoded
        .analysis_samples
        .len()
        .saturating_mul(size_of::<f32>());
    let peak_bytes = decoded.peaks.as_ref().map_or(0, |peaks| {
        let pair_size = size_of::<(f32, f32)>();
        let mono = peaks.mono.len().saturating_mul(pair_size);
        let left = peaks
            .left
            .as_ref()
            .map_or(0, |values| values.len().saturating_mul(pair_size));
        let right = peaks
            .right
            .as_ref()
            .map_or(0, |values| values.len().saturating_mul(pair_size));
        mono.saturating_add(left).saturating_add(right)
    });
    sample_bytes
        .saturating_add(analysis_bytes)
        .saturating_add(peak_bytes)
}

impl DecodeCache {
    /// Create a bounded cache with the requested maximum number of entries.
    pub(super) fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            max_entries: max_entries.max(1),
            next_stamp: 1,
            resident_bytes: 0,
        }
    }

    /// Return a cached decoded waveform for `key`, if present.
    ///
    /// When a hit occurs the entry is marked as most recently used.
    pub(super) fn get(&mut self, key: &DecodeCacheKey) -> Option<Arc<DecodedWaveform>> {
        let stamp = self.next_stamp();
        let waveform = match self.entries.get_mut(key) {
            Some(entry) => {
                entry.stamp = stamp;
                Arc::clone(&entry.waveform)
            }
            None => {
                record_decode_cache_miss();
                return None;
            }
        };
        self.order.push_back(TouchEntry { key: *key, stamp });
        self.compact_order_if_needed();
        record_decode_cache_hit();
        Some(waveform)
    }

    /// Insert a decoded waveform and evict least-recently-used entries if needed.
    pub(super) fn insert(&mut self, key: DecodeCacheKey, value: Arc<DecodedWaveform>) {
        let stamp = self.next_stamp();
        let bytes_estimate =
            decoded_waveform_bytes_estimate(&value).saturating_add(size_of::<DecodeCacheKey>());
        if let Some(replaced) = self.entries.insert(
            key,
            CacheEntry {
                waveform: value,
                stamp,
                bytes_estimate,
            },
        ) {
            self.resident_bytes = self.resident_bytes.saturating_sub(replaced.bytes_estimate);
        }
        self.resident_bytes = self.resident_bytes.saturating_add(bytes_estimate);
        update_decode_cache_resident_bytes(self.resident_bytes);
        record_decode_cache_insert();
        self.order.push_back(TouchEntry { key, stamp });
        self.compact_order_if_needed();
        self.evict_overflow();
    }

    /// Remove oldest entries until cache occupancy is within the configured limit.
    fn evict_overflow(&mut self) {
        while self.entries.len() > self.max_entries {
            let Some(touch) = self.order.pop_front() else {
                break;
            };
            let is_current = self
                .entries
                .get(&touch.key)
                .is_some_and(|entry| entry.stamp == touch.stamp);
            if is_current && let Some(removed) = self.entries.remove(&touch.key) {
                self.resident_bytes = self.resident_bytes.saturating_sub(removed.bytes_estimate);
                update_decode_cache_resident_bytes(self.resident_bytes);
                record_decode_cache_evict();
            }
        }
    }

    fn compact_order_if_needed(&mut self) {
        let compact_threshold = self.max_entries.saturating_mul(8).max(self.max_entries + 1);
        if self.order.len() <= compact_threshold {
            return;
        }

        let mut active: Vec<_> = self
            .entries
            .iter()
            .map(|(key, entry)| TouchEntry {
                key: *key,
                stamp: entry.stamp,
            })
            .collect();
        active.sort_by_key(|entry| entry.stamp);
        self.order = active.into_iter().collect();
        record_decode_cache_compaction();
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

struct CacheEntry {
    waveform: Arc<DecodedWaveform>,
    stamp: u64,
    bytes_estimate: usize,
}

struct TouchEntry {
    key: DecodeCacheKey,
    stamp: u64,
}

/// Compute a stable content hash for decoded bytes for cache keying.
pub(super) fn hash_bytes(bytes: &[u8]) -> DecodeCacheKey {
    blake3::hash(bytes)
}

#[cfg(test)]
mod tests {
    use super::{DecodeCache, hash_bytes};
    use crate::waveform::{DecodedWaveform, next_cache_token};
    use std::sync::Arc;

    fn decoded(sample: f32) -> Arc<DecodedWaveform> {
        Arc::new(DecodedWaveform {
            cache_token: next_cache_token(),
            samples: Arc::from([sample]),
            analysis_samples: Arc::from([sample]),
            analysis_sample_rate: 44_100,
            analysis_stride: 1,
            peaks: None,
            duration_seconds: 1.0,
            sample_rate: 44_100,
            channels: 1,
        })
    }

    #[test]
    fn get_refreshes_recency_for_eviction() {
        let mut cache = DecodeCache::new(2);
        let key_a = hash_bytes(b"a");
        let key_b = hash_bytes(b"b");
        let key_c = hash_bytes(b"c");
        cache.insert(key_a, decoded(0.1));
        cache.insert(key_b, decoded(0.2));

        let hit = cache.get(&key_a);
        assert!(hit.is_some());

        cache.insert(key_c, decoded(0.3));
        assert!(cache.get(&key_a).is_some());
        assert!(cache.get(&key_c).is_some());
        assert!(cache.get(&key_b).is_none());
    }

    #[test]
    fn recency_queue_is_compacted_after_many_hits() {
        let mut cache = DecodeCache::new(1);
        let key = hash_bytes(b"only");
        cache.insert(key, decoded(0.1));
        for _ in 0..128 {
            let _ = cache.get(&key);
        }

        assert_eq!(cache.entries.len(), 1);
        assert!(cache.order.len() <= 8);
    }
}
