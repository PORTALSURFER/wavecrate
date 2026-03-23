#[cfg(test)]
use std::time::Duration;

use crate::audio::SamplesBuffer;
use crate::audio::timebase::{duration_for_frames, frames_to_seconds, seconds_to_frames_round};
use crate::audio::{AsyncSource, Source};

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

        let source = decoder_from_bytes(bytes)?;
        let sample_rate = source.sample_rate().max(1);
        let channels = source.channels().max(1);
        let mut samples: Vec<f32> = source.collect();
        let frame_width = channels as usize;
        let usable_samples = (samples.len() / frame_width) * frame_width;
        samples.truncate(usable_samples);
        let total_frames = (usable_samples / frame_width) as u64;
        if total_frames == 0 {
            return Err("Load a .wav file first".into());
        }

        let buffer = SamplesBuffer::new(channels, sample_rate, samples);
        let offset_frames = ((start.clamp(0.0, 1.0) * total_frames as f64).floor() as u64)
            .min(total_frames.saturating_sub(1));
        let offset_samples = offset_frames.saturating_mul(channels as u64) as usize;
        let repeated = buffer.repeat_infinite().skip_samples(offset_samples);

        let (handle, format) = self.build_sink_with_fade(repeated);
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

        let mut source = decoder_from_bytes(bytes)?;
        let sample_rate = source.sample_rate().max(1);
        let channels = source.channels().max(1);
        let (track_frames, start_frame, end_frame) =
            Self::quantize_span_bounds(start_seconds, end_seconds, duration, sample_rate);
        let span_frames = end_frame.saturating_sub(start_frame).max(1);
        let span_samples = span_frames.saturating_mul(channels as u64);
        let start_secs = frames_to_seconds(start_frame, sample_rate);
        let end_secs = frames_to_seconds(end_frame, sample_rate);

        source
            .try_seek(duration_for_frames(start_frame, sample_rate))
            .map_err(map_seek_error)?;

        let fade = fade_duration(
            (end_secs - start_secs).max(f32::EPSILON),
            self.anti_clip_fade(),
        );
        let final_source: Box<dyn Source<Item = f32> + Send> = if looped {
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
            let buffer = SamplesBuffer::new(channels, sample_rate, samples);
            let diagnostic = crate::audio::loop_diagnostic::LoopDiagnostic::new(
                buffer.repeat_infinite(),
                span_samples,
            );
            let editable = EditFadeSource::new_looped(
                diagnostic,
                self.edit_fade_handle.clone(),
                start_secs,
                span_frames,
                0,
            );
            Box::new(editable)
        } else {
            let mut async_source = AsyncSource::new(source);
            async_source.prefill();
            let limited = async_source.take_samples(span_samples as usize).buffered();
            let editable = EditFadeSource::new(limited, self.edit_fade_handle.clone(), start_secs);
            let faded = EdgeFade::new(editable, fade);
            Box::new(faded)
        };

        let (handle, format) = self.build_sink_with_fade(final_source);
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

        let mut source = decoder_from_bytes(bytes)?;
        let sample_rate = source.sample_rate().max(1);
        let channels = source.channels().max(1);
        let (track_frames, start_frame, end_frame) =
            Self::quantize_span_bounds(start_seconds, end_seconds, duration, sample_rate);
        let span_frames = end_frame.saturating_sub(start_frame).max(1);
        let span_samples = span_frames.saturating_mul(channels as u64);
        let offset_frames = if span_frames == 0 {
            0
        } else {
            seconds_to_frames_round(offset_seconds, sample_rate) % span_frames
        };

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

        let buffer = SamplesBuffer::new(channels, sample_rate, samples);
        let offset_samples = offset_frames.saturating_mul(channels as u64) as usize;
        let start_secs = frames_to_seconds(start_frame, sample_rate);
        let end_secs = frames_to_seconds(end_frame, sample_rate);
        let editable = EditFadeSource::new_looped(
            buffer,
            self.edit_fade_handle.clone(),
            start_secs,
            span_frames,
            offset_frames,
        );
        let repeated = editable.repeat_infinite().skip_samples(offset_samples);
        let diagnostic = crate::audio::loop_diagnostic::LoopDiagnostic::new(repeated, span_samples);

        let (handle, format) = self.build_sink_with_fade(diagnostic);
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
}
