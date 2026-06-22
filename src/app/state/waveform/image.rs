use crate::waveform::WaveformImage;
use std::path::PathBuf;

#[derive(Default)]
pub(super) struct WaveformImageState {
    pub(super) image: Option<WaveformImage>,
    pub(super) waveform_image_signature: Option<u64>,
    pub(super) loading: Option<PathBuf>,
}
