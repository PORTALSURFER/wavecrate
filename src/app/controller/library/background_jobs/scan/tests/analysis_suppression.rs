use super::*;

#[test]
fn unchanged_scan_stays_analysis_free_when_similarity_prep_is_idle() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).expect("cache db");
    let wav_path = source.root.join("snare.wav");
    write_test_wav(&wav_path, &[0.3, -0.3, 0.2, -0.2]);
    controller
        .ensure_sample_db_entry(&source, Path::new("snare.wav"))
        .expect("sample db entry");

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Hard,
            ScanKind::Manual,
            Ok(ScanStats::default()),
        ),
    );

    assert!(controller.wav_entries.source_id.as_ref() == Some(&source.id));
    assert_eq!(controller.wav_entries.total, 1);
    assert_no_analysis_message(&mut controller);
    assert_no_analysis_jobs_inserted(&source);
}

#[test]
fn auto_changed_scan_refreshes_selected_source_without_enqueueing_analysis() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).expect("cache db");
    let wav_path = source.root.join("kick.wav");
    write_test_wav(&wav_path, &[0.1, -0.1, 0.2, -0.2]);
    controller
        .ensure_sample_db_entry(&source, Path::new("kick.wav"))
        .expect("sample db entry");

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Quick,
            ScanKind::Auto,
            Ok(ScanStats {
                added: 1,
                changed_samples: vec![changed_sample("kick.wav")],
                ..ScanStats::default()
            }),
        ),
    );

    assert_eq!(controller.ui.progress.task, None);
    assert_no_analysis_message(&mut controller);
    assert_no_analysis_jobs_inserted(&source);
}

#[test]
fn auto_unchanged_scan_does_not_backfill_analysis() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).expect("cache db");
    let wav_path = source.root.join("snare.wav");
    write_test_wav(&wav_path, &[0.3, -0.3, 0.2, -0.2]);
    controller
        .ensure_sample_db_entry(&source, Path::new("snare.wav"))
        .expect("sample db entry");

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Quick,
            ScanKind::Auto,
            Ok(ScanStats::default()),
        ),
    );

    assert_no_analysis_message(&mut controller);
    assert_no_analysis_jobs_inserted(&source);
}
