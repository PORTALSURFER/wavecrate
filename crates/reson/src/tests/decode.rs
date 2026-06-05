use std::sync::Arc;

use super::support::fixtures;
use crate::{Source, decoder::SymphoniaDecoder};

#[test]
fn decode_handles_varied_sample_rates_and_channels() {
    let specs = [
        fixtures::ToneSpec::new(8_000, 1, 0.25).with_pulse(fixtures::TonePulse {
            start_seconds: 0.0,
            duration_seconds: 0.05,
            amplitude: 0.9,
        }),
        fixtures::ToneSpec::new(48_000, 2, 1.2).with_pulse(fixtures::TonePulse {
            start_seconds: 0.9,
            duration_seconds: 0.1,
            amplitude: 0.6,
        }),
        fixtures::ToneSpec::new(11_025, 2, 0.5).with_pulse(fixtures::TonePulse {
            start_seconds: 0.4,
            duration_seconds: 0.05,
            amplitude: 0.75,
        }),
    ];

    for spec in specs {
        let fixture = fixtures::build_fixture(spec);
        assert_fixture_decodes(fixture);
    }
}

#[test]
fn decode_handles_various_bit_depths() {
    let sample_rate = 44100;
    let channels = 2;
    let duration = 0.5;

    let formats = [
        (16, hound::SampleFormat::Int),
        (24, hound::SampleFormat::Int),
        (32, hound::SampleFormat::Int),
        (32, hound::SampleFormat::Float),
    ];

    for (bits, format) in formats {
        println!("Testing bits: {}, format: {:?}", bits, format);
        let spec = fixtures::ToneSpec::new(sample_rate, channels, duration)
            .with_bit_depth(bits, format)
            .with_pulse(fixtures::TonePulse {
                start_seconds: 0.1,
                duration_seconds: 0.2,
                amplitude: 0.8,
            });

        let fixture = fixtures::build_fixture(spec);
        assert_fixture_decodes(fixture);
    }
}

fn assert_fixture_decodes(fixture: fixtures::WavFixture) {
    assert!(fixture.path.is_file());
    let mut decoder =
        SymphoniaDecoder::from_bytes(Arc::from(fixture.bytes.clone())).expect("decode fixture");
    assert_eq!(decoder.sample_rate(), fixture.spec.sample_rate);
    assert_eq!(decoder.channels(), fixture.spec.channels);
    let duration = decoder.total_duration().expect("duration");
    assert!((duration.as_secs_f32() - fixture.spec.duration_seconds).abs() < 0.02);

    let decoded: Vec<f32> = decoder.by_ref().collect();
    let expected_samples = fixture
        .frames
        .saturating_mul(fixture.spec.channels as usize);
    assert_eq!(decoded.len(), expected_samples);

    let pulse = fixture.spec.pulses.first().expect("missing pulse");
    let sample_time = pulse.start_seconds + pulse.duration_seconds * 0.5;
    let idx = fixture.sample_index_at(sample_time);
    let expected = fixture.expected_amplitude_at(sample_time);
    let actual = decoded[idx];
    assert!(
        (actual - expected).abs() < 1e-4,
        "Amplitude mismatch at time {sample_time}: expected {expected}, got {actual}"
    );
}
