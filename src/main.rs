#![deny(missing_docs)]
#![deny(warnings)]
#![allow(clippy::too_many_arguments)]

//! Entry point for the native Vello-based Sempal UI.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use sempal::app_core::ui::MIN_VIEWPORT_SIZE;
use sempal::app_dirs;
use sempal::gui_runtime::{NativeRunOptions, run_native_vello_app_declarative_with_artifacts};
use sempal::gui_test::{GuiFixtureBridge, GuiTestModeConfig};
use sempal::logging::{self, ActionDebugEvent, emit_action_debug_event};
use std::any::Any;
use std::ffi::OsString;
use std::panic::{self, AssertUnwindSafe};
use std::process;
use std::time::{Instant, SystemTime};
use tracing::{error, info};

use run_contract::{
    MILESTONE_RUNTIME_EXIT, MILESTONE_RUNTIME_STARTED, MILESTONE_STARTUP_BEGIN,
    MILESTONE_STARTUP_FAILED, RUN_PHASE_RUNTIME, RUN_PHASE_SHUTDOWN, RUN_PHASE_STARTUP,
    start_contract,
};

mod app_icon;
mod run_contract;

fn main() -> Result<(), String> {
    logging::install_panic_hook();
    let args: Vec<OsString> = std::env::args_os().collect();
    let startup_started_at = Instant::now();

    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    if log_console_requested(&args) {
        enable_windows_console();
    }

    if let Err(err) = logging::init(args.iter().cloned()) {
        error!("logging disabled: {err}");
    }

    let exe = std::env::current_exe()
        .ok()
        .and_then(|path| path.into_os_string().into_string().ok())
        .unwrap_or_else(|| String::from("<unknown>"));
    let cwd = std::env::current_dir()
        .map(|cwd| cwd.to_string_lossy().into_owned())
        .unwrap_or_else(|_| String::from("<unknown>"));
    let mut contract = start_contract(&exe, &cwd, args.len(), cfg!(debug_assertions));
    let now = SystemTime::now();
    info!(
        pid = process::id(),
        exe = exe,
        cwd = cwd,
        arg_count = args.len(),
        timestamp = ?now,
        debug = cfg!(debug_assertions),
        "sempal startup: process metadata captured"
    );
    info!("sempal startup: logging initialized");
    match app_dirs::resolve_persistence() {
        Ok(persistence) => {
            info!(
                persistence_mode = %persistence.mode,
                config_base = %persistence.config_base.display(),
                app_root = %persistence.app_root.display(),
                "sempal startup: persistence profile resolved"
            );
            let mode = persistence.mode.to_string();
            emit_action_debug_event(ActionDebugEvent {
                action: "runtime.startup.resolve_persistence",
                pane: Some("background"),
                source: Some(&mode),
                outcome: "success",
                elapsed: startup_started_at.elapsed(),
                error: None,
            });
        }
        Err(err) => {
            error!(err = %err, "sempal startup: failed to resolve persistence profile");
            let error = err.to_string();
            emit_action_debug_event(ActionDebugEvent {
                action: "runtime.startup.resolve_persistence",
                pane: Some("background"),
                source: None,
                outcome: "error",
                elapsed: startup_started_at.elapsed(),
                error: Some(&error),
            });
        }
    }

    let mut runtime_started = false;
    let run_result = panic::catch_unwind(AssertUnwindSafe(|| {
        run_application(&mut contract, &mut runtime_started)
    }));

    match run_result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(err)) => Err(err),
        Err(payload) => {
            let message = panic_payload_to_string(payload);
            error!("sempal startup: panic captured while running: {message}");
            if let Some(contract) = &mut contract {
                if runtime_started {
                    contract.record(RUN_PHASE_SHUTDOWN, MILESTONE_RUNTIME_EXIT, "error");
                } else {
                    contract.record(RUN_PHASE_STARTUP, MILESTONE_STARTUP_FAILED, "error");
                }
                contract.finish("error");
            }
            Err(format!("startup panic: {message}"))
        }
    }
}

