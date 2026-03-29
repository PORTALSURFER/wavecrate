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
        load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
    };
    use crate::app::controller::ui::hotkeys;
    use crate::app::state::SimilarQuery;
    use crate::app_core::controller::build_named_gui_fixture_controller;
    use crate::sample_sources::Rating;
    use crate::selection::SelectionRange;
    use crate::waveform::WaveformRenderer;
    use std::path::{Path, PathBuf};

    fn action_for(predicate: impl Fn(&radiant::app::UiAction) -> bool) -> HotkeyAction {
        hotkeys::find_action(predicate).expect("missing hotkey action")
    }

    fn sample_name(sample_id: &str) -> &str {
        sample_id
            .rsplit_once("::")
            .map_or(sample_id, |(_, sample_name)| sample_name)
    }

    fn seed_similarity_query_with_different_focus(
        controller: &mut AppController,
    ) -> (String, String) {
        controller.focus_browser_row_only(0);
        let anchor_sample_id = controller
            .sample_id_for_visible_row(0)
            .expect("anchor sample id");
        controller.ui.browser.search.similar_query = Some(SimilarQuery {
            sample_id: anchor_sample_id.clone(),
            label: String::from("kick_one.wav"),
            indices: vec![0],
            scores: vec![1.0],
            anchor_index: Some(0),
        });
        controller.focus_browser_row_only(1);
        let focused_sample_id = controller
            .sample_id_for_visible_row(1)
            .expect("focused sample id");
        (anchor_sample_id, focused_sample_id)
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
    fn compare_anchor_hotkey_sets_focused_sample_anchor() {
        let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("one.wav", Rating::NEUTRAL),
            sample_entry("two.wav", Rating::NEUTRAL),
        ]);
        controller.focus_browser_row_only(1);

        controller.handle_hotkey(
            action_for(|action| {
                matches!(
                    action,
                    radiant::app::UiAction::SetCompareAnchorFromFocusedBrowserSample
                )
            }),
            FocusContext::SampleBrowser,
        );

        assert_eq!(
            controller.ui.waveform.compare_anchor_label.as_deref(),
            Some("two")
        );
        assert_eq!(
            controller.sample_view.wav.compare_anchor.as_ref().map(|anchor| anchor.relative_path.as_path()),
            Some(Path::new("two.wav"))
        );
    }

    #[test]
    fn compare_anchor_play_hotkey_routes_global_compare_replay() {
        let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("anchor.wav", Rating::NEUTRAL),
            sample_entry("current.wav", Rating::NEUTRAL),
        ]);
        write_test_wav(&source.root.join("anchor.wav"), &[0.0, 0.1]);
        write_test_wav(&source.root.join("current.wav"), &[0.0, -0.1]);
        controller.focus_browser_row_only(0);
        controller.set_compare_anchor_from_focused_browser_sample();
        controller.focus_browser_row_only(1);
        controller.runtime.jobs.pending_audio = None;
        controller.runtime.jobs.pending_playback = None;

        controller.handle_hotkey(
            action_for(|action| matches!(action, radiant::app::UiAction::PlayCompareAnchor)),
            FocusContext::Waveform,
        );

        let pending = controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .expect("compare replay should queue");
        assert_eq!(pending.relative_path, PathBuf::from("anchor.wav"));
        assert!(pending.force_loaded_audio);
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

    #[test]
    fn browser_similarity_hotkey_matches_native_toggle_behavior_when_focus_changes() {
        let action = action_for(|action| {
            matches!(
                action,
                radiant::app::UiAction::ToggleFindSimilarFocusedSample
            )
        });
        let mut hotkey_bundle =
            build_named_gui_fixture_controller(WaveformRenderer::new(16, 16), "browser")
                .expect("browser fixture");
        let mut native_bundle =
            build_named_gui_fixture_controller(WaveformRenderer::new(16, 16), "browser")
                .expect("browser fixture");

        seed_similarity_query_with_different_focus(&mut hotkey_bundle.controller);
        seed_similarity_query_with_different_focus(&mut native_bundle.controller);

        hotkey_bundle
            .controller
            .handle_hotkey(action.clone(), FocusContext::SampleBrowser);
        native_bundle
            .controller
            .apply_native_ui_action(action.action);

        let hotkey_query = hotkey_bundle
            .controller
            .ui
            .browser
            .search
            .similar_query
            .as_ref()
            .map(|query| sample_name(&query.sample_id).to_string());
        let native_query = native_bundle
            .controller
            .ui
            .browser
            .search
            .similar_query
            .as_ref()
            .map(|query| sample_name(&query.sample_id).to_string());

        assert!(
            hotkey_query.is_some(),
            "hotkey should not clear a mismatched anchor"
        );
        assert_eq!(hotkey_query, native_query);
        assert_eq!(
            hotkey_bundle.controller.ui.browser.active_tab,
            native_bundle.controller.ui.browser.active_tab
        );
    }
}
