use super::RunContract;
use sempal::app_dirs::{ConfigBaseGuard, PersistenceProfileGuard};
use sempal::gui_runtime::{NativeShutdownTimingArtifact, NativeStartupTimingArtifact};
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

#[test]
fn successful_shutdown_timing_is_written_into_contract_artifacts() {
    let base = tempdir().expect("create temp config dir");
    let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
    let _profile_guard = PersistenceProfileGuard::live();
    let mut contract =
        RunContract::start("./target/app", "/tmp", 0, true).expect("contract should start");
    contract.record_shutdown_timing(
        &NativeShutdownTimingArtifact {
            status: String::from("complete"),
            failure_reason: None,
            bridge_exit_flush_ms: Some(1.0),
            config_persist_ms: Some(2.0),
            controller_jobs_shutdown_ms: Some(3.0),
            analysis_shutdown_ms: Some(4.0),
            controller_shutdown_ms: Some(7.0),
            runtime_exit_total_ms: Some(8.0),
        },
        "success",
    );
    let artifact_path = contract.artifact_path.clone();
    let manifest_path = contract.manifest_path.clone();
    contract.finish("success");

    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(manifest_path).expect("read manifest"))
            .expect("parse manifest");
    assert_eq!(manifest["shutdown_timing"]["status"], "complete");
    assert_eq!(manifest["shutdown_timing"]["controller_shutdown_ms"], 7.0);

    let events = fs::read_to_string(artifact_path).expect("read artifact");
    let timing_event = events
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("parse event"))
        .find(|event| event["milestone"] == super::MILESTONE_NATIVE_SHUTDOWN_TIMING)
        .expect("shutdown timing event");
    assert_eq!(timing_event["shutdown_timing"]["analysis_shutdown_ms"], 4.0);
}

#[test]
fn degraded_shutdown_timing_preserves_failure_reason_and_skipped_phases() {
    let base = tempdir().expect("create temp config dir");
    let _base_guard = ConfigBaseGuard::set(base.path().to_path_buf());
    let _profile_guard = PersistenceProfileGuard::live();
    let mut contract =
        RunContract::start("./target/app", "/tmp", 0, true).expect("contract should start");
    contract.record_shutdown_timing(
        &NativeShutdownTimingArtifact {
            status: String::from("error"),
            failure_reason: Some(String::from("config_persist_failed")),
            bridge_exit_flush_ms: Some(1.0),
            config_persist_ms: Some(2.0),
            controller_jobs_shutdown_ms: None,
            analysis_shutdown_ms: None,
            controller_shutdown_ms: None,
            runtime_exit_total_ms: Some(3.0),
        },
        "error",
    );
    let manifest_path = contract.manifest_path.clone();
    contract.finish("error");

    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(manifest_path).expect("read manifest"))
            .expect("parse manifest");
    assert_eq!(manifest["shutdown_timing"]["status"], "error");
    assert_eq!(
        manifest["shutdown_timing"]["failure_reason"],
        "config_persist_failed"
    );
    assert!(manifest["shutdown_timing"]["analysis_shutdown_ms"].is_null());
}
