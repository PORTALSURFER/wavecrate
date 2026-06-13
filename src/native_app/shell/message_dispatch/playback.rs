use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};

impl NativeAppState {
    pub(super) fn apply_playback_dispatch(
        &mut self,
        message: GuiMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match message {
            GuiMessage::AudioPlayerOpenFinished(completion) => {
                self.finish_audio_player_open(completion)
            }
            GuiMessage::PlaySelectedSample => self.play_selected_sample(context),
            GuiMessage::PlayRandomSampleRange => self.play_random_sample_range(context),
            GuiMessage::StopPlayback => self.stop_playback(),
            GuiMessage::ToggleLoopPlayback => self.toggle_loop_playback(),
            _ => unreachable!("playback dispatcher received a non-playback message"),
        }
    }
}
