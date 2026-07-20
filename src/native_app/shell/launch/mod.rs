mod args;
mod icon;
mod logging;
mod macos_icon;
mod options;
mod radiant_runtime;

use std::time::Instant;

use radiant::runtime::NativeRunOptions;

use crate::native_app::app::NativeAppState;

#[cfg(test)]
pub(in crate::native_app) use args::{
    DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, DEBUG_OVERLAYS_ARG, debug_layout_requested,
};
pub(in crate::native_app) use logging::emit_gui_action;
#[cfg(test)]
pub(in crate::native_app) use options::default_window_title;
#[cfg(any(test, feature = "legacy-controller"))]
pub(in crate::native_app) use radiant_runtime::native_app_runtime_bridge;

/// Run the default Radiant GUI application shell.
pub fn run() -> Result<(), String> {
    if let Some(result_json) =
        crate::native_app::source_processing::run_internal_source_analysis_from_args()?
    {
        println!("{result_json}");
        return Ok(());
    }
    if let Some(finalized) =
        crate::native_app::sample_library::similarity_artifacts::run_internal_similarity_finalizer_from_args()?
    {
        println!("{finalized}");
        return Ok(());
    }
    logging::install_panic_hook();
    let args = args::LaunchArgs::collect();
    let startup_started_at = Instant::now();

    init_logging(&args);
    macos_icon::install_macos_application_icon();
    let state = NativeAppState::load_default()?;
    let options = options::native_run_options(args.debug_layout());

    run_radiant_runtime(state, options, startup_started_at)
}

fn init_logging(args: &args::LaunchArgs) {
    logging::init_logging(args.raw());
    logging::log_default_gui_startup(args.raw());
}

fn run_radiant_runtime(
    state: NativeAppState,
    options: NativeRunOptions,
    startup_started_at: Instant,
) -> Result<(), String> {
    logging::log_radiant_runtime_starting(options.frame.debug_layout, startup_started_at);
    let runtime_result = radiant_runtime::run_catching_unwind(state, options);
    logging::finish_radiant_runtime(runtime_result, startup_started_at)
}
