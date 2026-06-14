mod browser_shell;
mod compatibility;
mod frame_preparation;
mod options_transport;
mod update;
mod waveform;

use super::*;

fn controller_for_grouped_dispatch() -> AppController {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    controller.ui.browser.search.search_focus_requested = true;
    controller.ui.focus.context = FocusContext::Waveform;
    controller.ui.waveform.selection = Some(crate::selection::SelectionRange::new(0.2, 0.8));
    controller.ui.waveform.edit_selection = Some(crate::selection::SelectionRange::new(0.3, 0.7));
    controller
}
