//! Run contract and startup milestone recording for machine-readable run artifacts.

use sempal::app_dirs;
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{self, Command},
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::error;

/// Startup phase used in run-contract milestones.
pub(crate) const RUN_PHASE_RUNTIME: &str = "runtime";
/// Shutdown phase used in run-contract milestones.
pub(crate) const RUN_PHASE_SHUTDOWN: &str = "shutdown";
/// Startup phase used in run-contract milestones.
pub(crate) const RUN_PHASE_STARTUP: &str = "startup";
/// Runtime milestone emitted when native app loop exits.
pub(crate) const MILESTONE_RUNTIME_EXIT: &str = "runtime_exit";
/// Runtime milestone emitted when the UI runtime starts.
pub(crate) const MILESTONE_RUNTIME_STARTED: &str = "runtime_started";
/// Startup milestone emitted before the bridge is built.
pub(crate) const MILESTONE_STARTUP_BEGIN: &str = "startup_begin";
/// Startup milestone emitted when startup fails.
pub(crate) const MILESTONE_STARTUP_FAILED: &str = "startup_failed";

#[derive(Serialize)]
struct RunContractEvent {
    run_id: String,
    git_sha: String,
    cfg_path: String,
    log_path: String,
    startup_phase: String,
    milestone: String,
    exit_status: String,
    timestamp_utc: String,
    process_id: u32,
    manifest_path: String,
    artifact_path: String,
}

#[derive(Clone, Serialize)]
struct RunContractMilestone {
    name: String,
    startup_phase: String,
    status: String,
    timestamp_utc: String,
}

#[derive(Serialize)]
struct RunContractManifest {
    run_id: String,
    git_sha: String,
    cfg_path: String,
    log_path: String,
    process_id: u32,
    executable_path: String,
    working_directory: String,
    arg_count: usize,
    debug: bool,
    started_utc: String,
    completed_utc: String,
    exit_status: String,
    artifact_path: String,
    manifest_path: String,
    milestones: Vec<RunContractMilestone>,
}

/// Writable manifest and event trace writer for a single application run.
pub(crate) struct RunContract {
    run_id: String,
    git_sha: String,
    cfg_path: String,
    log_path: String,
    executable_path: String,
    working_directory: String,
    arg_count: usize,
    debug: bool,
    process_id: u32,
    started_utc: String,
    artifact_path: PathBuf,
    manifest_path: PathBuf,
    milestones: Vec<RunContractMilestone>,
}

impl RunContract {
    /// Creates a new contract writer for this run, resolving file and runtime metadata.
    pub(crate) fn start(
        executable_path: &str,
        working_directory: &str,
        arg_count: usize,
        debug: bool,
    ) -> Result<Self, String> {
        let run_id = make_run_contract_id();

        let cfg_path = app_dirs::app_root_dir()
            .map(|path| path.to_string_lossy().into_owned())
            .map_err(|err| format!("failed to resolve cfg path: {err}"))?;

        let log_path = app_dirs::logs_dir()
            .map(|path| path.to_string_lossy().into_owned())
            .map_err(|err| format!("failed to resolve log path: {err}"))?;

        let contract_dir = Path::new(&log_path).join("contracts");
        let artifact_path = contract_dir.join(format!("run_contract_{run_id}.ndjson"));
        let manifest_path = contract_dir.join(format!("run_manifest_{run_id}.json"));

        Ok(Self {
            run_id,
            git_sha: resolve_git_sha(),
            cfg_path,
            log_path,
            artifact_path,
            manifest_path,
            milestones: Vec::new(),
            started_utc: UtcTimestamp::now().to_string(),
            process_id: process::id(),
            executable_path: executable_path.to_string(),
            working_directory: working_directory.to_string(),
            arg_count,
            debug,
        })
    }

