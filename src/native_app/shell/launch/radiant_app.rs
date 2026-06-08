use std::panic::{self, AssertUnwindSafe};

use radiant::runtime::NativeRunOptions;

use crate::native_app::app::{GuiMessage, NativeAppState, default_gui_shortcut_resolution, view};
use crate::native_app::app_chrome::presentation::{
    apply_message_with_presentation_repaint, native_app_presentation,
};
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
            .presentation(native_app_presentation())
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
            .update_with(apply_message_with_presentation_repaint)
            .run()
    }))
}
