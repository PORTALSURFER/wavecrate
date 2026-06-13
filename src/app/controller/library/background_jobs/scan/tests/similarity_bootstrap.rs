use super::*;

#[test]
fn unchanged_scan_finishes_similarity_prep_with_explicit_bootstrap_enqueue() {
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
            Ok(ScanStats::default()),
        ),
    );

    let prep = controller
        .runtime
        .similarity
        .prep
        .as_ref()
        .expect("similarity prep");
    assert!(matches!(
        prep.stage,
        SimilarityPrepStage::AwaitEmbeddings | SimilarityPrepStage::Finalizing
    ));
    assert!(!prep.skip_backfill);
    assert_eq!(
        controller.ui.progress.task,
        Some(ProgressTaskKind::Analysis)
    );
    assert!(controller.ui.progress.visible);
    match wait_for_analysis_message(&mut controller, |message| {
        matches!(message, AnalysisJobMessage::EnqueueFinished { .. })
    }) {
        AnalysisJobMessage::EnqueueFinished { announce: true, .. } => {}
        other => panic!("unexpected analysis message: {other:?}"),
    }
}
