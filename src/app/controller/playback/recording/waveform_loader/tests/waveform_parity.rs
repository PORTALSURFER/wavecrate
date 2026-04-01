use super::*;

#[test]
fn long_mono_recording_waveform_matches_reference_decode_peaks_and_analysis() {
    let wav_bytes = build_long_recording_fixture(RECORDING_MAX_FULL_FRAMES + 1, 1, 48_000);
    let recording = recording_waveform_from_wav_bytes(&wav_bytes, 48_000, 1);
    let reference = decode_reference_waveform(&wav_bytes);

    assert_decoded_peak_parity(&recording, &reference);
}

#[test]
fn long_stereo_recording_waveform_matches_reference_decode_peaks_and_analysis() {
    let wav_bytes = build_long_recording_fixture(RECORDING_MAX_FULL_FRAMES + 1, 2, 48_000);
    let recording = recording_waveform_from_wav_bytes(&wav_bytes, 48_000, 2);
    let reference = decode_reference_waveform(&wav_bytes);

    assert_decoded_peak_parity(&recording, &reference);
}

#[test]
fn incremental_recording_growth_matches_reference_after_full_to_peaks_transition() {
    let total_frames = RECORDING_MAX_FULL_FRAMES + 1;
    let wav_bytes = build_long_recording_fixture(total_frames, 1, 48_000);
    let data_offset = find_wav_data_chunk(&wav_bytes).expect("data chunk");
    let split_offset = data_offset + (RECORDING_MAX_FULL_FRAMES * 4);
    let mut state = RecordingWaveformState::new(48_000, 1, data_offset);
    state.prepare_for_total_frames(RECORDING_MAX_FULL_FRAMES);

    let consumed = state.consume_data_bytes(&wav_bytes[data_offset..split_offset]);
    assert_eq!(consumed, RECORDING_MAX_FULL_FRAMES);
    assert!(matches!(state.mode, RecordingWaveformMode::Full { .. }));

    let consumed = state.consume_data_bytes(&wav_bytes[split_offset..]);
    assert_eq!(consumed, 1);
    assert!(matches!(state.mode, RecordingWaveformMode::Full { .. }));

    state.convert_full_to_peaks();
    let reference = decode_reference_waveform(&wav_bytes);

    assert_decoded_peak_parity(&state.to_decoded(), &reference);
}
