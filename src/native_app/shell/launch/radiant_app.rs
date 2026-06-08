use std::panic::{self, AssertUnwindSafe};

use radiant::runtime::NativeRunOptions;

use crate::native_app::app::{GuiMessage, NativeAppState, default_gui_shortcut_resolution, view};
use crate::native_app::app_chrome::settings;
use crate::native_app::sample_library::folder_browser::{FOLDER_TREE_LIST_ID, TREE_ROW_HEIGHT};
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_ROW_HEIGHT,
};

pub(super) fn run_radiant_app(
    state: NativeAppState,
    options: NativeRunOptions,
) -> Result<Result<(), String>, Box<dyn std::any::Any + Send>> {
    panic::catch_unwind(AssertUnwindSafe(|| {
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
            .auxiliary_windows(settings::auxiliary_windows)
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
    }))
}
