#[test]
fn foreground_scan_admission_does_not_retain_database_writer() {
    let (_directory, source) = unhashed_source("database-waiter");
    let shared = Arc::new(Shared::new(vec![source.clone()], None));
    let database_guard = shared.database_writer.lock(DatabasePhase::Publish);
    let generation = shared.control().source_lifecycle_generations[source.id.as_str()];
    let states = Arc::new(Mutex::new(Vec::new()));
    let source_id = source.id.to_string();
    let permit = SourceProcessingBudgetHandle {
        shared: Arc::clone(&shared),
    }
    .acquire_scan_for_generation_with_state(&source_id, generation, |state| {
        states.lock().unwrap().push(state);
    })
    .expect("scan admission must not wait for database access");
    assert_eq!(
        states.lock().unwrap().as_slice(),
        [SourceScanAdmissionState::Admitted]
    );

    let writer = permit.scan_writer();
    let waiting = thread::spawn(move || writer.lock(DatabasePhase::ScanManifest));
    wait_until(Duration::from_secs(2), || {
        shared.database_writer.waiting_count() == 1
    });
    drop(database_guard);
    drop(waiting.join().expect("scan writer acquires after publication"));
    drop(permit);
}

#[test]
fn foreground_scan_traversal_does_not_block_independent_publication() {
    let directory = tempfile::tempdir().expect("scan source directory");
    let publication_directory = tempfile::tempdir().expect("publication source directory");
    for index in 0..128 {
        std::fs::write(
            directory.path().join(format!("sample-{index:03}.wav")),
            [index as u8; 16],
        )
        .expect("write source file");
    }
    SourceDatabase::open_for_scan(directory.path()).expect("create source database");
    let scan_source = SampleSource::new_with_id(
        SourceId::from_string("slow-scan-source"),
        directory.path().to_path_buf(),
    );
    let publication_source = SampleSource::new_with_id(
        SourceId::from_string("independent-publication-source"),
        publication_directory.path().to_path_buf(),
    );
    let shared = Arc::new(Shared::new(
        vec![scan_source.clone(), publication_source.clone()],
        None,
    ));
    let scan_permit = SourceProcessingBudgetHandle {
        shared: Arc::clone(&shared),
    }
    .acquire_scan(scan_source.id.as_str())
    .expect("admit foreground scan");
    let publication_budget = shared
        .budgets()
        .try_acquire(
            publication_source.id.as_str(),
            ProcessingLane::FeatureAnalysis,
        )
        .expect("independent feature work retains bounded execution capacity");
    let writer = scan_permit.scan_writer();
    let scan_writer = writer.clone();
    let root = directory.path().to_path_buf();
    let (traversal_started, wait_for_traversal) = std::sync::mpsc::channel();
    let (release_traversal, traversal_release) = std::sync::mpsc::channel();
    let scan = thread::spawn(move || {
        let database = SourceDatabase::open_for_scan(&root).expect("open source database");
        let mut traversal_started = Some(traversal_started);
        wavecrate_scan::sample_sources::scanner::scan_with_progress_and_writer(
            &database,
            wavecrate_scan::sample_sources::scanner::ScanMode::Quick,
            None,
            &mut |_, _| {
                if let Some(started) = traversal_started.take() {
                    started.send(()).expect("publish traversal start");
                    traversal_release.recv().expect("release slow traversal");
                }
            },
            &scan_writer,
        )
        .expect("complete coordinated scan");
    });
    wait_for_traversal
        .recv_timeout(Duration::from_secs(2))
        .expect("scan reaches filesystem traversal");

    let publication_writer = writer.clone();
    let (publication_finished, wait_for_publication) = std::sync::mpsc::channel();
    let publication = thread::spawn(move || {
        drop(publication_writer.lock(DatabasePhase::Publish));
        publication_finished
            .send(())
            .expect("publish completion signal");
    });
    wait_for_publication
        .recv_timeout(Duration::from_secs(1))
        .expect("publication must not wait for filesystem traversal");

    release_traversal
        .send(())
        .expect("release filesystem traversal");
    publication.join().expect("publication worker joins");
    scan.join().expect("scan worker joins");
    assert!(writer.snapshot().scan_manifest.count > 0);
    shared.budgets().release(publication_budget);
    drop(scan_permit);
}
