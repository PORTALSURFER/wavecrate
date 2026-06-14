use super::request::PlaybackRequest;
use super::*;

pub(super) struct PlaybackRangePlan {
    pub(super) selection: Option<SelectionRange>,
    pub(super) span_end: f32,
    pub(super) audition_start: f32,
    pub(super) audition_end: f32,
}

pub(super) fn configure_player_for_playback(
    controller: &AppController,
    player: &Rc<RefCell<AudioPlayer>>,
) {
    player
        .borrow_mut()
        .set_min_span_seconds(super::super::super::bpm_min_selection_seconds(controller));
    player
        .borrow()
        .set_edit_fade_state(crate::audio::edit_fade_range_from_selection(
            controller.ui.waveform.edit_selection,
        ));
}

pub(super) fn plan_playback_range(
    controller: &AppController,
    request: &PlaybackRequest,
) -> PlaybackRangePlan {
    let selection = playback_selection(controller);
    let span_end = selection.as_ref().map(|r| r.end()).unwrap_or(1.0);
    let (audition_start, audition_end) = audition_span(
        selection.as_ref(),
        request.looped,
        request.start_override,
        span_end,
    );
    PlaybackRangePlan {
        selection,
        span_end,
        audition_start,
        audition_end,
    }
}

pub(super) fn start_player_range(
    player: &Rc<RefCell<AudioPlayer>>,
    plan: PlaybackRangePlan,
    request: &PlaybackRequest,
) -> Result<f32, String> {
    if request.looped {
        return start_looped_range(player, plan.selection, request.start_override);
    }

    let start = request
        .start_override
        .map(|start| start.clamp(0.0, 1.0))
        .or_else(|| {
            plan.selection
                .as_ref()
                .map(|range| f64::from(range.start()))
        })
        .unwrap_or(0.0);
    player
        .borrow_mut()
        .play_range(start, f64::from(plan.span_end), false)?;
    Ok(start as f32)
}

fn playback_selection(controller: &AppController) -> Option<SelectionRange> {
    crate::app::controller::playback::transport::playback_audition_selection(controller)
}

fn audition_span(
    selection: Option<&SelectionRange>,
    looped: bool,
    start_override: Option<f64>,
    span_end: f32,
) -> (f32, f32) {
    if looped {
        selection
            .map(|range| (range.start(), range.end()))
            .unwrap_or((0.0, 1.0))
    } else {
        let span_start = start_override
            .map(|start| start.clamp(0.0, 1.0) as f32)
            .or_else(|| selection.map(SelectionRange::start))
            .unwrap_or(0.0);
        (span_start, span_end)
    }
}

fn start_looped_range(
    player: &Rc<RefCell<AudioPlayer>>,
    selection: Option<SelectionRange>,
    start_override: Option<f64>,
) -> Result<f32, String> {
    if let Some(range) = selection {
        return play_looped_selection(player, range, start_override);
    }
    if let Some(start_pos) = start_override {
        player.borrow_mut().play_full_wrapped_from(start_pos)?;
        return Ok(start_pos as f32);
    }
    player.borrow_mut().play_range(0.0, 1.0, true)?;
    Ok(0.0)
}

fn play_looped_selection(
    player: &Rc<RefCell<AudioPlayer>>,
    range: SelectionRange,
    start_override: Option<f64>,
) -> Result<f32, String> {
    if let Some(start_pos) = start_override
        && start_pos >= f64::from(range.start())
        && start_pos <= f64::from(range.end())
    {
        player.borrow_mut().play_looped_range_from(
            f64::from(range.start()),
            f64::from(range.end()),
            start_pos,
        )?;
        return Ok(start_pos as f32);
    }

    let start = range.start();
    player
        .borrow_mut()
        .play_range(f64::from(range.start()), f64::from(range.end()), true)?;
    Ok(start)
}
