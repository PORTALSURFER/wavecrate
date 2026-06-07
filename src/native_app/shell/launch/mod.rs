mod args;
mod logging;
mod options;
mod radiant_app;

use std::time::Instant;

use radiant::runtime::NativeRunOptions;

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
    LaunchSession::collect().run()
}

struct LaunchSession {
    args: args::LaunchArgs,
    startup_started_at: Instant,
}

impl LaunchSession {
    fn collect() -> Self {
        Self {
            args: args::LaunchArgs::collect(),
            startup_started_at: Instant::now(),
        }
    }

    fn run(self) -> Result<(), String> {
        self.init_logging();
        let state = Self::load_default_state()?;
        let options = self.native_run_options();
        self.run_radiant_app(state, options)
    }

    fn init_logging(&self) {
        logging::init_logging(self.args.raw());
        logging::log_default_gui_startup(self.args.raw());
    }

    fn load_default_state() -> Result<NativeAppState, String> {
        NativeAppState::load_default()
    }

    fn native_run_options(&self) -> NativeRunOptions {
        options::native_run_options(self.args.debug_layout())
    }

    fn run_radiant_app(
        self,
        state: NativeAppState,
        options: NativeRunOptions,
    ) -> Result<(), String> {
        logging::log_radiant_prepare(options.frame.debug_layout, self.startup_started_at);
        let run_result = radiant_app::run_radiant_app(state, options);
        logging::finish_radiant_run(run_result, self.startup_started_at)
    }
}
