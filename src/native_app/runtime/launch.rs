use super::app_scope::{
    FOLDER_TREE_LIST_ID, GuiMessage, NativeAppState, SAMPLE_BROWSER_LIST_ID,
    SAMPLE_BROWSER_ROW_HEIGHT, default_gui_shortcut_resolution, view,
};
use crate::native_app::{audio_settings, folder_browser::TREE_ROW_HEIGHT};
use radiant::runtime::{
    NativeFrameOptions, NativeRunOptions, NativeTextOptions, NativeWindowBehavior,
    NativeWindowGeometry, NativeWindowOptions,
};
use std::{
    ffi::OsString,
    panic::{self, AssertUnwindSafe},
    process,
    time::{Instant, SystemTime},
};
use wavecrate::logging::{self, ActionDebugEvent, emit_action_debug_event};
use wavecrate::native_runtime::wavecrate_ui_font_path;

pub(in crate::native_app) const DEBUG_LAYOUT_ARG: &str = "--debug-layout";
pub(in crate::native_app) const DEBUG_LAYOUT_SHORT_ARG: &str = "-debug-layout";
pub(in crate::native_app) const DEFAULT_WINDOW_TITLE: &str = "Wavecrate - alpha";

/// Run the default Radiant GUI application shell.
pub(crate) fn run() -> Result<(), String> {
    logging::install_panic_hook();
    let args: Vec<OsString> = std::env::args_os().collect();
    let startup_started_at = Instant::now();

    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    if log_console_requested(&args) {
        enable_windows_console();
    }

    if let Err(err) = logging::init(args.iter().cloned()) {
        eprintln!("logging disabled: {err}");
    }

    log_default_gui_startup(&args);
    let state = NativeAppState::load_default()?;
    let debug_layout = debug_layout_requested(args.iter().cloned());
    let options = NativeRunOptions {
        window: NativeWindowOptions {
            title: String::from(DEFAULT_WINDOW_TITLE),
            geometry: NativeWindowGeometry {
                inner_size: Some([960.0, 540.0]),
                min_inner_size: Some([640.0, 360.0]),
                ..NativeWindowGeometry::default()
            },
            behavior: NativeWindowBehavior {
                drag_and_drop: true,
                ..NativeWindowBehavior::default()
            },
            ..NativeWindowOptions::default()
        },
        frame: NativeFrameOptions {
            debug_layout,
            ..NativeFrameOptions::default()
        },
        text: NativeTextOptions {
            embedded_fonts: Vec::new(),
            font_paths: vec![wavecrate_ui_font_path()],
        },
        ..NativeRunOptions::default()
    };
    tracing::info!(
        debug_layout = options.frame.debug_layout,
        "default gui: preparing Radiant application"
    );
    emit_gui_action(
        "runtime.startup.prepare_radiant_app",
        Some("background"),
        None,
        "running",
        startup_started_at,
        None,
    );

    let run_result = panic::catch_unwind(AssertUnwindSafe(|| {
        radiant::app(state)
            .options(options)
            .view(view)
            .animation(|state| state.frame_message_animation_active())
            .on_frame(|| GuiMessage::Frame)
            .animated_transient_overlay_at(
                60,
                |state| state.waveform.is_playing(),
                |state, context, primitives| {
                    state.paint_playback_overlay(context, primitives);
                },
            )
            .subscriptions(NativeAppState::worker_subscription)
            .auxiliary_windows(audio_settings::auxiliary_windows)
            .on_shutdown(NativeAppState::shutdown)
            .on_scroll(|state, update, _context| {
                if update.node_id == SAMPLE_BROWSER_LIST_ID {
                    state
                        .folder_browser
                        .track_file_view_scroll_offset(update.offset.y, SAMPLE_BROWSER_ROW_HEIGHT);
                } else if update.node_id == FOLDER_TREE_LIST_ID {
                    state
                        .folder_browser
                        .set_tree_view_start_from_scroll_offset(update.offset.y, TREE_ROW_HEIGHT);
                }
            })
            .on_native_file_drop(|_state, drop, context| {
                context.emit(GuiMessage::NativeFileDrop(drop));
            })
            .shortcuts(|state, _, press, _| default_gui_shortcut_resolution(state, press))
            .update_with(|state, message, context| {
                let frame_scope = matches!(message, GuiMessage::Frame)
                    .then(|| state.frame_repaint_scope_before_update());
                state.apply_message(message, context);
                if frame_scope.is_some_and(|scope| state.frame_can_use_paint_only(scope)) {
                    context.request_paint_only();
                } else {
                    context.request_repaint();
                }
            })
            .run()
    }));

    match run_result {
        Ok(Ok(())) => {
            tracing::info!("default gui: Radiant runtime exited normally");
            emit_gui_action(
                "runtime.exit.radiant_app",
                Some("background"),
                None,
                "success",
                startup_started_at,
                None,
            );
            Ok(())
        }
        Ok(Err(err)) => {
            tracing::error!(err = %err, "default gui: Radiant runtime exited with error");
            emit_gui_action(
                "runtime.exit.radiant_app",
                Some("background"),
                None,
                "error",
                startup_started_at,
                Some(&err),
            );
            Err(err)
        }
        Err(payload) => {
            let message = panic_payload_to_string(payload);
            tracing::error!("default gui: panic captured while running: {message}");
            emit_gui_action(
                "runtime.exit.radiant_app",
                Some("background"),
                None,
                "panic",
                startup_started_at,
                Some(&message),
            );
            Err(format!("startup panic: {message}"))
        }
    }
}

pub(in crate::native_app) fn debug_layout_requested<I>(args: I) -> bool
where
    I: IntoIterator<Item = OsString>,
{
    args.into_iter()
        .any(|arg| arg == DEBUG_LAYOUT_ARG || arg == DEBUG_LAYOUT_SHORT_ARG)
}

fn log_default_gui_startup(args: &[OsString]) {
    let exe = std::env::current_exe()
        .ok()
        .and_then(|path| path.into_os_string().into_string().ok())
        .unwrap_or_else(|| String::from("<unknown>"));
    let cwd = std::env::current_dir()
        .map(|cwd| cwd.to_string_lossy().into_owned())
        .unwrap_or_else(|_| String::from("<unknown>"));
    tracing::info!(
        pid = process::id(),
        exe = exe,
        cwd = cwd,
        arg_count = args.len(),
        timestamp = ?SystemTime::now(),
        debug = cfg!(debug_assertions),
        "default gui startup: process metadata captured"
    );
    match wavecrate::app_dirs::resolve_persistence() {
        Ok(persistence) => {
            tracing::info!(
                persistence_mode = %persistence.mode,
                config_base = %persistence.config_base.display(),
                app_root = %persistence.app_root.display(),
                "default gui startup: persistence profile resolved"
            );
        }
        Err(err) => {
            tracing::error!(err = %err, "default gui startup: failed to resolve persistence profile");
        }
    }
}

pub(in crate::native_app) fn emit_gui_action(
    action: &'static str,
    pane: Option<&'static str>,
    source: Option<&str>,
    outcome: &'static str,
    started_at: Instant,
    error: Option<&str>,
) {
    emit_action_debug_event(ActionDebugEvent {
        action,
        pane,
        source,
        outcome,
        elapsed: started_at.elapsed(),
        error,
    });
}

fn panic_payload_to_string(panic_payload: Box<dyn std::any::Any + Send>) -> String {
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
