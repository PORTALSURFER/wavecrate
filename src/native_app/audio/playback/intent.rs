use super::span::ResolvedPlaybackSpan;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct PlaybackIntent {
    pub(in crate::native_app) start_ratio: f32,
    pub(in crate::native_app) end_ratio: f32,
    pub(in crate::native_app) loop_offset_ratio: Option<f32>,
    pub(in crate::native_app) show_start_marker: bool,
    pub(in crate::native_app) loop_region_policy: PlaybackLoopRegionPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct PlaybackCommand {
    pub(in crate::native_app) intent: PlaybackIntent,
    pub(in crate::native_app) resolved: ResolvedPlaybackSpan,
    pub(in crate::native_app) mode: PlaybackMode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) enum PlaybackMode {
    OneShot,
    Looped { offset_ratio: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum PlaybackLoopRegionPolicy {
    FollowPlaySelection,
    UseIntentSpan,
}

impl PlaybackIntent {
    pub(in crate::native_app) fn new(start_ratio: f32, end_ratio: f32) -> Self {
        Self {
            start_ratio,
            end_ratio,
            loop_offset_ratio: None,
            show_start_marker: true,
            loop_region_policy: PlaybackLoopRegionPolicy::FollowPlaySelection,
        }
    }

    pub(in crate::native_app) fn with_loop_offset(
        start_ratio: f32,
        end_ratio: f32,
        loop_offset_ratio: Option<f32>,
    ) -> Self {
        Self {
            start_ratio,
            end_ratio,
            loop_offset_ratio,
            show_start_marker: true,
            loop_region_policy: PlaybackLoopRegionPolicy::FollowPlaySelection,
        }
    }

    pub(in crate::native_app) fn random_region(start_ratio: f32, end_ratio: f32) -> Self {
        Self::fixed_region(start_ratio, end_ratio)
    }

    pub(in crate::native_app) fn fixed_region(start_ratio: f32, end_ratio: f32) -> Self {
        Self {
            loop_region_policy: PlaybackLoopRegionPolicy::UseIntentSpan,
            ..Self::new(start_ratio, end_ratio)
        }
    }
}

impl PlaybackCommand {
    pub(in crate::native_app) fn from_intent(
        intent: PlaybackIntent,
        resolved: ResolvedPlaybackSpan,
        loop_playback: bool,
    ) -> Self {
        let mode = if loop_playback {
            PlaybackMode::Looped {
                offset_ratio: resolved.offset_ratio,
            }
        } else {
            PlaybackMode::OneShot
        };
        Self {
            intent,
            resolved,
            mode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_preserves_requested_intent() {
        let intent = PlaybackIntent::with_loop_offset(0.2, 0.8, Some(0.4));
        let resolved = ResolvedPlaybackSpan {
            start_ratio: 0.1,
            end_ratio: 0.6,
            offset_ratio: 0.3,
        };
        let command = PlaybackCommand::from_intent(intent, resolved, true);

        assert_eq!(command.intent.start_ratio, 0.2);
        assert_eq!(command.intent.end_ratio, 0.8);
        assert_eq!(command.intent.loop_offset_ratio, Some(0.4));
        assert!(command.intent.show_start_marker);
        assert_eq!(
            command.intent.loop_region_policy,
            PlaybackLoopRegionPolicy::FollowPlaySelection
        );
    }

    #[test]
    fn command_uses_loop_mode_when_loop_playback_is_enabled() {
        let intent = PlaybackIntent::new(0.2, 0.8);
        let resolved = ResolvedPlaybackSpan {
            start_ratio: 0.2,
            end_ratio: 0.8,
            offset_ratio: 0.2,
        };
        let command = PlaybackCommand::from_intent(intent, resolved, true);

        assert_eq!(command.mode, PlaybackMode::Looped { offset_ratio: 0.2 });
    }

    #[test]
    fn random_region_intent_uses_requested_span_for_loop_region() {
        let intent = PlaybackIntent::random_region(0.2, 0.8);

        assert_eq!(
            intent.loop_region_policy,
            PlaybackLoopRegionPolicy::UseIntentSpan
        );
    }
}
