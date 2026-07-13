use super::super::{FolderBrowserState, SourceEntry, source_scan_cache::load_source_scan_cache};
use wavecrate::sample_sources::SampleSource;

impl FolderBrowserState {
    #[cfg(test)]
    pub(in crate::native_app) fn from_sample_sources(sources: &[SampleSource]) -> Self {
        let entries = sources
            .iter()
            .map(SourceEntry::from_sample_source)
            .filter(|source| !source.is_default_assets_source())
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return Self::empty();
        }
        let selected_source = entries[0].id.clone();
        Self::from_sources(entries, selected_source)
    }

    pub(in crate::native_app) fn from_sample_sources_deferred(sources: &[SampleSource]) -> Self {
        let scan_cache = load_source_scan_cache().unwrap_or_else(|error| {
            tracing::warn!("{error}; falling back to source disk scan");
            Default::default()
        });
        let entries = sources
            .iter()
            .map(|source| {
                let mut entry = SourceEntry::from_sample_source(source);
                if let Some(snapshot) =
                    scan_cache.source_snapshot_for_source(source.id.as_str(), &source.root)
                {
                    entry.root_folder = Some(snapshot.root_folder);
                    entry.parked_tree_loaded = true;
                    entry.missing_collection_snapshot = snapshot.missing_collection_snapshot;
                }
                entry
            })
            .filter(|source| !source.is_default_assets_source())
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return Self::empty();
        }
        let selected_source = entries[0].id.clone();
        Self::from_sources_deferred(entries, selected_source)
    }
}
