//! Focused checks for shared release workflow helper scripts.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use serde_json::json;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use zip::write::SimpleFileOptions;

#[test]
fn toolchain_helper_emits_github_output_channel() {
    let output = Command::new("python3")
        .arg(repo_path(
            "scripts/internal/release/emit_rust_toolchain_channel.py",
        ))
        .current_dir(repo_root())
        .output()
        .expect("run toolchain helper");

    assert!(
        output.status.success(),
        "toolchain helper failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("toolchain output utf-8");
    assert!(
        stdout.trim().starts_with("channel="),
        "toolchain helper must emit a GitHub output assignment"
    );
}

#[test]
fn assemble_release_files_copies_zips_and_combines_checksum_entries() {
    let temp = tempfile::tempdir().expect("create helper fixture");
    let artifacts = temp.path().join("artifacts");
    let release = temp.path().join("release");
    fs::create_dir_all(&artifacts).expect("create artifacts dir");
    fs::write(
        artifacts.join("wavecrate-nightly-windows-x86_64.zip"),
        "zip",
    )
    .expect("write zip");
    fs::write(
        artifacts.join("checksums-entry-windows-x86_64.txt"),
        "abc  wavecrate-nightly-windows-x86_64.zip\n",
    )
    .expect("write checksum entry");

    run_success(
        Command::new("bash")
            .arg(repo_path(
                "scripts/internal/release/assemble_release_files.sh",
            ))
            .arg("--artifact-dir")
            .arg(&artifacts)
            .arg("--out-dir")
            .arg(&release)
            .arg("--checksum-name")
            .arg("checksums-nightly.txt"),
    );

    assert!(
        release
            .join("wavecrate-nightly-windows-x86_64.zip")
            .is_file()
    );
    assert_eq!(
        fs::read_to_string(release.join("checksums-nightly.txt")).expect("read checksums"),
        "abc  wavecrate-nightly-windows-x86_64.zip\n"
    );
}

#[test]
fn release_summary_helper_writes_public_metadata_without_secret_values() {
    let temp = tempfile::tempdir().expect("create summary fixture");
    let artifact_dir = temp.path().join("release");
    let summary_file = temp.path().join("summary.md");
    fs::create_dir_all(&artifact_dir).expect("create release dir");
    fs::write(
        artifact_dir.join("wavecrate-19.1.0-windows-x86_64.zip"),
        "not a real zip but enough for summary hashing\n",
    )
    .expect("write artifact");
    fs::write(
        artifact_dir.join("checksums-19.1.0.txt"),
        "abc  wavecrate-19.1.0-windows-x86_64.zip\n",
    )
    .expect("write checksum");

    run_success(
        Command::new("bash")
            .arg(repo_path(
                "scripts/internal/release/write_release_step_summary.sh",
            ))
            .arg("--title")
            .arg("Stable release publication")
            .arg("--status")
            .arg("success")
            .arg("--channel")
            .arg("stable")
            .arg("--version")
            .arg("19.1.0")
            .arg("--target-version")
            .arg("19.1.0")
            .arg("--commit")
            .arg("abcdef123456")
            .arg("--build-id")
            .arg("wavecrate-19.1.0")
            .arg("--build-number")
            .arg("6258")
            .arg("--github-release-url")
            .arg("https://github.com/PORTALSURFER/wavecrate/releases/tag/v19.1.0")
            .arg("--portal-catalog-url")
            .arg("https://portalsurfer.org/wavecrate/api/v1/releases")
            .arg("--portal-build-id")
            .arg("wavecrate-19.1.0")
            .arg("--artifact-dir")
            .arg(&artifact_dir)
            .arg("--checksum-file")
            .arg(artifact_dir.join("checksums-19.1.0.txt"))
            .arg("--note")
            .arg("fixture note")
            .env("GITHUB_STEP_SUMMARY", &summary_file)
            .env(
                "PORTALSURFER_RELEASE_UPLOAD_TOKEN",
                "super-secret-upload-token",
            )
            .env("CHECKSUMS_SIGNING_KEY", "super-secret-signing-key")
            .env(
                "APPLE_DEVELOPER_ID_APPLICATION_CERT_BASE64",
                "super-secret-apple-cert",
            ),
    );

    let summary = fs::read_to_string(summary_file).expect("read summary");
    assert!(summary.contains("## Stable release publication"));
    assert!(summary.contains("wavecrate-19.1.0-windows-x86_64.zip"));
    assert!(summary.contains("checksums-19.1.0.txt"));
    assert!(summary.contains("PortalSurfer catalog"));
    assert!(summary.contains("SHA-256"));
    assert!(!summary.contains("super-secret"));
}

#[test]
fn github_release_body_helper_leaves_short_logs_unchanged() {
    let temp = tempfile::tempdir().expect("create GitHub body fixture");
    let input = temp.path().join("release-log.md");
    let output = temp.path().join("github-release-body.md");
    let body = "# Wavecrate 19.1.0-rc.1\n\n- Short release log\n";
    fs::write(&input, body).expect("write release log");

    run_success(
        Command::new("python3")
            .arg(repo_path(
                "scripts/internal/release/prepare_github_release_body.py",
            ))
            .arg("--input")
            .arg(&input)
            .arg("--output")
            .arg(&output)
            .arg("--max-chars")
            .arg("1000"),
    );

    assert_eq!(
        fs::read_to_string(output).expect("read GitHub release body"),
        body
    );
}

#[test]
fn github_release_body_helper_caps_oversized_logs_with_full_log_notice() {
    let temp = tempfile::tempdir().expect("create GitHub body fixture");
    let input = temp.path().join("release-log.md");
    let output = temp.path().join("github-release-body.md");
    let body = format!(
        "# Wavecrate 19.1.0-rc.1\n\n## Generated Changes\n\n- {}\n",
        "very long generated change ".repeat(200)
    );
    fs::write(&input, body).expect("write release log");

    run_success(
        Command::new("python3")
            .arg(repo_path(
                "scripts/internal/release/prepare_github_release_body.py",
            ))
            .arg("--input")
            .arg(&input)
            .arg("--output")
            .arg(&output)
            .arg("--max-chars")
            .arg("1000"),
    );

    let github_body = fs::read_to_string(output).expect("read GitHub release body");
    assert!(github_body.len() <= 1000);
    assert!(github_body.starts_with("# Wavecrate 19.1.0-rc.1"));
    assert!(github_body.contains("## Full Release Log"));
    assert!(github_body.contains("`release-log.md`"));
    assert!(github_body.contains("PortalSurfer release changelog"));
}

#[test]
fn portal_changelog_assembler_accepts_current_staged_release_not_yet_in_catalog() {
    let temp = tempfile::tempdir().expect("create changelog fixture");
    let catalog = temp.path().join("catalog.json");
    let release_log = temp.path().join("release-log.md");
    let output = temp.path().join("changelog.md");
    fs::write(
        &catalog,
        serde_json::to_string_pretty(&json!({
            "releases": [
                {
                    "build_id": "wavecrate-nightly-b6306-deadbee",
                    "build_number": 6306,
                    "version": "19.1.0-nightly.6306+deadbee",
                    "released_at": "2026-07-03T19:00:00Z",
                    "changelog": {
                        "title": "Wavecrate nightly-b6306-deadbee",
                        "format": "markdown",
                        "body": "## [nightly-b6306-deadbee]\n\n- Prior nightly\n"
                    }
                }
            ]
        }))
        .expect("serialize catalog"),
    )
    .expect("write catalog");
    fs::write(
        &release_log,
        "# Wavecrate 19.1.0-nightly.6307+8f1a7aa2\n\n- Current staged nightly\n",
    )
    .expect("write current log");

    run_success(
        Command::new("python3")
            .arg(repo_path(
                "scripts/internal/release/assemble_portal_changelog.py",
            ))
            .arg("--catalog-file")
            .arg(&catalog)
            .arg("--current-build-id")
            .arg("wavecrate-nightly-b6307-8f1a7aa2")
            .arg("--current-build-number")
            .arg("6307")
            .arg("--current-version")
            .arg("19.1.0-nightly.6307+8f1a7aa2")
            .arg("--current-released-at")
            .arg("2026-07-03T20:00:00Z")
            .arg("--current-log")
            .arg(&release_log)
            .arg("--generated-at")
            .arg("2026-07-03T20:00:00Z")
            .arg("--output")
            .arg(&output),
    );

    let changelog = fs::read_to_string(output).expect("read generated changelog");
    assert!(changelog.contains("Latest release: wavecrate-nightly-b6307-8f1a7aa2"));
    assert!(changelog.contains("Latest build: 6307"));
    assert!(changelog.contains("Current staged nightly"));
    assert!(changelog.contains("Prior nightly"));
}

#[test]
fn checksum_signing_helper_writes_signature_and_verifies_expected_pubkey() {
    let temp = tempfile::tempdir().expect("create signing fixture");
    let key = temp.path().join("ed25519.pem");
    let checksum = temp.path().join("checksums.txt");
    let signature = temp.path().join("checksums.txt.sig");
    fs::write(&checksum, "abc  wavecrate.zip\n").expect("write checksum");
    let keygen = Command::new("openssl")
        .arg("genpkey")
        .arg("-algorithm")
        .arg("Ed25519")
        .arg("-out")
        .arg(&key)
        .output()
        .expect("run openssl key generation");
    if !keygen.status.success() {
        let stderr = String::from_utf8_lossy(&keygen.stderr);
        if stderr.contains("Algorithm Ed25519 not found") {
            eprintln!(
                "local openssl does not support Ed25519 key generation; skipping signing roundtrip"
            );
            return;
        }
        panic!(
            "openssl key generation failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&keygen.stdout),
            stderr
        );
    }
    let expected_pubkey = expected_public_key(&key, &temp);
    let key_pem = fs::read_to_string(&key).expect("read generated key");

    run_success(
        Command::new("bash")
            .arg(repo_path(
                "scripts/internal/release/sign_release_checksums.sh",
            ))
            .arg("--checksum-file")
            .arg(&checksum)
            .arg("--signature-file")
            .arg(&signature)
            .arg("--verify-public-key")
            .arg(expected_pubkey.trim())
            .env("CHECKSUMS_SIGNING_KEY", key_pem),
    );

    assert!(signature.is_file(), "signature file should be written");
    assert!(
        !fs::read_to_string(signature)
            .expect("read signature")
            .trim()
            .is_empty(),
        "signature should not be empty"
    );
}

#[test]
fn build_release_artifact_helper_names_zip_and_checksum_entry() {
    let temp = tempfile::tempdir().expect("create artifact fixture");
    let dummy = repo_root().join("target/x86_64-pc-windows-msvc/release/wavecrate.exe");
    fs::create_dir_all(dummy.parent().expect("dummy binary parent")).expect("create dummy parent");
    fs::write(&dummy, "dry-run wavecrate exe\n").expect("write dummy binary");
    let output = Command::new("bash")
        .arg(repo_path(
            "scripts/internal/release/build_release_artifact.sh",
        ))
        .arg("--target")
        .arg("x86_64-pc-windows-msvc")
        .arg("--platform")
        .arg("windows")
        .arg("--arch")
        .arg("x86_64")
        .arg("--channel")
        .arg("nightly")
        .arg("--version")
        .arg("19.1.0-nightly.20260702+abcdef0")
        .arg("--target-version")
        .arg("19.1.0")
        .arg("--build-number")
        .arg("123")
        .arg("--git-sha")
        .arg("abcdef1")
        .arg("--build-date")
        .arg("2026-07-02")
        .arg("--out-dir")
        .arg(temp.path())
        .env("WAVECRATE_SKIP_BUILD", "1")
        .current_dir(repo_root())
        .output()
        .expect("run artifact helper");
    let _ = fs::remove_file(dummy);
    assert!(
        output.status.success(),
        "artifact helper failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        temp.path()
            .join("wavecrate-nightly-windows-x86_64.zip")
            .is_file()
    );
    let zip_path = temp.path().join("wavecrate-nightly-windows-x86_64.zip");
    let zip_file = fs::File::open(&zip_path).expect("open release zip");
    let mut archive = zip::ZipArchive::new(zip_file).expect("read release zip");
    let mut archive_names = Vec::new();
    for index in 0..archive.len() {
        archive_names.push(
            archive
                .by_index(index)
                .expect("read release zip entry")
                .name()
                .to_owned(),
        );
    }
    assert!(
        archive_names
            .iter()
            .any(|name| name == "wavecrate/update-manifest.json"),
        "release zip must keep update manifest under the wavecrate root: {archive_names:?}"
    );
    assert!(
        archive_names
            .iter()
            .any(|name| name == "wavecrate/wavecrate.exe"),
        "release zip must keep the executable under the wavecrate root: {archive_names:?}"
    );
    assert!(
        !archive_names
            .iter()
            .any(|name| name == "update-manifest.json" || name == "wavecrate.exe"),
        "release zip must not flatten archive entries at the root: {archive_names:?}"
    );
    assert!(
        temp.path()
            .join("checksums-entry-windows-x86_64.txt")
            .is_file()
    );
}

#[test]
fn promoted_rc_validator_accepts_complete_release_fixture() {
    let temp = tempfile::tempdir().expect("create promoted RC fixture");
    let (release_json, asset_dir) =
        write_promoted_rc_fixture(&temp, None, false, "# Wavecrate 19.1.0-rc.2\n", true);

    run_success(
        Command::new("python3")
            .arg(repo_path(
                "scripts/internal/release/validate_promoted_rc_release.py",
            ))
            .arg("--version")
            .arg("19.1.0")
            .arg("--rc-tag")
            .arg("v19.1.0-rc.2")
            .arg("--release-json")
            .arg(&release_json)
            .arg("--asset-dir")
            .arg(&asset_dir),
    );
}

#[test]
fn promoted_rc_validator_verifies_checksum_signature_when_public_key_is_supplied() {
    let temp = tempfile::tempdir().expect("create promoted RC fixture");
    let (release_json, asset_dir) =
        write_promoted_rc_fixture(&temp, None, false, "# Wavecrate 19.1.0-rc.2\n", true);
    let key = temp.path().join("ed25519.pem");
    let keygen = Command::new("openssl")
        .arg("genpkey")
        .arg("-algorithm")
        .arg("Ed25519")
        .arg("-out")
        .arg(&key)
        .output()
        .expect("run openssl key generation");
    if !keygen.status.success() {
        let stderr = String::from_utf8_lossy(&keygen.stderr);
        if stderr.contains("Algorithm Ed25519 not found") {
            eprintln!(
                "local openssl does not support Ed25519 key generation; skipping validator signature roundtrip"
            );
            return;
        }
        panic!(
            "openssl key generation failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&keygen.stdout),
            stderr
        );
    }
    let expected_pubkey = expected_public_key(&key, &temp);
    let key_pem = fs::read_to_string(&key).expect("read generated key");

    run_success(
        Command::new("bash")
            .arg(repo_path(
                "scripts/internal/release/sign_release_checksums.sh",
            ))
            .arg("--checksum-file")
            .arg(asset_dir.join("checksums-19.1.0-rc.2.txt"))
            .arg("--signature-file")
            .arg(asset_dir.join("checksums-19.1.0-rc.2.txt.sig"))
            .env("CHECKSUMS_SIGNING_KEY", key_pem),
    );
    run_success(
        Command::new("python3")
            .arg(repo_path(
                "scripts/internal/release/validate_promoted_rc_release.py",
            ))
            .arg("--version")
            .arg("19.1.0")
            .arg("--rc-tag")
            .arg("v19.1.0-rc.2")
            .arg("--release-json")
            .arg(&release_json)
            .arg("--asset-dir")
            .arg(&asset_dir)
            .arg("--checksum-public-key")
            .arg(expected_pubkey.trim()),
    );
}

#[test]
fn promoted_rc_validator_rejects_missing_release_assets() {
    let temp = tempfile::tempdir().expect("create promoted RC fixture");
    let missing = "wavecrate-19.1.0-rc.2-macos-aarch64.zip";
    let (release_json, asset_dir) = write_promoted_rc_fixture(
        &temp,
        Some(missing),
        false,
        "# Wavecrate 19.1.0-rc.2\n",
        true,
    );

    let output = run_failure(
        Command::new("python3")
            .arg(repo_path(
                "scripts/internal/release/validate_promoted_rc_release.py",
            ))
            .arg("--version")
            .arg("19.1.0")
            .arg("--rc-tag")
            .arg("v19.1.0-rc.2")
            .arg("--release-json")
            .arg(&release_json)
            .arg("--asset-dir")
            .arg(&asset_dir),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("RC release is missing required assets"),
        "missing asset failure should name the release asset problem\nstderr:\n{stderr}"
    );
}

#[test]
fn promoted_rc_validator_rejects_undownloadable_release_assets() {
    let temp = tempfile::tempdir().expect("create promoted RC fixture");
    let missing = "wavecrate-19.1.0-rc.2-macos-aarch64.zip";
    let (release_json, asset_dir) =
        write_promoted_rc_fixture(&temp, None, false, "# Wavecrate 19.1.0-rc.2\n", true);
    fs::remove_file(asset_dir.join(missing)).expect("remove fixture asset");

    let output = run_failure(
        Command::new("python3")
            .arg(repo_path(
                "scripts/internal/release/validate_promoted_rc_release.py",
            ))
            .arg("--version")
            .arg("19.1.0")
            .arg("--rc-tag")
            .arg("v19.1.0-rc.2")
            .arg("--release-json")
            .arg(&release_json)
            .arg("--asset-dir")
            .arg(&asset_dir),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Downloaded RC asset is missing"),
        "missing asset failure should prove the asset was not downloadable\nstderr:\n{stderr}"
    );
}

#[test]
fn promoted_rc_validator_rejects_checksum_mismatches() {
    let temp = tempfile::tempdir().expect("create promoted RC fixture");
    let (release_json, asset_dir) =
        write_promoted_rc_fixture(&temp, None, true, "# Wavecrate 19.1.0-rc.2\n", true);

    let output = run_failure(
        Command::new("python3")
            .arg(repo_path(
                "scripts/internal/release/validate_promoted_rc_release.py",
            ))
            .arg("--version")
            .arg("19.1.0")
            .arg("--rc-tag")
            .arg("v19.1.0-rc.2")
            .arg("--release-json")
            .arg(&release_json)
            .arg("--asset-dir")
            .arg(&asset_dir),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Checksum mismatch"),
        "checksum failure should report the mismatched asset\nstderr:\n{stderr}"
    );
}

#[test]
fn published_release_verifier_accepts_portalsurfer_catalog_fixture() {
    let temp = tempfile::tempdir().expect("create published release fixture");
    let key = temp.path().join("ed25519.pem");
    if !generate_ed25519_key(&key) {
        eprintln!(
            "local openssl does not support Ed25519 key generation; skipping verifier roundtrip"
        );
        return;
    }
    let expected_pubkey = expected_public_key(&key, &temp);
    let (release_json, asset_dir) = write_published_release_fixture(&temp, &key, None);

    run_success(
        Command::new("python3")
            .arg(repo_path(
                "scripts/internal/release/verify_published_release.py",
            ))
            .arg("--surface")
            .arg("portalsurfer")
            .arg("--channel")
            .arg("stable")
            .arg("--version")
            .arg("19.1.0")
            .arg("--target-version")
            .arg("19.1.0")
            .arg("--commit")
            .arg("abcdef1")
            .arg("--build-date")
            .arg("2026-07-02")
            .arg("--portal-build-id")
            .arg("wavecrate-19.1.0")
            .arg("--build-number")
            .arg("6200")
            .arg("--release-json")
            .arg(&release_json)
            .arg("--asset-dir")
            .arg(&asset_dir)
            .arg("--checksum-public-key")
            .arg(expected_pubkey.trim()),
    );
}

#[test]
fn published_release_verifier_accepts_windows_backslash_archive_entries() {
    let temp = tempfile::tempdir().expect("create published release fixture");
    let key = temp.path().join("ed25519.pem");
    if !generate_ed25519_key(&key) {
        eprintln!(
            "local openssl does not support Ed25519 key generation; skipping verifier roundtrip"
        );
        return;
    }
    let expected_pubkey = expected_public_key(&key, &temp);
    let (release_json, asset_dir) = write_published_release_fixture_with_zip_mutation(
        &temp,
        &key,
        None,
        Some(PublishedZipMutation::WindowsBackslashes),
    );

    let mut command =
        published_release_verifier_command(&release_json, &asset_dir, expected_pubkey.trim());
    run_success(&mut command);
}

#[test]
fn published_release_verifier_ignores_macos_directory_entries() {
    let temp = tempfile::tempdir().expect("create published release fixture");
    let key = temp.path().join("ed25519.pem");
    if !generate_ed25519_key(&key) {
        eprintln!(
            "local openssl does not support Ed25519 key generation; skipping verifier roundtrip"
        );
        return;
    }
    let expected_pubkey = expected_public_key(&key, &temp);
    let (release_json, asset_dir) = write_published_release_fixture_with_zip_mutation(
        &temp,
        &key,
        None,
        Some(PublishedZipMutation::MacosDirectoryEntries),
    );

    let mut command =
        published_release_verifier_command(&release_json, &asset_dir, expected_pubkey.trim());
    run_success(&mut command);
}

#[test]
fn published_release_verifier_rejects_manifest_mismatches() {
    let temp = tempfile::tempdir().expect("create published release fixture");
    let key = temp.path().join("ed25519.pem");
    if !generate_ed25519_key(&key) {
        eprintln!(
            "local openssl does not support Ed25519 key generation; skipping verifier roundtrip"
        );
        return;
    }
    let expected_pubkey = expected_public_key(&key, &temp);
    let (release_json, asset_dir) =
        write_published_release_fixture(&temp, &key, Some(("commit", "deadbee")));

    let output = run_failure(
        Command::new("python3")
            .arg(repo_path(
                "scripts/internal/release/verify_published_release.py",
            ))
            .arg("--surface")
            .arg("portalsurfer")
            .arg("--channel")
            .arg("stable")
            .arg("--version")
            .arg("19.1.0")
            .arg("--target-version")
            .arg("19.1.0")
            .arg("--commit")
            .arg("abcdef1")
            .arg("--build-date")
            .arg("2026-07-02")
            .arg("--portal-build-id")
            .arg("wavecrate-19.1.0")
            .arg("--build-number")
            .arg("6200")
            .arg("--release-json")
            .arg(&release_json)
            .arg("--asset-dir")
            .arg(&asset_dir)
            .arg("--checksum-public-key")
            .arg(expected_pubkey.trim()),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("manifest commit"),
        "manifest mismatch should report the wrong manifest field\nstderr:\n{stderr}"
    );
}

#[test]
fn published_release_verifier_rejects_flattened_archives() {
    let temp = tempfile::tempdir().expect("create published release fixture");
    let key = temp.path().join("ed25519.pem");
    if !generate_ed25519_key(&key) {
        eprintln!(
            "local openssl does not support Ed25519 key generation; skipping verifier roundtrip"
        );
        return;
    }
    let expected_pubkey = expected_public_key(&key, &temp);
    let (release_json, asset_dir) = write_published_release_fixture_with_zip_mutation(
        &temp,
        &key,
        None,
        Some(PublishedZipMutation::FlattenWindows),
    );

    let mut command =
        published_release_verifier_command(&release_json, &asset_dir, expected_pubkey.trim());
    let output = run_failure(&mut command);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "wavecrate-19.1.0-windows-x86_64.zip manifest must be at wavecrate/update-manifest.json"
        ) && stderr.contains("must contain expected archive root wavecrate/"),
        "flattened archive failure should name the asset and missing root\nstderr:\n{stderr}"
    );
}

