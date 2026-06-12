/// Translate local waveform channel-view settings into UI runtime model enums.
pub(in crate::app_core::ui_projection) fn project_waveform_channel_view_model(
    channel_view: crate::waveform::WaveformChannelView,
) -> crate::app_core::actions::NativeWaveformChannelViewModel {
    match channel_view {
        crate::waveform::WaveformChannelView::Mono => {
            crate::app_core::actions::NativeWaveformChannelViewModel::Mono
        }
        crate::waveform::WaveformChannelView::SplitStereo => {
            crate::app_core::actions::NativeWaveformChannelViewModel::Stereo
        }
    }
}
