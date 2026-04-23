//! Run contract and startup milestone recording for machine-readable run artifacts.

use sempal::app_dirs;
use sempal::gui_runtime::NativeStartupTimingArtifact;
use serde::Serialize;
use std::{
    fmt,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
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
/// Startup milestone emitted when detailed native startup timing is exported.
pub(crate) const MILESTONE_NATIVE_STARTUP_TIMING: &str = "native_startup_timing";
const BUILD_GIT_SHA: Option<&str> = option_env!("SEMPAL_BUILD_GIT_SHA");

#[derive(Serialize)]
struct RunContractEvent {
    run_id: String,
    git_sha: String,
    persistence_mode: String,
    cfg_path: String,
    log_path: String,
    startup_phase: String,
    milestone: String,
    exit_status: String,
    timestamp_utc: String,
    process_id: u32,
    manifest_path: String,
    artifact_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    startup_timing: Option<RunContractStartupTiming>,
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
    persistence_mode: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    startup_timing: Option<RunContractStartupTiming>,
}

#[derive(Clone, Serialize)]
struct RunContractStartupTiming {
    status: String,
    failure_reason: Option<String>,
    window_create_ms: Option<f64>,
    window_revealed_ms: Option<f64>,
    wgpu_surface_create_ms: Option<f64>,
    wgpu_device_ready_ms: Option<f64>,
    surface_ready_ms: Option<f64>,
    renderer_build_ms: Option<f64>,
    renderer_ready_ms: Option<f64>,
    first_scene_ready_ms: Option<f64>,
    first_redraw_started_ms: Option<f64>,
    first_present_draw_ms: Option<f64>,
    first_present_ms: Option<f64>,
    deferred_model_refresh_ms: Option<f64>,
    deferred_model_refresh_total_ms: Option<f64>,
}

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
}