    /// Records a milestone event into the NDJSON artifact.
    pub(crate) fn record(&mut self, startup_phase: &str, milestone: &str, exit_status: &str) {
        let timestamp = UtcTimestamp::now().to_string();
        let event = RunContractEvent {
            run_id: self.run_id.clone(),
            git_sha: self.git_sha.clone(),
            cfg_path: self.cfg_path.clone(),
            log_path: self.log_path.clone(),
            startup_phase: startup_phase.to_string(),
            milestone: milestone.to_string(),
            exit_status: exit_status.to_string(),
            timestamp_utc: timestamp.clone(),
            process_id: self.process_id,
            manifest_path: self.manifest_path.to_string_lossy().into_owned(),
            artifact_path: self.artifact_path.to_string_lossy().into_owned(),
        };

        let Ok(serialized) = serde_json::to_string(&event) else {
            error!(
                "[run_contract] failed to serialize run contract event for run_id {}",
                self.run_id
            );
            return;
        };

        if let Some(parent) = self.artifact_path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                error!(
                    "[run_contract] failed to ensure artifact directory {}: {err}",
                    parent.display()
                );
                return;
            }
        }

        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.artifact_path)
            .and_then(|mut file| writeln!(file, "{serialized}"))
        {
            Ok(()) => {}
            Err(err) => {
                error!(
                    "[run_contract] failed to write {}: {err}",
                    self.artifact_path.display()
                );
            }
        }

        self.milestones.push(RunContractMilestone {
            name: milestone.to_string(),
            startup_phase: startup_phase.to_string(),
            status: exit_status.to_string(),
            timestamp_utc: timestamp,
        });
    }

    /// Persists the run manifest JSON and closes the contract.
    pub(crate) fn finish(&self, exit_status: &str) {
        let manifest = RunContractManifest {
            run_id: self.run_id.clone(),
            git_sha: self.git_sha.clone(),
            cfg_path: self.cfg_path.clone(),
            log_path: self.log_path.clone(),
            process_id: self.process_id,
            executable_path: self.executable_path.clone(),
            working_directory: self.working_directory.clone(),
            arg_count: self.arg_count,
            debug: self.debug,
            started_utc: self.started_utc.clone(),
            completed_utc: UtcTimestamp::now().to_string(),
            exit_status: exit_status.to_string(),
            artifact_path: self.artifact_path.to_string_lossy().into_owned(),
            manifest_path: self.manifest_path.to_string_lossy().into_owned(),
            milestones: self.milestones.clone(),
        };

        let Ok(serialized) = serde_json::to_string_pretty(&manifest) else {
            error!(
                "[run_contract] failed to serialize run manifest for run_id {}",
                self.run_id
            );
            return;
        };

        if let Some(parent) = self.manifest_path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                error!(
                    "[run_contract] failed to ensure manifest directory {}: {err}",
                    parent.display()
                );
                return;
            }
        }

        if let Err(err) = fs::write(&self.manifest_path, serialized) {
            error!(
                "[run_contract] failed to write {}: {err}",
                self.manifest_path.display()
            );
        }
    }
}

/// Millisecond-free UTC timestamp wrapper used by contract artifacts.
#[derive(Clone, Copy)]
struct UtcTimestamp(u64);

impl UtcTimestamp {
    fn now() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self(now)
    }

    fn to_string(self) -> String {
        self.0.to_string()
    }
}

fn make_run_contract_id() -> String {
    let unix_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{}-{}", unix_nanos, process::id())
}

fn resolve_git_sha() -> String {
    if let Ok(git_sha) = std::env::var("SEMPAL_GIT_SHA") {
        let trimmed = git_sha.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let current_dir = std::env::current_dir().ok();
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));

    let mut candidates = Vec::new();
    if let Some(dir) = current_dir {
        candidates.push(dir);
    }
    if let Some(dir) = exe_dir {
        candidates.push(dir);
    }

    for base in candidates {
        if let Some(sha) = find_git_sha_in_tree(base.as_path()) {
            return sha;
        }
    }

    String::from("<unknown>")
}

fn find_git_sha_in_tree(base: &Path) -> Option<String> {
    let mut current = Some(base);
    while let Some(dir) = current {
        if let Some(sha) = resolve_git_sha_in_dir(dir) {
            return Some(sha);
        }
        current = dir.parent();
    }
    None
}

fn resolve_git_sha_in_dir(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let sha = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if sha.is_empty() { None } else { Some(sha) }
}

/// Creates and returns a run contract if all metadata paths can be resolved.
pub(crate) fn start_contract(
    executable_path: &str,
    working_directory: &str,
    arg_count: usize,
    debug: bool,
) -> Option<RunContract> {
    RunContract::start(executable_path, working_directory, arg_count, debug).ok()
}

#[cfg(test)]
mod tests {
    use super::RunContract;

    #[test]
    fn run_contract_id_is_nonempty() {
        let id = super::make_run_contract_id();
        assert!(!id.trim().is_empty());
    }

    #[test]
    fn can_start_contract_in_test_dir() {
        let contract =
            RunContract::start("./target/app", "/tmp", 0, true).expect("contract should start");
        assert!(!contract.run_id.is_empty());
    }
}
