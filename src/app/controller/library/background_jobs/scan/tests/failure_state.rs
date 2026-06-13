use super::*;

#[test]
fn canceled_scan_clears_similarity_prep_and_reports_warning_for_selected_source() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.runtime.similarity.prep = Some(SimilarityPrepState {
        source_id: source.id.clone(),
        stage: SimilarityPrepStage::AwaitScan,
        umap_version: "v1".to_string(),
        scan_completed_at: None,
        skip_backfill: false,
        force_full_analysis: false,
    });
    controller.show_status_progress(ProgressTaskKind::Analysis, "Preparing similarity", 1, true);

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Quick,
            ScanKind::Manual,
            Err(ScanError::Canceled),
        ),
    );

    assert_eq!(controller.ui.status.text, "Quick sync canceled");
    assert!(controller.runtime.similarity.prep.is_none());
    assert_eq!(controller.ui.progress.task, None);
    assert!(!controller.ui.progress.visible);
}

#[test]
fn failed_scan_clears_similarity_prep_and_reports_error_for_selected_source() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.runtime.similarity.prep = Some(SimilarityPrepState {
        source_id: source.id.clone(),
        stage: SimilarityPrepStage::AwaitScan,
        umap_version: "v1".to_string(),
        scan_completed_at: None,
        skip_backfill: false,
        force_full_analysis: false,
    });

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Hard,
            ScanKind::Manual,
            Err(ScanError::InvalidRoot(PathBuf::from("missing"))),
        ),
    );

    assert!(controller.ui.status.text.starts_with("Hard sync failed: "));
    assert!(controller.runtime.similarity.prep.is_none());
}
