use super::*;

#[test]
fn save_analysis_settings_round_trip() {
    let fixture = settings_round_trip_fixture();

    assert_eq!(
        fixture.actual.core.analysis.max_analysis_duration_seconds,
        fixture.expected.core.analysis.max_analysis_duration_seconds
    );
    assert_eq!(
        fixture.actual.core.analysis.limit_similarity_prep_duration,
        fixture
            .expected
            .core
            .analysis
            .limit_similarity_prep_duration
    );
    assert_eq!(
        fixture.actual.core.analysis.long_sample_threshold_seconds,
        fixture.expected.core.analysis.long_sample_threshold_seconds
    );
    assert_eq!(
        fixture.actual.core.analysis.analysis_worker_count,
        fixture.expected.core.analysis.analysis_worker_count
    );
    assert_eq!(
        fixture.actual.core.analysis.fast_similarity_prep,
        fixture.expected.core.analysis.fast_similarity_prep
    );
    assert_eq!(
        fixture
            .actual
            .core
            .analysis
            .fast_similarity_prep_sample_rate,
        fixture
            .expected
            .core
            .analysis
            .fast_similarity_prep_sample_rate
    );
}
