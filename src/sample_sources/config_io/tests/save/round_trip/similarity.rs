use super::*;

#[test]
fn similarity_aspect_settings_round_trip() {
    let fixture = settings_round_trip_fixture();

    assert_eq!(
        fixture.actual.core.similarity,
        fixture.expected.core.similarity
    );
}
