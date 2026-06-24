use std::sync::{Arc, RwLock};

use super::PlaybackSpanPlan;

#[derive(Clone, Debug)]
pub(crate) struct LoopSpanHandle {
    shared: Arc<LoopSpanShared>,
}

#[derive(Debug)]
struct LoopSpanShared {
    state: RwLock<LoopSpanState>,
}

#[derive(Clone, Copy, Debug)]
struct LoopSpanState {
    start_frame: u64,
    end_frame: u64,
    pending_seek_frame: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LoopSpanSnapshot {
    start_frame: u64,
    end_frame: u64,
}

impl LoopSpanHandle {
    pub(crate) fn from_plan(plan: &PlaybackSpanPlan) -> Self {
        let start_frame = plan.start_frame();
        let end_frame = plan.end_frame().max(start_frame.saturating_add(1));
        Self {
            shared: Arc::new(LoopSpanShared {
                state: RwLock::new(LoopSpanState {
                    start_frame,
                    end_frame,
                    pending_seek_frame: None,
                }),
            }),
        }
    }

    pub(crate) fn update_from_plan(&self, plan: &PlaybackSpanPlan, seek_frame: Option<u64>) {
        let start_frame = plan.start_frame();
        let end_frame = plan.end_frame().max(start_frame.saturating_add(1));
        let pending_seek_frame =
            seek_frame.map(|frame| clamp_frame_to_span(frame, start_frame, end_frame));
        if let Ok(mut state) = self.shared.state.write() {
            *state = LoopSpanState {
                start_frame,
                end_frame,
                pending_seek_frame,
            };
        }
    }

    pub(crate) fn snapshot(&self) -> LoopSpanSnapshot {
        let state = self
            .shared
            .state
            .read()
            .unwrap_or_else(|err| err.into_inner());
        LoopSpanSnapshot {
            start_frame: state.start_frame,
            end_frame: state.end_frame,
        }
    }

    pub(crate) fn take_pending_seek_frame(&self) -> Option<u64> {
        let mut state = self
            .shared
            .state
            .write()
            .unwrap_or_else(|err| err.into_inner());
        state.pending_seek_frame.take()
    }
}

impl LoopSpanSnapshot {
    pub(crate) fn start_frame(self) -> u64 {
        self.start_frame
    }

    pub(crate) fn contains(self, frame: u64) -> bool {
        (self.start_frame..self.end_frame).contains(&frame)
    }
}

fn clamp_frame_to_span(frame: u64, start_frame: u64, end_frame: u64) -> u64 {
    frame.clamp(start_frame, end_frame.saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{
        PlaybackChannelLayout, PlaybackSeekBehavior, PlaybackSourceIdentity, PlaybackSourceKind,
        PlaybackSpanRequest,
    };

    #[test]
    fn pending_seek_is_clamped_to_updated_span() {
        let handle = LoopSpanHandle::from_plan(&span_plan(0, 10, 0));

        handle.update_from_plan(&span_plan(20, 40, 0), Some(90));

        assert_eq!(handle.take_pending_seek_frame(), Some(39));
        assert_eq!(handle.take_pending_seek_frame(), None);
    }

    fn span_plan(start_frame: u64, end_frame: u64, offset_frame: u64) -> PlaybackSpanPlan {
        PlaybackSpanPlan::new(
            PlaybackSourceIdentity::new(PlaybackSourceKind::Bytes, None),
            PlaybackChannelLayout::new(1, 1_000).expect("layout"),
            PlaybackSpanRequest::new(
                start_frame as f32 / 1_000.0,
                end_frame as f32 / 1_000.0,
                1.0,
                true,
                PlaybackSeekBehavior::FrameOffset(offset_frame),
            ),
        )
        .expect("span plan")
    }
}
