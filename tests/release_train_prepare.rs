//! Script-level checks for release-train preparation.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

#[test]
fn prepare_release_train_updates_wavecrate_packages_and_lockfile() {
    let repo = FixtureRepo::new();
    repo.write_workspace("0.19.1");
    repo.git(&["add", "."]);
    repo.git(&["commit", "-m", "seed workspace"]);
    let output = repo.run_prepare(&[
        "--version",
        "0.20.0",
        "--source-ref",
        "HEAD",
        "--skip-release-tests",
    ]);

    assert!(
        output.status.success(),
        "prepare script failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        repo.git_stdout(&["branch", "--show-current"]),
        "release/0.20"
    );
    assert!(
        repo.read("Cargo.toml")
            .contains("name = \"wavecrate\"\nversion = \"0.20.0\"")
    );
    assert!(
        repo.read("crates/wavecrate-tool/Cargo.toml")
            .contains("name = \"wavecrate-tool\"\nversion = \"0.20.0\"")
    );
    assert!(
        repo.read("crates/reson/Cargo.toml")
            .contains("name = \"reson\"\nversion = \"0.1.0\"")
    );
    assert!(
        repo.read("tools/gui-test-cli/Cargo.toml")
            .contains("name = \"gui-test-cli\"\nversion = \"0.1.0\"")
    );
    assert!(
        repo.read("Cargo.lock")
            .contains("name = \"wavecrate\"\nversion = \"0.20.0\"")
    );
    assert!(
        repo.read("Cargo.lock")
            .contains("name = \"wavecrate-tool\"\nversion = \"0.20.0\"")
    );
    assert!(
        repo.git_stdout(&["log", "-1", "--pretty=%s"])
            .contains("Prepare Wavecrate 0.20.0 release train")
    );
}

#[test]
fn prepare_release_train_rejects_stale_prerelease_package_versions() {
    let repo = FixtureRepo::new();
    repo.write_workspace("0.19.1-alpha.1");
    repo.git(&["add", "."]);
    repo.git(&["commit", "-m", "seed prerelease workspace"]);
    let output = repo.run_prepare(&[
        "--version",
        "0.19.1",
        "--source-ref",
        "HEAD",
        "--dry-run",
        "--skip-release-tests",
    ]);

    assert!(
        !output.status.success(),
        "prepare script should reject prerelease package versions"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("stale prerelease versions"),
        "stderr should explain prerelease rejection\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

struct FixtureRepo {
    temp: TempDir,
}

impl FixtureRepo {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("create release train fixture repo");
        let repo = Self { temp };
        repo.git(&["init"]);
        repo.git(&["config", "user.email", "release-tests@example.invalid"]);
        repo.git(&["config", "user.name", "Release Tests"]);
        repo
    }

    fn path(&self) -> &Path {
        self.temp.path()
    }

    fn write_workspace(&self, wavecrate_version: &str) {
        self.write(
            "Cargo.toml",
            &format!(
                r#"[workspace]
members = [
    ".",
    "crates/wavecrate-tool",
    "crates/reson",
    "tools/gui-test-cli",
]
resolver = "3"

[package]
name = "wavecrate"
version = "{wavecrate_version}"
edition = "2024"

[dependencies]
wavecrate-tool = {{ path = "crates/wavecrate-tool" }}
"#
            ),
        );
        self.write(
            "crates/wavecrate-tool/Cargo.toml",
            &format!(
                r#"[package]
name = "wavecrate-tool"
version = "{wavecrate_version}"
edition = "2024"
"#
            ),
        );
        self.write(
            "crates/reson/Cargo.toml",
            r#"[package]
name = "reson"
version = "0.1.0"
edition = "2024"
"#,
        );
        self.write(
            "tools/gui-test-cli/Cargo.toml",
            r#"[package]
name = "gui-test-cli"
version = "0.1.0"
edition = "2024"
"#,
        );
        self.write("src/lib.rs", "");
        self.write("crates/wavecrate-tool/src/lib.rs", "");
        self.write("crates/reson/src/lib.rs", "");
        self.write("tools/gui-test-cli/src/lib.rs", "");
    }

    fn run_prepare(&self, args: &[&str]) -> std::process::Output {
        let script = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("scripts/internal/release/prepare_release_train.py");
        Command::new("python3")
            .arg(script)
            .args(args)
            .current_dir(self.path())
            .output()
            .expect("run release train prep script")
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.path().join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent");
        }
        fs::write(path, contents).expect("write fixture file");
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path().join(relative)).expect("read fixture file")
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
