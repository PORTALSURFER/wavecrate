#![deny(missing_docs)]
#![deny(warnings)]

//! Entry point for the native Vello-based Sempal UI.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use sempal::app_core::native_bridge::new_native_bridge;
use sempal::app_core::ui::MIN_VIEWPORT_SIZE;
use sempal::audio::AudioPlayer;
use sempal::gui_runtime::{NativeRunOptions, WindowIconRgba, run_native_vello_app};
use sempal::logging;
use sempal::waveform::WaveformRenderer;
use std::{path::PathBuf, process, time::SystemTime};
use tracing::{error, info};

fn main() -> Result<(), String> {
    logging::install_panic_hook();

    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    if log_console_requested() {
        enable_windows_console();
    }

    if let Err(err) = logging::init() {
        eprintln!("Logging disabled: {err}");
    }
    let exe = std::env::current_exe()
        .ok()
        .and_then(|path| path.into_os_string().into_string().ok())
        .unwrap_or_else(|| String::from("<unknown>"));
    let cwd = std::env::current_dir()
        .map(PathBuf::to_string_lossy)
        .map(|cwd| cwd.to_string())
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
    let player = None::<std::rc::Rc<std::cell::RefCell<AudioPlayer>>>;
    let bridge = new_native_bridge(renderer, player).map_err(|err| {
        error!(err = %err, "sempal startup: failed to construct native bridge");
        err
    })?;
    info!("sempal startup: native bridge constructed");
    let result = run_native_vello_app(options, bridge);
    if let Err(err) = &result {
        error!(err = %err, "sempal startup: runtime exited with error");
    } else {
        info!("sempal startup: native runtime exited normally");
    }
    result
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
