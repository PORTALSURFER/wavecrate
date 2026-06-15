use crate::SamplesBuffer;
use crate::telemetry;
use crate::timebase::{frames_to_seconds, seconds_to_frames_round};
use crate::{AsyncSource, Source};

use super::super::fade::{EdgeFade, fade_duration};
use super::{
    AudioPlaybackSource, AudioPlayer, EditFadeSource, PlaybackChannelLayout, PlaybackSeekBehavior,
    PlaybackSpanPlan, PlaybackSpanRequest,
};

use lazy_sources::{
    InterleavedF32FileRepeatingSpanSource, InterleavedF32FileSpanSource, LazyRepeatingSpanSource,
    LazySpanSource,
};
mod lazy_sources;
#[cfg(test)]
mod test_support;

impl AudioPlayer {
    /// Begin playback from the stored buffer.
    pub fn play(&mut self) -> Result<(), String> {
        self.play_range(0.0, 1.0, false)
    }

    /// Begin playback at the given normalized position (0.0 - 1.0).
    pub fn play_from_fraction(&mut self, fraction: f64) -> Result<(), String> {
        self.play_range(fraction, 1.0, false)
    }

    /// Play between two normalized points, optionally looping the segment.
    pub fn play_range(&mut self, start: f64, end: f64, looped: bool) -> Result<(), String> {
        let (bounded_start, bounded_end, duration) = self.normalized_span(start, end)?;
        self.loop_offset = None;
        self.loop_offset_frames = None;
        self.start_with_span(bounded_start, bounded_end, duration, looped)
    }

    /// Loop a selection while starting playback at an offset within the selection.
    pub fn play_looped_range_from(
        &mut self,
        start: f64,
        end: f64,
        offset: f64,
    ) -> Result<(), String> {
        let (bounded_start, bounded_end, duration) = self.normalized_span(start, end)?;
        let clamped_offset = offset.clamp(start.min(end), start.max(end));
        let offset_seconds =
            ((clamped_offset * f64::from(duration)) - f64::from(bounded_start)).max(0.0) as f32;
        self.start_with_looped_span_offset(bounded_start, bounded_end, duration, offset_seconds)
    }

    /// Loop the full track while starting playback at the given normalized position.
    pub fn play_full_wrapped_from(&mut self, start: f64) -> Result<(), String> {
        let duration = self
            .track_duration
            .ok_or_else(|| "Load a .wav file first".to_string())?;
        if duration <= 0.0 {
            return Err("Load a .wav file first".into());
        }

        self.fade_out_current_sink(self.anti_clip_fade());

        let sample_rate = self.sample_rate.unwrap_or(44_100).max(1);
        let channels = self.track_channels.unwrap_or(1).max(1);

        let total_frames = self
            .track_total_frames
            .unwrap_or_else(|| seconds_to_frames_round(duration, sample_rate).max(1));
        let plan_duration = frames_to_seconds(total_frames, sample_rate);
        let offset_frame = ((start.clamp(0.0, 1.0) * total_frames as f64).floor() as u64)
            .min(total_frames.saturating_sub(1));
        let plan = self.playback_span_plan(
            0.0,
            plan_duration,
            plan_duration,
            true,
            PlaybackSeekBehavior::FrameOffset(offset_frame),
        )?;
        let diagnostic: Box<dyn Source<Item = f32> + Send> =
            if let Some(samples) = self.playback_samples.as_ref().cloned() {
                let source = SamplesBuffer::from_arc_span_at(
                    channels,
                    sample_rate,
                    samples,
                    0,
                    plan.sample_count() as usize,
                    plan.seek_sample(),
                )
                .repeat_infinite();
                Box::new(crate::loop_diagnostic::LoopDiagnostic::new(
                    source,
                    plan.sample_count(),
                ))
            } else {
                let loop_source = repeating_source_for_audio_source(self.audio_source()?, &plan)?;
                let mut async_source = AsyncSource::new(loop_source);
                async_source.prefill();
                Box::new(crate::loop_diagnostic::LoopDiagnostic::new(
                    async_source,
                    plan.sample_count(),
                ))
            };

        let (handle, format) = self.build_sink_with_fade(diagnostic)?;
        self.finish_span_playback(&plan, Some(plan.seek_offset_frames()));
        self.fade_out = Some(handle);
        self.sink_format = Some(format);
        Ok(())
    }

