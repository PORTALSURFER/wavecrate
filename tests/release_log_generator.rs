//! Script-level checks for structured RC and stable release logs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

#[test]
fn rc_first_release_log_uses_previous_stable_boundary_and_manual_notes() {
    let repo = FixtureRepo::new();
    repo.commit("initial stable");
    repo.tag("v19.0.0");
    let target_sha = repo.commit("add sampler polishing");
    let release_dir = repo.release_dir();
    write_release_files(
        &release_dir,
        "wavecrate-0.19.1-rc.1-windows-x86_64.zip",
        "checksums-0.19.1-rc.1.txt",
    );

    let out = release_dir.join("release-log.md");
    run_generator(
        &repo,
        &[
            "--channel",
            "rc",
            "--version",
            "0.19.1-rc.1",
            "--target-version",
            "0.19.1",
            "--target-sha",
            &target_sha,
            "--target-branch",
            "release/0.19",
            "--build-date",
            "2026-07-02",
            "--artifact-dir",
            release_dir.to_str().expect("release dir utf-8"),
            "--checksum-name",
            "checksums-0.19.1-rc.1.txt",
            "--checksum-sig-name",
            "checksums-0.19.1-rc.1.txt.sig",
            "--rc-number",
            "1",
            "--release-tag",
            "v0.19.1-rc.1",
            "--out",
            out.to_str().expect("output path utf-8"),
        ],
        Some("Manual review note."),
    );

    let log = fs::read_to_string(out).expect("read generated log");
    assert!(log.contains("# Wavecrate 0.19.1-rc.1"));
    assert!(log.contains("- Channel: Release Candidate"));
    assert!(log.contains("- Target branch: release/0.19"));
    assert!(log.contains("- RC number: 1"));
    assert!(log.contains("- Previous release boundary: v19.0.0"));
    assert!(log.contains("- windows / x86_64: `wavecrate-0.19.1-rc.1-windows-x86_64.zip`"));
    assert!(log.contains("- Checksums: `checksums-0.19.1-rc.1.txt`"));
    assert!(log.contains("- Signature: `checksums-0.19.1-rc.1.txt.sig`"));
    assert!(log.contains("## Manual Notes"));
    assert!(log.contains("Manual review note."));
    assert!(log.contains("## Generated Changes"));
    assert!(log.contains("- add sampler polishing"));
}

#[test]
fn rc_release_log_prefers_nearest_pre_one_stable_over_legacy_version_sort() {
    let repo = FixtureRepo::new();
    repo.commit("legacy stable");
    repo.tag("v19.1.0");
    repo.commit("publish pre-one stable");
    repo.tag("v0.19.1");
    let target_sha = repo.commit("add current train change");
    let release_dir = repo.release_dir();
    write_release_files(
        &release_dir,
        "wavecrate-0.20.0-rc.1-windows-x86_64.zip",
        "checksums-0.20.0-rc.1.txt",
    );

    let out = release_dir.join("release-log.md");
    run_generator(
        &repo,
        &[
            "--channel",
            "rc",
            "--version",
            "0.20.0-rc.1",
            "--target-version",
            "0.20.0",
            "--target-sha",
            &target_sha,
            "--target-branch",
            "release/0.20",
            "--build-date",
            "2026-07-21",
            "--artifact-dir",
            release_dir.to_str().expect("release dir utf-8"),
            "--checksum-name",
            "checksums-0.20.0-rc.1.txt",
            "--checksum-sig-name",
            "checksums-0.20.0-rc.1.txt.sig",
            "--rc-number",
            "1",
            "--release-tag",
            "v0.20.0-rc.1",
            "--out",
            out.to_str().expect("output path utf-8"),
        ],
        None,
    );

    let log = fs::read_to_string(out).expect("read generated log");
    assert!(log.contains("- Previous release boundary: v0.19.1"));
    assert!(log.contains("- add current train change"));
    assert!(!log.contains("- publish pre-one stable"));
}

