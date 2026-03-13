#![deny(missing_docs)]
#![deny(warnings)]

//! Entry point for the native Vello-based Sempal UI.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use sempal::app_core::ui::MIN_VIEWPORT_SIZE;
use sempal::gui_runtime::{NativeRunOptions, run_native_vello_app_declarative};
use sempal::gui_test::{GuiFixtureBridge, GuiTestModeConfig};
use sempal::logging;
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::process;
use std::time::SystemTime;
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

    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    if log_console_requested() {
        enable_windows_console();
    }

    if let Err(err) = logging::init() {
        error!("logging disabled: {err}");
    }

    let exe = std::env::current_exe()
        .ok()
        .and_then(|path| path.into_os_string().into_string().ok())
        .unwrap_or_else(|| String::from("<unknown>"));
    let cwd = std::env::current_dir()
        .map(|cwd| cwd.to_string_lossy().into_owned())
        .unwrap_or_else(|_| String::from("<unknown>"));
    let args: Vec<_> = std::env::args_os().collect();

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
        .map(|config| config.fixture_tag.as_str())
        .unwrap_or("default");
    let viewport = gui_test_mode
        .as_ref()
        .map(|config| config.viewport)
        .unwrap_or([680, 260]);
    info!(
        fixture_tag,
        ?viewport,
        "sempal startup: preparing GUI bridge"
    );
    let mut bridge = match GuiFixtureBridge::new_with_viewport(fixture_tag, viewport) {
        Ok(bridge) => bridge,
        Err(err) => {
            error!(err = %err, "sempal startup: failed to construct native bridge");
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
    if let Some(contract) = contract {
        contract.record(RUN_PHASE_RUNTIME, MILESTONE_RUNTIME_STARTED, "running");
    }
    *runtime_started = true;

    let result = run_native_vello_app_declarative(options, bridge);
    let exit_status = match &result {
        Ok(_) => {
            info!("sempal startup: native runtime exited normally");
            String::from("success")
        }
        Err(err) => {
            error!(err = %err, "sempal startup: runtime exited with error");
            String::from("error")
        }
    };

    if let Some(contract) = contract {
        contract.record(RUN_PHASE_SHUTDOWN, MILESTONE_RUNTIME_EXIT, &exit_status);
        contract.finish(&exit_status);
    }

    result
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
fn log_console_requested() -> bool {
    std::env::args_os().any(|arg| arg == "-log" || arg == "--log")
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
