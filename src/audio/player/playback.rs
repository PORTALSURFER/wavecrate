use std::{sync::Arc, time::Duration};

use crate::audio::SamplesBuffer;
use crate::audio::timebase::{duration_for_frames, frames_to_seconds, seconds_to_frames_round};
use crate::audio::{AsyncSource, Source};

use super::super::decoder::SymphoniaDecoder;
use super::super::fade::{EdgeFade, fade_duration};
use super::super::mixer::{decoder_from_bytes, map_seek_error};
use super::{AudioPlayer, EditFadeSource};

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
        let bytes = self.audio_bytes()?;
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
                Box::new(crate::audio::loop_diagnostic::LoopDiagnostic::new(
                    source,
                    span_samples,
                ))
            } else {
                let loop_source = LazyRepeatingSpanSource::new(
                    bytes,
                    sample_rate,
                    channels,
                    0,
                    span_samples,
                    offset_frames,
                );
                let mut async_source = AsyncSource::new(loop_source);
                async_source.prefill();
                Box::new(crate::audio::loop_diagnostic::LoopDiagnostic::new(
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
        let bytes = self.audio_bytes()?;
        if duration <= 0.0 {
            return Err("Load a .wav file first".into());
        }

        self.fade_out_current_sink(self.anti_clip_fade());

        let sample_rate = self.sample_rate.unwrap_or(44_100).max(1);
        let channels = self.track_channels.unwrap_or(1).max(1);
        let (track_frames, start_frame, end_frame) =
            Self::quantize_span_bounds(start_seconds, end_seconds, duration, sample_rate);
        let span_frames = end_frame.saturating_sub(start_frame).max(1);
        let span_samples = span_frames.saturating_mul(channels as u64);
        let start_secs = frames_to_seconds(start_frame, sample_rate);
        let end_secs = frames_to_seconds(end_frame, sample_rate);

        let fade = fade_duration(
            (end_secs - start_secs).max(f32::EPSILON),
            self.anti_clip_fade(),
        );
        let final_source: Box<dyn Source<Item = f32> + Send> = if looped {
            let diagnostic: Box<dyn Source<Item = f32> + Send> =
                if let Some(samples) = self.playback_samples.as_ref().cloned() {
                    let start_sample = start_frame.saturating_mul(channels as u64) as usize;
                    let end_sample = start_sample.saturating_add(span_samples as usize);
                    let source = SamplesBuffer::from_arc_span(
                        channels,
                        sample_rate,
                        samples,
                        start_sample,
                        end_sample,
                    )
                    .repeat_infinite();
                    Box::new(crate::audio::loop_diagnostic::LoopDiagnostic::new(
                        source,
                        span_samples,
                    ))
                } else {
                    let loop_source = LazyRepeatingSpanSource::new(
                        bytes,
                        sample_rate,
                        channels,
                        start_frame,
                        span_samples,
                        0,
                    );
                    let mut async_source = AsyncSource::new(loop_source);
                    async_source.prefill();
                    Box::new(crate::audio::loop_diagnostic::LoopDiagnostic::new(
                        async_source,
                        span_samples,
                    ))
                };
            let editable = EditFadeSource::new_looped(
                diagnostic,
                self.edit_fade_handle.clone(),
                start_secs,
                span_frames,
                0,
            );
            Box::new(editable)
        } else {
            let source: Box<dyn Source<Item = f32> + Send> =
                if let Some(samples) = self.playback_samples.as_ref().cloned() {
                    let start_sample = start_frame.saturating_mul(channels as u64) as usize;
                    let end_sample = start_sample.saturating_add(span_samples as usize);
                    Box::new(SamplesBuffer::from_arc_span(
                        channels,
                        sample_rate,
                        samples,
                        start_sample,
                        end_sample,
                    ))
                } else {
                    let lazy_source = LazySpanSource::new(
                        bytes,
                        sample_rate,
                        channels,
                        start_frame,
                        span_samples,
                        duration,
                    );
                    let mut async_source = AsyncSource::new(lazy_source);
                    async_source.prefill();
                    Box::new(async_source.take_samples(span_samples as usize).buffered())
                };
            let editable = EditFadeSource::new(source, self.edit_fade_handle.clone(), start_secs);
            let faded = EdgeFade::new(editable, fade);
            Box::new(faded)
        };

        let (handle, format) = self.build_sink_with_fade(final_source)?;
        self.started_at = Some(std::time::Instant::now());
        self.play_span = Some((start_secs, end_secs));
        self.play_span_frames = Some((start_frame, end_frame));
        self.looping = looped;
        self.loop_offset = None;
        self.loop_offset_frames = None;
        self.track_duration = Some(frames_to_seconds(track_frames, sample_rate));
        self.track_total_frames = Some(track_frames);
        self.sample_rate = Some(sample_rate);
        self.fade_out = Some(handle);
        self.sink_format = Some(format);
        #[cfg(test)]
        {
            self.elapsed_override = None;
        }
        Ok(())
    }

    fn start_with_looped_span_offset(
        &mut self,
        start_seconds: f32,
        end_seconds: f32,
        duration: f32,
        offset_seconds: f32,
    ) -> Result<(), String> {
        let bytes = self.audio_bytes()?;
        if duration <= 0.0 {
            return Err("Load a .wav file first".into());
        }

        self.fade_out_current_sink(self.anti_clip_fade());

        let sample_rate = self.sample_rate.unwrap_or(44_100).max(1);
        let channels = self.track_channels.unwrap_or(1).max(1);
        let (track_frames, start_frame, end_frame) =
            Self::quantize_span_bounds(start_seconds, end_seconds, duration, sample_rate);
        let span_frames = end_frame.saturating_sub(start_frame).max(1);
        let span_samples = span_frames.saturating_mul(channels as u64);
        let offset_frames = if span_frames == 0 {
            0
        } else {
            seconds_to_frames_round(offset_seconds, sample_rate) % span_frames
        };

        let start_secs = frames_to_seconds(start_frame, sample_rate);
        let end_secs = frames_to_seconds(end_frame, sample_rate);
        let diagnostic: Box<dyn Source<Item = f32> + Send> =
            if let Some(samples) = self.playback_samples.as_ref().cloned() {
                let start_sample = start_frame.saturating_mul(channels as u64) as usize;
                let end_sample = start_sample.saturating_add(span_samples as usize);
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
                    start_secs,
                    span_frames,
                    offset_frames,
                );
                Box::new(crate::audio::loop_diagnostic::LoopDiagnostic::new(
                    editable.repeat_infinite(),
                    span_samples,
                ))
            } else {
                let loop_source = LazyRepeatingSpanSource::new(
                    bytes,
                    sample_rate,
                    channels,
                    start_frame,
                    span_samples,
                    offset_frames,
                );
                let mut async_source = AsyncSource::new(loop_source);
                async_source.prefill();
                let editable = EditFadeSource::new_looped(
                    async_source,
                    self.edit_fade_handle.clone(),
                    start_secs,
                    span_frames,
                    offset_frames,
                );
                Box::new(crate::audio::loop_diagnostic::LoopDiagnostic::new(
                    editable,
                    span_samples,
                ))
            };

        let (handle, format) = self.build_sink_with_fade(diagnostic)?;
        self.started_at = Some(std::time::Instant::now());
        self.play_span = Some((start_secs, end_secs));
        self.play_span_frames = Some((start_frame, end_frame));
        self.looping = true;
        self.loop_offset = Some(frames_to_seconds(offset_frames, sample_rate));
        self.loop_offset_frames = Some(offset_frames);
        self.track_duration = Some(frames_to_seconds(track_frames, sample_rate));
        self.track_total_frames = Some(track_frames);
        self.sample_rate = Some(sample_rate);
        self.fade_out = Some(handle);
        self.sink_format = Some(format);
        #[cfg(test)]
        {
            self.elapsed_override = None;
        }
        Ok(())
    }

    /// Calculate a frame-aligned span duration that never extends beyond the
    /// original floating-point span request.
    #[cfg(test)]
    pub(crate) fn aligned_span_duration(span_seconds: f32, sample_rate: u32) -> Duration {
        if sample_rate == 0 {
            return Duration::from_secs_f32(span_seconds.max(0.0));
        }
        let frames =
            crate::audio::timebase::seconds_to_frames_floor(span_seconds.max(0.0), sample_rate)
                .max(1);
        duration_for_frames(frames, sample_rate)
    }

    fn quantize_span_bounds(
        start_seconds: f32,
        end_seconds: f32,
        track_duration: f32,
        sample_rate: u32,
    ) -> (u64, u64, u64) {
        let track_frames = seconds_to_frames_round(track_duration.max(0.0), sample_rate).max(1);
        let mut start_frame =
            seconds_to_frames_round(start_seconds.max(0.0), sample_rate).min(track_frames - 1);
        let mut end_frame =
            seconds_to_frames_round(end_seconds.max(0.0), sample_rate).min(track_frames);

        if end_frame <= start_frame {
            end_frame = (start_frame + 1).min(track_frames);
        }
        if end_frame <= start_frame {
            start_frame = 0;
            end_frame = 1.min(track_frames);
        }
        (track_frames, start_frame, end_frame)
    }

    #[cfg(test)]
    pub(crate) fn loop_cycle_sample_count_for_tests(
        bytes: std::sync::Arc<[u8]>,
        start_seconds: f32,
        end_seconds: f32,
        offset_seconds: Option<f32>,
    ) -> Result<(usize, usize, u32, u16), String> {
        let duration = super::super::mixer::decoder_duration(&bytes)
            .or_else(|| super::super::mixer::wav_header_duration(&bytes))
            .ok_or_else(|| "Load a .wav file first".to_string())?;
        if duration <= 0.0 {
            return Err("Load a .wav file first".into());
        }

        let mut source = decoder_from_bytes(bytes)?;
        let sample_rate = source.sample_rate().max(1);
        let channels = source.channels().max(1);
        let (_, start_frame, end_frame) =
            Self::quantize_span_bounds(start_seconds, end_seconds, duration, sample_rate);
        let span_frames = end_frame.saturating_sub(start_frame).max(1);
        let span_samples = span_frames.saturating_mul(channels as u64);

        source
            .try_seek(duration_for_frames(start_frame, sample_rate))
            .map_err(map_seek_error)?;

        let mut limited = source.take_samples(span_samples as usize);
        let mut samples = Vec::with_capacity(span_samples as usize);
        for _ in 0..span_samples {
            if let Some(sample) = limited.next() {
                samples.push(sample);
            } else {
                break;
            }
        }
        while samples.len() < span_samples as usize {
            samples.push(0.0);
        }

        let offset_frames = offset_seconds
            .map(|seconds| seconds_to_frames_round(seconds, sample_rate) % span_frames)
            .unwrap_or(0);
        let offset_samples = offset_frames.saturating_mul(channels as u64) as usize;
        let buffer = SamplesBuffer::new(channels, sample_rate, samples);
        let mut looped: Box<dyn Source<Item = f32>> = if offset_seconds.is_some() {
            Box::new(buffer.repeat_infinite().skip_samples(offset_samples))
        } else {
            Box::new(buffer.repeat_infinite())
        };

        let mut emitted = 0usize;
        for _ in 0..span_samples as usize {
            if looped.next().is_none() {
                break;
            }
            emitted = emitted.saturating_add(1);
        }
        Ok((emitted, span_frames as usize, sample_rate, channels))
    }
}

