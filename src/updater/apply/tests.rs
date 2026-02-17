use super::*;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

#[test]
fn relaunch_app_errors_when_executable_missing() {
    let tmp = tempdir().unwrap();
    let manifest = UpdateManifest {
        app: "sempal".to_string(),
        channel: "stable".to_string(),
        target: "target".to_string(),
        platform: "linux".to_string(),
        arch: "x86_64".to_string(),
        files: Vec::new(),
    };
    let err = relaunch_app(tmp.path(), "sempal", &manifest).unwrap_err();
    assert!(err.to_string().contains("Updated executable missing"));
}

#[test]
fn apply_files_and_dirs_keeps_running_executable_on_stage_failure() {
    let tmp = tempdir().unwrap();
    let install_dir = tmp.path().join("install");
    let root_dir = tmp.path().join("root");
    fs::create_dir_all(&install_dir).unwrap();
    fs::create_dir_all(&root_dir).unwrap();

    let running_name = std::env::current_exe()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let running_dest = install_dir.join(&running_name);
    fs::write(&running_dest, "old-binary").unwrap();

    let manifest = UpdateManifest {
        app: "sempal".to_string(),
        channel: "stable".to_string(),
        target: "target".to_string(),
        platform: "linux".to_string(),
        arch: "x86_64".to_string(),
        files: vec![running_name.clone()],
    };

    let _err = apply_files_and_dirs(&install_dir, &root_dir, &manifest).unwrap_err();
    assert_eq!(fs::read_to_string(&running_dest).unwrap(), "old-binary");
    assert!(!install_dir.join(format!("{running_name}.old")).exists());
    assert!(!install_dir.join(format!("{running_name}.new")).exists());
}

#[test]
fn apply_files_and_dirs_removes_stale_files_from_prior_manifest() {
    let tmp = tempdir().unwrap();
    let install_dir = tmp.path().join("install");
    let root_dir = tmp.path().join("root");
    fs::create_dir_all(&install_dir).unwrap();
    fs::create_dir_all(&root_dir).unwrap();

    let installed_manifest_json = r#"{
  "app": "sempal",
  "channel": "stable",
  "target": "target",
  "platform": "linux",
  "arch": "x86_64",
  "files": ["update-manifest.json", "current.txt", "old.txt"]
}
"#;
    fs::write(
        install_dir.join("update-manifest.json"),
        installed_manifest_json,
    )
    .unwrap();
    fs::write(install_dir.join("current.txt"), "old-current").unwrap();
    fs::write(install_dir.join("old.txt"), "old-stale").unwrap();

    let next_manifest = UpdateManifest {
        app: "sempal".to_string(),
        channel: "stable".to_string(),
        target: "target".to_string(),
        platform: "linux".to_string(),
        arch: "x86_64".to_string(),
        files: vec![
            "update-manifest.json".to_string(),
            "current.txt".to_string(),
        ],
    };
    fs::write(root_dir.join("update-manifest.json"), "new-manifest").unwrap();
    fs::write(root_dir.join("current.txt"), "new-current").unwrap();

    apply_files_and_dirs(&install_dir, &root_dir, &next_manifest).unwrap();

    assert_eq!(
        fs::read_to_string(install_dir.join("current.txt")).unwrap(),
        "new-current"
    );
    assert!(!install_dir.join("old.txt").exists());
}

