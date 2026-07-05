use crate::native_app::app::{
    SamplePlaybackIntent, SamplePlaybackRequest, SamplePlaybackSourceProbe,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::native_app) struct SamplePlaybackAvailableSources {
    pub(in crate::native_app) loaded_decoded_samples: bool,
    pub(in crate::native_app) loaded_f32_cache: bool,
    pub(in crate::native_app) loaded_audio_bytes: bool,
    pub(in crate::native_app) persisted_f32_descriptor: bool,
    pub(in crate::native_app) file_backed_audio_descriptor: bool,
    pub(in crate::native_app) preview_clip: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub(in crate::native_app) enum SamplePlaybackPlan {
    SubmitRuntime { source_kind: &'static str },
    QueueLoad,
    QueuePreviewDecode,
    PromoteActiveSession,
    HandOffPreviewToFullSource,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::native_app) struct ActiveSamplePlaybackPlanState {
    pub(in crate::native_app) same_path: bool,
    pub(in crate::native_app) preview_source: bool,
    pub(in crate::native_app) streamable_source: bool,
    pub(in crate::native_app) waveform_visible: bool,
}

pub(in crate::native_app) fn plan_sample_playback(
    request: &SamplePlaybackRequest,
    sources: SamplePlaybackAvailableSources,
    active: ActiveSamplePlaybackPlanState,
) -> SamplePlaybackPlan {
    if active.same_path
        && active.streamable_source
        && request.intent == SamplePlaybackIntent::SettledNavigation
        && request.visibility.updates_waveform_playhead()
    {
        return SamplePlaybackPlan::PromoteActiveSession;
    }
    if active.same_path
        && active.preview_source
        && !preview_allowed_for_intent(request.intent)
        && real_source_available(sources)
    {
        return SamplePlaybackPlan::HandOffPreviewToFullSource;
    }
    if sources.loaded_decoded_samples {
        return SamplePlaybackPlan::SubmitRuntime {
            source_kind: "decoded_samples",
        };
    }
    if sources.loaded_f32_cache {
        return SamplePlaybackPlan::SubmitRuntime {
            source_kind: "interleaved_f32_file",
        };
    }
    if sources.loaded_audio_bytes {
        return SamplePlaybackPlan::SubmitRuntime {
            source_kind: "audio_bytes",
        };
    }
    if sources.persisted_f32_descriptor {
        return SamplePlaybackPlan::SubmitRuntime {
            source_kind: "interleaved_f32_file",
        };
    }
    if sources.file_backed_audio_descriptor
        && request.source_probe == SamplePlaybackSourceProbe::AllowFileProbe
    {
        return SamplePlaybackPlan::SubmitRuntime {
            source_kind: "audio_file",
        };
    }
    if sources.preview_clip && preview_allowed_for_intent(request.intent) {
        return SamplePlaybackPlan::SubmitRuntime {
            source_kind: "preview_samples",
        };
    }
    if preview_allowed_for_intent(request.intent) {
        SamplePlaybackPlan::QueuePreviewDecode
    } else {
        SamplePlaybackPlan::QueueLoad
    }
}

fn preview_allowed_for_intent(intent: SamplePlaybackIntent) -> bool {
    matches!(
        intent,
        SamplePlaybackIntent::TransientNavigation | SamplePlaybackIntent::StarmapDrag
    )
}

fn real_source_available(sources: SamplePlaybackAvailableSources) -> bool {
    sources.loaded_decoded_samples
        || sources.loaded_f32_cache
        || sources.loaded_audio_bytes
        || sources.persisted_f32_descriptor
        || sources.file_backed_audio_descriptor
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::app::{
        SamplePlaybackHistory, SamplePlaybackIntent, SamplePlaybackRequest,
    };

    fn request(intent: SamplePlaybackIntent) -> SamplePlaybackRequest {
        SamplePlaybackRequest::waveform(
            String::from("kick.wav"),
            (0.0, 1.0),
            intent,
            "test",
            SamplePlaybackHistory::Record,
        )
    }

    #[test]
    fn planner_prefers_real_sources_over_preview() {
        let plan = plan_sample_playback(
            &request(SamplePlaybackIntent::ExplicitPlayback),
            SamplePlaybackAvailableSources {
                persisted_f32_descriptor: true,
                preview_clip: true,
                ..Default::default()
            },
            ActiveSamplePlaybackPlanState::default(),
        );

        assert_eq!(
            plan,
            SamplePlaybackPlan::SubmitRuntime {
                source_kind: "interleaved_f32_file"
            }
        );
    }

    #[test]
    fn transient_navigation_may_use_preview() {
        let plan = plan_sample_playback(
            &SamplePlaybackRequest::transient(
                String::from("hat.wav"),
                SamplePlaybackIntent::TransientNavigation,
                "browser",
            ),
            SamplePlaybackAvailableSources {
                preview_clip: true,
                ..Default::default()
            },
            ActiveSamplePlaybackPlanState::default(),
        );

        assert_eq!(
            plan,
            SamplePlaybackPlan::SubmitRuntime {
                source_kind: "preview_samples"
            }
        );
    }

    #[test]
    fn normalized_playback_never_uses_preview() {
        let plan = plan_sample_playback(
            &request(SamplePlaybackIntent::NormalizedResume),
            SamplePlaybackAvailableSources {
                preview_clip: true,
                ..Default::default()
            },
            ActiveSamplePlaybackPlanState::default(),
        );

        assert_eq!(plan, SamplePlaybackPlan::QueueLoad);
    }

    #[test]
    fn settled_streamable_session_promotes_without_restart() {
        let plan = plan_sample_playback(
            &request(SamplePlaybackIntent::SettledNavigation),
            SamplePlaybackAvailableSources::default(),
            ActiveSamplePlaybackPlanState {
                same_path: true,
                streamable_source: true,
                ..Default::default()
            },
        );

        assert_eq!(plan, SamplePlaybackPlan::PromoteActiveSession);
    }

    #[test]
    fn explicit_same_path_playback_still_starts_runtime() {
        let plan = plan_sample_playback(
            &request(SamplePlaybackIntent::ExplicitPlayback),
            SamplePlaybackAvailableSources {
                loaded_decoded_samples: true,
                ..Default::default()
            },
            ActiveSamplePlaybackPlanState {
                same_path: true,
                streamable_source: true,
                waveform_visible: true,
                ..Default::default()
            },
        );

        assert_eq!(
            plan,
            SamplePlaybackPlan::SubmitRuntime {
                source_kind: "decoded_samples"
            }
        );
    }
}
