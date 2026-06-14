use crate::waveform::WaveformImage;
use std::path::PathBuf;

pub(super) struct WaveformImageState {
    pub(super) image: Option<WaveformImage>,
    pub(super) waveform_image_signature: Option<u64>,
    pub(super) loading: Option<PathBuf>,
}

impl Default for WaveformImageState {
    fn default() -> Self {
        Self {
            image: None,
            waveform_image_signature: None,
            loading: None,
        }
    }
}
