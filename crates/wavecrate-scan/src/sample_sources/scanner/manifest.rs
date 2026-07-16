use std::collections::{BTreeMap, BTreeSet};

use wavecrate_library::sample_sources::{SourceDatabase, SourceManifestEntry};

use super::scan::{
    CommittedSourceDelta, ManifestIdentityDelta, MovedManifestIdentity, ScanError, ScanStats,
};

pub(super) fn capture_manifest(
    database: &SourceDatabase,
) -> Result<Vec<SourceManifestEntry>, ScanError> {
    database.list_manifest_entries().map_err(ScanError::from)
}

pub(super) fn publish_committed_delta(
    database: &SourceDatabase,
    stats: &mut ScanStats,
    before: Vec<SourceManifestEntry>,
) -> Result<(), ScanError> {
    let (revision, after) = database.manifest_snapshot_with_revision()?;
    stats.committed_delta = build_committed_delta(&before, &after, revision);
    stats.manifest_before = before;
    stats.manifest_after = after;
    Ok(())
}

pub(super) fn build_committed_delta(
    before: &[SourceManifestEntry],
    after: &[SourceManifestEntry],
    revision: u64,
) -> CommittedSourceDelta {
    let mut matched_before = BTreeSet::new();
    let mut matched_after = BTreeSet::new();
    let mut matches = Vec::new();

    match_unique(
        before,
        after,
        |entry| normalized(entry.file_identity.as_deref()),
        &mut matched_before,
        &mut matched_after,
        &mut matches,
    );
    match_unique(
        before,
        after,
        |entry| {
            normalized(entry.file_identity.as_deref())
                .is_none()
                .then(|| entry.relative_path.to_string_lossy().into_owned())
        },
        &mut matched_before,
        &mut matched_after,
        &mut matches,
    );
    match_unique(
        before,
        after,
        |entry| {
            normalized(entry.file_identity.as_deref())
                .is_none()
                .then(|| normalized(entry.content_hash.as_deref()))
                .flatten()
        },
        &mut matched_before,
        &mut matched_after,
        &mut matches,
    );

    let mut delta = CommittedSourceDelta {
        revision,
        ..CommittedSourceDelta::default()
    };
    for (before_index, after_index) in matches {
        let previous = &before[before_index];
        let current = &after[after_index];
        let identity = stable_identity(current, Some(previous));
        let generation = content_generation(current);
        if previous.relative_path != current.relative_path {
            delta.moved.push(MovedManifestIdentity {
                identity: identity.clone(),
                old_relative_path: previous.relative_path.clone(),
                new_relative_path: current.relative_path.clone(),
                content_generation: generation.clone(),
            });
        }
        if content_generation(previous) != generation {
            delta.changed.push(ManifestIdentityDelta {
                identity,
                relative_path: current.relative_path.clone(),
                content_generation: generation,
            });
        }
    }
    for (index, entry) in after.iter().enumerate() {
        if !matched_after.contains(&index) {
            delta.created.push(identity_delta(entry, None));
        }
    }
    for (index, entry) in before.iter().enumerate() {
        if !matched_before.contains(&index) {
            delta.deleted.push(identity_delta(entry, None));
        }
    }
    delta
        .created
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    delta
        .changed
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    delta.moved.sort_by(|left, right| {
        left.old_relative_path
            .cmp(&right.old_relative_path)
            .then_with(|| left.new_relative_path.cmp(&right.new_relative_path))
    });
    delta
        .deleted
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    delta
}

fn match_unique(
    before: &[SourceManifestEntry],
    after: &[SourceManifestEntry],
    key: impl Fn(&SourceManifestEntry) -> Option<String>,
    matched_before: &mut BTreeSet<usize>,
    matched_after: &mut BTreeSet<usize>,
    matches: &mut Vec<(usize, usize)>,
) {
    let before_by_key = unique_indexes(before, &key, matched_before);
    let after_by_key = unique_indexes(after, &key, matched_after);
    for (value, before_index) in before_by_key {
        let Some(after_index) = after_by_key.get(&value).copied() else {
            continue;
        };
        matched_before.insert(before_index);
        matched_after.insert(after_index);
        matches.push((before_index, after_index));
    }
}

