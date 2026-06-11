use super::span::ResolvedPlaybackSpan;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct PlaybackIntent {
    pub(in crate::native_app) start_ratio: f32,
    pub(in crate::native_app) end_ratio: f32,
    pub(in crate::native_app) loop_offset_ratio: Option<f32>,
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

impl PlaybackIntent {
    pub(in crate::native_app) fn new(start_ratio: f32, end_ratio: f32) -> Self {
        Self {
            start_ratio,
            end_ratio,
            loop_offset_ratio: None,
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
    }
}
