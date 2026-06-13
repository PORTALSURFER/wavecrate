use super::*;

#[test]
fn save_runtime_and_updates_round_trip() {
    let fixture = settings_round_trip_fixture();

    assert_eq!(
        fixture.actual.core.feature_flags.autoplay_selection,
        fixture.expected.core.feature_flags.autoplay_selection
    );
    assert_eq!(
        fixture.actual.core.job_message_queue_capacity,
        fixture.expected.core.job_message_queue_capacity
    );
    assert_eq!(
        fixture.actual.core.updates.channel,
        fixture.expected.core.updates.channel
    );
    assert_eq!(
        fixture.actual.core.updates.check_on_startup,
        fixture.expected.core.updates.check_on_startup
    );
    assert_eq!(
        fixture.actual.core.updates.last_seen_nightly_published_at,
        fixture.expected.core.updates.last_seen_nightly_published_at
    );
}