    fn start_with_span(
        &mut self,
        start_seconds: f32,
        end_seconds: f32,
        duration: f32,
        looped: bool,
    ) -> Result<(), String> {
        let total_started_at = playback_stage_started();
        let source_kind = self.current_source_kind();
        if duration <= 0.0 {
            return Err("Load a .wav file first".into());
        }

        let clear_started_at = playback_stage_started();
        self.fade_out_current_sink(self.anti_clip_fade());
        log_playback_stage(
            "clear_or_fade_current",
            clear_started_at,
            source_kind,
            looped,
        );

        let plan_started_at = playback_stage_started();
        let plan = self.playback_span_plan(
            start_seconds,
            end_seconds,
            duration,
            looped,
            PlaybackSeekBehavior::SpanStart,
        )?;
        log_playback_stage("span_plan", plan_started_at, source_kind, looped);
        let sample_rate = plan.layout().sample_rate();
        let channels = plan.layout().channels();

        let source_started_at = playback_stage_started();
        let fade = fade_duration(
            (plan.end_seconds() - plan.start_seconds()).max(f32::EPSILON),
            self.anti_clip_fade(),
        );
        let final_source: Box<dyn Source<Item = f32> + Send> = if looped {
            let diagnostic: Box<dyn Source<Item = f32> + Send> = if let Some(samples) =
                self.playback_samples.as_ref().cloned()
            {
                let source = SamplesBuffer::from_arc_span(
                    channels,
                    sample_rate,
                    samples,
                    plan.start_sample(),
                    plan.end_sample(),
                )
                .repeat_infinite();
                Box::new(crate::loop_diagnostic::LoopDiagnostic::new(
                    source,
                    plan.sample_count(),
                ))
            } else {
                let loop_source = repeating_source_for_audio_source(self.audio_source()?, &plan)?;
                let mut async_source = AsyncSource::new(loop_source);
                async_source.prefill();
                Box::new(crate::loop_diagnostic::LoopDiagnostic::new(
                    async_source,
                    plan.sample_count(),
                ))
            };
            let editable = EditFadeSource::new_looped(
                diagnostic,
                self.edit_fade_handle.clone(),
                plan.start_seconds(),
                plan.frame_count(),
                0,
            );
            Box::new(editable)
        } else {
            let source: Box<dyn Source<Item = f32> + Send> =
                if let Some(samples) = self.playback_samples.as_ref().cloned() {
                    Box::new(SamplesBuffer::from_arc_span(
                        channels,
                        sample_rate,
                        samples,
                        plan.start_sample(),
                        plan.end_sample(),
                    ))
                } else {
                    let lazy_source = span_source_for_audio_source(self.audio_source()?, &plan)?;
                    let mut async_source = AsyncSource::new(lazy_source);
                    async_source.prefill();
                    Box::new(
                        async_source
                            .take_samples(plan.sample_count() as usize)
                            .buffered(),
                    )
                };
            let editable =
                EditFadeSource::new(source, self.edit_fade_handle.clone(), plan.start_seconds());
            let faded = EdgeFade::new(editable, fade);
            Box::new(faded)
        };
        log_playback_stage(
            "source_construction",
            source_started_at,
            source_kind,
            looped,
        );

        let (handle, format) = self.build_sink_with_fade(final_source)?;
        let finish_started_at = playback_stage_started();
        self.finish_span_playback(&plan, None);
        self.fade_out = Some(handle);
        self.sink_format = Some(format);
        log_playback_stage("finish_span_state", finish_started_at, source_kind, looped);
        log_playback_stage(
            "start_with_span_total",
            total_started_at,
            source_kind,
            looped,
        );
        Ok(())
    }

    fn start_with_looped_span_offset(
        &mut self,
        start_seconds: f32,
        end_seconds: f32,
        duration: f32,
        offset_seconds: f32,
    ) -> Result<(), String> {
        let total_started_at = playback_stage_started();
        let source_kind = self.current_source_kind();
        if duration <= 0.0 {
            return Err("Load a .wav file first".into());
        }

        let clear_started_at = playback_stage_started();
        self.fade_out_current_sink(self.anti_clip_fade());
        log_playback_stage("clear_or_fade_current", clear_started_at, source_kind, true);

        let sample_rate = self.sample_rate.unwrap_or(44_100).max(1);
        let offset_frames = seconds_to_frames_round(offset_seconds, sample_rate);
        let plan_started_at = playback_stage_started();
        let plan = self.playback_span_plan(
            start_seconds,
            end_seconds,
            duration,
            true,
            PlaybackSeekBehavior::FrameOffset(offset_frames),
        )?;
        log_playback_stage("span_plan", plan_started_at, source_kind, true);
        let sample_rate = plan.layout().sample_rate();
        let channels = plan.layout().channels();

        let source_started_at = playback_stage_started();
        let diagnostic: Box<dyn Source<Item = f32> + Send> =
            if let Some(samples) = self.playback_samples.as_ref().cloned() {
                let source = SamplesBuffer::from_arc_span_at(
                    channels,
                    sample_rate,
                    samples,
                    plan.start_sample(),
                    plan.end_sample(),
                    plan.seek_sample(),
                );
                let editable = EditFadeSource::new_looped(
                    source,
                    self.edit_fade_handle.clone(),
                    plan.start_seconds(),
                    plan.frame_count(),
                    plan.seek_offset_frames(),
                );
                Box::new(crate::loop_diagnostic::LoopDiagnostic::new(
                    editable.repeat_infinite(),
                    plan.sample_count(),
                ))
            } else {
                let loop_source = repeating_source_for_audio_source(self.audio_source()?, &plan)?;
                let mut async_source = AsyncSource::new(loop_source);
                async_source.prefill();
                let editable = EditFadeSource::new_looped(
                    async_source,
                    self.edit_fade_handle.clone(),
                    plan.start_seconds(),
                    plan.frame_count(),
                    plan.seek_offset_frames(),
                );
                Box::new(crate::loop_diagnostic::LoopDiagnostic::new(
                    editable,
                    plan.sample_count(),
                ))
            };
        log_playback_stage("source_construction", source_started_at, source_kind, true);

        let (handle, format) = self.build_sink_with_fade(diagnostic)?;
        let finish_started_at = playback_stage_started();
        self.finish_span_playback(&plan, Some(plan.seek_offset_frames()));
        self.fade_out = Some(handle);
        self.sink_format = Some(format);
        log_playback_stage("finish_span_state", finish_started_at, source_kind, true);
        log_playback_stage(
            "start_with_looped_span_total",
            total_started_at,
            source_kind,
            true,
        );
        Ok(())
    }

