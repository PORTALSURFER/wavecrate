//! Regression tests for audio duration edge cases.

#[cfg(test)]
mod tests {
    use sempal::audio::Source;
    use std::time::Duration;

    /// A dummy source that produces infinite interleaved samples.
    struct EndlessSource {
        sample_rate: u32,
        channels: u16,
    }

    impl Iterator for EndlessSource {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            Some(0.0)
        }
    }

    impl Source for EndlessSource {
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
    }

    fn ceil_nanos_for_frames(frames: u64, sample_rate: u32) -> u64 {
        (frames * 1_000_000_000 + sample_rate as u64 - 1) / sample_rate as u64
    }

    #[test]
    fn test_duration_truncation() {
        let rate = 44100;
        let channels = 2; // Stereo
        let target_frames = 1;

        // Floating-point construction currently rounds this frame duration up to 22_676ns.
        let duration_f32 = Duration::from_secs_f32(1.0 / rate as f32);
        let source = EndlessSource {
            sample_rate: rate,
            channels,
        };
        let count_f32 = source.take_duration(duration_f32).count();

        // Integer ceiling preserves one full frame in nanoseconds.
        let nanos = ceil_nanos_for_frames(target_frames as u64, rate);
        let duration_u64 = Duration::from_nanos(nanos);
        let source2 = EndlessSource {
            sample_rate: rate,
            channels,
        };
        let count_u64 = source2.take_duration(duration_u64).count();

        let expected_samples = target_frames * channels as usize;
        assert!(
            count_f32 == expected_samples || count_f32 == expected_samples + channels as usize,
            "f32 duration should stay close to one-frame precision"
        );
        assert_eq!(
            count_f32 % channels as usize,
            0,
            "f32-derived duration must stay channel-aligned"
        );
        assert_eq!(
            count_u64, expected_samples,
            "u64 duration should yield exact frame samples"
        );
        assert_eq!(
            count_u64 % channels as usize,
            0,
            "sample count must stay channel-aligned"
        );
    }

    #[test]
    fn test_skip_duration_precision() {
        let rate = 44100;
        let target_frames = 1;

        let skip_f32 = Duration::from_secs_f32(1.0 / rate as f32);
        let nanos = (target_frames as u64 * 1_000_000_000) / rate as u64;
        let skip_u64 = Duration::from_nanos(nanos);

        let precise_nanos = (1_000_000_000.0f64 / 44100.0f64) as u64;
        assert!(
            skip_f32.as_nanos() > skip_u64.as_nanos(),
            "f32 duration rounds up compared with integer floor duration"
        );
        assert_eq!(
            skip_u64.as_nanos(),
            precise_nanos as u128,
            "u64 floor duration must match truncated one-frame nanoseconds"
        );
    }
}
