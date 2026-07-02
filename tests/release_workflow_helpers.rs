//! Focused checks for shared release workflow helper scripts.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

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
    assert!(
        temp.path()
            .join("checksums-entry-windows-x86_64.txt")
            .is_file()
    );
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
