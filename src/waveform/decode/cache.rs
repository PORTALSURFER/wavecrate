use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use crate::waveform::DecodedWaveform;

/// LRU cache of decoded waveform payloads used by [`WaveformRenderer`].
///
/// Cache keys are derived from input bytes and entries are kept in insertion/access
/// order with bounded eviction.
pub(crate) struct DecodeCache {
    entries: HashMap<String, CacheEntry>,
    order: VecDeque<TouchEntry>,
    max_entries: usize,
    next_stamp: u64,
}

impl DecodeCache {
    /// Create a bounded cache with the requested maximum number of entries.
    pub(super) fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            max_entries: max_entries.max(1),
            next_stamp: 1,
        }
    }

    /// Return a cached decoded waveform for `key`, if present.
    ///
    /// When a hit occurs the entry is marked as most recently used.
    pub(super) fn get(&mut self, key: &str) -> Option<Arc<DecodedWaveform>> {
        let stamp = self.next_stamp();
        let entry = self.entries.get_mut(key)?;
        entry.stamp = stamp;
        self.order.push_back(TouchEntry {
            key: key.to_string(),
            stamp,
        });
        self.compact_order_if_needed();
        Some(Arc::clone(&entry.waveform))
    }

    /// Insert a decoded waveform and evict least-recently-used entries if needed.
    pub(super) fn insert(&mut self, key: String, value: Arc<DecodedWaveform>) {
        let stamp = self.next_stamp();
        self.entries.insert(
            key.clone(),
            CacheEntry {
                waveform: value,
                stamp,
            },
        );
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
            if is_current {
                self.entries.remove(&touch.key);
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
                key: key.clone(),
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

struct CacheEntry {
    waveform: Arc<DecodedWaveform>,
    stamp: u64,
}

struct TouchEntry {
    key: String,
    stamp: u64,
}

/// Compute a stable content hash for decoded bytes for cache keying.
pub(super) fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::DecodeCache;
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
        cache.insert(String::from("a"), decoded(0.1));
        cache.insert(String::from("b"), decoded(0.2));

        let hit = cache.get("a");
        assert!(hit.is_some());

        cache.insert(String::from("c"), decoded(0.3));
        assert!(cache.get("a").is_some());
        assert!(cache.get("c").is_some());
        assert!(cache.get("b").is_none());
    }

    #[test]
    fn recency_queue_is_compacted_after_many_hits() {
        let mut cache = DecodeCache::new(1);
        cache.insert(String::from("only"), decoded(0.1));
        for _ in 0..128 {
            let _ = cache.get("only");
        }

        assert_eq!(cache.entries.len(), 1);
        assert!(cache.order.len() <= 8);
    }
}