struct LazySpanSource {
    bytes: Arc<[u8]>,
    decoder: Option<SymphoniaDecoder>,
    sample_rate: u32,
    channels: u16,
    seek_to: Duration,
    remaining_samples: usize,
    total_duration: Duration,
    last_error: Option<String>,
}

impl LazySpanSource {
    fn new(
        bytes: Arc<[u8]>,
        sample_rate: u32,
        channels: u16,
        start_frame: u64,
        span_samples: u64,
        total_duration: f32,
    ) -> Self {
        let sample_rate = sample_rate.max(1);
        let channels = channels.max(1);
        Self {
            bytes,
            decoder: None,
            sample_rate,
            channels,
            seek_to: duration_for_frames(start_frame, sample_rate),
            remaining_samples: span_samples as usize,
            total_duration: Duration::from_secs_f32(total_duration.max(0.0)),
            last_error: None,
        }
    }

    fn decoder_mut(&mut self) -> Option<&mut SymphoniaDecoder> {
        if self.decoder.is_none() {
            match decoder_from_bytes(Arc::clone(&self.bytes)).and_then(|mut decoder| {
                decoder.try_seek(self.seek_to).map_err(map_seek_error)?;
                Ok(decoder)
            }) {
                Ok(decoder) => {
                    self.decoder = Some(decoder);
                }
                Err(error) => {
                    self.last_error = Some(error);
                    self.remaining_samples = 0;
                    return None;
                }
            }
        }
        self.decoder.as_mut()
    }
}

