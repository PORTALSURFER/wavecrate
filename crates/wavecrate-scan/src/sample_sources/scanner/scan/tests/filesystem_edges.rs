use super::*;
use std::path::PathBuf;

#[test]
fn scan_tolerates_vanishing_nested_directories() {
    let dir = tempdir().unwrap();
    let one = dir.path().join("one.wav");
    std::fs::write(&one, b"one").unwrap();

    let vanishing = dir.path().join("vanishing");
    std::fs::create_dir_all(&vanishing).unwrap();
    std::fs::write(vanishing.join("two.wav"), b"two").unwrap();

    let vanishing_for_thread = vanishing.clone();
    let killer = std::thread::spawn(move || {
        for _ in 0..200 {
            let _ = std::fs::remove_dir_all(&vanishing_for_thread);
            std::thread::sleep(Duration::from_millis(1));
        }
    });

    let db = SourceDatabase::open(dir.path()).unwrap();
    let stats = scan_once(&db).unwrap();
    assert!(stats.total_files >= 1);

    let rows = db.list_files().unwrap();
    assert!(
        rows.iter()
            .any(|row| row.relative_path == Path::new("one.wav"))
    );

    let _ = killer.join();
}

#[test]
fn scan_skips_symlink_directories() {
    use std::os::unix::fs as unix_fs;

    let dir = tempdir().unwrap();
    let nested = dir.path().join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("two.wav"), b"two").unwrap();
    std::fs::write(dir.path().join("one.wav"), b"one").unwrap();

    let link = dir.path().join("nested_link");
    unix_fs::symlink(&nested, &link).unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    let stats = scan_once(&db).unwrap();
    assert_eq!(stats.total_files, 2);
    assert_eq!(stats.added, 2);
}

#[test]
fn scan_skips_symlink_files() {
    use std::os::unix::fs as unix_fs;

    let dir = tempdir().unwrap();
    let target = dir.path().join("one.wav");
    std::fs::write(&target, b"one").unwrap();
    let link = dir.path().join("one_link.wav");
    unix_fs::symlink(&target, &link).unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    let stats = scan_once(&db).unwrap();
    assert_eq!(stats.total_files, 1);
    assert_eq!(stats.added, 1);
}

#[test]
fn scan_skips_appledouble_sidecar_files() {
    let dir = tempdir().unwrap();
    std::fs::write(dir.path().join("kick.wav"), b"kick").unwrap();
    std::fs::write(dir.path().join("._kick.wav"), b"sidecar").unwrap();
    let nested = dir.path().join("drums");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("snare.wav"), b"snare").unwrap();
    std::fs::write(nested.join("._snare.wav"), b"sidecar").unwrap();

    let db = SourceDatabase::open(dir.path()).unwrap();
    let stats = scan_once(&db).unwrap();
    let paths = db
        .list_files()
        .unwrap()
        .into_iter()
        .map(|entry| entry.relative_path)
        .collect::<Vec<_>>();

    assert_eq!(stats.total_files, 2);
    assert_eq!(stats.added, 2);
    assert_eq!(
        paths,
        vec![PathBuf::from("drums/snare.wav"), PathBuf::from("kick.wav")]
    );
}
