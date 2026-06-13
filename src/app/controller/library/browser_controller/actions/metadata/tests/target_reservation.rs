use super::*;

#[test]
fn resolve_auto_rename_target_skips_existing_and_reserved_names() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    write_test_wav(&source.root.join("artistname_SS_kick.wav"), &[0.0]);
    write_test_wav(&source.root.join("artistname_SS_kick_001.wav"), &[0.0]);

    let browser = BrowserController::new(&mut controller);
    let mut reserved_targets = HashSet::from([PathBuf::from("artistname_SS_kick_002.wav")]);
    let resolved = browser
        .resolve_auto_rename_target(
            &source.root,
            Path::new("raw.wav"),
            Some("artistname_SS_kick"),
            "artistname",
            &mut reserved_targets,
        )
        .expect("target resolution should succeed");

    assert_eq!(resolved, PathBuf::from("artistname_SS_kick_003.wav"));
    assert!(reserved_targets.contains(&resolved));
}
