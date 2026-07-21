use std::panic::{self, AssertUnwindSafe};

use radiant::runtime::NativeRunOptions;

use crate::native_app::app::{GuiMessage, NativeAppState, view};
use crate::native_app::app_chrome::settings;
use crate::native_app::app_chrome::view_models::sample_browser::prepare_sample_browser_view;
use crate::native_app::audio::playback::playhead_frame_diagnostics_observer_enabled;

pub(super) fn run_catching_unwind(
    mut state: NativeAppState,
    options: NativeRunOptions,
) -> Result<Result<(), String>, Box<dyn std::any::Any + Send>> {
    panic::catch_unwind(AssertUnwindSafe(|| {
        prepare_sample_browser_view(&mut state);
        radiant::runtime::run_native_vello_runtime(options, native_app_runtime_bridge(state))
    }))
}

/// Lower the production app composition into Radiant's backend-neutral host boundary.
pub(in crate::native_app) fn native_app_runtime_bridge(
    state: NativeAppState,
) -> impl radiant::runtime::RuntimeBridge<GuiMessage> {
    let app = radiant::app(state)
        .view(view)
        .subscriptions(NativeAppState::worker_subscription)
        .auxiliary_windows(settings::auxiliary_windows)
        .on_native_focus_regained(|state, _context| {
            state.reconcile_sources_after_focus_regained();
        })
        .on_native_file_open(|state, open, context| {
            state.open_audio_documents(open.paths, context);
        });
    let app = if playhead_frame_diagnostics_observer_enabled() {
        app.on_native_frame_diagnostics(|state, diagnostics| {
            state.observe_playhead_native_frame_diagnostics(diagnostics);
        })
    } else {
        app
    };
    app.on_shutdown(NativeAppState::shutdown)
        .handle_message(NativeAppState::handle_message)
        .into_bridge()
}