#[test]
fn later_rc_release_log_uses_previous_rc_boundary() {
    let repo = FixtureRepo::new();
    repo.commit("initial stable");
    repo.tag("v19.0.0");
    repo.commit("prepare rc one");
    repo.tag("v0.19.1-rc.1");
    let target_sha = repo.commit("fix rc two blocker");
    let release_dir = repo.release_dir();
    write_release_files(
        &release_dir,
        "wavecrate-0.19.1-rc.2-macos-aarch64.zip",
        "checksums-0.19.1-rc.2.txt",
    );

    let out = release_dir.join("release-log.md");
    run_generator(
        &repo,
        &[
            "--channel",
            "rc",
            "--version",
            "0.19.1-rc.2",
            "--target-version",
            "0.19.1",
            "--target-sha",
            &target_sha,
            "--target-branch",
            "release/0.19",
            "--build-date",
            "2026-07-02",
            "--artifact-dir",
            release_dir.to_str().expect("release dir utf-8"),
            "--checksum-name",
            "checksums-0.19.1-rc.2.txt",
            "--checksum-sig-name",
            "checksums-0.19.1-rc.2.txt.sig",
            "--rc-number",
            "2",
            "--release-tag",
            "v0.19.1-rc.2",
            "--out",
            out.to_str().expect("output path utf-8"),
        ],
        None,
    );

    let log = fs::read_to_string(out).expect("read generated log");
    assert!(log.contains("- Previous release boundary: v0.19.1-rc.1"));
    assert!(log.contains("- fix rc two blocker"));
    assert!(!log.contains("- prepare rc one"));
}

#[test]
fn stable_release_log_records_promoted_rc_and_previous_stable_boundary() {
    let repo = FixtureRepo::new();
    repo.commit("initial stable");
    repo.tag("v19.0.0");
    repo.commit("prepare final train");
    let target_sha = repo.commit("final release polish");
    repo.tag("v0.19.1-rc.2");
    let release_dir = repo.release_dir();
    write_release_files(
        &release_dir,
        "wavecrate-0.19.1-macos-x86_64.zip",
        "checksums-0.19.1.txt",
    );

    let out = release_dir.join("release-log.md");
    run_generator(
        &repo,
        &[
            "--channel",
            "stable",
            "--version",
            "0.19.1",
            "--target-sha",
            &target_sha,
            "--target-branch",
            "release/0.19",
            "--build-date",
            "2026-07-02",
            "--artifact-dir",
            release_dir.to_str().expect("release dir utf-8"),
            "--checksum-name",
            "checksums-0.19.1.txt",
            "--checksum-sig-name",
            "checksums-0.19.1.txt.sig",
            "--promoted-rc-tag",
            "v0.19.1-rc.2",
            "--release-tag",
            "v0.19.1",
            "--out",
            out.to_str().expect("output path utf-8"),
        ],
        None,
    );

    let log = fs::read_to_string(out).expect("read generated log");
    assert!(log.contains("# Wavecrate 0.19.1"));
    assert!(log.contains("- Channel: Stable"));
    assert!(log.contains("- Promoted from: v0.19.1-rc.2"));
    assert!(log.contains("- Previous release boundary: v19.0.0"));
    assert!(log.contains("- macos / x86_64: `wavecrate-0.19.1-macos-x86_64.zip`"));
    assert!(log.contains("- prepare final train"));
    assert!(log.contains("- final release polish"));
}

fn run_generator(repo: &FixtureRepo, args: &[&str], release_notes: Option<&str>) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/internal/release/generate_release_log.sh");
    let mut command = Command::new("bash");
    command
        .arg(script)
        .args(args)
        .current_dir(repo.path())
        .env("WAVECRATE_RELEASE_LOG_DISABLE_GIT_CLIFF", "1");
    if let Some(notes) = release_notes {
        command.env("RELEASE_NOTES", notes);
    }
    let output = command.output().expect("run release log generator");
    assert!(
        output.status.success(),
        "release log generator failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_release_files(release_dir: &Path, zip_name: &str, checksum_name: &str) {
    fs::create_dir_all(release_dir).expect("create release dir");
    fs::write(release_dir.join(zip_name), "zip fixture").expect("write zip fixture");
    fs::write(
        release_dir.join(checksum_name),
        format!("abc123  {zip_name}\n"),
    )
    .expect("write checksum fixture");
}

struct FixtureRepo {
    temp: TempDir,
}

impl FixtureRepo {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("create fixture repo");
        let repo = Self { temp };
        repo.git(&["init"]);
        repo.git(&["config", "user.email", "release-tests@example.invalid"]);
        repo.git(&["config", "user.name", "Release Tests"]);
        repo
    }

    fn path(&self) -> &Path {
        self.temp.path()
    }

    fn release_dir(&self) -> PathBuf {
        self.path().join("dist/release")
    }

    fn commit(&self, message: &str) -> String {
        let file_name = format!("{}.txt", message.replace(' ', "-"));
        fs::write(self.path().join(file_name), message).expect("write commit fixture");
        self.git(&["add", "."]);
        self.git(&["commit", "-m", message]);
        self.git_stdout(&["rev-parse", "HEAD"])
    }

    fn tag(&self, tag: &str) {
        self.git(&["tag", tag]);
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.path())
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_stdout(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.path())
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("git stdout utf-8")
            .trim()
            .to_string()
    }
}
