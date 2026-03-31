use super::*;
use crate::app::controller::playback::audio_samples::decode_samples_from_bytes;
use std::time::Instant;

/// Start playback using the current loaded audio or queue loading first if needed.
pub(crate) fn play_audio(
    controller: &mut AppController,
    looped: bool,
    start_override: Option<f64>,
) -> Result<(), String> {
    if controller.is_recording() {
        return Err("Stop recording before playback".into());
    }
    controller.audio.pending_loop_disable_at = None;
    controller.audio.clear_pending_loop_retarget();
    if let Some((source, relative_path)) = browser_selection_playback_target(controller) {
        return queue_or_load_explicit_pending_playback(
            controller,
            &source,
            &relative_path,
            looped,
            start_override,
            false,
        );
    }
    if controller.sample_view.wav.loaded_audio.is_none() {
        return queue_or_load_pending_playback(controller, looped, start_override);
    }

    let player = super::ensure_player(controller)?;
    let Some(player) = player else {
        return Err("Audio unavailable".into());
    };
    configure_player_for_playback(controller, &player);
    let selection = playback_selection(controller);
    let span_end = selection.as_ref().map(|r| r.end()).unwrap_or(1.0);
    let (audition_start, audition_end) = audition_span(selection, looped, start_override, span_end);
    let audition_gain = normalized_audition_gain(controller, audition_start, audition_end);
    player.borrow_mut().set_playback_gain(audition_gain);
    let start = start_player_range(&player, selection, looped, start_override, span_end)?;
    sync_playback_ui(controller, start, span_end, start_override);
    refresh_waveform_image_if_view_stale(controller);
    controller.record_loaded_audio_playback();
    Ok(())
}

/// Return true if audio playback is currently active.
pub(crate) fn is_playing(controller: &AppController) -> bool {
    controller
        .audio
        .player
        .as_ref()
        .map(|p| p.borrow().is_playing())
        .unwrap_or(false)
}

/// Return live player progress while transport is actively running.
///
/// This accessor lets projection paths sample the transport clock directly so
/// animation-only redraws can avoid visible stepping between UI-state updates.
pub(crate) fn live_progress(controller: &AppController) -> Option<f32> {
    let player = controller.audio.player.as_ref()?;
    let player = player.borrow();
    player
        .is_playing()
        .then(|| player.progress())
        .flatten()
        .filter(|progress| progress.is_finite())
        .map(|progress| progress.clamp(0.0, 1.0))
}

fn queue_or_load_pending_playback(
    controller: &mut AppController,
    looped: bool,
    start_override: Option<f64>,
) -> Result<(), String> {
    if let Some(pending) = controller.runtime.jobs.pending_audio() {
        controller
            .runtime
            .jobs
            .set_pending_playback(Some(PendingPlayback {
                source_id: pending.source_id,
                relative_path: pending.relative_path,
                looped,
                start_override,
                force_loaded_audio: false,
            }));
        controller.set_status("Loading audio…", StatusTone::Busy);
        return Ok(());
    }

    let Some(selected) = controller.sample_view.wav.selected_wav.clone() else {
        return Err("Load a .wav file first".into());
    };
    let Some(source) = controller.current_source() else {
        return Err("Load a .wav file first".into());
    };
    let pending_playback = PendingPlayback {
        source_id: source.id.clone(),
        relative_path: selected.clone(),
        looped,
        start_override,
        force_loaded_audio: false,
    };
    controller
        .runtime
        .jobs
        .set_pending_playback(Some(pending_playback.clone()));
    controller.queue_audio_load_for(
        &source,
        &selected,
        AudioLoadIntent::Selection,
        Some(pending_playback),
    )?;
    controller.set_status(format!("Loading {}", selected.display()), StatusTone::Busy);
    Ok(())
}

fn queue_or_load_explicit_pending_playback(
    controller: &mut AppController,
    source: &SampleSource,
    relative_path: &Path,
    looped: bool,
    start_override: Option<f64>,
    force_loaded_audio: bool,
) -> Result<(), String> {
    if controller
        .runtime
        .jobs
        .pending_audio()
        .as_ref()
        .is_some_and(|pending| {
            pending.source_id == source.id && pending.relative_path.as_path() == relative_path
        })
    {
        controller
            .runtime
            .jobs
            .set_pending_playback(Some(PendingPlayback {
                source_id: source.id.clone(),
                relative_path: relative_path.to_path_buf(),
                looped,
                start_override,
                force_loaded_audio,
            }));
        controller.set_status("Loading audio…", StatusTone::Busy);
        return Ok(());
    }
    let pending_playback = PendingPlayback {
        source_id: source.id.clone(),
        relative_path: relative_path.to_path_buf(),
        looped,
        start_override,
        force_loaded_audio,
    };
    controller
        .runtime
        .jobs
        .set_pending_playback(Some(pending_playback.clone()));
    controller.queue_audio_load_for(
        source,
        relative_path,
        AudioLoadIntent::Selection,
        Some(pending_playback),
    )?;
    controller.set_status(
        format!("Loading {}", relative_path.display()),
        StatusTone::Busy,
    );
    Ok(())
}

/// Return the currently selected browser sample when browser focus should start new playback.
///
/// This suppresses redundant reloads when the selected browser row already
/// matches the loaded sample for the active source.
fn browser_selection_playback_target(
    controller: &mut AppController,
) -> Option<(SampleSource, PathBuf)> {
    if !matches!(
        controller.ui.focus.context,
        crate::app::state::FocusContext::SampleBrowser
    ) {
        return None;
    }
    let selected = controller.sample_view.wav.selected_wav.clone()?;
    let source = controller.current_source()?;
    let loaded_matches = controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .is_some_and(|audio| audio.source_id == source.id && audio.relative_path == selected);
    (!loaded_matches).then_some((source, selected))
}