fn unique_indexes(
    entries: &[SourceManifestEntry],
    key: &impl Fn(&SourceManifestEntry) -> Option<String>,
    already_matched: &BTreeSet<usize>,
) -> BTreeMap<String, usize> {
    let mut indexes = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    for (index, entry) in entries.iter().enumerate() {
        if already_matched.contains(&index) {
            continue;
        }
        let Some(value) = key(entry) else {
            continue;
        };
        if indexes.insert(value.clone(), index).is_some() {
            duplicates.insert(value);
        }
    }
    indexes.retain(|value, _| !duplicates.contains(value));
    indexes
}

fn normalized(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn identity_delta(
    entry: &SourceManifestEntry,
    previous: Option<&SourceManifestEntry>,
) -> ManifestIdentityDelta {
    ManifestIdentityDelta {
        identity: stable_identity(entry, previous),
        relative_path: entry.relative_path.clone(),
        content_generation: content_generation(entry),
    }
}

fn stable_identity(entry: &SourceManifestEntry, previous: Option<&SourceManifestEntry>) -> String {
    normalized(entry.file_identity.as_deref())
        .or_else(|| previous.and_then(|entry| normalized(entry.file_identity.as_deref())))
        .map(|identity| format!("file:{identity}"))
        .or_else(|| {
            normalized(entry.content_hash.as_deref())
                .map(|content_hash| format!("hash:{content_hash}"))
        })
        .unwrap_or_else(|| format!("path:{}", entry.relative_path.display()))
}

fn content_generation(entry: &SourceManifestEntry) -> String {
    normalized(entry.content_hash.as_deref()).unwrap_or_else(|| {
        format!(
            "pending:{}:{}:{}",
            normalized(entry.file_identity.as_deref())
                .unwrap_or_else(|| entry.relative_path.display().to_string()),
            entry.file_size,
            entry.modified_ns,
        )
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::build_committed_delta;
    use wavecrate_library::sample_sources::SourceManifestEntry;

    fn entry(
        path: &str,
        identity: &str,
        hash: Option<&str>,
        size: u64,
        modified: i64,
    ) -> SourceManifestEntry {
        SourceManifestEntry {
            relative_path: PathBuf::from(path),
            file_identity: Some(identity.to_string()),
            content_hash: hash.map(str::to_string),
            file_size: size,
            modified_ns: modified,
        }
    }

    #[test]
    fn committed_delta_classifies_create_change_move_and_delete() {
        let before = vec![
            entry("changed.wav", "changed", Some("old"), 4, 1),
            entry("old.wav", "moved", Some("same"), 4, 1),
            entry("deleted.wav", "deleted", Some("gone"), 4, 1),
        ];
        let after = vec![
            entry("changed.wav", "changed", Some("new"), 4, 1),
            entry("new.wav", "moved", Some("same"), 4, 1),
            entry("created.wav", "created", None, 4, 1),
        ];

        let delta = build_committed_delta(&before, &after, 17);

        assert_eq!(delta.revision, 17);
        assert_eq!(delta.created.len(), 1);
        assert_eq!(delta.changed.len(), 1);
        assert_eq!(delta.moved.len(), 1);
        assert_eq!(delta.deleted.len(), 1);
        assert_eq!(delta.moved[0].old_relative_path, PathBuf::from("old.wav"));
        assert_eq!(delta.moved[0].new_relative_path, PathBuf::from("new.wav"));
    }

    #[test]
    fn same_path_identity_replacement_retires_the_old_identity() {
        let before = vec![entry("same.wav", "old-file", Some("same"), 4, 1)];
        let after = vec![entry("same.wav", "new-file", Some("same"), 4, 1)];

        let delta = build_committed_delta(&before, &after, 18);

        assert_eq!(delta.created.len(), 1);
        assert_eq!(delta.deleted.len(), 1);
        assert!(delta.changed.is_empty());
        assert!(delta.moved.is_empty());
    }
}
