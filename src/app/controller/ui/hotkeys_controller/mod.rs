use crate::app::controller::AppController;
use crate::app::controller::ui::hotkeys::HotkeyAction;
use crate::app::state::FocusContext;
use crate::app_core::controller::AppControllerNativeRuntimeExt;

pub(crate) trait HotkeysActions {
    fn handle_hotkey(&mut self, action: HotkeyAction, focus: FocusContext);
}

pub(crate) struct HotkeysController<'a> {
    controller: &'a mut AppController,
}

impl<'a> HotkeysController<'a> {
    pub(crate) fn new(controller: &'a mut AppController) -> Self {
        Self { controller }
    }
}

impl std::ops::Deref for HotkeysController<'_> {
    type Target = AppController;

    fn deref(&self) -> &Self::Target {
        self.controller
    }
}

impl std::ops::DerefMut for HotkeysController<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.controller
    }
}

impl HotkeysActions for HotkeysController<'_> {
    fn handle_hotkey(&mut self, action: HotkeyAction, focus: FocusContext) {
        if !action.is_active(focus) {
            return;
        }
        self.apply_native_ui_action(action.action);
    }
}

impl AppController {
    pub(crate) fn handle_hotkey(&mut self, action: HotkeyAction, focus: FocusContext) {
        self.hotkeys_ctrl().handle_hotkey(action, focus);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support::{
        load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry,
    };
    use crate::app::controller::ui::hotkeys;
    use crate::sample_sources::Rating;
    use crate::selection::SelectionRange;
    use std::path::Path;

    fn action_for(predicate: impl Fn(&radiant::app::UiAction) -> bool) -> HotkeyAction {
        hotkeys::find_action(predicate).expect("missing hotkey action")
    }

    #[test]
    fn waveform_hotkey_respects_focus() {
        let (mut controller, source) =
            prepare_with_source_and_wav_entries(vec![sample_entry("one.wav", Rating::NEUTRAL)]);
        load_waveform_selection(
            &mut controller,
            &source,
            "one.wav",
            &[0.1, -0.2, 0.3, -0.4],
            SelectionRange::new(0.0, 0.5),
        );
        let action =
            action_for(|action| matches!(action, radiant::app::UiAction::CropWaveformSelection));

        controller.handle_hotkey(action.clone(), FocusContext::Waveform);
        assert!(controller.ui.waveform.pending_destructive.is_some());

        controller.ui.waveform.pending_destructive = None;
        controller.handle_hotkey(action, FocusContext::SampleBrowser);
        assert!(controller.ui.waveform.pending_destructive.is_none());
    }

    #[test]
    fn play_hotkeys_route_start_and_playhead_positions() {
        let (mut controller, source) =
            prepare_with_source_and_wav_entries(vec![sample_entry("one.wav", Rating::NEUTRAL)]);
        load_waveform_selection(
            &mut controller,
            &source,
            "one.wav",
            &[0.1, -0.2, 0.3, -0.4],
            SelectionRange::new(0.0, 0.5),
        );
        controller.ui.waveform.playhead.visible = true;
        controller.ui.waveform.playhead.position = 0.37;
        controller.ui.waveform.cursor = Some(0.22);

        controller.handle_hotkey(
            action_for(|action| matches!(action, radiant::app::UiAction::PlayFromStart)),
            FocusContext::SampleBrowser,
        );
        assert_eq!(controller.ui.waveform.last_start_marker, Some(0.0));

        controller.handle_hotkey(
            action_for(|action| matches!(action, radiant::app::UiAction::PlayFromCurrentPlayhead)),
            FocusContext::SampleBrowser,
        );
        assert_eq!(controller.ui.waveform.last_start_marker, Some(0.37));
    }

    #[test]
    fn browser_focus_hotkey_uses_explicit_scope() {
        let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("one.wav", Rating::NEUTRAL),
            sample_entry("two.wav", Rating::NEUTRAL),
        ]);
        controller.sample_view.wav.loaded_wav = Some("two.wav".into());
        controller.ui.focus.set_context(FocusContext::Waveform);

        controller.handle_hotkey(
            action_for(|action| {
                matches!(action, radiant::app::UiAction::FocusLoadedSampleInBrowser)
            }),
            FocusContext::SampleBrowser,
        );

        assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
        assert_eq!(
            controller.sample_view.wav.selected_wav.as_deref(),
            Some(Path::new("two.wav"))
        );
    }

    #[test]
    fn browser_scoped_hotkey_is_ignored_when_no_section_is_focused() {
        let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("one.wav", Rating::NEUTRAL),
            sample_entry("two.wav", Rating::NEUTRAL),
        ]);
        controller.focus_browser_row(0);
        let before = controller.ui.browser.selection.selected_paths.clone();

        controller.handle_hotkey(
            action_for(|action| {
                matches!(
                    action,
                    radiant::app::UiAction::ToggleFocusedBrowserRowSelection
                )
            }),
            FocusContext::None,
        );

        assert_eq!(controller.ui.browser.selection.selected_paths, before);
    }

    #[test]
    fn browser_focus_move_hotkey_moves_the_selected_row() {
        let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("one.wav", Rating::NEUTRAL),
            sample_entry("two.wav", Rating::NEUTRAL),
        ]);
        controller.focus_browser_row_only(0);

        controller.handle_hotkey(
            action_for(|action| {
                matches!(
                    action,
                    radiant::app::UiAction::MoveBrowserFocus { delta: 1 }
                )
            }),
            FocusContext::SampleBrowser,
        );

        assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
        assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
        assert_eq!(
            controller.sample_view.wav.selected_wav.as_deref(),
            Some(Path::new("two.wav"))
        );
    }
}
