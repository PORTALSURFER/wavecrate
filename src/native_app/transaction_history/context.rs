use crate::native_app::app::NativeAppState;

pub(in crate::native_app) struct TransactionContext<'a> {
    pub(in crate::native_app) state: &'a mut NativeAppState,
}

impl TransactionContext<'_> {
    #[cfg(test)]
    pub(in crate::native_app) fn set_audio_volume(&mut self, volume: f32) {
        self.state.audio.volume = volume;
    }
}
