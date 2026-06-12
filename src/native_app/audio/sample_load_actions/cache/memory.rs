use std::{
    collections::hash_map::Entry,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::native_app::{
    app::{NativeAppState, WaveformCacheEntry, WaveformState},
    audio::sample_load_actions::cache::logging::log_slow_cache_phase,
};

impl NativeAppState {
    pub(in crate::native_app) fn remember_waveform(&mut self, waveform: &WaveformState) {
        if !waveform.has_loaded_sample() {
            return;
        }
        let started_at = Instant::now();
        let file = waveform.file();
        let entry = WaveformCacheEntry {
            byte_len: waveform.audio_bytes().len()
                + waveform
                    .playback_samples()
                    .map(|samples| samples.len() * std::mem::size_of::<f32>())
                    .unwrap_or(0),
            file,
        };
        self.insert_waveform_cache_entry(waveform.path(), entry);
        log_slow_cache_phase(
            "browser.sample_cache.remember",
            &waveform.path(),
            started_at,
        );
    }

    pub(in crate::native_app) fn remap_renamed_waveform_cache_path(
        &mut self,
        old_path: &Path,
        new_path: &Path,
    ) {
        let cache_paths = self
            .waveform
            .cache
            .entries
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for path in cache_paths {
            let Some(mapped) = remapped_cache_path(&path, old_path, new_path) else {
                continue;
            };
            if mapped == path {
                continue;
            }
            if let Some(entry) = self.waveform.cache.entries.remove(&path) {
                self.waveform.cache.entries.insert(mapped, entry);
            }
        }

        self.waveform.cache.order = self
            .waveform
            .cache
            .order
            .iter()
            .map(|path| {
                remapped_cache_path(path, old_path, new_path).unwrap_or_else(|| path.clone())
            })
            .collect();
        self.waveform.cache.warm_pending = self
            .waveform
            .cache
            .warm_pending
            .iter()
            .map(|path| {
                remapped_cache_path(path, old_path, new_path).unwrap_or_else(|| path.clone())
            })
            .collect();
        self.waveform.cache.cached_sample_paths = self
            .waveform
            .cache
            .cached_sample_paths
            .iter()
            .map(|id| {
                let path = PathBuf::from(id);
                remapped_cache_path(&path, old_path, new_path)
                    .map(|mapped| mapped.display().to_string())
                    .unwrap_or_else(|| id.clone())
            })
            .collect();
    }

    fn insert_waveform_cache_entry(&mut self, path: PathBuf, entry: WaveformCacheEntry) {
        match self.waveform.cache.entries.entry(path.clone()) {
            Entry::Occupied(mut occupied) => {
                let old_entry = std::mem::replace(occupied.get_mut(), entry);
                self.waveform.cache.bytes = self
                    .waveform
                    .cache
                    .bytes
                    .saturating_sub(old_entry.byte_len)
                    .saturating_add(occupied.get().byte_len);
            }
            Entry::Vacant(vacant) => {
                self.waveform.cache.bytes =
                    self.waveform.cache.bytes.saturating_add(entry.byte_len);
                vacant.insert(entry);
            }
        }
        self.waveform
            .cache
            .cached_sample_paths
            .insert(path.display().to_string());
        self.touch_cached_waveform_path(path);
        self.enforce_waveform_cache_limit();
    }

    pub(in crate::native_app::audio::sample_load_actions) fn touch_cached_waveform_path(
        &mut self,
        path: PathBuf,
    ) {
        self.waveform.cache.order.retain(|cached| cached != &path);
        self.waveform.cache.order.push_back(path);
    }
}

fn remapped_cache_path(path: &Path, old_path: &Path, new_path: &Path) -> Option<PathBuf> {
    if path == old_path {
        return Some(new_path.to_path_buf());
    }
    path.strip_prefix(old_path)
        .ok()
        .map(|relative| new_path.join(relative))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remapped_cache_path_maps_exact_path_and_children() {
        let old_path = Path::new("old");
        let new_path = Path::new("new");

        assert_eq!(
            remapped_cache_path(Path::new("old"), old_path, new_path),
            Some(PathBuf::from("new"))
        );
        assert_eq!(
            remapped_cache_path(Path::new("old/kick.wav"), old_path, new_path),
            Some(PathBuf::from("new/kick.wav"))
        );
        assert_eq!(
            remapped_cache_path(Path::new("other"), old_path, new_path),
            None
        );
    }
}
