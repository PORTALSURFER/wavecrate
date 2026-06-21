use super::*;
use crate::sample_sources::library::LibraryState;

mod analysis;
mod audio;
mod interaction;
mod naming;
mod paths;
mod runtime;
mod similarity;

struct SettingsRoundTripFixture {
    expected: AppConfig,
    actual: AppConfig,
}

fn settings_round_trip_fixture() -> SettingsRoundTripFixture {
    let source_id = SourceId::from_string("source_id::test");
    let cfg = AppConfig {
        sources: vec![SampleSource::new_with_id(
            source_id.clone(),
            std::path::PathBuf::from("samples"),
        )],
        core: AppSettingsCore {
            feature_flags: FeatureFlags {
                autoplay_selection: false,
            },
            analysis: AnalysisSettings {
                max_analysis_duration_seconds: 12.5,
                limit_similarity_prep_duration: false,
                long_sample_threshold_seconds: 42.0,
                analysis_worker_count: 2,
                fast_similarity_prep: true,
                fast_similarity_prep_sample_rate: 8_000,
            },
            updates: UpdateSettings {
                channel: UpdateChannel::Nightly,
                check_on_startup: false,
                last_seen_nightly_published_at: Some("2024-01-01".into()),
            },
            job_message_queue_capacity: 512,
            app_data_dir: Some(std::path::PathBuf::from("data_root")),
            trash_folder: Some(std::path::PathBuf::from("trash_bin")),
            drop_targets: vec![
                DropTargetConfig {
                    path: std::path::PathBuf::from("drops/a"),
                    color: Some(DropTargetColor::Mint),
                },
                DropTargetConfig::new(std::path::PathBuf::from("drops/b")),
            ],
            last_selected_source: Some(source_id.clone()),
            upper_folder_pane_source: Some(source_id.clone()),
            lower_folder_pane_source: None,
            active_folder_pane: Some(String::from("upper")),
            collection_names: std::collections::BTreeMap::from([(
                String::from("0"),
                String::from("Drums"),
            )]),
            folder_locks: vec![std::path::PathBuf::from("samples/locked")],
            audio_output: AudioOutputConfig {
                host: Some("coreaudio".into()),
                device: Some("Test Interface".into()),
                sample_rate: Some(96_000),
                buffer_size: Some(256),
            },
            audio_input: AudioInputConfig {
                host: Some("asio".into()),
                device: Some("Test Mic".into()),
                sample_rate: Some(44_100),
                buffer_size: Some(256),
                channels: vec![1],
            },
            audio_write_format: AudioWriteFormatConfig {
                sample_rate: AudioWriteSampleRate::Hz(48_000),
                sample_format: AudioWriteSampleFormat::Pcm16,
                channel_behavior: AudioWriteChannelBehavior::PreserveMonoStereo,
                dither: AudioWriteDither::None,
            },
            volume: 0.75,
            controls: InteractionOptions {
                invert_waveform_scroll: false,
                waveform_scroll_speed: 2.5,
                wheel_zoom_factor: 1.5,
                keyboard_zoom_factor: 1.2,
                anti_clip_fade_enabled: false,
                anti_clip_fade_ms: 12.0,
                auto_edge_fades_on_selection_exports: false,
                destructive_yolo_mode: true,
                waveform_channel_view: WaveformChannelView::SplitStereo,
                bpm_snap_enabled: true,
                relative_bpm_grid_enabled: true,
                bpm_lock_enabled: true,
                bpm_stretch_enabled: true,
                bpm_value: 123.0,
                transient_snap_enabled: true,
                transient_markers_enabled: false,
                input_monitoring_enabled: false,
                normalized_audition_enabled: true,
                advance_after_rating: true,
                tooltip_mode: TooltipMode::Regular,
                loop_lock_enabled: true,
            },
            similarity: {
                let mut settings = SimilarityAspectSettings::default();
                settings.set_weighting_enabled(true);
                settings.set_aspect_enabled(
                    wavecrate_analysis::aspects::SimilarityAspect::Pitch,
                    false,
                );
                settings.set_aspect_weight(
                    wavecrate_analysis::aspects::SimilarityAspect::Spectrum,
                    0.35,
                );
                settings
            },
            default_identifier: String::from("artist"),
            tag_dictionary: std::collections::BTreeMap::from([(
                String::from("deep-kick"),
                String::from("sound-type"),
            )]),
        },
    };

    let env = TestConfigEnv::new();
    let path = env.path("cfg.toml");
    save_to_path(&cfg, &path).unwrap();
    let loaded_settings = load_settings_from(&path).unwrap();
    let library_state = LibraryState {
        sources: cfg.sources.clone(),
    };
    let round_trip = AppConfig::from((loaded_settings, library_state));

    SettingsRoundTripFixture {
        expected: cfg,
        actual: round_trip,
    }
}
