use std::path::{Path, PathBuf};

use rusqlite::params;
use tempfile::tempdir;

use super::super::SourceDatabase;

#[test]
fn list_files_page_orders_supported_audio_and_applies_offsets() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    for name in [
        "delta.wav",
        "alpha.wav",
        "notes.txt",
        "charlie.wav",
        "bravo.wav",
    ] {
        db.upsert_file(Path::new(name), 10, 5).unwrap();
    }

    let page = db.list_files_page(2, 1).unwrap();
    let paths = page
        .into_iter()
        .map(|entry| entry.relative_path)
        .collect::<Vec<_>>();

    assert_eq!(
        paths,
        vec![PathBuf::from("bravo.wav"), PathBuf::from("charlie.wav")]
    );
}

#[test]
fn list_queries_skip_invalid_relative_paths() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.upsert_file(Path::new("valid.wav"), 10, 5).unwrap();
    db.set_missing(Path::new("valid.wav"), true).unwrap();
    db.connection
        .execute(
            "INSERT INTO wav_files (path, file_size, modified_ns, content_hash, tag, looped, locked, missing, extension)
             VALUES (?1, 1, 1, NULL, 0, 0, 0, 1, 'wav')",
            params!["../escape.wav"],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO wav_files (path, file_size, modified_ns, content_hash, tag, looped, locked, missing, extension)
             VALUES (?1, 1, 1, NULL, 0, 0, 0, 1, 'wav')",
            params!["C:/absolute.wav"],
        )
        .unwrap();

    let listed = db.list_files().unwrap();
    let missing = db.list_missing_paths().unwrap();

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].relative_path, PathBuf::from("valid.wav"));
    assert_eq!(missing, vec![PathBuf::from("valid.wav")]);
}

#[test]
fn bpm_queries_return_only_present_rows_and_preserve_null_values() {
    let dir = tempdir().unwrap();
    let db = SourceDatabase::open(dir.path()).unwrap();
    db.connection
        .execute(
            "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, bpm)
             VALUES (?1, 'h1', 1, 1, ?2)",
            params!["source::one.wav", 124.0f64],
        )
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, bpm)
             VALUES (?1, 'h2', 1, 1, NULL)",
            params!["source::two.wav"],
        )
        .unwrap();

    assert_eq!(
        db.bpm_for_sample_id("source::one.wav").unwrap(),
        Some(124.0)
    );
    assert_eq!(db.bpm_for_sample_id("source::two.wav").unwrap(), None);
    assert_eq!(db.bpm_for_sample_id("source::missing.wav").unwrap(), None);

    let lookup = db
        .bpms_for_sample_ids(&[
            String::from("source::one.wav"),
            String::from("source::two.wav"),
            String::from("source::missing.wav"),
        ])
        .unwrap();

    assert_eq!(lookup.get("source::one.wav"), Some(&Some(124.0)));
    assert_eq!(lookup.get("source::two.wav"), Some(&None));
    assert!(!lookup.contains_key("source::missing.wav"));
    assert!(db.bpms_for_sample_ids(&[]).unwrap().is_empty());
}