#[test]
fn published_release_verifier_rejects_missing_platform_executable() {
    let temp = tempfile::tempdir().expect("create published release fixture");
    let key = temp.path().join("ed25519.pem");
    if !generate_ed25519_key(&key) {
        eprintln!(
            "local openssl does not support Ed25519 key generation; skipping verifier roundtrip"
        );
        return;
    }
    let expected_pubkey = expected_public_key(&key, &temp);
    let (release_json, asset_dir) = write_published_release_fixture_with_zip_mutation(
        &temp,
        &key,
        None,
        Some(PublishedZipMutation::MissingWindowsExecutable),
    );

    let mut command =
        published_release_verifier_command(&release_json, &asset_dir, expected_pubkey.trim());
    let output = run_failure(&mut command);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "wavecrate-19.1.0-windows-x86_64.zip is missing required archive file wavecrate/wavecrate.exe"
        ),
        "missing executable failure should name the asset and missing executable path\nstderr:\n{stderr}"
    );
}

#[test]
fn portalsurfer_upload_catalog_verifier_accepts_equivalent_timestamp_precision() {
    let temp = tempfile::tempdir().expect("create PortalSurfer catalog fixture");
    let catalog = temp.path().join("catalog.json");
    write_portalsurfer_upload_catalog(&catalog, "2026-07-02T10:10:24.000Z");

    run_success(
        Command::new("python3")
            .arg(repo_path(
                "scripts/internal/release/verify_portalsurfer_upload_catalog.py",
            ))
            .arg("--catalog-file")
            .arg(&catalog)
            .arg("--build-id")
            .arg("wavecrate-nightly-b6242-5e5f4198")
            .arg("--build-number")
            .arg("6242")
            .arg("--release-version")
            .arg("19.1.0-nightly.20260702+5e5f4198")
            .arg("--released-at")
            .arg("2026-07-02T10:10:24Z")
            .arg("--expected-file")
            .arg("wavecrate-nightly-macos-aarch64.zip"),
    );
}

