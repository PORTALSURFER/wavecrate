use super::{
    AudioAppState, SamplePlaybackSession, SamplePlaybackSessionState, SamplePlaybackVisibility,
};
use wavecrate::audio::PlaybackRequestId;

#[derive(Clone, Copy, Debug, PartialEq)]
struct PendingPlaybackSpanRetarget {
    request_id: u64,
    span: (f32, f32),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PlaybackSpanRetargetState {
    confirmed_span: (f32, f32),
    pending: Vec<PendingPlaybackSpanRetarget>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) enum PlaybackSpanRetargetRejection {
    Superseded,
    Restore((f32, f32)),
}

impl PlaybackSpanRetargetState {
    pub(super) fn new(confirmed_span: (f32, f32)) -> Self {
        Self {
            confirmed_span,
            pending: Vec::new(),
        }
    }
}

impl SamplePlaybackSession {
    pub(in crate::native_app) fn record_span_retarget(
        &mut self,
        request_id: PlaybackRequestId,
        span: (f32, f32),
    ) {
        self.record_span_retarget_id(request_id.get(), span);
    }

    pub(super) fn record_span_retarget_id(&mut self, request_id: u64, span: (f32, f32)) {
        self.request.span = span;
        self.span_retarget
            .pending
            .push(PendingPlaybackSpanRetarget { request_id, span });
    }

    pub(in crate::native_app) fn has_pending_span_retarget(&self) -> bool {
        !self.span_retarget.pending.is_empty()
    }

    pub(in crate::native_app) fn confirmed_span(&self) -> (f32, f32) {
        self.span_retarget.confirmed_span
    }

    pub(in crate::native_app) fn confirm_span_retarget_id(&mut self, request_id: u64) -> bool {
        let Some(index) = self
            .span_retarget
            .pending
            .iter()
            .position(|pending| pending.request_id == request_id)
        else {
            return false;
        };
        self.span_retarget.confirmed_span = self.span_retarget.pending[index].span;
        self.span_retarget.pending.drain(..=index);
        true
    }

    pub(in crate::native_app) fn reject_span_retarget(
        &mut self,
        request_id: PlaybackRequestId,
    ) -> Option<PlaybackSpanRetargetRejection> {
        self.reject_span_retarget_id(request_id.get())
    }

    fn reject_span_retarget_id(
        &mut self,
        request_id: u64,
    ) -> Option<PlaybackSpanRetargetRejection> {
        let index = self
            .span_retarget
            .pending
            .iter()
            .position(|pending| pending.request_id == request_id)?;
        self.span_retarget.pending.drain(..=index);
        if self.span_retarget.pending.is_empty() {
            self.request.span = self.span_retarget.confirmed_span;
            Some(PlaybackSpanRetargetRejection::Restore(
                self.span_retarget.confirmed_span,
            ))
        } else {
            Some(PlaybackSpanRetargetRejection::Superseded)
        }
    }

    pub(super) fn set_confirmed_span(&mut self, span: (f32, f32)) {
        self.request.span = span;
        self.span_retarget = PlaybackSpanRetargetState::new(span);
    }
}

impl AudioAppState {
    pub(in crate::native_app) fn promote_sample_playback_session_to_waveform(
        &mut self,
        path: &str,
    ) -> bool {
        let Some(session) = self.sample_playback_session.as_mut() else {
            return false;
        };
        if session.request.path != path
            || session.source_kind == "preview_samples"
            || !matches!(
                session.state,
                SamplePlaybackSessionState::RuntimePending
                    | SamplePlaybackSessionState::AudibleTransient
                    | SamplePlaybackSessionState::WaveformVisible
            )
        {
            return false;
        }
        session.request.visibility = SamplePlaybackVisibility::Waveform;
        if !matches!(session.state, SamplePlaybackSessionState::RuntimePending) {
            session.state = SamplePlaybackSessionState::WaveformVisible;
        }
        true
    }

    #[cfg(test)]
    pub(in crate::native_app) fn record_span_retarget_for_tests(
        &mut self,
        request_id: u64,
        span: (f32, f32),
    ) {
        self.sample_playback_session
            .as_mut()
            .expect("sample playback session")
            .record_span_retarget_id(request_id, span);
    }

    #[cfg(test)]
    pub(in crate::native_app) fn confirm_span_retarget_for_tests(
        &mut self,
        request_id: u64,
    ) -> bool {
        self.sample_playback_session
            .as_mut()
            .expect("sample playback session")
            .confirm_span_retarget_id(request_id)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::super::{
        SamplePlaybackHistory, SamplePlaybackIntent, SamplePlaybackRequest,
        SamplePlaybackSessionState,
    };
    use super::*;

    fn waveform_session(span: (f32, f32)) -> SamplePlaybackSession {
        SamplePlaybackSession {
            generation: 1,
            request: SamplePlaybackRequest::waveform(
                String::from("sample.wav"),
                span,
                SamplePlaybackIntent::WaveformSpan,
                "test",
                SamplePlaybackHistory::Skip,
            ),
            runtime_request_id: None,
            source_kind: "decoded_samples",
            submitted_at: Instant::now(),
            audible_started_at: None,
            state: SamplePlaybackSessionState::RuntimePending,
            span_retarget: PlaybackSpanRetargetState::new(span),
        }
    }

    #[test]
    fn pending_retarget_is_authoritative_before_original_start_arrives() {
        let mut session = waveform_session((0.0, 1.0));

        session.record_span_retarget_id(2, (0.25, 0.60));

        assert_eq!(session.request.span, (0.25, 0.60));
        assert!(session.has_pending_span_retarget());
        assert_eq!(session.confirmed_span(), (0.0, 1.0));
    }

    #[test]
    fn latest_coalesced_retarget_confirmation_discards_superseded_requests() {
        let mut session = waveform_session((0.20, 0.60));
        session.record_span_retarget_id(2, (0.20, 0.80));
        session.record_span_retarget_id(3, (0.10, 0.70));

        assert!(session.confirm_span_retarget_id(3));

        assert!(!session.has_pending_span_retarget());
        assert_eq!(session.span_retarget.confirmed_span, (0.10, 0.70));
        assert_eq!(session.reject_span_retarget_id(2), None);
    }

    #[test]
    fn latest_retarget_failure_restores_last_confirmed_span() {
        let mut session = waveform_session((0.20, 0.60));
        session.record_span_retarget_id(2, (0.20, 0.80));
        session.record_span_retarget_id(3, (0.10, 0.70));
        assert!(session.confirm_span_retarget_id(2));

        let restored = session.reject_span_retarget_id(3);

        assert_eq!(
            restored,
            Some(PlaybackSpanRetargetRejection::Restore((0.20, 0.80)))
        );
        assert_eq!(session.request.span, (0.20, 0.80));
    }

    #[test]
    fn stale_retarget_failure_keeps_newer_requested_span() {
        let mut session = waveform_session((0.20, 0.60));
        session.record_span_retarget_id(2, (0.20, 0.80));
        session.record_span_retarget_id(3, (0.10, 0.70));

        let restored = session.reject_span_retarget_id(2);

        assert_eq!(restored, Some(PlaybackSpanRetargetRejection::Superseded));
        assert_eq!(session.request.span, (0.10, 0.70));
        assert!(session.has_pending_span_retarget());
    }
}
