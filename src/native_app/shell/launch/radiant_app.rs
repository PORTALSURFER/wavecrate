use std::panic::{self, AssertUnwindSafe};

use radiant::runtime::NativeRunOptions;

use crate::native_app::app::{GuiMessage, NativeAppState, default_gui_shortcut_resolution, view};
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
            .on_native_file_drop(|_state, drop, context| {
                context.emit(GuiMessage::NativeFileDrop(drop));
            })
            .shortcuts(|state, _, press, _| default_gui_shortcut_resolution(state, press))
            .update_with(|state, message, context| {
                let frame_message = matches!(message, GuiMessage::Frame);
                state.apply_message(message, context);
                if !frame_message {
                    context.request_repaint();
                }
            })
            .run()
    }))
}
