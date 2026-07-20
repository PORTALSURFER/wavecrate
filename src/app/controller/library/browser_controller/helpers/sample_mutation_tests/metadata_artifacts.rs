use super::*;
#[test]
/// Auto-rename persists inferred sound type in the worker when the old DB row is missing it.
fn sample_auto_rename_persists_inferred_sound_type_without_controller_db_write() {
    let temp = tempdir().expect("create temp dir");
    let source = SampleSource::new(temp.path().join("source"));
    std::fs::create_dir_all(&source.root).expect("create source root");
    let relative = Path::new("mystery.wav");
    let absolute = source.root.join(relative);
    write_test_wav(&absolute, &[0.0, 0.1, -0.1]);
    let metadata = std::fs::metadata(&absolute).expect("read file metadata");
    let db =
        SourceDatabase::open_for_test_fixture_source_write(&source.root).expect("open source db");
    db.upsert_file(relative, metadata.len(), 0)
        .expect("insert db row");
    db.set_tag(relative, Rating::KEEP_3).expect("set tag");

    let result = run_sample_auto_rename_job(
        source.clone(),
        vec![SampleAutoRenameRequest {
            old_relative: relative.to_path_buf(),
            new_relative: PathBuf::from("portal_SS_kick.wav"),
            tag: Rating::KEEP_3,
            looped: false,
            locked: false,
            sound_type: Some(SampleSoundType::Kick),
            user_tag: None,
            tag_named: true,
            last_played_at: None,
            resume_playback: false,
            resume_looped: false,
            resume_start_override: None,
        }],
        Arc::new(AtomicBool::new(false)),
        None,
    );

    assert!(result.errors.is_empty());
    assert_eq!(result.renamed.len(), 1);
    let renamed = Path::new("portal_SS_kick.wav");
    let db =
        SourceDatabase::open_for_test_fixture_source_write(&source.root).expect("reopen source db");
    assert_eq!(
        db.sound_type_for_path(renamed).expect("renamed sound type"),
        Some(SampleSoundType::Kick)
    );
}

#[test]
fn sample_auto_rename_marks_already_matching_tag_named_path() {
    let (_temp, source) = setup_fixture(&["portal_SS_kick.wav"]);
    let relative = Path::new("portal_SS_kick.wav");
    let db =
        SourceDatabase::open_for_test_fixture_source_write(&source.root).expect("open source db");
    assert_eq!(
        db.tag_named_for_path(relative).expect("initial marker"),
        Some(false)
    );

    let result = run_sample_auto_rename_job(
        source.clone(),
        vec![SampleAutoRenameRequest {
            old_relative: relative.to_path_buf(),
            new_relative: relative.to_path_buf(),
            tag: Rating::KEEP_3,
            looped: true,
            locked: true,
            sound_type: Some(SampleSoundType::Kick),
            user_tag: Some(String::from("Vintage")),
            tag_named: true,
            last_played_at: Some(42),
            resume_playback: false,
            resume_looped: false,
            resume_start_override: None,
        }],
        Arc::new(AtomicBool::new(false)),
        None,
    );

    assert!(result.errors.is_empty());
    assert_eq!(result.renamed.len(), 1);
    assert_eq!(
        result.renamed[0].old_relative,
        result.renamed[0].new_relative
    );
    assert!(result.renamed[0].entry.tag_named);
    let db =
        SourceDatabase::open_for_test_fixture_source_write(&source.root).expect("reopen source db");
    assert_eq!(
        db.tag_named_for_path(relative).expect("updated marker"),
        Some(true)
    );
}

#[test]
fn repeated_sample_auto_rename_preserves_analysis_artifacts() {
    let (_temp, source) = setup_fixture(&["alpha.wav"]);
    let first = run_sample_auto_rename_job(
        source.clone(),
        vec![rename_request("alpha.wav", "alpha_renamed.wav")],
        Arc::new(AtomicBool::new(false)),
        None,
    );
    assert!(first.errors.is_empty());

    let second = run_sample_auto_rename_job(
        source.clone(),
        vec![rename_request("alpha_renamed.wav", "alpha_final.wav")],
        Arc::new(AtomicBool::new(false)),
        None,
    );
    assert!(second.errors.is_empty());

    let conn = rusqlite::Connection::open(source.root.join(DB_FILE_NAME)).expect("open sqlite");
    let old_sample_id = format!("{}::alpha.wav", source.id);
    let first_sample_id = format!("{}::alpha_renamed.wav", source.id);
    let final_sample_id = format!("{}::alpha_final.wav", source.id);
    for table in ["samples", "features", "embeddings", "analysis_jobs"] {
        assert_eq!(
            sample_id_count(&conn, table, &old_sample_id),
            0,
            "{table} retained old identity"
        );
        assert_eq!(
            sample_id_count(&conn, table, &first_sample_id),
            0,
            "{table} retained intermediate identity"
        );
        assert_eq!(
            sample_id_count(&conn, table, &final_sample_id),
            1,
            "{table} did not remap to final identity"
        );
    }
    let pending_jobs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM analysis_jobs WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(pending_jobs, 0);
    let job_relative: String = conn
        .query_row(
            "SELECT relative_path FROM analysis_jobs WHERE sample_id = ?1",
            [&final_sample_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(job_relative, "alpha_final.wav");
}