fn run_application(
    contract: &mut Option<run_contract::RunContract>,
    runtime_started: &mut bool,
) -> Result<(), String> {
    let startup_started_at = Instant::now();
    if let Some(contract) = contract {
        contract.record(RUN_PHASE_STARTUP, MILESTONE_STARTUP_BEGIN, "running");
    }

    let gui_test_mode = GuiTestModeConfig::from_env(
        contract.as_ref().map(|value| value.run_id()),
        contract
            .as_ref()
            .map(|value| value.manifest_path().to_path_buf()),
    );

    let mut options = NativeRunOptions {
        title: String::from("Sempal"),
        inner_size: None,
        min_inner_size: Some(MIN_VIEWPORT_SIZE),
        maximized: true,
        decorations: true,
        target_fps: 120,
        icon: app_icon::load_app_icon(),
    };
    if let Some(config) = gui_test_mode.as_ref() {
        options.title = String::from("Sempal GUI Test");
        config.apply_to_run_options(&mut options);
    }

    let fixture_tag = gui_test_mode
        .as_ref()
        .map(|config| config.fixture_tag.clone())
        .unwrap_or_else(|| String::from("live"));
    let viewport = gui_test_mode
        .as_ref()
        .map(|config| config.viewport)
        .unwrap_or([680, 260]);
    info!(
        fixture_tag,
        ?viewport,
        "sempal startup: preparing GUI bridge"
    );
    emit_action_debug_event(ActionDebugEvent {
        action: "runtime.startup.prepare_gui_bridge",
        pane: Some("background"),
        source: Some(&fixture_tag),
        outcome: "running",
        elapsed: startup_started_at.elapsed(),
        error: None,
    });
    let mut bridge = match GuiFixtureBridge::new_with_viewport(&fixture_tag, viewport) {
        Ok(bridge) => bridge,
        Err(err) => {
            error!(err = %err, "sempal startup: failed to construct native bridge");
            emit_action_debug_event(ActionDebugEvent {
                action: "runtime.startup.prepare_gui_bridge",
                pane: Some("background"),
                source: Some(&fixture_tag),
                outcome: "error",
                elapsed: startup_started_at.elapsed(),
                error: Some(&err),
            });
            if let Some(contract) = contract {
                contract.record(RUN_PHASE_STARTUP, MILESTONE_STARTUP_FAILED, "error");
                contract.finish("error");
            }
            return Err(err);
        }
    };
    if let Some(config) = gui_test_mode {
        bridge.install_gui_test_mode(config);
    }

    info!("sempal startup: native bridge constructed");
    emit_action_debug_event(ActionDebugEvent {
        action: "runtime.startup.prepare_gui_bridge",
        pane: Some("background"),
        source: Some(&fixture_tag),
        outcome: "success",
        elapsed: startup_started_at.elapsed(),
        error: None,
    });
    if let Some(contract) = contract {
        contract.record(RUN_PHASE_RUNTIME, MILESTONE_RUNTIME_STARTED, "running");
    }
    *runtime_started = true;

    let report = run_native_vello_app_declarative_with_artifacts(options, bridge);
    let runtime_elapsed = startup_started_at.elapsed();
    let exit_status = match &report.result {
        Ok(_) => {
            info!("sempal startup: native runtime exited normally");
            emit_action_debug_event(ActionDebugEvent {
                action: "runtime.exit.native_runtime",
                pane: Some("background"),
                source: Some(&fixture_tag),
                outcome: "success",
                elapsed: runtime_elapsed,
                error: None,
            });
            String::from("success")
        }
        Err(err) => {
            error!(err = %err, "sempal startup: runtime exited with error");
            emit_action_debug_event(ActionDebugEvent {
                action: "runtime.exit.native_runtime",
                pane: Some("background"),
                source: Some(&fixture_tag),
                outcome: "error",
                elapsed: runtime_elapsed,
                error: Some(err),
            });
            String::from("error")
        }
    };

    if let Some(contract) = contract {
        if let Some(startup_timing) = report.artifacts.startup_timing.as_ref() {
            contract.record_startup_timing(startup_timing, &exit_status);
        }
        if let Some(shutdown_timing) = report.artifacts.shutdown_timing.as_ref() {
            contract.record_shutdown_timing(shutdown_timing, &exit_status);
        }
        contract.record(RUN_PHASE_SHUTDOWN, MILESTONE_RUNTIME_EXIT, &exit_status);
        contract.finish(&exit_status);
    }

    report.result
}

fn panic_payload_to_string(panic_payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = panic_payload.downcast_ref::<&str>() {
        return message.to_string();
    }
    if let Some(message) = panic_payload.downcast_ref::<String>() {
        return message.clone();
    }

    "<non-string panic payload>".to_string()
}

#[cfg(all(target_os = "windows", not(debug_assertions)))]
fn log_console_requested(args: &[OsString]) -> bool {
    args.iter().any(|arg| {
        arg == &OsString::from(logging::DEBUG_LOGGING_SHORT_ARG)
            || arg == &OsString::from(logging::DEBUG_LOGGING_ARG)
    })
}

#[cfg(all(target_os = "windows", not(debug_assertions)))]
fn enable_windows_console() {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        OPEN_EXISTING,
    };
    use windows::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AllocConsole, AttachConsole, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
        SetStdHandle,
    };

    unsafe {
        let attached = AttachConsole(ATTACH_PARENT_PROCESS).is_ok();
        if !attached {
            let _ = AllocConsole();
        }

        let Ok(handle) = CreateFileW(
            windows::core::w!("CONOUT$"),
            FILE_GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        ) else {
            return;
        };

        let handle = HANDLE(handle.0);
        let _ = SetStdHandle(STD_OUTPUT_HANDLE, handle);
        let _ = SetStdHandle(STD_ERROR_HANDLE, handle);
    }
}
