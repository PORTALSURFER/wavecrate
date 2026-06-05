use crate::SamplesBuffer;
use crate::timebase::{frames_to_seconds, seconds_to_frames_round};
use crate::{AsyncSource, Source};

use super::super::fade::{EdgeFade, fade_duration};
use super::{AudioPlaybackSource, AudioPlayer, EditFadeSource};

use lazy_sources::{
    InterleavedF32FileRepeatingSpanSource, InterleavedF32FileSpanSource, LazyRepeatingSpanSource,
    LazySpanSource,
};
use span::QuantizedSpan;
mod lazy_sources;
mod span;
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
        if total_frames == 0 {
            return Err("Load a .wav file first".into());
        }

        let offset_frames = ((start.clamp(0.0, 1.0) * total_frames as f64).floor() as u64)
            .min(total_frames.saturating_sub(1));
        let span_samples = total_frames.saturating_mul(channels as u64);
        let diagnostic: Box<dyn Source<Item = f32> + Send> =
            if let Some(samples) = self.playback_samples.as_ref().cloned() {
                let offset_samples = offset_frames.saturating_mul(channels as u64) as usize;
                let source = SamplesBuffer::from_arc_span_at(
                    channels,
                    sample_rate,
                    samples,
                    0,
                    span_samples as usize,
                    offset_samples,
                )
                .repeat_infinite();
                Box::new(crate::loop_diagnostic::LoopDiagnostic::new(
                    source,
                    span_samples,
                ))
            } else {
                let loop_source = repeating_source_for_audio_source(
                    self.audio_source()?,
                    sample_rate,
                    channels,
                    0,
                    span_samples,
                    offset_frames,
                )?;
                let mut async_source = AsyncSource::new(loop_source);
                async_source.prefill();
                Box::new(crate::loop_diagnostic::LoopDiagnostic::new(
                    async_source,
                    span_samples,
                ))
            };

        let (handle, format) = self.build_sink_with_fade(diagnostic)?;
        let track_duration_seconds = frames_to_seconds(total_frames, sample_rate);
        self.started_at = Some(std::time::Instant::now());
        self.play_span = Some((0.0, track_duration_seconds));
        self.play_span_frames = Some((0, total_frames));
        self.looping = true;
        self.loop_offset = Some(frames_to_seconds(offset_frames, sample_rate));
        self.loop_offset_frames = Some(offset_frames);
        self.track_duration = Some(track_duration_seconds);
        self.track_total_frames = Some(total_frames);
        self.sample_rate = Some(sample_rate);
        self.fade_out = Some(handle);
        self.sink_format = Some(format);
        #[cfg(test)]
        {
            self.elapsed_override = None;
        }
        Ok(())
    }

    fn start_with_span(
        &mut self,
        start_seconds: f32,
        end_seconds: f32,
        duration: f32,
        looped: bool,
    ) -> Result<(), String> {
        if duration <= 0.0 {
            return Err("Load a .wav file first".into());
        }

        self.fade_out_current_sink(self.anti_clip_fade());

        let sample_rate = self.sample_rate.unwrap_or(44_100).max(1);
        let channels = self.track_channels.unwrap_or(1).max(1);
        let span = QuantizedSpan::new(start_seconds, end_seconds, duration, sample_rate, channels);

        let fade = fade_duration(
            (span.end_seconds - span.start_seconds).max(f32::EPSILON),
            self.anti_clip_fade(),
        );
        let final_source: Box<dyn Source<Item = f32> + Send> = if looped {
            let diagnostic: Box<dyn Source<Item = f32> + Send> =
                if let Some(samples) = self.playback_samples.as_ref().cloned() {
                    let start_sample = span.start_sample();
                    let end_sample = start_sample.saturating_add(span.samples as usize);
                    let source = SamplesBuffer::from_arc_span(
                        channels,
                        sample_rate,
                        samples,
                        start_sample,
                        end_sample,
                    )
                    .repeat_infinite();
                    Box::new(crate::loop_diagnostic::LoopDiagnostic::new(
                        source,
                        span.samples,
                    ))
                } else {
                    let loop_source = repeating_source_for_audio_source(
                        self.audio_source()?,
                        sample_rate,
                        channels,
                        span.start_frame,
                        span.samples,
                        0,
                    )?;
                    let mut async_source = AsyncSource::new(loop_source);
                    async_source.prefill();
                    Box::new(crate::loop_diagnostic::LoopDiagnostic::new(
                        async_source,
                        span.samples,
                    ))
                };
            let editable = EditFadeSource::new_looped(
                diagnostic,
                self.edit_fade_handle.clone(),
                span.start_seconds,
                span.frames,
                0,
            );
            Box::new(editable)
        } else {
            let source: Box<dyn Source<Item = f32> + Send> =
                if let Some(samples) = self.playback_samples.as_ref().cloned() {
                    let start_sample = span.start_sample();
                    let end_sample = start_sample.saturating_add(span.samples as usize);
                    Box::new(SamplesBuffer::from_arc_span(
                        channels,
                        sample_rate,
                        samples,
                        start_sample,
                        end_sample,
                    ))
                } else {
                    let lazy_source = span_source_for_audio_source(
                        self.audio_source()?,
                        sample_rate,
                        channels,
                        span.start_frame,
                        span.samples,
                        duration,
                    )?;
                    let mut async_source = AsyncSource::new(lazy_source);
                    async_source.prefill();
                    Box::new(async_source.take_samples(span.samples as usize).buffered())
                };
            let editable =
                EditFadeSource::new(source, self.edit_fade_handle.clone(), span.start_seconds);
            let faded = EdgeFade::new(editable, fade);
            Box::new(faded)
        };

        let (handle, format) = self.build_sink_with_fade(final_source)?;
        self.finish_span_playback(&span, sample_rate, looped, None);
        self.fade_out = Some(handle);
        self.sink_format = Some(format);
        Ok(())
    }

    fn start_with_looped_span_offset(
        &mut self,
        start_seconds: f32,
        end_seconds: f32,
        duration: f32,
        offset_seconds: f32,
    ) -> Result<(), String> {
        if duration <= 0.0 {
            return Err("Load a .wav file first".into());
        }

        self.fade_out_current_sink(self.anti_clip_fade());

        let sample_rate = self.sample_rate.unwrap_or(44_100).max(1);
        let channels = self.track_channels.unwrap_or(1).max(1);
        let span = QuantizedSpan::new(start_seconds, end_seconds, duration, sample_rate, channels);
        let offset_frames = seconds_to_frames_round(offset_seconds, sample_rate) % span.frames;

        let diagnostic: Box<dyn Source<Item = f32> + Send> =
            if let Some(samples) = self.playback_samples.as_ref().cloned() {
                let start_sample = span.start_sample();
                let end_sample = start_sample.saturating_add(span.samples as usize);
                let offset_samples = start_sample
                    .saturating_add(offset_frames.saturating_mul(channels as u64) as usize);
                let source = SamplesBuffer::from_arc_span_at(
                    channels,
                    sample_rate,
                    samples,
                    start_sample,
                    end_sample,
                    offset_samples,
                );
                let editable = EditFadeSource::new_looped(
                    source,
                    self.edit_fade_handle.clone(),
                    span.start_seconds,
                    span.frames,
                    offset_frames,
                );
                Box::new(crate::loop_diagnostic::LoopDiagnostic::new(
                    editable.repeat_infinite(),
                    span.samples,
                ))
            } else {
                let loop_source = repeating_source_for_audio_source(
                    self.audio_source()?,
                    sample_rate,
                    channels,
                    span.start_frame,
                    span.samples,
                    offset_frames,
                )?;
                let mut async_source = AsyncSource::new(loop_source);
                async_source.prefill();
                let editable = EditFadeSource::new_looped(
                    async_source,
                    self.edit_fade_handle.clone(),
                    span.start_seconds,
                    span.frames,
                    offset_frames,
                );
                Box::new(crate::loop_diagnostic::LoopDiagnostic::new(
                    editable,
                    span.samples,
                ))
            };

        let (handle, format) = self.build_sink_with_fade(diagnostic)?;
        self.finish_span_playback(&span, sample_rate, true, Some(offset_frames));
        self.fade_out = Some(handle);
        self.sink_format = Some(format);
        Ok(())
    }

    fn finish_span_playback(
        &mut self,
        span: &QuantizedSpan,
        sample_rate: u32,
        looped: bool,
        offset_frames: Option<u64>,
    ) {
        self.started_at = Some(std::time::Instant::now());
        self.play_span = Some((span.start_seconds, span.end_seconds));
        self.play_span_frames = Some((span.start_frame, span.end_frame));
        self.looping = looped;
        self.loop_offset = offset_frames.map(|frames| frames_to_seconds(frames, sample_rate));
        self.loop_offset_frames = offset_frames;
        self.track_duration = Some(frames_to_seconds(span.track_frames, sample_rate));
        self.track_total_frames = Some(span.track_frames);
        self.sample_rate = Some(sample_rate);
        #[cfg(test)]
        {
            self.elapsed_override = None;
        }
    }
}