#[test]
fn portalsurfer_upload_catalog_verifier_rejects_different_timestamps() {
    let temp = tempfile::tempdir().expect("create PortalSurfer catalog fixture");
    let catalog = temp.path().join("catalog.json");
    write_portalsurfer_upload_catalog(&catalog, "2026-07-02T10:10:25.000Z");

    let output = run_failure(
        Command::new("python3")
            .arg(repo_path(
                "scripts/internal/release/verify_portalsurfer_upload_catalog.py",
            ))
            .arg("--catalog-file")
            .arg(&catalog)
            .arg("--build-id")
            .arg("wavecrate-nightly-b6242-5e5f4198")
            .arg("--build-number")
            .arg("6242")
            .arg("--release-version")
            .arg("19.1.0-nightly.20260702+5e5f4198")
            .arg("--released-at")
            .arg("2026-07-02T10:10:24Z")
            .arg("--expected-file")
            .arg("wavecrate-nightly-macos-aarch64.zip"),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Release catalog timestamp mismatch"),
        "timestamp mismatch should still fail for a different instant\nstderr:\n{stderr}"
    );
}

#[test]
fn published_release_verifier_adds_portalsurfer_download_tokens_to_file_urls() {
    let script = repo_path("scripts/internal/release/verify_published_release.py");
    let python = format!(
        r#"
import importlib.util
import sys
spec = importlib.util.spec_from_file_location("verify_published_release", {script:?})
module = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = module
spec.loader.exec_module(module)
url = module.portalsurfer_download_url(
    "https://portalsurfer.org/wavecrate/api/v1/releases/build/files/file.zip/download?existing=1",
    "token with spaces",
)
assert url == "https://portalsurfer.org/wavecrate/api/v1/releases/build/files/file.zip/download?existing=1&download_token=token+with+spaces", url
"#,
        script = script.display().to_string(),
    );
    run_success(Command::new("python3").arg("-c").arg(python));
}

