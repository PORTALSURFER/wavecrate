use super::*;
use rusqlite::params;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::sample_sources::{Rating, SourceDatabase};

struct MoveRecoveryFixture {
    _temp: TempDir,
    source_root: PathBuf,
    target_root: PathBuf,
    source_db: SourceDatabase,
    target_db: SourceDatabase,
    source_relative: PathBuf,
    target_relative: PathBuf,
    staged_relative: PathBuf,
    entry: FileOpJournalEntry,
}

impl MoveRecoveryFixture {
    fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let source_root = temp.path().join("source");
        let target_root = temp.path().join("target");
        std::fs::create_dir_all(&source_root).unwrap();
        std::fs::create_dir_all(&target_root).unwrap();
        let source_db = SourceDatabase::open(&source_root).unwrap();
        let target_db = SourceDatabase::open(&target_root).unwrap();
        let source_relative = PathBuf::from("one.wav");
        let source_absolute = source_root.join(&source_relative);
        write_wav(&source_absolute);
        source_db.upsert_file(&source_relative, 16, 1).unwrap();
        source_db.set_tag(&source_relative, Rating::KEEP_1).unwrap();
        source_db.set_looped(&source_relative, true).unwrap();
        source_db.set_last_played_at(&source_relative, 123).unwrap();
        let target_relative = PathBuf::from("moved.wav");
        let staged_relative = staged_relative_for_target(&target_relative, "test").unwrap();
        let entry = FileOpJournalEntry::new_move(
            String::from("move-test"),
            MoveJournalEntryInit {
                source_root: source_root.clone(),
                source_relative: source_relative.clone(),
                target_relative: target_relative.clone(),
                staged_relative: staged_relative.clone(),
                tag: Rating::KEEP_1,
                looped: true,
                last_played_at: Some(123),
            },
        )
        .unwrap();
        insert_entry(&target_db, &entry).unwrap();
        Self {
            _temp: temp,
            source_root,
            target_root,
            source_db,
            target_db,
            source_relative,
            target_relative,
            staged_relative,
            entry,
        }
    }

    fn source_absolute(&self) -> PathBuf {
        self.source_root.join(&self.source_relative)
    }

    fn target_absolute(&self) -> PathBuf {
        self.target_root.join(&self.target_relative)
    }

    fn staged_absolute(&self) -> PathBuf {
        self.target_root.join(&self.staged_relative)
    }

    fn stage_target_db(&self) {
        update_stage(
            &self.target_db,
            &self.entry.id,
            FileOpStage::TargetDb,
            Some(16),
            Some(1),
        )
        .unwrap();
    }

    fn stage_source_db(&self) {
        update_stage(
            &self.target_db,
            &self.entry.id,
            FileOpStage::SourceDb,
            Some(16),
            Some(1),
        )
        .unwrap();
    }
}

fn write_wav(path: &Path) {
    std::fs::write(path, [0u8; 16]).unwrap();
}

fn assert_no_journal_entries(db: &SourceDatabase) {
    let listed = list_entries(db).unwrap();
    assert!(listed.entries.is_empty());
    assert!(listed.malformed.is_empty());
}

fn assert_target_metadata_preserved(fixture: &MoveRecoveryFixture) {
    assert_eq!(
        fixture
            .target_db
            .tag_for_path(&fixture.target_relative)
            .unwrap(),
        Some(Rating::KEEP_1)
    );
    assert_eq!(
        fixture
            .target_db
            .looped_for_path(&fixture.target_relative)
            .unwrap(),
        Some(true)
    );
    assert_eq!(
        fixture
            .target_db
            .last_played_at_for_path(&fixture.target_relative)
            .unwrap(),
        Some(123)
    );
}

