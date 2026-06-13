use super::*;

#[test]
fn save_naming_and_tag_dictionary_round_trip() {
    let fixture = settings_round_trip_fixture();

    assert_eq!(
        fixture.actual.core.default_identifier,
        fixture.expected.core.default_identifier
    );
    assert_eq!(
        fixture.actual.core.tag_dictionary,
        fixture.expected.core.tag_dictionary
    );
}