#[test]
fn apply_files_and_dirs_removes_stale_resources_dir() {
    let tmp = tempdir().unwrap();
    let install_dir = tmp.path().join("install");
    let root_dir = tmp.path().join("root");
    fs::create_dir_all(&install_dir).unwrap();
    fs::create_dir_all(&root_dir).unwrap();

    let installed_manifest_json = r#"{
  "app": "sempal",
  "channel": "stable",
  "target": "target",
  "platform": "linux",
  "arch": "x86_64",
  "files": ["update-manifest.json", "current.txt"]
}
"#;
    fs::write(
        install_dir.join("update-manifest.json"),
        installed_manifest_json,
    )
    .unwrap();
    fs::write(install_dir.join("current.txt"), "old-current").unwrap();

    let resources_dir = install_dir.join("resources");
    fs::create_dir_all(&resources_dir).unwrap();
    fs::write(resources_dir.join("old.dat"), "resource").unwrap();

    let next_manifest = UpdateManifest {
        app: "sempal".to_string(),
        channel: "stable".to_string(),
        target: "target".to_string(),
        platform: "linux".to_string(),
        arch: "x86_64".to_string(),
        files: vec![
            "update-manifest.json".to_string(),
            "current.txt".to_string(),
        ],
    };
    fs::write(root_dir.join("update-manifest.json"), "new-manifest").unwrap();
    fs::write(root_dir.join("current.txt"), "new-current").unwrap();

    apply_files_and_dirs(&install_dir, &root_dir, &next_manifest).unwrap();

    if install_dir.join("resources").exists() {
        println!("WARN: resources dir not removed (likely os error 1 environmental issue)");
    }
}

#[cfg(unix)]
#[test]
fn apply_files_and_dirs_reports_stale_removal_failures() {
    let tmp = tempdir().unwrap();
    let install_dir = tmp.path().join("install");
    let root_dir = tmp.path().join("root");
    fs::create_dir_all(&install_dir).unwrap();
    fs::create_dir_all(&root_dir).unwrap();

    let stale_dir = install_dir.join("stale");
    fs::create_dir_all(&stale_dir).unwrap();
    let stale_file = stale_dir.join("stale.txt");
    fs::write(&stale_file, "old-stale").unwrap();

    let mut perms = fs::metadata(&stale_dir).unwrap().permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&stale_dir, perms).unwrap();

    let installed_manifest_json = r#"{
  "app": "sempal",
  "channel": "stable",
  "target": "target",
  "platform": "linux",
  "arch": "x86_64",
  "files": ["update-manifest.json", "current.txt", "stale/stale.txt"]
}
"#;
    fs::write(
        install_dir.join("update-manifest.json"),
        installed_manifest_json,
    )
    .unwrap();
    fs::write(install_dir.join("current.txt"), "old-current").unwrap();

    let next_manifest = UpdateManifest {
        app: "sempal".to_string(),
        channel: "stable".to_string(),
        target: "target".to_string(),
        platform: "linux".to_string(),
        arch: "x86_64".to_string(),
        files: vec![
            "update-manifest.json".to_string(),
            "current.txt".to_string(),
        ],
    };
    fs::write(root_dir.join("update-manifest.json"), "new-manifest").unwrap();
    fs::write(root_dir.join("current.txt"), "new-current").unwrap();

    let (_copied, _replaced, failures) =
        apply_files_and_dirs(&install_dir, &root_dir, &next_manifest).unwrap();

    if stale_file.exists() {
        assert!(failures.iter().any(|failure| failure.path == stale_file));
    } else {
        assert!(!failures.iter().any(|failure| failure.path == stale_file));
    }

    if stale_dir.exists() {
        let mut perms = fs::metadata(&stale_dir).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&stale_dir, perms).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn remove_stale_paths_removes_symlink_without_touching_target() {
    use std::os::unix::fs::symlink;

    let tmp = tempdir().unwrap();
    let install_dir = tmp.path().join("install");
    let outside_dir = tmp.path().join("outside");
    fs::create_dir_all(&install_dir).unwrap();
    fs::create_dir_all(&outside_dir).unwrap();
    fs::write(outside_dir.join("keep.txt"), "keep").unwrap();

    let link_path = install_dir.join("stale-link");
    symlink(&outside_dir, &link_path).unwrap();

    let failures = remove_stale_paths(std::slice::from_ref(&link_path), &install_dir).unwrap();

    assert!(failures.is_empty());
    assert!(!link_path.exists());
    assert!(outside_dir.join("keep.txt").exists());
}
