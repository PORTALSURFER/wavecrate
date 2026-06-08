use std::panic::{self, AssertUnwindSafe};

use radiant::runtime::NativeRunOptions;

use crate::native_app::app::{NativeAppState, default_gui_shortcut_resolution, view};
use crate::native_app::app_chrome::settings;

pub(super) fn run_radiant_app(
    state: NativeAppState,
    options: NativeRunOptions,
) -> Result<Result<(), String>, Box<dyn std::any::Any + Send>> {
    panic::catch_unwind(AssertUnwindSafe(|| {
        radiant::app(state)
            .options(options)
            .view(view)
            .subscriptions(NativeAppState::worker_subscription)
            .auxiliary_windows(settings::auxiliary_windows)
            .on_shutdown(NativeAppState::shutdown)
            .shortcuts(|state, _, press, _| default_gui_shortcut_resolution(state, press))
            .reducer(NativeAppState::update)
            .run()
    }))
}
