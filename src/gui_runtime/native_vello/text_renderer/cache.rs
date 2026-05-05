//! Layout-cache and atom-cache helpers for the native text renderer.

use super::*;

impl NativeTextRenderer {
    pub(super) fn layout_for<'a>(
        &'a mut self,
        font: &FontData,
        text: &str,
        font_size: f32,
    ) -> Option<&'a TextLayout> {
        let text_atom = self.intern_text(text);
        let key = TextLayoutKey {
            text: text_atom,
            font_size_bits: font_size.to_bits(),
        };

        if let Some(layout) = self
            .layout_cache
            .get(&key)
            .map(|layout| layout as *const TextLayout)
        {
            self.text_layout_hits = self.text_layout_hits.saturating_add(1);
            return Some(unsafe { &*layout });
        }

        self.text_layout_misses = self.text_layout_misses.saturating_add(1);

        if self.layout_cache.len() >= TEXT_LAYOUT_CACHE_CAPACITY
            && let Some(evicted_key) = self.layout_cache_order.pop_front()
            && self.layout_cache.remove(&evicted_key).is_some()
        {
            self.text_layout_evictions = self.text_layout_evictions.saturating_add(1);
        }

        let Some(layout) = Self::compute_layout(font, text, font_size) else {
            return None;
        };
        self.layout_cache_order.push_back(key.clone());
        let cached_layout = self.layout_cache.entry(key).or_insert(layout);
        Some(cached_layout)
    }

    pub(in crate::gui_runtime::native_vello) fn take_layout_profile_counters(
        &mut self,
    ) -> (u64, u64, u64, u64, u64, u64) {
        let counters = (
            self.text_layout_hits,
            self.text_layout_misses,
            self.text_layout_evictions,
            self.text_atom_hits,
            self.text_atom_misses,
            self.text_atom_evictions,
        );
        self.text_layout_hits = 0;
        self.text_layout_misses = 0;
        self.text_layout_evictions = 0;
        self.text_atom_hits = 0;
        self.text_atom_misses = 0;
        self.text_atom_evictions = 0;
        counters
    }

    /// Intern text into a bounded atom cache so layout-key construction avoids
    /// hot-path `String` allocations on repeated runs.
    pub(super) fn intern_text(&mut self, text: &str) -> Arc<str> {
        self.atom_cache_clock = self.atom_cache_clock.saturating_add(1);
        let stamp = self.atom_cache_clock;
        if let Some((cached, _)) = self.atom_cache.get_key_value(text) {
            let atom = Arc::clone(cached);
            if let Some(last_seen) = self.atom_cache.get_mut(text) {
                *last_seen = stamp;
            }
            self.atom_cache_order.push_back((Arc::clone(&atom), stamp));
            self.compact_atom_cache_order_if_needed();
            self.text_atom_hits = self.text_atom_hits.saturating_add(1);
            return atom;
        }

        self.text_atom_misses = self.text_atom_misses.saturating_add(1);
        let atom: Arc<str> = Arc::from(text);
        self.atom_cache.insert(Arc::clone(&atom), stamp);
        self.atom_cache_order.push_back((Arc::clone(&atom), stamp));
        self.evict_stale_atoms();
        atom
    }

    /// Compact queued atom-order metadata after repeated cache hits append stale stamps.
    fn compact_atom_cache_order_if_needed(&mut self) {
        if self.atom_cache_order.len() <= TEXT_ATOM_CACHE_CAPACITY.saturating_mul(2) {
            return;
        }
        let mut ordered_atoms: Vec<_> = self
            .atom_cache
            .iter()
            .map(|(atom, stamp)| (Arc::clone(atom), *stamp))
            .collect();
        ordered_atoms.sort_by_key(|(_, stamp)| *stamp);
        self.atom_cache_order = ordered_atoms.into_iter().collect();
    }

    /// Evict stale atom-cache entries using insertion stamps for bounded memory.
    pub(super) fn evict_stale_atoms(&mut self) {
        while self.atom_cache.len() > TEXT_ATOM_CACHE_CAPACITY {
            let Some((candidate, queued_stamp)) = self.atom_cache_order.pop_front() else {
                break;
            };
            let Some(current_stamp) = self.atom_cache.get(candidate.as_ref()) else {
                continue;
            };
            if *current_stamp != queued_stamp {
                continue;
            }
            if self.atom_cache.remove(candidate.as_ref()).is_some() {
                self.text_atom_evictions = self.text_atom_evictions.saturating_add(1);
            }
        }
    }
}
