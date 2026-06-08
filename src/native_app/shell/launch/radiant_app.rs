use std::panic::{self, AssertUnwindSafe};

use radiant::{prelude as ui, runtime::NativeRunOptions};

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
            .reducer(NativeAppState::update)
            .repaint_policy(ui::RepaintPolicy::after_messages_except_value(
                GuiMessage::Frame,
            ))
            .run()
    }))
}
