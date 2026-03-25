use super::*;

#[test]
fn build_selection_export_audio_payload_prefers_loaded_decoded_samples() {
    let payload = crate::app::controller::jobs::build_selection_export_audio_payload(
        Some(&Arc::new(DecodedWaveform {
            cache_token: next_cache_token(),
            samples: Arc::from(vec![0.1, 0.2, 0.3, 0.4]),
            analysis_samples: Arc::from(Vec::<f32>::new()),
            analysis_sample_rate: 0,
            analysis_stride: 1,
            peaks: None,
            duration_seconds: 1.0,
            sample_rate: 44_100,
            channels: 2,
        })),
        Arc::from(vec![1u8, 2, 3]),
    );

    match payload {
        SelectionExportAudioPayload::Decoded {
            samples,
            channels,
            sample_rate,
        } => {
            assert_eq!(samples.as_ref(), &[0.1, 0.2, 0.3, 0.4]);
            assert_eq!(channels, 2);
            assert_eq!(sample_rate, 44_100);
        }
        SelectionExportAudioPayload::Encoded { .. } => {
            panic!("expected resident decoded samples to be reused");
        }
    }
}

#[test]
fn build_selection_export_audio_payload_falls_back_when_only_peak_data_is_loaded() {
    let payload = crate::app::controller::jobs::build_selection_export_audio_payload(
        Some(&Arc::new(DecodedWaveform {
            cache_token: next_cache_token(),
            samples: Arc::from(Vec::<f32>::new()),
            analysis_samples: Arc::from(Vec::<f32>::new()),
            analysis_sample_rate: 0,
            analysis_stride: 1,
            peaks: Some(Arc::new(WaveformPeaks {
                total_frames: 8,
                channels: 1,
                bucket_size_frames: 1,
                mono: vec![(0.0, 1.0)],
                left: None,
                right: None,
            })),
            duration_seconds: 1.0,
            sample_rate: 44_100,
            channels: 1,
        })),
        Arc::from(vec![1u8, 2, 3]),
    );

    match payload {
        SelectionExportAudioPayload::Encoded { bytes } => {
            assert_eq!(bytes.as_ref(), &[1, 2, 3]);
        }
        SelectionExportAudioPayload::Decoded { .. } => {
            panic!("expected peak-only waveforms to fall back to encoded bytes");
        }
    }
}
