/// UI-state/result application routines after move workers complete.
mod apply_result;
/// Shared journal rollback/cleanup helpers for move workers.
mod journal;
/// Drag/drop planning and validation entrypoints for folder and sample moves.
mod plan;
/// Background worker tasks that execute filesystem/database move operations.
mod worker;

#[cfg(test)]
mod tests {
    use super::super::super::file_metadata;
    use super::worker::{run_folder_move_task, run_folder_sample_move_task};
    use crate::app::controller::jobs::{FolderMoveRequest, FolderSampleMoveRequest};
    use crate::app::controller::test_support::write_test_wav;
    use crate::sample_sources::{Rating, SampleSource, SourceDatabase};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, atomic::AtomicBool};
    use tempfile::tempdir;

    #[test]
    fn folder_sample_move_updates_db_entry() {
        let temp = tempdir().unwrap();
        let source_root = temp.path().join("source");
        let target_dir = source_root.join("folder");
        std::fs::create_dir_all(&target_dir).unwrap();
        let source = SampleSource::new(source_root.clone());
        let wav_path = source_root.join("one.wav");
        write_test_wav(&wav_path, &[0.0, 0.1, -0.1]);
        let (file_size, modified_ns) = file_metadata(&wav_path).unwrap();
        let db = SourceDatabase::open(&source_root).unwrap();
        let mut batch = db.write_batch().unwrap();
        batch
            .upsert_file(Path::new("one.wav"), file_size, modified_ns)
            .unwrap();
        batch.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();
        batch.set_looped(Path::new("one.wav"), true).unwrap();
        batch.set_last_played_at(Path::new("one.wav"), 42).unwrap();
        batch.commit().unwrap();

        let request = FolderSampleMoveRequest {
            relative_path: PathBuf::from("one.wav"),
            target_relative: PathBuf::from("folder/one.wav"),
        };
        let result = run_folder_sample_move_task(
            source.id.clone(),
            source_root.clone(),
            vec![request],
            Vec::new(),
            Arc::new(AtomicBool::new(false)),
            None,
        );

        assert!(result.errors.is_empty());
        assert_eq!(result.moved.len(), 1);
        assert!(source_root.join("folder/one.wav").is_file());

        let db = SourceDatabase::open(&source_root).unwrap();
        assert!(db.tag_for_path(Path::new("one.wav")).unwrap().is_none());
        assert_eq!(
            db.tag_for_path(Path::new("folder/one.wav")).unwrap(),
            Some(Rating::KEEP_1)
        );
        assert_eq!(
            db.looped_for_path(Path::new("folder/one.wav")).unwrap(),
            Some(true)
        );
        assert_eq!(
            db.last_played_at_for_path(Path::new("folder/one.wav"))
                .unwrap(),
            Some(42)
        );
    }

    #[test]
    fn folder_move_updates_db_entries() {
        let temp = tempdir().unwrap();
        let source_root = temp.path().join("source");
        let old_dir = source_root.join("old");
        let target_dir = source_root.join("dest");
        std::fs::create_dir_all(&old_dir).unwrap();
        std::fs::create_dir_all(&target_dir).unwrap();
        let source = SampleSource::new(source_root.clone());
        let wav_path = old_dir.join("one.wav");
        write_test_wav(&wav_path, &[0.0, 0.1, -0.1]);
        let (file_size, modified_ns) = file_metadata(&wav_path).unwrap();
        let db = SourceDatabase::open(&source_root).unwrap();
        let mut batch = db.write_batch().unwrap();
        batch
            .upsert_file(Path::new("old/one.wav"), file_size, modified_ns)
            .unwrap();
        batch
            .set_tag(Path::new("old/one.wav"), Rating::KEEP_1)
            .unwrap();
        batch.commit().unwrap();

        let request = FolderMoveRequest {
            source_id: source.id.clone(),
            source_root: source_root.clone(),
            folder: PathBuf::from("old"),
            target_folder: PathBuf::from("dest"),
        };
        let result = run_folder_move_task(request, Arc::new(AtomicBool::new(false)), None);

        assert!(result.errors.is_empty());
        assert_eq!(result.moved.len(), 1);
        assert!(source_root.join("dest/old/one.wav").is_file());

        let db = SourceDatabase::open(&source_root).unwrap();
        assert!(db.tag_for_path(Path::new("old/one.wav")).unwrap().is_none());
        assert_eq!(
            db.tag_for_path(Path::new("dest/old/one.wav")).unwrap(),
            Some(Rating::KEEP_1)
        );
    }
}
