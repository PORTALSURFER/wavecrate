//! Run contract and startup milestone recording for machine-readable run artifacts.

use sempal::app_dirs;
use sempal::gui_runtime::{NativeShutdownTimingArtifact, NativeStartupTimingArtifact};
use std::{
    fmt, fs,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

mod artifacts;
mod storage;
#[cfg(test)]
mod tests;

use artifacts::{
    RunContractEvent, RunContractManifest, RunContractMilestone, RunContractShutdownTiming,
    RunContractStartupTiming,
};

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
/// Startup milestone emitted when detailed native startup timing is exported.
pub(crate) const MILESTONE_NATIVE_STARTUP_TIMING: &str = "native_startup_timing";
/// Shutdown milestone emitted when detailed native shutdown timing is exported.
pub(crate) const MILESTONE_NATIVE_SHUTDOWN_TIMING: &str = "native_shutdown_timing";
const BUILD_GIT_SHA: Option<&str> = option_env!("SEMPAL_BUILD_GIT_SHA");

/// Writable manifest and event trace writer for a single application run.
pub(crate) struct RunContract {
    run_id: String,
    git_sha: String,
    persistence_mode: String,
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
    startup_timing: Option<RunContractStartupTiming>,
    shutdown_timing: Option<RunContractShutdownTiming>,
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
        let persistence = app_dirs::resolve_persistence()
            .map_err(|err| format!("failed to resolve persistence mode: {err}"))?;

        let cfg_path = persistence.app_root.to_string_lossy().into_owned();
        let log_path = persistence.app_root.join("logs");
        fs::create_dir_all(&log_path)
            .map_err(|err| format!("failed to prepare log path {}: {err}", log_path.display()))?;
        let log_path = log_path.to_string_lossy().into_owned();

        let contract_dir = Path::new(&log_path).join("contracts");
        let artifact_path = contract_dir.join(format!("run_contract_{run_id}.ndjson"));
        let manifest_path = contract_dir.join(format!("run_manifest_{run_id}.json"));

        Ok(Self {
            run_id,
            git_sha: resolve_git_sha(),
            persistence_mode: persistence.mode.to_string(),
            cfg_path,
            log_path,
            artifact_path,
            manifest_path,
            milestones: Vec::new(),
            startup_timing: None,
            shutdown_timing: None,
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
        self.write_event(
            startup_phase,
            milestone,
            exit_status,
            timestamp,
            None,
            None,
            true,
        );
    }

    /// Records the structured native startup timing payload into the NDJSON artifact.
    pub(crate) fn record_startup_timing(
        &mut self,
        startup_timing: &NativeStartupTimingArtifact,
        exit_status: &str,
    ) {
        let timestamp = UtcTimestamp::now().to_string();
        let startup_timing = RunContractStartupTiming::from(startup_timing);
        self.startup_timing = Some(startup_timing.clone());
        self.write_event(
            RUN_PHASE_STARTUP,
            MILESTONE_NATIVE_STARTUP_TIMING,
            exit_status,
            timestamp,
            Some(startup_timing),
            None,
            false,
        );
    }

    /// Records the structured native shutdown timing payload into the NDJSON artifact.
    pub(crate) fn record_shutdown_timing(
        &mut self,
        shutdown_timing: &NativeShutdownTimingArtifact,
        exit_status: &str,
    ) {
        let timestamp = UtcTimestamp::now().to_string();
        let shutdown_timing = RunContractShutdownTiming::from(shutdown_timing);
        self.shutdown_timing = Some(shutdown_timing.clone());
        self.write_event(
            RUN_PHASE_SHUTDOWN,
            MILESTONE_NATIVE_SHUTDOWN_TIMING,
            exit_status,
            timestamp,
            None,
            Some(shutdown_timing),
            false,
        );
    }

    fn write_event(
        &mut self,
        startup_phase: &str,
        milestone: &str,
        exit_status: &str,
        timestamp: String,
        startup_timing: Option<RunContractStartupTiming>,
        shutdown_timing: Option<RunContractShutdownTiming>,
        track_milestone: bool,
    ) {
        let event = RunContractEvent {
            run_id: self.run_id.clone(),
            git_sha: self.git_sha.clone(),
            persistence_mode: self.persistence_mode.clone(),
            cfg_path: self.cfg_path.clone(),
            log_path: self.log_path.clone(),
            startup_phase: startup_phase.to_string(),
            milestone: milestone.to_string(),
            exit_status: exit_status.to_string(),
            timestamp_utc: timestamp.clone(),
            process_id: self.process_id,
            manifest_path: self.manifest_path.to_string_lossy().into_owned(),
            artifact_path: self.artifact_path.to_string_lossy().into_owned(),
            startup_timing,
            shutdown_timing,
        };

        storage::append_event(&self.artifact_path, &self.run_id, &event);
        if track_milestone {
            self.milestones.push(RunContractMilestone {
                name: milestone.to_string(),
                startup_phase: startup_phase.to_string(),
                status: exit_status.to_string(),
                timestamp_utc: timestamp,
            });
        }
    }

    /// Persists the run manifest JSON and closes the contract.
    pub(crate) fn finish(&self, exit_status: &str) {
        let manifest = RunContractManifest {
            run_id: self.run_id.clone(),
            git_sha: self.git_sha.clone(),
            persistence_mode: self.persistence_mode.clone(),
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
            startup_timing: self.startup_timing.clone(),
            shutdown_timing: self.shutdown_timing.clone(),
        };

        storage::write_manifest(&self.manifest_path, &self.run_id, &manifest);
    }

    /// Return the stable run identifier for this contract instance.
    pub(crate) fn run_id(&self) -> &str {
        &self.run_id
    }

    /// Return the manifest path written by this contract instance.
    pub(crate) fn manifest_path(&self) -> &Path {
        &self.manifest_path
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
}

impl fmt::Display for UtcTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
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
    if let Ok(git_sha) = std::env::var("SEMPAL_GIT_SHA")
        && let Some(trimmed) = trim_nonempty(&git_sha)
    {
        return trimmed.to_string();
    }

    if let Some(git_sha) = BUILD_GIT_SHA.and_then(trim_nonempty) {
        return git_sha.to_string();
    }

    String::from("<unknown>")
}

fn trim_nonempty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
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
