use std::path::{Path, PathBuf};
use wavecrate::sample_sources::SampleSource;

pub(super) fn source_for_path<'a>(
    sources: &'a [SampleSource],
    path: &Path,
) -> Option<&'a SampleSource> {
    sources
        .iter()
        .filter(|source| path.starts_with(&source.root))
        .max_by_key(|source| source.root.components().count())
}

pub(super) fn source_relative_path(source: &SampleSource, path: &Path) -> Option<PathBuf> {
    let relative = path.strip_prefix(&source.root).ok()?;
    (!relative.as_os_str().is_empty()).then(|| relative.to_path_buf())
}
