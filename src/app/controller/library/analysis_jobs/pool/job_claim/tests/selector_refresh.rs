use super::*;

#[test]
fn shared_selector_refreshes_sources_once_across_idle_probes() {
    let dir = TempDir::new().unwrap();
    let source = SampleSource::new(dir.path().to_path_buf());
    let state = crate::sample_sources::library::LibraryState {
        sources: vec![source.clone()],
    };
    crate::sample_sources::library::save(&state).unwrap();
    let reset_done = Arc::new(Mutex::new(HashSet::new()));
    let shared = selection::shared(reset_done);
    let claim_wakeup = ClaimWakeup::new();

    {
        let mut selector = shared.lock().unwrap();
        assert!(matches!(
            selector.select_next(None, &claim_wakeup),
            selection::ClaimSelection::Idle
        ));
        assert_eq!(selector.refresh_count(), 1);
    }

    {
        let mut selector = shared.lock().unwrap();
        assert!(matches!(
            selector.select_next(None, &claim_wakeup),
            selection::ClaimSelection::Idle
        ));
        assert_eq!(
            selector.refresh_count(),
            1,
            "shared selector should reuse the first refresh during idle backoff"
        );
    }
}
