use super::*;

#[test]
fn deleting_similarity_result_recomputes_filter_from_same_anchor() {
    let temp = tempdir().unwrap();
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("anchor.wav", Rating::NEUTRAL),
        sample_entry("close.wav", Rating::NEUTRAL),
        sample_entry("far.wav", Rating::NEUTRAL),
    ]);
    let trash_root = configure_test_trash(&mut controller, &temp);
    write_test_wav(&source.root.join("anchor.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("close.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("far.wav"), &[0.0, 0.1]);
    insert_similarity_embedding(&source, "anchor.wav", 1.0, 0.0);
    insert_similarity_embedding(&source, "close.wav", 0.9, 0.1);
    insert_similarity_embedding(&source, "far.wav", 0.7, 0.3);

    controller.find_similar_for_visible_row(0).unwrap();

    controller.delete_browser_samples(&[1]).unwrap();

    let query = controller
        .ui
        .browser
        .search
        .similar_query
        .as_ref()
        .expect("recomputed similarity query");
    assert_eq!(
        query.sample_id,
        analysis_jobs::build_sample_id(source.id.as_str(), Path::new("anchor.wav"))
    );
    assert_eq!(
        visible_browser_paths(&mut controller),
        vec![PathBuf::from("anchor.wav"), PathBuf::from("far.wav")]
    );
    assert!(trash_root.join("close.wav").exists());
}

#[test]
fn deleting_similarity_anchor_promotes_next_best_survivor() {
    let temp = tempdir().unwrap();
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("anchor.wav", Rating::NEUTRAL),
        sample_entry("close.wav", Rating::NEUTRAL),
        sample_entry("far.wav", Rating::NEUTRAL),
    ]);
    let trash_root = configure_test_trash(&mut controller, &temp);
    write_test_wav(&source.root.join("anchor.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("close.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("far.wav"), &[0.0, 0.1]);
    insert_similarity_embedding(&source, "anchor.wav", 1.0, 0.0);
    insert_similarity_embedding(&source, "close.wav", 0.9, 0.1);
    insert_similarity_embedding(&source, "far.wav", 0.7, 0.3);

    controller.find_similar_for_visible_row(0).unwrap();

    controller.delete_browser_samples(&[0]).unwrap();

    let query = controller
        .ui
        .browser
        .search
        .similar_query
        .as_ref()
        .expect("recomputed similarity query");
    assert_eq!(
        query.sample_id,
        analysis_jobs::build_sample_id(source.id.as_str(), Path::new("close.wav"))
    );
    assert_eq!(
        visible_browser_paths(&mut controller),
        vec![PathBuf::from("close.wav"), PathBuf::from("far.wav")]
    );
    assert_eq!(
        controller.focused_browser_path().as_deref(),
        Some(Path::new("close.wav"))
    );
    assert!(trash_root.join("anchor.wav").exists());
}
