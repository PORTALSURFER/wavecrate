#![deny(missing_docs)]
#![deny(warnings)]

//! Entry point for the native Vello-based Sempal UI.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use sempal::app_core::native_bridge::new_native_bridge;
use sempal::app_core::ui::MIN_VIEWPORT_SIZE;
use sempal::app_dirs;
use sempal::audio::AudioPlayer;
use sempal::gui_runtime::{run_native_vello_app, NativeRunOptions, WindowIconRgba};
use sempal::logging;
use sempal::waveform::WaveformRenderer;
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{error, info};

fn main() -> Result<(), String> {
    logging::install_panic_hook();

    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    if log_console_requested() {
        enable_windows_console();
    }

    if let Err(err) = logging::init() {
        error!("logging disabled: {err}");
    }

    let contract = RunContract::start().ok();

    let exe = std::env::current_exe()
        .ok()
        .and_then(|path| path.into_os_string().into_string().ok())
        .unwrap_or_else(|| String::from("<unknown>"));
    let cwd = std::env::current_dir()
        .map(|cwd| cwd.to_string_lossy().into_owned())
        .unwrap_or_else(|_| String::from("<unknown>"));
    let args: Vec<_> = std::env::args_os().collect();
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

    if let Some(contract) = &contract {
        contract.record("startup", "running");
    }

    let options = NativeRunOptions {
        title: String::from("Sempal"),
        inner_size: None,
        min_inner_size: Some(MIN_VIEWPORT_SIZE),
        maximized: true,
        target_fps: 120,
        icon: load_app_icon(),
    };

    let renderer = WaveformRenderer::new(680, 260);
    info!("sempal startup: waveform renderer initialized");
    let player: Option<std::rc::Rc<std::cell::RefCell<AudioPlayer>>> = None;
    let bridge = new_native_bridge(renderer, player).map_err(|err| {
        error!(err = %err, "sempal startup: failed to construct native bridge");
        err
    })?;
    info!("sempal startup: native bridge constructed");
    if let Some(contract) = &contract {
        contract.record("runtime", "running");
    }
    let result = run_native_vello_app(options, bridge);
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

    if let Some(contract) = &contract {
        contract.record("shutdown", &exit_status);
    }

    result
}

#[derive(Serialize)]
struct RunContractEvent {
    run_id: String,
    git_sha: String,
    cfg_path: String,
    log_path: String,
    startup_phase: String,
    exit_status: String,
    timestamp_utc: String,
    process_id: u32,
}

struct RunContract {
    run_id: String,
    git_sha: String,
    cfg_path: String,
    log_path: String,
    artifact_path: PathBuf,
}

impl RunContract {
    fn start() -> Result<Self, String> {
        let run_id = make_run_contract_id();

        let cfg_path = match app_dirs::app_root_dir() {
            Ok(path) => path.to_string_lossy().into_owned(),
            Err(err) => {
                return Err(format!("failed to resolve cfg path: {err}"));
            }
        };

        let log_path = match app_dirs::logs_dir() {
            Ok(path) => path.to_string_lossy().into_owned(),
            Err(err) => {
                return Err(format!("failed to resolve log path: {err}"));
            }
        };

        let artifact_path = Path::new(&log_path).join(format!("run_contract_{run_id}.ndjson"));

        Ok(Self {
            run_id,
            git_sha: resolve_git_sha(),
            cfg_path,
            log_path,
            artifact_path,
        })
    }

    fn record(&self, startup_phase: &str, exit_status: &str) {
        let event = RunContractEvent {
            run_id: self.run_id.clone(),
            git_sha: self.git_sha.clone(),
            cfg_path: self.cfg_path.clone(),
            log_path: self.log_path.clone(),
            startup_phase: startup_phase.to_string(),
            exit_status: exit_status.to_string(),
            timestamp_utc: UtcTimestamp::now().to_string(),
            process_id: process::id(),
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
    }
}

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
    if sha.is_empty() {
        None
    } else {
        Some(sha)
    }
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
        AllocConsole, AttachConsole, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE,
        STD_OUTPUT_HANDLE,
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

fn load_app_icon() -> Option<WindowIconRgba> {
    decode_icon(include_bytes!("../assets/logo3.ico")).or_else(|| {
        eprintln!("Failed to decode logo3.ico; falling back to PNG icon.");
        let fallback = decode_icon(include_bytes!("../assets/logo3.png"));
        if fallback.is_none() {
            eprintln!("Failed to decode logo3.png fallback for window icon.");
        }
        fallback
    })
}

/// Convert raw embedded bytes into icon-friendly RGBA data.
fn decode_icon(bytes: &[u8]) -> Option<WindowIconRgba> {
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    Some(WindowIconRgba {
        rgba: image.into_raw(),
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icons_decode() {
        assert!(decode_icon(include_bytes!("../assets/logo3.ico")).is_some());
        assert!(decode_icon(include_bytes!("../assets/logo3.png")).is_some());
    }
}