    fn playback_span_plan(
        &self,
        start_seconds: f32,
        end_seconds: f32,
        duration: f32,
        looped: bool,
        seek: PlaybackSeekBehavior,
    ) -> Result<PlaybackSpanPlan, String> {
        let source = self.audio_source()?;
        let layout = PlaybackChannelLayout::new(
            self.track_channels.unwrap_or(1).max(1),
            self.sample_rate.unwrap_or(44_100).max(1),
        )
        .map_err(|err| err.to_string())?;
        PlaybackSpanPlan::new(
            source.identity(),
            layout,
            PlaybackSpanRequest::new(start_seconds, end_seconds, duration, looped, seek),
        )
        .map_err(|err| err.to_string())
    }

    fn finish_span_playback(&mut self, span: &PlaybackSpanPlan, offset_frames: Option<u64>) {
        let sample_rate = span.layout().sample_rate();
        self.started_at = Some(std::time::Instant::now());
        self.play_span = Some((span.start_seconds(), span.end_seconds()));
        self.play_span_frames = Some((span.start_frame(), span.end_frame()));
        self.looping = span.looped();
        self.loop_offset = offset_frames.map(|frames| frames_to_seconds(frames, sample_rate));
        self.loop_offset_frames = offset_frames;
        self.track_duration = Some(frames_to_seconds(span.track_frames(), sample_rate));
        self.track_total_frames = Some(span.track_frames());
        self.sample_rate = Some(sample_rate);
        #[cfg(test)]
        {
            self.elapsed_override = None;
        }
    }

    fn current_source_kind(&self) -> &'static str {
        self.current_audio
            .as_ref()
            .map(AudioPlaybackSource::kind)
            .unwrap_or("none")
    }
}

fn playback_stage_started() -> Option<std::time::Instant> {
    telemetry::playback_telemetry_enabled().then(std::time::Instant::now)
}

fn log_playback_stage(
    stage: &'static str,
    started_at: Option<std::time::Instant>,
    source_kind: &'static str,
    looped: bool,
) {
    let Some(started_at) = started_at else {
        return;
    };
    tracing::info!(
        target: "perf::audio_start",
        module = "reson_player",
        stage,
        source_kind,
        looped,
        elapsed_ms = telemetry::elapsed_ms(started_at.elapsed()),
        "Audio player stage"
    );
}

fn span_source_for_audio_source(
    source: AudioPlaybackSource,
    plan: &PlaybackSpanPlan,
) -> Result<Box<dyn Source<Item = f32> + Send>, String> {
    match source {
        AudioPlaybackSource::InterleavedF32File { path, sample_count } => Ok(Box::new(
            InterleavedF32FileSpanSource::new(path, plan, sample_count),
        )),
        source => Ok(Box::new(LazySpanSource::new(source, plan))),
    }
}

fn repeating_source_for_audio_source(
    source: AudioPlaybackSource,
    plan: &PlaybackSpanPlan,
) -> Result<Box<dyn Source<Item = f32> + Send>, String> {
    match source {
        AudioPlaybackSource::InterleavedF32File { path, sample_count } => Ok(Box::new(
            InterleavedF32FileRepeatingSpanSource::new(path, plan, sample_count),
        )),
        source => Ok(Box::new(LazyRepeatingSpanSource::new(source, plan))),
    }
}
