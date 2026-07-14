use super::*;
use crate::player::{
    PlaybackChannelLayout, PlaybackSeekBehavior, PlaybackSourceIdentity, PlaybackSourceKind,
    PlaybackSpanRequest,
};

fn span_plan(offset_frame: u64) -> PlaybackSpanPlan {
    PlaybackSpanPlan::new(
        PlaybackSourceIdentity::new(PlaybackSourceKind::Bytes, None),
        PlaybackChannelLayout::new(1, 1_000).expect("layout"),
        PlaybackSpanRequest::new(
            0.2,
            0.8,
            1.0,
            true,
            PlaybackSeekBehavior::FrameOffset(offset_frame),
        ),
    )
    .expect("span plan")
}

#[test]
fn non_seeking_retarget_rebases_progress_from_runtime_time() {
    let plan = span_plan(40);

    let offset = live_retarget_progress_offset_frames(&plan, false, Some(525));

    assert_eq!(offset, 325);
}

#[test]
fn non_seeking_retarget_wraps_progress_that_left_the_new_span() {
    let plan = span_plan(40);

    let offset = live_retarget_progress_offset_frames(&plan, false, Some(825));

    assert_eq!(offset, 0);
}

#[test]
fn seeking_retarget_uses_the_requested_offset() {
    let plan = span_plan(40);

    let offset = live_retarget_progress_offset_frames(&plan, true, Some(525));

    assert_eq!(offset, 40);
}