fn generate_ed25519_key(path: &Path) -> bool {
    let keygen = Command::new("openssl")
        .arg("genpkey")
        .arg("-algorithm")
        .arg("Ed25519")
        .arg("-out")
        .arg(path)
        .output()
        .expect("run openssl key generation");
    if keygen.status.success() {
        return true;
    }
    let stderr = String::from_utf8_lossy(&keygen.stderr);
    if stderr.contains("Algorithm Ed25519 not found") {
        return false;
    }
    panic!(
        "openssl key generation failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&keygen.stdout),
        stderr
    );
}

fn write_portalsurfer_upload_catalog(path: &Path, released_at: &str) {
    let catalog = json!({
        "app": "wavecrate",
        "releases": [{
            "build_id": "wavecrate-nightly-b6242-5e5f4198",
            "build_number": 6242,
            "version": "19.1.0-nightly.20260702+5e5f4198",
            "released_at": released_at,
            "changelog": {
                "title": "Wavecrate nightly",
                "format": "markdown",
                "url": "/wavecrate/api/v1/releases/wavecrate-nightly-b6242-5e5f4198/changelog"
            },
            "files": [{
                "name": "wavecrate-nightly-macos-aarch64.zip",
                "url": "/wavecrate/api/v1/releases/wavecrate-nightly-b6242-5e5f4198/files/wavecrate-nightly-macos-aarch64.zip/download",
                "sha256": "0".repeat(64),
                "size_bytes": 123
            }]
        }]
    });
    fs::write(
        path,
        serde_json::to_string_pretty(&catalog).expect("serialize PortalSurfer catalog"),
    )
    .expect("write PortalSurfer catalog fixture");
}

