use super::*;

#[test]
fn recover_skips_unjournaled_restore_when_delete_journal_is_unreadable() {
    let (_temp, source) = sample_source();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let staged = staging_root.join("gone");
    fs::create_dir_all(&staged).unwrap();
    fs::write(staging_root.join("delete_journal.json"), b"{broken").unwrap();

    let report = recover_staged_deletes(std::slice::from_ref(&source));

    assert!(!source.root.join("gone").exists());
    assert!(staged.is_dir());
    assert!(report.entries.is_empty());
    assert!(report.retained_entries.is_empty());
    assert!(report.scan_sources.is_empty());
    assert_eq!(report.errors.len(), 1);
    assert!(report.errors[0].contains("Failed to read delete journal"));
    assert!(report.errors[0].contains("leaving staged deletes untouched"));
}
