use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use wavecrate::sample_sources::SourceDatabase;

use super::command::{CollectionOperation, CollectionUpdate};

pub(super) fn group_updates_by_source(
    updates: &[CollectionUpdate],
) -> BTreeMap<PathBuf, Vec<CollectionUpdate>> {
    let mut by_source: BTreeMap<PathBuf, Vec<CollectionUpdate>> = BTreeMap::new();
    for update in updates {
        by_source
            .entry(update.root.clone())
            .or_default()
            .push(update.clone());
    }
    by_source
}

pub(super) fn persist_collection_updates(
    root: &Path,
    updates: &[CollectionUpdate],
) -> Result<(), String> {
    let db = SourceDatabase::open_for_user_metadata_write(root).map_err(|err| err.to_string())?;
    let mut batch = db.write_batch().map_err(|err| err.to_string())?;
    for update in updates {
        let (file_size, modified_ns) = file_metadata(&update.absolute_path)?;
        batch
            .upsert_file(&update.relative_path, file_size, modified_ns)
            .map_err(|err| err.to_string())?;
        match update.operation {
            CollectionOperation::Add => batch
                .add_collection(&update.relative_path, update.collection)
                .map_err(|err| err.to_string())?,
            CollectionOperation::Remove => batch
                .remove_collection(&update.relative_path, update.collection)
                .map_err(|err| err.to_string())?,
        }
    }
    batch.commit().map_err(|err| err.to_string())
}

fn file_metadata(path: &Path) -> Result<(u64, i64), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let modified_ns = metadata
        .modified()
        .map_err(|err| format!("Missing modified time for {}: {err}", path.display()))?
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map_err(|_| String::from("File modified time is before epoch"))?
        .as_nanos() as i64;
    Ok((metadata.len(), modified_ns))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wavecrate::sample_sources::SampleCollection;

    fn update(root: &str, relative_path: &str) -> CollectionUpdate {
        CollectionUpdate {
            root: PathBuf::from(root),
            relative_path: PathBuf::from(relative_path),
            absolute_path: PathBuf::from(root).join(relative_path),
            collection: SampleCollection::new(0).expect("collection"),
            operation: CollectionOperation::Add,
        }
    }

    #[test]
    fn group_updates_by_source_preserves_per_source_order() {
        let updates = vec![
            update("C:/one", "a.wav"),
            update("C:/two", "b.wav"),
            update("C:/one", "c.wav"),
        ];

        let grouped = group_updates_by_source(&updates);

        assert_eq!(
            grouped
                .get(&PathBuf::from("C:/one"))
                .expect("first source")
                .iter()
                .map(|update| update.relative_path.as_path())
                .collect::<Vec<_>>(),
            vec![Path::new("a.wav"), Path::new("c.wav")]
        );
        assert_eq!(
            grouped
                .get(&PathBuf::from("C:/two"))
                .expect("second source")
                .iter()
                .map(|update| update.relative_path.as_path())
                .collect::<Vec<_>>(),
            vec![Path::new("b.wav")]
        );
    }
}