fn expected_public_key(key: &Path, temp: &TempDir) -> String {
    let pub_der = temp.path().join("public.der");
    run_success(
        Command::new("openssl")
            .arg("pkey")
            .arg("-in")
            .arg(key)
            .arg("-pubout")
            .arg("-outform")
            .arg("DER")
            .arg("-out")
            .arg(&pub_der),
    );
    let output = Command::new("bash")
        .arg("-lc")
        .arg(format!(
            "tail -c 32 '{}' | openssl base64 -A",
            pub_der.display()
        ))
        .output()
        .expect("derive expected pubkey");
    assert!(
        output.status.success(),
        "derive expected pubkey failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("pubkey utf-8")
}

fn write_promoted_rc_fixture(
    temp: &TempDir,
    missing_asset: Option<&str>,
    corrupt_checksum: bool,
    body: &str,
    is_prerelease: bool,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let asset_dir = temp.path().join("assets");
    fs::create_dir_all(&asset_dir).expect("create asset dir");
    let zip_assets = [
        "wavecrate-19.1.0-rc.2-macos-aarch64.zip",
        "wavecrate-19.1.0-rc.2-macos-x86_64.zip",
        "wavecrate-19.1.0-rc.2-windows-x86_64.zip",
    ];
    let checksum_asset = "checksums-19.1.0-rc.2.txt";
    let signature_asset = "checksums-19.1.0-rc.2.txt.sig";

    let mut checksum_lines = Vec::new();
    for zip in zip_assets {
        let bytes = format!("fixture bytes for {zip}\n");
        if missing_asset != Some(zip) {
            fs::write(asset_dir.join(zip), bytes.as_bytes()).expect("write fixture zip");
        }
        let mut hash = format!("{:x}", Sha256::digest(bytes.as_bytes()));
        if corrupt_checksum && zip.ends_with("windows-x86_64.zip") {
            hash = "0".repeat(64);
        }
        checksum_lines.push(format!("{hash}  {zip}\n"));
    }
    fs::write(asset_dir.join(checksum_asset), checksum_lines.concat()).expect("write checksum");
    fs::write(asset_dir.join(signature_asset), "c2lnbmF0dXJl\n").expect("write signature");

    let asset_names = [zip_assets.as_slice(), &[checksum_asset, signature_asset]]
        .concat()
        .into_iter()
        .filter(|asset| missing_asset != Some(*asset))
        .map(|name| json!({ "name": name }))
        .collect::<Vec<_>>();
    let release = json!({
        "tagName": "v19.1.0-rc.2",
        "isPrerelease": is_prerelease,
        "body": body,
        "assets": asset_names,
    });
    let release_json = temp.path().join("release.json");
    fs::write(
        &release_json,
        serde_json::to_string_pretty(&release).expect("serialize release json"),
    )
    .expect("write release json");
    (release_json, asset_dir)
}

fn write_published_release_fixture(
    temp: &TempDir,
    key: &Path,
    manifest_override: Option<(&str, &str)>,
) -> (std::path::PathBuf, std::path::PathBuf) {
    write_published_release_fixture_with_zip_mutation(temp, key, manifest_override, None)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PublishedZipMutation {
    FlattenWindows,
    MacosDirectoryEntries,
    MissingWindowsExecutable,
    WindowsBackslashes,
}

fn write_published_release_fixture_with_zip_mutation(
    temp: &TempDir,
    key: &Path,
    manifest_override: Option<(&str, &str)>,
    zip_mutation: Option<PublishedZipMutation>,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let asset_dir = temp.path().join("published-assets");
    fs::create_dir_all(&asset_dir).expect("create asset dir");
    let zip_assets = [
        (
            "wavecrate-19.1.0-macos-aarch64.zip",
            "aarch64-apple-darwin",
            "macos",
            "aarch64",
        ),
        (
            "wavecrate-19.1.0-macos-x86_64.zip",
            "x86_64-apple-darwin",
            "macos",
            "x86_64",
        ),
        (
            "wavecrate-19.1.0-windows-x86_64.zip",
            "x86_64-pc-windows-msvc",
            "windows",
            "x86_64",
        ),
    ];
    let checksum_asset = "checksums-19.1.0.txt";
    let signature_asset = "checksums-19.1.0.txt.sig";

    let mut checksum_lines = Vec::new();
    let mut files = Vec::new();
    for (zip_name, target, platform, arch) in zip_assets {
        let mutation = match (platform, zip_mutation) {
            ("windows", Some(PublishedZipMutation::MacosDirectoryEntries)) => None,
            ("macos", Some(PublishedZipMutation::MacosDirectoryEntries)) => zip_mutation,
            ("windows", _) => zip_mutation,
            _ => None,
        };
        write_release_zip(
            &asset_dir.join(zip_name),
            target,
            platform,
            arch,
            manifest_override,
            mutation,
        );
        let hash = sha256_file(&asset_dir.join(zip_name));
        checksum_lines.push(format!("{hash}  {zip_name}\n"));
        files.push(json!({
            "name": zip_name,
            "url": format!("/wavecrate/api/v1/releases/wavecrate-19.1.0/files/{zip_name}/download"),
            "sha256": hash,
            "size_bytes": fs::metadata(asset_dir.join(zip_name)).expect("zip metadata").len()
        }));
    }

    fs::write(asset_dir.join(checksum_asset), checksum_lines.concat()).expect("write checksum");
    let key_pem = fs::read_to_string(key).expect("read generated key");
    run_success(
        Command::new("bash")
            .arg(repo_path(
                "scripts/internal/release/sign_release_checksums.sh",
            ))
            .arg("--checksum-file")
            .arg(asset_dir.join(checksum_asset))
            .arg("--signature-file")
            .arg(asset_dir.join(signature_asset))
            .env("CHECKSUMS_SIGNING_KEY", key_pem),
    );
    for file_name in [checksum_asset, signature_asset] {
        files.push(json!({
            "name": file_name,
            "url": format!("/wavecrate/api/v1/releases/wavecrate-19.1.0/files/{file_name}/download"),
            "sha256": sha256_file(&asset_dir.join(file_name)),
            "size_bytes": fs::metadata(asset_dir.join(file_name)).expect("asset metadata").len()
        }));
    }

    let catalog = json!({
        "app": "wavecrate",
        "releases": [{
            "build_id": "wavecrate-19.1.0",
            "build_number": 6200,
            "version": "19.1.0",
            "released_at": "2026-07-02T09:00:00Z",
            "changelog": {
                "title": "Wavecrate 19.1.0",
                "format": "markdown",
                "body": "# Wavecrate 19.1.0\n\n## Release Metadata\n",
                "url": "/wavecrate/api/v1/releases/wavecrate-19.1.0/changelog"
            },
            "files": files
        }]
    });
    let release_json = temp.path().join("published-catalog.json");
    fs::write(
        &release_json,
        serde_json::to_string_pretty(&catalog).expect("serialize catalog json"),
    )
    .expect("write release json");
    (release_json, asset_dir)
}

fn published_release_verifier_command(
    release_json: &Path,
    asset_dir: &Path,
    expected_pubkey: &str,
) -> Command {
    let mut command = Command::new("python3");
    command
        .arg(repo_path(
            "scripts/internal/release/verify_published_release.py",
        ))
        .arg("--surface")
        .arg("portalsurfer")
        .arg("--channel")
        .arg("stable")
        .arg("--version")
        .arg("19.1.0")
        .arg("--target-version")
        .arg("19.1.0")
        .arg("--commit")
        .arg("abcdef1")
        .arg("--build-date")
        .arg("2026-07-02")
        .arg("--portal-build-id")
        .arg("wavecrate-19.1.0")
        .arg("--build-number")
        .arg("6200")
        .arg("--release-json")
        .arg(release_json)
        .arg("--asset-dir")
        .arg(asset_dir)
        .arg("--checksum-public-key")
        .arg(expected_pubkey);
    command
}

fn write_release_zip(
    path: &Path,
    target: &str,
    platform: &str,
    arch: &str,
    manifest_override: Option<(&str, &str)>,
    mutation: Option<PublishedZipMutation>,
) {
    let file = fs::File::create(path).expect("create zip file");
    let mut zip = zip::ZipWriter::new(file);
    let (manifest_path, payload_files) = release_zip_payload(platform, mutation);
    let mut manifest = json!({
        "app": "wavecrate",
        "channel": "stable",
        "target": target,
        "platform": platform,
        "arch": arch,
        "version": "19.1.0",
        "target_version": "19.1.0",
        "commit": "abcdef1",
        "build_date": "2026-07-02",
        "files": payload_files.iter().chain(["update-manifest.json"].iter()).collect::<Vec<_>>()
    });
    if let Some((key, value)) = manifest_override {
        manifest[key] = json!(value);
    }
    if mutation == Some(PublishedZipMutation::MacosDirectoryEntries) {
        for directory in [
            "wavecrate/Wavecrate.app/",
            "wavecrate/Wavecrate.app/Contents/",
            "wavecrate/Wavecrate.app/Contents/MacOS/",
            "wavecrate/Wavecrate.app/Contents/Resources/",
            "wavecrate/Wavecrate.app/Contents/_CodeSignature/",
        ] {
            zip.add_directory(directory, SimpleFileOptions::default())
                .expect("add app bundle directory");
        }
    }
    for file_name in payload_files {
        let archive_path = if mutation == Some(PublishedZipMutation::FlattenWindows) {
            file_name.to_string()
        } else if mutation == Some(PublishedZipMutation::WindowsBackslashes) {
            format!("wavecrate\\{file_name}")
        } else {
            format!("wavecrate/{file_name}")
        };
        zip.start_file(archive_path, SimpleFileOptions::default())
            .expect("start payload file");
        zip.write_all(format!("fixture payload for {file_name}\n").as_bytes())
            .expect("write payload file");
    }
    zip.start_file(manifest_path, SimpleFileOptions::default())
        .expect("start manifest file");
    zip.write_all(
        serde_json::to_string(&manifest)
            .expect("serialize manifest")
            .as_bytes(),
    )
    .expect("write manifest");
    zip.finish().expect("finish zip");
}

fn release_zip_payload(
    platform: &str,
    mutation: Option<PublishedZipMutation>,
) -> (&'static str, Vec<&'static str>) {
    match (platform, mutation) {
        ("windows", Some(PublishedZipMutation::FlattenWindows)) => {
            ("update-manifest.json", vec!["wavecrate.exe"])
        }
        ("windows", Some(PublishedZipMutation::MissingWindowsExecutable)) => {
            ("wavecrate/update-manifest.json", vec![])
        }
        ("windows", Some(PublishedZipMutation::WindowsBackslashes)) => {
            ("wavecrate\\update-manifest.json", vec!["wavecrate.exe"])
        }
        ("windows", _) => ("wavecrate/update-manifest.json", vec!["wavecrate.exe"]),
        ("macos", _) => (
            "wavecrate/update-manifest.json",
            vec![
                "Wavecrate.app/Contents/Info.plist",
                "Wavecrate.app/Contents/MacOS/wavecrate",
            ],
        ),
        _ => ("wavecrate/update-manifest.json", vec!["wavecrate"]),
    }
}

fn sha256_file(path: &Path) -> String {
    format!(
        "{:x}",
        Sha256::digest(fs::read(path).expect("read file for sha256"))
    )
}

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn repo_path(relative: &str) -> std::path::PathBuf {
    repo_root().join(relative)
}

fn run_success(command: &mut Command) {
    let output = command.output().expect("run command");
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_failure(command: &mut Command) -> std::process::Output {
    let output = command.output().expect("run command");
    assert!(
        !output.status.success(),
        "command should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