impl Iterator for LazySpanSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_samples == 0 {
            return None;
        }
        let decoder = self.decoder_mut()?;
        match decoder.next() {
            Some(sample) => {
                self.remaining_samples = self.remaining_samples.saturating_sub(1);
                Some(sample)
            }
            None => {
                if let Some(error) = decoder.last_error() {
                    self.last_error = Some(error);
                }
                self.remaining_samples = 0;
                None
            }
        }
    }
}

impl Source for LazySpanSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.remaining_samples)
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(self.total_duration)
    }

    fn last_error(&self) -> Option<String> {
        self.last_error.clone()
    }
}

struct LazyRepeatingSpanSource {
    bytes: Arc<[u8]>,
    decoder: Option<SymphoniaDecoder>,
    sample_rate: u32,
    channels: u16,
    start_frame: u64,
    span_samples: u64,
    samples_into_cycle: u64,
    initial_offset_samples: u64,
    last_error: Option<String>,
}

impl LazyRepeatingSpanSource {
    fn new(
        bytes: Arc<[u8]>,
        sample_rate: u32,
        channels: u16,
        start_frame: u64,
        span_samples: u64,
        offset_frames: u64,
    ) -> Self {
        let sample_rate = sample_rate.max(1);
        let channels = channels.max(1);
        let span_samples = span_samples.max(channels as u64);
        let initial_offset_samples = offset_frames.saturating_mul(channels as u64) % span_samples;
        Self {
            bytes,
            decoder: None,
            sample_rate,
            channels,
            start_frame,
            span_samples,
            samples_into_cycle: initial_offset_samples,
            initial_offset_samples,
            last_error: None,
        }
    }

