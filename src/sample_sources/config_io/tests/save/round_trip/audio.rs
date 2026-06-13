use super::*;

#[test]
fn save_audio_io_and_write_format_round_trip() {
    let fixture = settings_round_trip_fixture();

    assert_eq!(
        fixture.actual.core.audio_output,
        fixture.expected.core.audio_output
    );
    assert_eq!(
        fixture.actual.core.audio_input,
        fixture.expected.core.audio_input
    );
    assert_eq!(
        fixture.actual.core.audio_write_format,
        fixture.expected.core.audio_write_format
    );
    assert!((fixture.actual.core.volume - fixture.expected.core.volume).abs() < f32::EPSILON);
}
