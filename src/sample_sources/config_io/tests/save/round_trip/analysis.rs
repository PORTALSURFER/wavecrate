use super::*;

#[test]
fn save_analysis_settings_round_trip() {
    let fixture = settings_round_trip_fixture();

    assert_eq!(
        fixture.actual.core.analysis.max_analysis_duration_seconds,
        fixture.expected.core.analysis.max_analysis_duration_seconds
    );
    assert_eq!(
        fixture.actual.core.analysis.long_sample_threshold_seconds,
        fixture.expected.core.analysis.long_sample_threshold_seconds
    );
    assert_eq!(
        fixture.actual.core.analysis.analysis_worker_count,
        fixture.expected.core.analysis.analysis_worker_count
    );
}
