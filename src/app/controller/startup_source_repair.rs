//! Conservative startup repair for persisted transient test-source roots.

use std::path::{Path, PathBuf};

use crate::sample_sources::SampleSource;

const ALWAYS_TRANSIENT_TEMP_SOURCE_NAMES: &[&str] =
    &["gui-source", "browser-source", "sources-source"];
const GENERIC_TRANSIENT_TEMP_SOURCE_NAMES: &[&str] =
    &["source", "source-a", "source-b", "source_a", "source_b"];
const TEMPFILE_DIR_PREFIX: &str = ".tmp";

/// Result of pruning persisted startup sources before controller initialization.
#[derive(Debug)]
pub(crate) struct StartupSourceRepair {
    /// Sources kept for normal startup.
    pub(crate) retained_sources: Vec<SampleSource>,
    /// Source roots removed as obvious transient test fixtures.
    pub(crate) removed_roots: Vec<PathBuf>,
}

/// Remove persisted temp/test source roots that should never survive into live startup.
pub(crate) fn repair_persisted_startup_sources(sources: Vec<SampleSource>) -> StartupSourceRepair {
    let mut retained_sources = Vec::with_capacity(sources.len());
    let mut removed_roots = Vec::new();
    for source in sources {
        if is_transient_test_source_root(&source.root) {
            removed_roots.push(source.root);
        } else {
            retained_sources.push(source);
        }
    }
    StartupSourceRepair {
        retained_sources,
        removed_roots,
    }
}

fn is_transient_test_source_root(root: &Path) -> bool {
    if !root.starts_with(std::env::temp_dir()) {
        return false;
    }
    let Some(file_name) = root.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if matches_known_name(file_name, ALWAYS_TRANSIENT_TEMP_SOURCE_NAMES) {
        return true;
    }
    if !matches_known_name(file_name, GENERIC_TRANSIENT_TEMP_SOURCE_NAMES) {
        return false;
    }
    !root.exists() || has_tempfile_parent(root)
}

fn matches_known_name(name: &str, known_names: &[&str]) -> bool {
    known_names
        .iter()
        .any(|known_name| known_name.eq_ignore_ascii_case(name))
}

fn has_tempfile_parent(root: &Path) -> bool {
    root.parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(TEMPFILE_DIR_PREFIX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn removes_named_gui_fixture_source_roots_under_temp() {
        let fixture_dir = tempdir().expect("fixture tempdir");
        let fixture_root = fixture_dir.path().join("browser-source");
        std::fs::create_dir_all(&fixture_root).expect("create browser fixture root");

        let repair = repair_persisted_startup_sources(vec![SampleSource::new(fixture_root)]);

        assert!(repair.retained_sources.is_empty());
        assert_eq!(repair.removed_roots.len(), 1);
    }

    #[test]
    fn removes_generic_source_roots_when_parent_is_a_tempfile_dir() {
        let fixture_dir = tempdir().expect("fixture tempdir");
        let fixture_root = fixture_dir.path().join("source");
        std::fs::create_dir_all(&fixture_root).expect("create source fixture root");

        let repair = repair_persisted_startup_sources(vec![SampleSource::new(fixture_root)]);

        assert!(repair.retained_sources.is_empty());
        assert_eq!(repair.removed_roots.len(), 1);
    }

    #[test]
    fn keeps_generic_temp_named_sources_outside_tempfile_layouts() {
        let base = tempdir().expect("base tempdir");
        let user_root = base.path().join("user-project").join("source");
        std::fs::create_dir_all(&user_root).expect("create user source root");

        let repair = repair_persisted_startup_sources(vec![SampleSource::new(user_root.clone())]);

        assert_eq!(repair.retained_sources.len(), 1);
        assert_eq!(repair.retained_sources[0].root, user_root);
        assert!(repair.removed_roots.is_empty());
    }

    #[test]
    fn removes_missing_generic_fixture_roots_from_temp_dir() {
        let missing_root = std::env::temp_dir().join("source_a");

        let repair = repair_persisted_startup_sources(vec![SampleSource::new(missing_root)]);

        assert!(repair.retained_sources.is_empty());
        assert_eq!(repair.removed_roots.len(), 1);
    }
}
