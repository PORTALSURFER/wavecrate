mod args;
mod logging;
mod options;
mod radiant_app;

use std::time::Instant;

use crate::native_app::app::NativeAppState;

#[cfg(test)]
pub(in crate::native_app) use args::{
    DEBUG_LAYOUT_ARG, DEBUG_LAYOUT_SHORT_ARG, debug_layout_requested,
};
pub(in crate::native_app) use logging::emit_gui_action;
#[cfg(test)]
pub(in crate::native_app) use options::DEFAULT_WINDOW_TITLE;

/// Run the default Radiant GUI application shell.
pub(crate) fn run() -> Result<(), String> {
    logging::install_panic_hook();
    let args = args::collect_launch_args();
    let startup_started_at = Instant::now();

    logging::init_logging(&args);
    logging::log_default_gui_startup(&args);
    let state = NativeAppState::load_default()?;
    let debug_layout = args::debug_layout_requested(args.iter().cloned());
    let options = options::native_run_options(debug_layout);
    logging::log_radiant_prepare(options.frame.debug_layout, startup_started_at);
    let run_result = radiant_app::run_radiant_app(state, options);
    logging::finish_radiant_run(run_result, startup_started_at)
}
