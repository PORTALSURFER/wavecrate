use super::super::{
    FolderBrowserState, SourceEntry, path_helpers::folder_label,
    source_scan_cache::load_source_scan_cache,
};
use wavecrate::sample_sources::SampleSource;

impl FolderBrowserState {
    #[cfg(test)]
    pub(in crate::native_app) fn from_sample_sources(sources: &[SampleSource]) -> Self {
        if sources.is_empty() {
            return Self::load_default();
        }
        let entries = sources
            .iter()
            .map(|source| {
                SourceEntry::new(
                    source.id.as_str().to_string(),
                    folder_label(&source.root),
                    source.root.clone(),
                )
            })
            .collect::<Vec<_>>();
        Self::from_sources(entries, sources[0].id.as_str().to_string())
    }

    pub(in crate::native_app) fn from_sample_sources_deferred(sources: &[SampleSource]) -> Self {
        if sources.is_empty() {
            return Self::load_default();
        }
        let scan_cache = load_source_scan_cache().unwrap_or_else(|error| {
            tracing::warn!("{error}; falling back to source disk scan");
            Default::default()
        });
        let entries = sources
            .iter()
            .map(|source| {
                let mut entry = SourceEntry::new(
                    source.id.as_str().to_string(),
                    folder_label(&source.root),
                    source.root.clone(),
                );
                if let Some(snapshot) =
                    scan_cache.source_snapshot_for_source(source.id.as_str(), &source.root)
                {
                    entry.root_folder = Some(snapshot.root_folder);
                    entry.missing_collection_snapshot = snapshot.missing_collection_snapshot;
                }
                entry
            })
            .collect::<Vec<_>>();
        Self::from_sources_deferred(entries, sources[0].id.as_str().to_string())
    }
}