fn configure_player_for_playback(controller: &AppController, player: &Rc<RefCell<AudioPlayer>>) {
    player
        .borrow_mut()
        .set_min_span_seconds(super::super::bpm_min_selection_seconds(controller));
    player
        .borrow()
        .set_edit_fade_state(controller.ui.waveform.edit_selection);
}

fn playback_selection(controller: &AppController) -> Option<SelectionRange> {
    crate::app::controller::playback::transport::playback_audition_selection(controller)
}

fn audition_span(
    selection: Option<SelectionRange>,
    looped: bool,
    start_override: Option<f64>,
    span_end: f32,
) -> (f32, f32) {
    if looped {
        selection
            .as_ref()
            .map(|range| (range.start(), range.end()))
            .unwrap_or((0.0, 1.0))
    } else {
        let span_start = start_override
            .map(|start| start.clamp(0.0, 1.0) as f32)
            .or_else(|| selection.as_ref().map(|range| range.start()))
            .unwrap_or(0.0);
        (span_start, span_end)
    }
}

fn start_player_range(
    player: &Rc<RefCell<AudioPlayer>>,
    selection: Option<SelectionRange>,
    looped: bool,
    start_override: Option<f64>,
    span_end: f32,
) -> Result<f32, String> {
    if looped {
        return start_looped_range(player, selection, start_override);
    }

    let start = start_override
        .map(|start| start.clamp(0.0, 1.0))
        .or_else(|| selection.as_ref().map(|range| f64::from(range.start())))
        .unwrap_or(0.0);
    player
        .borrow_mut()
        .play_range(start, f64::from(span_end), false)?;
    Ok(start as f32)
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

fn sync_playback_ui(
    controller: &mut AppController,
    start: f32,
    span_end: f32,
    start_override: Option<f64>,
) {
    controller.ui.waveform.playhead.active_span_end = Some(span_end.clamp(0.0, 1.0));
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = start;
    super::super::playhead_trail::start_or_seek_trail(
        &mut controller.ui.waveform.playhead,
        start,
        start_override.is_some(),
    );
    if start_override.is_some() {
        controller.ui.waveform.playhead.recent_seek = Some(crate::app::state::PlayheadSeek {
            position: start,
            started_at: Instant::now(),
        });
    }
}

/// Refresh the waveform raster before playback overlays animate over it.
///
/// Zoom and selection interactions can leave a queued or stale waveform image
/// behind the current view window until the next explicit redraw. When playback
/// starts, the selection/playhead overlays immediately begin using the current
/// zoom bounds, so refresh the raster first when those retained inputs drift.
fn refresh_waveform_image_if_view_stale(controller: &mut AppController) {
    let Some(decoded) = controller.sample_view.waveform.decoded.as_ref() else {
        return;
    };
    let Some(render_meta) = controller.sample_view.waveform.render_meta.as_ref() else {
        controller.refresh_waveform_image();
        return;
    };
    let view = controller.ui.waveform.view.clamp();
    let stale_view = (render_meta.view_start - view.start).abs() > f64::EPSILON
        || (render_meta.view_end - view.end).abs() > f64::EPSILON;
    let stale_layout = render_meta.size != controller.sample_view.waveform.size
        || render_meta.samples_len != decoded.frame_count()
        || render_meta.channel_view != controller.ui.waveform.channel_view
        || render_meta.channels != decoded.channels;
    if stale_view || stale_layout || controller.ui.waveform.image.is_none() {
        controller.refresh_waveform_image();
    }
}

fn normalized_audition_gain(controller: &AppController, start: f32, end: f32) -> f32 {
    if !controller.ui.waveform.normalized_audition_enabled {
        return 1.0;
    }
    let Some(peak) = normalized_audition_peak(controller, start, end) else {
        return 1.0;
    };
    if peak <= f32::EPSILON {
        return 1.0;
    }
    1.0 / peak
}

/// Resolve the peak amplitude used for normalized audition over one playback span.
///
/// The retained decoded waveform is the fast path, but it is not guaranteed to
/// be present for every loaded sample. Plain transport playback should still
/// honor normalized audition in that state, so fall back to the loaded audio
/// bytes when the waveform decode cache is unavailable.
fn normalized_audition_peak(controller: &AppController, start: f32, end: f32) -> Option<f32> {
    if let Some(decoded) = controller.sample_view.waveform.decoded.as_ref() {
        return decoded.max_abs_in_span(start, end);
    }
    let loaded = controller.sample_view.wav.loaded_audio.as_ref()?;
    let decoded = decode_samples_from_bytes(&loaded.bytes).ok()?;
    max_abs_from_samples_span(
        &decoded.samples,
        decoded.channels.max(1) as usize,
        start,
        end,
    )
}

/// Compute the largest absolute sample amplitude inside one normalized span.
fn max_abs_from_samples_span(
    samples: &[f32],
    channels: usize,
    start: f32,
    end: f32,
) -> Option<f32> {
    if samples.is_empty() || !start.is_finite() || !end.is_finite() {
        return None;
    }
    let channels = channels.max(1);
    let total_frames = samples.len() / channels;
    if total_frames == 0 {
        return None;
    }
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    let start_frame = (start.clamp(0.0, 1.0) * total_frames as f32).floor() as usize;
    let mut end_frame = (end.clamp(0.0, 1.0) * total_frames as f32).ceil() as usize;
    if end_frame <= start_frame {
        end_frame = (start_frame + 1).min(total_frames);
    }
    let start_idx = start_frame.saturating_mul(channels);
    let end_idx = end_frame.saturating_mul(channels).min(samples.len());
    (start_idx < end_idx).then(|| {
        samples[start_idx..end_idx]
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
    })
}