    fn decoder_mut(&mut self) -> Option<&mut SymphoniaDecoder> {
        if self.decoder.is_none() {
            if self
                .seek_to_cycle_position(self.initial_offset_samples)
                .is_err()
            {
                return None;
            }
        }
        self.decoder.as_mut()
    }

    fn seek_to_cycle_position(&mut self, cycle_sample_offset: u64) -> Result<(), ()> {
        let frame_offset = cycle_sample_offset / self.channels as u64;
        match decoder_from_bytes(Arc::clone(&self.bytes)).and_then(|mut decoder| {
            decoder
                .try_seek(duration_for_frames(
                    self.start_frame.saturating_add(frame_offset),
                    self.sample_rate,
                ))
                .map_err(map_seek_error)?;
            Ok(decoder)
        }) {
            Ok(decoder) => {
                self.decoder = Some(decoder);
                self.samples_into_cycle = cycle_sample_offset.min(self.span_samples);
                Ok(())
            }
            Err(error) => {
                self.last_error = Some(error);
                self.decoder = None;
                Err(())
            }
        }
    }

    fn restart_cycle(&mut self) -> Option<()> {
        self.seek_to_cycle_position(0).ok()
    }
}

impl Iterator for LazyRepeatingSpanSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples_into_cycle >= self.span_samples {
            self.restart_cycle()?;
        }
        let decoder = self.decoder_mut()?;
        match decoder.next() {
            Some(sample) => {
                self.samples_into_cycle = self.samples_into_cycle.saturating_add(1);
                Some(sample)
            }
            None => {
                if let Some(error) = decoder.last_error() {
                    self.last_error = Some(error);
                }
                self.restart_cycle()?;
                let sample = self.decoder_mut()?.next()?;
                self.samples_into_cycle = self.samples_into_cycle.saturating_add(1);
                Some(sample)
            }
        }
    }
}

impl Source for LazyRepeatingSpanSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }

    fn last_error(&self) -> Option<String> {
        self.last_error.clone()
    }
}
