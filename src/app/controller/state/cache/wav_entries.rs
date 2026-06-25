//! Paged WAV-entry storage and normalized relative-path lookup.

use crate::sample_sources::{SourceId, WavEntry};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(crate) struct WavEntriesState {
    pub(crate) total: usize,
    pub(crate) page_size: usize,
    pub(crate) pages: HashMap<usize, Vec<WavEntry>>,
    pub(crate) lookup: HashMap<PathBuf, usize>,
    pub(crate) source_id: Option<SourceId>,
}

impl WavEntriesState {
    pub(crate) fn new(total: usize, page_size: usize) -> Self {
        Self {
            total,
            page_size: page_size.max(1),
            pages: HashMap::new(),
            lookup: HashMap::new(),
            source_id: None,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.total = 0;
        self.pages.clear();
        self.lookup.clear();
        self.source_id = None;
    }

    pub(crate) fn insert_page(&mut self, page_index: usize, entries: Vec<WavEntry>) {
        let offset = page_index * self.page_size;
        for (idx, entry) in entries.iter().enumerate() {
            self.insert_lookup(entry.relative_path.clone(), offset + idx);
        }
        self.pages.insert(page_index, entries);
    }

    pub(crate) fn entry(&self, index: usize) -> Option<&WavEntry> {
        let page_index = index / self.page_size;
        let in_page = index % self.page_size;
        self.pages
            .get(&page_index)
            .and_then(|page| page.get(in_page))
    }

    pub(crate) fn entry_mut(&mut self, index: usize) -> Option<&mut WavEntry> {
        let page_index = index / self.page_size;
        let in_page = index % self.page_size;
        self.pages
            .get_mut(&page_index)
            .and_then(|page| page.get_mut(in_page))
    }

    pub(crate) fn update_entry(&mut self, path: &Path, entry: WavEntry) -> bool {
        let normalized = path.to_string_lossy().replace('\\', "/");
        let Some(index) = self.lookup.get(Path::new(&normalized)).copied() else {
            return false;
        };
        let Some(slot) = self.entry_mut(index) else {
            return false;
        };
        *slot = entry;
        true
    }

    /// Insert one entry at the absolute index when the surrounding page window is loaded.
    ///
    /// Returns `true` when any loaded page content changed. If the insertion falls wholly
    /// outside the loaded page window, only `total` is updated and callers should treat the
    /// change as metadata-only for the current cache.
    pub(crate) fn insert_entry_at(&mut self, index: usize, entry: WavEntry) -> bool {
        self.total = self.total.saturating_add(1);
        let mut changed = false;
        let mut current_index = index;
        let mut carry = entry;
        loop {
            let page_index = current_index / self.page_size;
            let in_page = current_index % self.page_size;
            let Some(page) = self.pages.get_mut(&page_index) else {
                break;
            };
            let insert_at = in_page.min(page.len());
            page.insert(insert_at, carry);
            changed = true;
            if page.len() <= self.page_size {
                break;
            }
            let Some(displaced) = page.pop() else {
                break;
            };
            carry = displaced;
            current_index = (page_index + 1) * self.page_size;
        }
        if changed {
            self.lookup.clear();
            let mut lookup_entries = Vec::new();
            for (page_index, page) in &self.pages {
                let offset = page_index * self.page_size;
                for (idx, item) in page.iter().enumerate() {
                    lookup_entries.push((item.relative_path.clone(), offset + idx));
                }
            }
            for (path, index) in lookup_entries {
                self.insert_lookup(path, index);
            }
        }
        changed
    }

    pub(crate) fn insert_lookup(&mut self, path: PathBuf, index: usize) {
        let normalized = path.to_string_lossy().replace('\\', "/");
        self.lookup.insert(PathBuf::from(normalized), index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav_entry(path: &str, tag: crate::sample_sources::Rating) -> WavEntry {
        WavEntry {
            relative_path: PathBuf::from(path),
            file_size: 0,
            modified_ns: 0,
            content_hash: None,
            tag,
            looped: false,
            sound_type: None,
            locked: false,
            missing: false,
            last_played_at: None,
            last_curated_at: None,
            user_tag: None,
            tag_named: false,
            normal_tags: Vec::new(),
        }
    }

    #[test]
    fn insert_lookup_normalizes_paths() {
        let mut cache = WavEntriesState::new(10, 10);

        cache.insert_lookup(PathBuf::from("foo\\bar.wav"), 1);

        assert_eq!(cache.lookup.get(Path::new("foo/bar.wav")), Some(&1));
        assert_eq!(cache.lookup.len(), 1);
        assert!(cache.lookup.contains_key(Path::new("foo/bar.wav")));
    }

    #[test]
    fn update_entry_normalizes_lookup_key() {
        let mut cache = WavEntriesState::new(10, 10);
        cache.insert_page(
            0,
            vec![wav_entry(
                "foo/bar.wav",
                crate::sample_sources::Rating::NEUTRAL,
            )],
        );

        let new_entry = wav_entry("foo/bar.wav", crate::sample_sources::Rating::KEEP_1);
        let success = cache.update_entry(Path::new("foo\\bar.wav"), new_entry);

        assert!(success, "Should find entry even with backslash path");
        let entry = cache.entry(0).unwrap();
        assert_eq!(entry.tag, crate::sample_sources::Rating::KEEP_1);
    }
}
