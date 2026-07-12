use std::path::Path;

use tempfile::tempdir;

use super::SourceDatabase;

#[test]
fn role_scoped_entrypoints_preserve_read_write_contracts() {
    let dir = tempdir().unwrap();
    let writer = SourceDatabase::open_for_source_write(dir.path()).unwrap();
    writer.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

    let ui_read = SourceDatabase::open_for_ui_read(dir.path()).unwrap();
    assert_eq!(ui_read.count_files().unwrap(), 1);

    let background = SourceDatabase::open_for_background_job(dir.path()).unwrap();
    background
        .set_metadata("contract_probe", "background")
        .unwrap();

    let maintenance = SourceDatabase::open_for_maintenance(dir.path()).unwrap();
    assert_eq!(
        maintenance
            .get_metadata("contract_probe")
            .unwrap()
            .as_deref(),
        Some("background")
    );
}
