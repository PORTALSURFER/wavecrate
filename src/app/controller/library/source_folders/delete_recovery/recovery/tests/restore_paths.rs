use super::*;

#[test]
fn unique_restore_path_avoids_collisions() {
    let dir = tempdir().unwrap();
    let original = dir.path().join("folder");
    fs::create_dir_all(&original).unwrap();
    let (target, detail) = unique_restore_path(&original);
    assert_ne!(target, original);
    assert!(detail.is_some());
}

#[test]
fn recover_uses_restore_suffix_when_original_exists() -> Result<(), String> {
    let (_temp, source) = sample_source();
    let original = source.root.join("gone");
    fs::create_dir_all(&original).unwrap();
    let staging_root = source.root.join(DELETE_STAGING_DIR);
    let _staged = stage_folder_for_delete(&original, &staging_root, Path::new("gone"), &[])?;
    fs::create_dir_all(&original).unwrap();

    let report = recover_staged_deletes(std::slice::from_ref(&source));
    let restored = source.root.join("gone.restored-1");

    assert!(original.is_dir());
    assert!(restored.is_dir());
    assert_eq!(
        report.entries[0].detail.as_deref(),
        Some(format!("Restored as {}", restored.display()).as_str())
    );
    Ok(())
}
