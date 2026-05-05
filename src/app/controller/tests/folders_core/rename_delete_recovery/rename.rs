use super::super::support::*;
use crate::app::state::{InlineFolderEdit, InlineFolderEditKind};
use crate::sample_sources::{DB_FILE_NAME, SampleSoundType, SourceDatabase};
use std::time::Duration;

#[test]
fn renaming_folder_updates_entries_and_tree() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    let folder = source.root.join("old");
    std::fs::create_dir_all(&folder).unwrap();
    write_test_wav(&folder.join("clip.wav"), &[0.1, -0.1]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "old/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    controller.rename_folder(Path::new("old"), "new")?;

    assert!(!folder.exists());
    assert!(source.root.join("new/clip.wav").is_file());
    assert_eq!(
        controller.wav_entry(0).unwrap().relative_path,
        PathBuf::from("new/clip.wav")
    );
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == PathBuf::from("new"))
    );
    Ok(())
}

#[test]
fn renaming_folder_rolls_back_disk_move_when_db_rewrite_fails() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let folder = source.root.join("old");
    std::fs::create_dir_all(&folder).unwrap();
    write_test_wav(&folder.join("clip.wav"), &[0.1, -0.1]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "old/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    let (lock_release_tx, lock_done_rx) = lock_source_db_until_released(&source.root);

    controller.rename_folder(Path::new("old"), "new")?;
    let _ = lock_release_tx.send(());
    lock_done_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    assert!(source.root.join("old/clip.wav").is_file());
    assert!(!source.root.join("new").exists());
    assert_eq!(
        controller.wav_entry(0).unwrap().relative_path,
        PathBuf::from("old/clip.wav")
    );
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Failed to start database update")
    );
    let db = SourceDatabase::open(&source.root).unwrap();
    assert_eq!(db.count_files().unwrap(), 1);
    assert_eq!(
        db.tag_for_path(Path::new("old/clip.wav")).unwrap(),
        Some(crate::sample_sources::Rating::NEUTRAL)
    );
    assert!(
        db.tag_for_path(Path::new("new/clip.wav"))
            .unwrap()
            .is_none()
    );
    Ok(())
}

#[test]
fn renaming_folder_preserves_row_metadata_and_analysis_identity() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let folder = source.root.join("old");
    std::fs::create_dir_all(&folder).unwrap();
    write_test_wav(&folder.join("clip.wav"), &[0.1, -0.1]);
    let mut entry = sample_entry("old/clip.wav", crate::sample_sources::Rating::KEEP_1);
    entry.file_size = 123;
    entry.modified_ns = 456;
    entry.content_hash = Some("hash-a".into());
    entry.looped = true;
    entry.locked = true;
    entry.last_played_at = Some(42);
    entry.sound_type = Some(SampleSoundType::Fx);
    entry.user_tag = Some("Vintage FX".into());
    controller.set_wav_entries_for_tests(vec![entry]);
    insert_analysis_job(&source, Path::new("old/clip.wav"), "hash-a");
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    controller.rename_folder(Path::new("old"), "new")?;

    let renamed = controller.wav_entry(0).unwrap();
    assert_eq!(renamed.relative_path, PathBuf::from("new/clip.wav"));
    assert_eq!(renamed.content_hash.as_deref(), Some("hash-a"));
    assert!(renamed.looped);
    assert!(renamed.locked);
    assert_eq!(renamed.last_played_at, Some(42));
    assert_eq!(renamed.sound_type, Some(SampleSoundType::Fx));
    assert_eq!(renamed.user_tag.as_deref(), Some("Vintage FX"));
    let db = SourceDatabase::open(&source.root).unwrap();
    let rows = db.list_files().unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.relative_path, PathBuf::from("new/clip.wav"));
    assert_eq!(row.content_hash.as_deref(), Some("hash-a"));
    assert_eq!(row.tag, crate::sample_sources::Rating::KEEP_1);
    assert!(row.looped);
    assert!(row.locked);
    assert_eq!(row.last_played_at, Some(42));
    assert_eq!(row.sound_type, Some(SampleSoundType::Fx));
    assert_eq!(row.user_tag.as_deref(), Some("Vintage FX"));
    assert_analysis_job_remapped(&source, Path::new("new/clip.wav"));
    Ok(())
}