impl From<&NativeStartupTimingArtifact> for RunContractStartupTiming {
    fn from(value: &NativeStartupTimingArtifact) -> Self {
        Self {
            status: value.status.clone(),
            failure_reason: value.failure_reason.clone(),
            window_create_ms: value.window_create_ms,
            window_revealed_ms: value.window_revealed_ms,
            wgpu_surface_create_ms: value.wgpu_surface_create_ms,
            wgpu_device_ready_ms: value.wgpu_device_ready_ms,
            surface_ready_ms: value.surface_ready_ms,
            renderer_build_ms: value.renderer_build_ms,
            renderer_ready_ms: value.renderer_ready_ms,
            first_scene_ready_ms: value.first_scene_ready_ms,
            first_redraw_started_ms: value.first_redraw_started_ms,
            first_present_draw_ms: value.first_present_draw_ms,
            first_present_ms: value.first_present_ms,
            deferred_model_refresh_ms: value.deferred_model_refresh_ms,
            deferred_model_refresh_total_ms: value.deferred_model_refresh_total_ms,
        }
    }
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
        self.write_event(startup_phase, milestone, exit_status, timestamp, None, true);
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
        };

        self.append_event(&event);
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
        };

        let Ok(serialized) = serde_json::to_string_pretty(&manifest) else {
            error!(
                "[run_contract] failed to serialize run manifest for run_id {}",
                self.run_id
            );
            return;
        };

        if let Some(parent) = self.manifest_path.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            error!(
                "[run_contract] failed to ensure manifest directory {}: {err}",
                parent.display()
            );
            return;
        }

        if let Err(err) = fs::write(&self.manifest_path, serialized) {
            error!(
                "[run_contract] failed to write {}: {err}",
                self.manifest_path.display()
            );
        }
    }

    /// Return the stable run identifier for this contract instance.
    pub(crate) fn run_id(&self) -> &str {
        &self.run_id
    }

    /// Return the manifest path written by this contract instance.
    pub(crate) fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    fn append_event(&self, event: &RunContractEvent) {
        let Ok(serialized) = serde_json::to_string(event) else {
            error!(
                "[run_contract] failed to serialize run contract event for run_id {}",
                self.run_id
            );
            return;
        };

        if let Some(parent) = self.artifact_path.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            error!(
                "[run_contract] failed to ensure artifact directory {}: {err}",
                parent.display()
            );
            return;
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

#[cfg(test)]
mod tests {
    use super::RunContract;
    use sempal::app_dirs::{ConfigBaseGuard, PersistenceProfileGuard};
    use sempal::gui_runtime::NativeStartupTimingArtifact;
    use serde_json::Value;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn run_contract_id_is_nonempty() {
        let id = super::make_run_contract_id();
        assert!(!id.trim().is_empty());
    }

    #[test]
    fn can_start_contract_in_test_dir() {
        let base = match tempdir() {
            Ok(base) => base,
            Err(err) => panic!("create temp config dir: {err}"),
        };
        let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
        let _profile_guard = PersistenceProfileGuard::live();
        let contract =
            RunContract::start("./target/app", "/tmp", 0, true).expect("contract should start");
        assert!(!contract.run_id.is_empty());
    }

    #[test]
    fn successful_startup_timing_is_written_into_contract_artifacts() {
        let base = tempdir().expect("create temp config dir");
        let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
        let _profile_guard = PersistenceProfileGuard::live();
        let mut contract =
            RunContract::start("./target/app", "/tmp", 0, true).expect("contract should start");
        contract.record(
            super::RUN_PHASE_STARTUP,
            super::MILESTONE_STARTUP_BEGIN,
            "running",
        );
        contract.record_startup_timing(
            &NativeStartupTimingArtifact {
                status: String::from("complete"),
                failure_reason: None,
                window_create_ms: Some(10.0),
                window_revealed_ms: Some(14.0),
                wgpu_surface_create_ms: Some(3.0),
                wgpu_device_ready_ms: Some(4.0),
                surface_ready_ms: Some(18.0),
                renderer_build_ms: Some(5.0),
                renderer_ready_ms: Some(23.0),
                first_scene_ready_ms: Some(30.0),
                first_redraw_started_ms: Some(31.0),
                first_present_draw_ms: Some(2.0),
                first_present_ms: Some(33.0),
                deferred_model_refresh_ms: Some(0.0),
                deferred_model_refresh_total_ms: Some(33.0),
            },
            "success",
        );
        contract.record(
            super::RUN_PHASE_SHUTDOWN,
            super::MILESTONE_RUNTIME_EXIT,
            "success",
        );
        let artifact_path = contract.artifact_path.clone();
        let manifest_path = contract.manifest_path.clone();
        contract.finish("success");

        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(manifest_path).expect("read manifest"))
                .expect("parse manifest");
        assert_eq!(manifest["startup_timing"]["status"], "complete");
        assert_eq!(manifest["startup_timing"]["first_present_ms"], 33.0);

        let events = fs::read_to_string(artifact_path).expect("read artifact");
        let timing_event = events
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("parse event"))
            .find(|event| event["milestone"] == super::MILESTONE_NATIVE_STARTUP_TIMING)
            .expect("startup timing event");
        assert_eq!(timing_event["startup_timing"]["window_revealed_ms"], 14.0);
    }

    #[test]
    fn incomplete_startup_timing_preserves_failure_reason_in_contract_artifacts() {
        let base = tempdir().expect("create temp config dir");
        let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
        let _profile_guard = PersistenceProfileGuard::live();
        let mut contract =
            RunContract::start("./target/app", "/tmp", 0, true).expect("contract should start");
        contract.record_startup_timing(
            &NativeStartupTimingArtifact {
                status: String::from("incomplete"),
                failure_reason: Some(String::from("startup_exited_before_first_present")),
                window_create_ms: Some(8.0),
                window_revealed_ms: None,
                wgpu_surface_create_ms: Some(1.5),
                wgpu_device_ready_ms: Some(3.0),
                surface_ready_ms: Some(12.0),
                renderer_build_ms: None,
                renderer_ready_ms: None,
                first_scene_ready_ms: None,
                first_redraw_started_ms: None,
                first_present_draw_ms: None,
                first_present_ms: None,
                deferred_model_refresh_ms: None,
                deferred_model_refresh_total_ms: None,
            },
            "error",
        );
        let manifest_path = contract.manifest_path.clone();
        contract.finish("error");

        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(manifest_path).expect("read manifest"))
                .expect("parse manifest");
        assert_eq!(manifest["startup_timing"]["status"], "incomplete");
        assert_eq!(
            manifest["startup_timing"]["failure_reason"],
            "startup_exited_before_first_present"
        );
        assert!(manifest["startup_timing"]["first_present_ms"].is_null());
    }
}
