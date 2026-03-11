use super::Source;
use std::time::Duration;

#[derive(Clone)]
struct DummySource {
    sample_rate: u32,
    channels: u16,
    next_value: f32,
}

impl Iterator for DummySource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.next_value;
        self.next_value += 1.0;
        Some(value)
    }
}

impl Source for DummySource {
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

#[test]
fn take_samples_returns_exact_count() {
    let source = DummySource {
        sample_rate: 48_000,
        channels: 2,
        next_value: 0.0,
    };
    let count = source.take_samples(5).count();
    assert_eq!(count, 5);
}

#[test]
fn take_duration_preserves_channel_alignment() {
    let source = DummySource {
        sample_rate: 44_100,
        channels: 2,
        next_value: 0.0,
    };
    let duration = Duration::from_nanos(22_676);
    let count = source.take_duration(duration).count();
    assert_eq!(count, 2);
    assert_eq!(count % 2, 0);
}

#[test]
fn skip_duration_uses_floor_semantics() {
    let source = DummySource {
        sample_rate: 44_100,
        channels: 2,
        next_value: 0.0,
    };
    let duration = Duration::from_nanos(22_675);
    let mut skipped = source.skip_duration(duration);
    let first = skipped.next().expect("sample");
    assert_eq!(first as usize, 1);
}