#[test]
fn start_folder_rename_creates_inline_edit_with_select_all() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let folder = source.root.join("folder");
    std::fs::create_dir_all(&folder).unwrap();
    write_test_wav(&folder.join("clip.wav"), &[0.1, -0.1]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "folder/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    let focus_row = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from("folder"))
        .unwrap();
    controller.focus_folder_row(focus_row);

    controller.start_folder_rename();

    let draft = controller.ui.sources.folders.inline_edit.as_ref().unwrap();
    assert!(matches!(
        draft.kind,
        InlineFolderEditKind::Rename { ref target } if target == &PathBuf::from("folder")
    ));
    assert_eq!(draft.name, "folder");
    assert!(draft.focus_requested);
    assert!(draft.select_all_on_focus_requested);
    Ok(())
}

#[test]
fn start_folder_rename_rejects_root_folder() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.refresh_folder_browser_for_tests();
    controller.focus_folder_row(0);

    controller.start_folder_rename();

    assert!(controller.ui.sources.folders.inline_edit.is_none());
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Info
    );
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Root folder cannot be renamed")
    );
}

#[test]
fn cancelling_folder_rename_clears_inline_edit() {
    let (mut controller, _source) = dummy_controller();
    controller.ui.sources.folders.inline_edit = Some(InlineFolderEdit {
        kind: InlineFolderEditKind::Rename {
            target: PathBuf::from("folder"),
        },
        name: "folder".into(),
        focus_requested: true,
        select_all_on_focus_requested: true,
    });

    controller.cancel_folder_rename();

    assert!(controller.ui.sources.folders.inline_edit.is_none());
}

#[test]
fn applying_pending_folder_rename_updates_tree_and_focus() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let folder = source.root.join("old");
    std::fs::create_dir_all(&folder).unwrap();
    write_test_wav(&folder.join("clip.wav"), &[0.1, -0.1]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "old/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    controller.ui.sources.folders.inline_edit = Some(InlineFolderEdit {
        kind: InlineFolderEditKind::Rename {
            target: PathBuf::from("old"),
        },
        name: "new".into(),
        focus_requested: true,
        select_all_on_focus_requested: true,
    });

    assert!(controller.apply_pending_folder_rename());

    assert!(controller.ui.sources.folders.inline_edit.is_none());
    let focused = controller
        .ui
        .sources
        .folders
        .focused
        .expect("focused row after rename");
    assert_eq!(
        controller.ui.sources.folders.rows[focused].path,
        PathBuf::from("new")
    );
    assert!(source.root.join("new/clip.wav").is_file());
    Ok(())
}

fn lock_source_db_until_released(
    source_root: &Path,
) -> (std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>) {
    let (lock_release_tx, lock_release_rx) = std::sync::mpsc::channel();
    let (lock_done_tx, lock_done_rx) = std::sync::mpsc::channel();
    let (locked_tx, locked_rx) = std::sync::mpsc::channel();
    let db_file = source_root.join(DB_FILE_NAME);
    std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(db_file).unwrap();
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        let _ = locked_tx.send(());
        let _ = lock_release_rx.recv();
        let _ = conn.execute_batch("COMMIT");
        let _ = lock_done_tx.send(());
    });
    locked_rx.recv().unwrap();
    (lock_release_tx, lock_done_rx)
}

fn insert_analysis_job(
    source: &crate::sample_sources::SampleSource,
    relative_path: &Path,
    content_hash: &str,
) {
    let db_path = source.root.join(DB_FILE_NAME);
    let conn = rusqlite::Connection::open(db_path).unwrap();
    let sample_id = crate::app::controller::library::analysis_jobs::db::build_sample_id(
        source.id.as_str(),
        relative_path,
    );
    conn.execute(
        "INSERT INTO analysis_jobs (
             sample_id, source_id, relative_path, job_type, content_hash, status, attempts, created_at
         ) VALUES (?1, ?2, ?3, 'analyze_sample', ?4, 'done', 1, 7)",
        rusqlite::params![
            sample_id,
            source.id.as_str(),
            relative_path.to_string_lossy(),
            content_hash
        ],
    )
    .unwrap();
}

fn assert_analysis_job_remapped(
    source: &crate::sample_sources::SampleSource,
    relative_path: &Path,
) {
    let db_path = source.root.join(DB_FILE_NAME);
    let conn = rusqlite::Connection::open(db_path).unwrap();
    let sample_id = crate::app::controller::library::analysis_jobs::db::build_sample_id(
        source.id.as_str(),
        relative_path,
    );
    let (stored_sample_id, stored_relative_path): (String, String) = conn
        .query_row(
            "SELECT sample_id, relative_path FROM analysis_jobs",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(stored_sample_id, sample_id);
    assert_eq!(
        stored_relative_path,
        relative_path.to_string_lossy().replace('\\', "/")
    );
}