fn span_source_for_audio_source(
    source: AudioPlaybackSource,
    sample_rate: u32,
    channels: u16,
    start_frame: u64,
    span_samples: u64,
    duration: f32,
) -> Result<Box<dyn Source<Item = f32> + Send>, String> {
    match source {
        AudioPlaybackSource::InterleavedF32File { path, sample_count } => {
            Ok(Box::new(InterleavedF32FileSpanSource::new(
                path,
                sample_rate,
                channels,
                start_frame,
                span_samples,
                sample_count,
                duration,
            )))
        }
        source => Ok(Box::new(LazySpanSource::new(
            source,
            sample_rate,
            channels,
            start_frame,
            span_samples,
            duration,
        ))),
    }
}

fn repeating_source_for_audio_source(
    source: AudioPlaybackSource,
    sample_rate: u32,
    channels: u16,
    start_frame: u64,
    span_samples: u64,
    offset_frames: u64,
) -> Result<Box<dyn Source<Item = f32> + Send>, String> {
    match source {
        AudioPlaybackSource::InterleavedF32File { path, sample_count } => {
            Ok(Box::new(InterleavedF32FileRepeatingSpanSource::new(
                path,
                sample_rate,
                channels,
                start_frame,
                span_samples,
                offset_frames,
                sample_count,
            )))
        }
        source => Ok(Box::new(LazyRepeatingSpanSource::new(
            source,
            sample_rate,
            channels,
            start_frame,
            span_samples,
            offset_frames,
        ))),
    }
}
