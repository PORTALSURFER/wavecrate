use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use crate::waveform::DecodedWaveform;

/// LRU cache of decoded waveform payloads used by [`WaveformRenderer`].
///
/// Cache keys are derived from input bytes and entries are kept in insertion/access
/// order with bounded eviction.
pub(crate) struct DecodeCache {
    entries: HashMap<String, Arc<DecodedWaveform>>,
    order: VecDeque<String>,
    max_entries: usize,
}

impl DecodeCache {
    /// Create a bounded cache with the requested maximum number of entries.
    pub(super) fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            max_entries: max_entries.max(1),
        }
    }

    /// Return a cached decoded waveform for `key`, if present.
    ///
    /// When a hit occurs the entry is marked as most recently used.
    pub(super) fn get(&mut self, key: &str) -> Option<Arc<DecodedWaveform>> {
        let value = self.entries.get(key).cloned();
        if value.is_some() {
            self.touch(key);
        }
        value
    }

    /// Insert a decoded waveform and evict least-recently-used entries if needed.
    pub(super) fn insert(&mut self, key: String, value: Arc<DecodedWaveform>) {
        self.entries.insert(key.clone(), value);
        self.touch(&key);
        self.evict_overflow();
    }

    /// Update the recency ordering for `key`.
    fn touch(&mut self, key: &str) {
        self.order.retain(|existing| existing != key);
        self.order.push_front(key.to_string());
    }

    /// Remove oldest entries until cache occupancy is within the configured limit.
    fn evict_overflow(&mut self) {
        while self.order.len() > self.max_entries {
            if let Some(removed) = self.order.pop_back() {
                self.entries.remove(&removed);
            }
        }
    }
}

/// Compute a stable content hash for decoded bytes for cache keying.
pub(super) fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}
