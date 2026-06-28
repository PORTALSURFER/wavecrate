use crate::native_app::app::{
    NativeAppState, WaveformEditSelectionSnapshot, WaveformPlaySelectionSnapshot,
};
use crate::native_app::transaction_history::TransactionResult;

pub(in crate::native_app) struct TransactionContext<'a> {
    pub(in crate::native_app) state: &'a mut NativeAppState,
}

impl TransactionContext<'_> {
    pub(in crate::native_app) fn restore_play_selection(
        &mut self,
        snapshot: WaveformPlaySelectionSnapshot,
    ) -> TransactionResult {
        let current_path = self.state.waveform.current.path();
        if current_path != snapshot.path {
            return Err(format!(
                "Loaded sample changed; expected {}",
                snapshot.path.display()
            ));
        }
        self.state.waveform.current.restore_play_selection_state(
            snapshot.play_mark_ratio,
            snapshot.play_selection,
            snapshot.marked_play_ranges,
        );
        self.state.waveform.current.ensure_play_selection_visible();
        self.state.retarget_playback_to_play_selection();
        Ok(())
    }

    pub(in crate::native_app) fn restore_edit_selection(
        &mut self,
        snapshot: WaveformEditSelectionSnapshot,
    ) -> TransactionResult {
        let current_path = self.state.waveform.current.path();
        if current_path != snapshot.path {
            return Err(format!(
                "Loaded sample changed; expected {}",
                snapshot.path.display()
            ));
        }
        self.state
            .waveform
            .current
            .restore_edit_selection_state(snapshot.edit_selection);
        self.state.sync_edit_fade_audio_state();
        Ok(())
    }

    #[cfg(test)]
    pub(in crate::native_app) fn set_audio_volume(&mut self, volume: f32) {
        self.state.audio.volume = volume;
    }
}
