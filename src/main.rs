#![deny(missing_docs)]
#![deny(warnings)]

//! Entry point for the egui-based Sempal UI.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use egui::{self, Context};
use sempal::audio::AudioPlayer;
use sempal::gui_app::{MIN_VIEWPORT_SIZE, SempalGuiApp, new_native_bridge, new_sempal_app};
use sempal::gui_runtime::{
    EguiAppRuntime, EguiRunOptions, WindowIconRgba, run_egui_wgpu_app, run_native_vello_app,
};
use sempal::logging;
use sempal::waveform::WaveformRenderer;

const GUI_BACKEND_ENV_VAR: &str = "SEMPAL_GUI_BACKEND";
const GUI_BACKEND_ARG: &str = "--gui-backend";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GuiBackend {
    LegacyEgui,
    NativeVello,
}

impl GuiBackend {
    fn as_str(self) -> &'static str {
        match self {
            Self::LegacyEgui => "legacy_egui",
            Self::NativeVello => "native_vello",
        }
    }
}

fn main() -> Result<(), String> {
    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    if log_console_requested() {
        enable_windows_console();
    }

    if let Err(err) = logging::init() {
        eprintln!("Logging disabled: {err}");
    }

    let backend = resolve_gui_backend()?;
    eprintln!("GUI backend: {}", backend.as_str());

    let options = EguiRunOptions {
        title: String::from("Sempal"),
        inner_size: None,
        min_inner_size: Some(MIN_VIEWPORT_SIZE),
        maximized: true,
        icon: load_app_icon(),
    };

    match backend {
        GuiBackend::LegacyEgui => {
            let renderer = WaveformRenderer::new(680, 260);
            let player = None::<std::rc::Rc<std::cell::RefCell<AudioPlayer>>>;
            let app = match new_sempal_app(renderer, player) {
                Ok(app) => RootApp::Main(app),
                Err(err) => RootApp::LaunchError(LaunchError { message: err }),
            };
            run_egui_wgpu_app(options, app)
        }
        GuiBackend::NativeVello => {
            let renderer = WaveformRenderer::new(680, 260);
            let player = None::<std::rc::Rc<std::cell::RefCell<AudioPlayer>>>;
            let bridge = new_native_bridge(renderer, player)?;
            run_native_vello_app(options, bridge)
        }
    }
}

fn resolve_gui_backend() -> Result<GuiBackend, String> {
    if let Some(value) = backend_from_arg_list(std::env::args().skip(1))? {
        return Ok(value);
    }
    if let Ok(value) = std::env::var(GUI_BACKEND_ENV_VAR) {
        return parse_backend_value(&value);
    }
    Ok(GuiBackend::LegacyEgui)
}

fn backend_from_arg_list(
    args: impl IntoIterator<Item = String>,
) -> Result<Option<GuiBackend>, String> {
    let mut selected = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--gui-backend=") {
            selected = Some(parse_backend_value(value)?);
            continue;
        }
        if arg == GUI_BACKEND_ARG {
            let Some(value) = args.next() else {
                return Err(format!(
                    "{GUI_BACKEND_ARG} requires a value: legacy_egui or native_vello"
                ));
            };
            selected = Some(parse_backend_value(&value)?);
        }
    }
    Ok(selected)
}

fn parse_backend_value(value: &str) -> Result<GuiBackend, String> {
    match value.trim() {
        "legacy_egui" => Ok(GuiBackend::LegacyEgui),
        "native_vello" => Ok(GuiBackend::NativeVello),
        other => Err(format!(
            "Unsupported GUI backend '{other}'. Supported values: legacy_egui, native_vello",
        )),
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

/// Minimal fallback app to display initialization errors.
struct LaunchError {
    message: String,
}

impl EguiAppRuntime for LaunchError {
    fn update(&mut self, ctx: &Context, _window: &winit::window::Window) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Failed to start UI");
                ui.label(&self.message);
            });
        });
    }
}

/// Root app wrapper that can render either the full UI or a launch error fallback.
enum RootApp {
    Main(SempalGuiApp),
    LaunchError(LaunchError),
}

impl EguiAppRuntime for RootApp {
    fn setup(&mut self, ctx: &Context) {
        match self {
            Self::Main(app) => app.setup(ctx),
            Self::LaunchError(app) => app.setup(ctx),
        }
    }

    fn update(&mut self, ctx: &Context, window: &winit::window::Window) {
        match self {
            Self::Main(app) => app.update(ctx, window),
            Self::LaunchError(app) => app.update(ctx, window),
        }
    }

    fn on_exit(&mut self) {
        match self {
            Self::Main(app) => app.on_exit(),
            Self::LaunchError(app) => app.on_exit(),
        }
    }

    fn clear_color(&self) -> [f32; 4] {
        match self {
            Self::Main(app) => app.clear_color(),
            Self::LaunchError(app) => app.clear_color(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icons_decode() {
        assert!(decode_icon(include_bytes!("../assets/logo3.ico")).is_some());
        assert!(decode_icon(include_bytes!("../assets/logo3.png")).is_some());
    }

    #[test]
    fn backend_arg_parsing_accepts_explicit_value_forms() {
        let selected = backend_from_arg_list(vec![String::from("--gui-backend=native_vello")])
            .expect("arg parsing should succeed");
        assert_eq!(selected, Some(GuiBackend::NativeVello));

        let selected = backend_from_arg_list(vec![
            String::from("--gui-backend"),
            String::from("legacy_egui"),
        ])
        .expect("arg parsing should succeed");
        assert_eq!(selected, Some(GuiBackend::LegacyEgui));
    }

    #[test]
    fn backend_arg_parsing_rejects_invalid_values() {
        let err = backend_from_arg_list(vec![String::from("--gui-backend=invalid")])
            .expect_err("invalid backend should fail");
        assert!(err.contains("Unsupported GUI backend"));
    }
}