#[test]
fn reconcile_move_from_staged_file() {
    let fixture = MoveRecoveryFixture::new();
    std::fs::rename(fixture.source_absolute(), fixture.staged_absolute()).unwrap();
    update_stage(
        &fixture.target_db,
        &fixture.entry.id,
        FileOpStage::Staged,
        Some(16),
        Some(1),
    )
    .unwrap();

    let summary = reconcile_pending_ops(&fixture.target_db).unwrap();
    assert_eq!(summary.completed, 1);
    assert!(!fixture.staged_absolute().exists());
    assert!(fixture.target_absolute().exists());
    assert!(
        fixture
            .source_db
            .tag_for_path(&fixture.source_relative)
            .unwrap()
            .is_none()
    );
    assert_target_metadata_preserved(&fixture);
    assert_no_journal_entries(&fixture.target_db);
}

#[test]
fn reconcile_same_source_move_from_staged_file() {
    let temp = TempDir::new().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();
    let db = SourceDatabase::open(&source_root).unwrap();
    let source_relative = PathBuf::from("one.wav");
    let source_absolute = source_root.join(&source_relative);
    write_wav(&source_absolute);
    db.upsert_file(&source_relative, 16, 1).unwrap();
    db.set_tag(&source_relative, Rating::KEEP_1).unwrap();
    db.set_looped(&source_relative, true).unwrap();
    db.set_last_played_at(&source_relative, 123).unwrap();
    let target_relative = PathBuf::from("moved.wav");
    let staged_relative = staged_relative_for_target(&target_relative, "test").unwrap();
    let entry = FileOpJournalEntry::new_move(
        String::from("move-test"),
        MoveJournalEntryInit {
            source_root: source_root.clone(),
            source_relative: source_relative.clone(),
            target_relative: target_relative.clone(),
            staged_relative: staged_relative.clone(),
            tag: Rating::KEEP_1,
            looped: true,
            last_played_at: Some(123),
        },
    )
    .unwrap();
    insert_entry(&db, &entry).unwrap();
    let staged_absolute = source_root.join(&staged_relative);
    std::fs::rename(&source_absolute, &staged_absolute).unwrap();
    update_stage(&db, &entry.id, FileOpStage::Staged, Some(16), Some(1)).unwrap();

    let summary = reconcile_pending_ops(&db).unwrap();
    assert_eq!(summary.completed, 1);
    assert!(!staged_absolute.exists());
    assert!(source_root.join(&target_relative).exists());
    assert!(db.tag_for_path(&source_relative).unwrap().is_none());
    assert_eq!(
        db.tag_for_path(&target_relative).unwrap(),
        Some(Rating::KEEP_1)
    );
    assert_eq!(db.looped_for_path(&target_relative).unwrap(), Some(true));
    assert_eq!(
        db.last_played_at_for_path(&target_relative).unwrap(),
        Some(123)
    );
    assert_no_journal_entries(&db);
}

#[test]
fn reconcile_copy_from_staged_file() {
    let temp = TempDir::new().unwrap();
    let target_root = temp.path().join("target");
    std::fs::create_dir_all(&target_root).unwrap();
    let target_db = SourceDatabase::open(&target_root).unwrap();
    let source_path = temp.path().join("external.wav");
    write_wav(&source_path);
    let target_relative = PathBuf::from("copied.wav");
    let staged_relative = staged_relative_for_target(&target_relative, "copy").unwrap();
    let entry = FileOpJournalEntry::new_copy(
        String::from("copy-test"),
        target_relative.clone(),
        staged_relative.clone(),
    )
    .unwrap();
    insert_entry(&target_db, &entry).unwrap();
    let staged_absolute = target_root.join(&staged_relative);
    std::fs::copy(&source_path, &staged_absolute).unwrap();
    update_stage(
        &target_db,
        &entry.id,
        FileOpStage::Staged,
        Some(16),
        Some(1),
    )
    .unwrap();

    let summary = reconcile_pending_ops(&target_db).unwrap();
    assert_eq!(summary.completed, 1);
    assert!(target_root.join(&target_relative).exists());
    assert!(target_db.tag_for_path(&target_relative).unwrap().is_some());
    assert_no_journal_entries(&target_db);
}

