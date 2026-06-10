use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

pub(super) struct BoundedLruCache<K, V> {
    entries: HashMap<K, CacheEntry<V>>,
    order: VecDeque<TouchEntry<K>>,
    max_entries: usize,
    next_stamp: u64,
    resident_bytes: usize,
}

impl<K, V> BoundedLruCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    pub(super) fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            max_entries: max_entries.max(1),
            next_stamp: 1,
            resident_bytes: 0,
        }
    }

    pub(super) fn get(&mut self, key: &K) -> CacheGet<V> {
        let stamp = self.next_stamp();
        let value = match self.entries.get_mut(key) {
            Some(entry) => {
                entry.stamp = stamp;
                entry.value.clone()
            }
            None => {
                return CacheGet {
                    value: None,
                    compacted: false,
                };
            }
        };
        self.order.push_back(TouchEntry {
            key: key.clone(),
            stamp,
        });
        CacheGet {
            value: Some(value),
            compacted: self.compact_order_if_needed(),
        }
    }

    pub(super) fn insert(&mut self, key: K, value: V, bytes_estimate: usize) -> CacheInsert {
        let stamp = self.next_stamp();
        let replaced_bytes = self
            .entries
            .insert(
                key.clone(),
                CacheEntry {
                    value,
                    stamp,
                    bytes_estimate,
                },
            )
            .map_or(0, |replaced| {
                self.resident_bytes = self.resident_bytes.saturating_sub(replaced.bytes_estimate);
                replaced.bytes_estimate
            });
        self.resident_bytes = self.resident_bytes.saturating_add(bytes_estimate);
        self.order.push_back(TouchEntry { key, stamp });
        let compacted = self.compact_order_if_needed();
        let evicted = self.evict_overflow();
        CacheInsert {
            inserted_bytes: bytes_estimate,
            replaced_bytes,
            evicted_bytes: evicted.bytes,
            evicted_count: evicted.count,
            compacted,
            resident_bytes: self.resident_bytes,
        }
    }

    pub(super) fn clear(&mut self) -> usize {
        let resident_bytes = self.resident_bytes;
        self.entries.clear();
        self.order.clear();
        self.next_stamp = 1;
        self.resident_bytes = 0;
        resident_bytes
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub(super) fn order_len(&self) -> usize {
        self.order.len()
    }

    #[cfg(test)]
    pub(super) fn resident_bytes(&self) -> usize {
        self.resident_bytes
    }

    fn evict_overflow(&mut self) -> Eviction {
        let mut eviction = Eviction::default();
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
                eviction.bytes = eviction.bytes.saturating_add(removed.bytes_estimate);
                eviction.count = eviction.count.saturating_add(1);
            }
        }
        eviction
    }

    fn compact_order_if_needed(&mut self) -> bool {
        let compact_threshold = self.max_entries.saturating_mul(8).max(self.max_entries + 1);
        if self.order.len() <= compact_threshold {
            return false;
        }

        let mut active = self
            .entries
            .iter()
            .map(|(key, entry)| TouchEntry {
                key: key.clone(),
                stamp: entry.stamp,
            })
            .collect::<Vec<_>>();
        active.sort_by_key(|entry| entry.stamp);
        self.order = active.into_iter().collect();
        true
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

pub(super) struct CacheGet<V> {
    pub(super) value: Option<V>,
    pub(super) compacted: bool,
}

pub(super) struct CacheInsert {
    pub(super) inserted_bytes: usize,
    pub(super) replaced_bytes: usize,
    pub(super) evicted_bytes: usize,
    pub(super) evicted_count: usize,
    pub(super) compacted: bool,
    pub(super) resident_bytes: usize,
}

struct CacheEntry<V> {
    value: V,
    stamp: u64,
    bytes_estimate: usize,
}

struct TouchEntry<K> {
    key: K,
    stamp: u64,
}

#[derive(Default)]
struct Eviction {
    bytes: usize,
    count: usize,
}

#[cfg(test)]
mod tests {
    use super::BoundedLruCache;

    #[test]
    fn replacement_updates_value_and_byte_accounting() {
        let mut cache = BoundedLruCache::new(2);

        let first = cache.insert("a", "first", 10);
        let second = cache.insert("a", "second", 4);

        assert_eq!(first.resident_bytes, 10);
        assert_eq!(second.replaced_bytes, 10);
        assert_eq!(second.resident_bytes, 4);
        assert_eq!(cache.resident_bytes(), 4);
        assert_eq!(cache.get(&"a").value, Some("second"));
    }

    #[test]
    fn insert_evicts_least_recently_used_entry() {
        let mut cache = BoundedLruCache::new(2);
        cache.insert("a", 1, 1);
        cache.insert("b", 2, 1);
        assert_eq!(cache.get(&"a").value, Some(1));

        let inserted = cache.insert("c", 3, 1);

        assert_eq!(inserted.evicted_count, 1);
        assert_eq!(cache.get(&"a").value, Some(1));
        assert_eq!(cache.get(&"b").value, None);
        assert_eq!(cache.get(&"c").value, Some(3));
    }

    #[test]
    fn stale_touch_queue_compacts_after_repeated_hits() {
        let mut cache = BoundedLruCache::new(1);
        cache.insert("only", 1, 1);
        for _ in 0..128 {
            let _ = cache.get(&"only");
        }

        assert_eq!(cache.len(), 1);
        assert!(cache.order_len() <= 8);
    }

    #[test]
    fn byte_accounting_tracks_replacement_eviction_and_clear() {
        let mut cache = BoundedLruCache::new(2);
        cache.insert("a", 1, 10);
        cache.insert("b", 2, 20);

        let replacement = cache.insert("a", 3, 5);
        assert_eq!(replacement.replaced_bytes, 10);
        assert_eq!(replacement.resident_bytes, 25);

        let eviction = cache.insert("c", 4, 7);
        assert_eq!(eviction.evicted_bytes, 20);
        assert_eq!(eviction.resident_bytes, 12);
        assert_eq!(cache.clear(), 12);
        assert_eq!(cache.resident_bytes(), 0);
        assert_eq!(cache.len(), 0);
    }
}