#[test]
fn reconcile_target_db_stage_removes_orphaned_source_row() {
    let fixture = MoveRecoveryFixture::new();
    std::fs::rename(fixture.source_absolute(), fixture.target_absolute()).unwrap();
    fixture.stage_target_db();

    let summary = reconcile_pending_ops(&fixture.target_db).unwrap();
    assert_eq!(summary.completed, 1);
    assert!(fixture.target_absolute().exists());
    assert!(
        fixture
            .source_db
            .tag_for_path(&fixture.source_relative)
            .unwrap()
            .is_none()
    );
    assert_target_metadata_preserved(&fixture);
    assert_no_journal_entries(&fixture.target_db);
}

#[test]
fn reconcile_source_db_stage_is_idempotent_after_move_completion() {
    let fixture = MoveRecoveryFixture::new();
    std::fs::rename(fixture.source_absolute(), fixture.target_absolute()).unwrap();
    fixture
        .target_db
        .upsert_file(&fixture.target_relative, 16, 1)
        .unwrap();
    fixture
        .target_db
        .set_tag(&fixture.target_relative, Rating::KEEP_1)
        .unwrap();
    fixture
        .target_db
        .set_looped(&fixture.target_relative, true)
        .unwrap();
    fixture
        .target_db
        .set_last_played_at(&fixture.target_relative, 123)
        .unwrap();
    fixture
        .source_db
        .remove_file(&fixture.source_relative)
        .unwrap();
    fixture.stage_source_db();

    let summary = reconcile_pending_ops(&fixture.target_db).unwrap();
    assert_eq!(summary.completed, 1);
    assert!(fixture.target_absolute().exists());
    assert_target_metadata_preserved(&fixture);
    assert_no_journal_entries(&fixture.target_db);
}

#[test]
fn reconcile_missing_staged_file_keeps_original_source_when_target_missing() {
    let fixture = MoveRecoveryFixture::new();
    update_stage(
        &fixture.target_db,
        &fixture.entry.id,
        FileOpStage::Staged,
        Some(16),
        Some(1),
    )
    .unwrap();
    fixture
        .target_db
        .upsert_file(&fixture.target_relative, 16, 1)
        .unwrap();

    let summary = reconcile_pending_ops(&fixture.target_db).unwrap();
    assert_eq!(summary.completed, 1);
    assert!(fixture.source_absolute().exists());
    assert!(!fixture.target_absolute().exists());
    assert_eq!(
        fixture
            .source_db
            .tag_for_path(&fixture.source_relative)
            .unwrap(),
        Some(Rating::KEEP_1)
    );
    assert!(
        fixture
            .target_db
            .tag_for_path(&fixture.target_relative)
            .unwrap()
            .is_none()
    );
    assert_no_journal_entries(&fixture.target_db);
}

#[test]
fn reconcile_reports_and_drops_malformed_journal_rows() {
    let temp = TempDir::new().unwrap();
    let target_root = temp.path().join("target");
    std::fs::create_dir_all(&target_root).unwrap();
    let target_db = SourceDatabase::open(&target_root).unwrap();
    target_db
        .connection
        .execute(
            "INSERT INTO file_ops_journal (
                id, op_type, stage, source_root, source_relative, target_relative,
                staged_relative, file_size, modified_ns, tag, looped, last_played_at, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                "bad-row",
                "move",
                "intent",
                Option::<String>::None,
                Option::<String>::None,
                "/absolute.wav",
                Option::<String>::None,
                Option::<i64>::None,
                Option::<i64>::None,
                Option::<i64>::None,
                Option::<i64>::None,
                Option::<i64>::None,
                1i64,
            ],
        )
        .unwrap();

    let summary = reconcile_pending_ops(&target_db).unwrap();
    assert_eq!(summary.total, 1);
    assert_eq!(summary.completed, 0);
    assert_eq!(summary.errors.len(), 1);
    assert!(summary.errors[0].contains("bad-row"));
    assert!(summary.errors[0].contains("dropped malformed journal row"));
    let entry_count = target_db
        .connection
        .query_row(
            "SELECT COUNT(*) FROM file_ops_journal",
            [],
            |row: &rusqlite::Row<'_>| row.get::<_, i64>(0),
        )
        .unwrap();
    assert_eq!(entry_count, 0);
}
