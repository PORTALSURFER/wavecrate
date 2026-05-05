thread_local! {
    static STUB_STARTUP_AUDIO_REFRESH_FOR_TESTS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static STARTUP_AUDIO_REFRESH_COUNT_FOR_TESTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Stub startup audio probing so tests can assert the deferral boundary without touching drivers.
pub(crate) fn with_stubbed_startup_audio_refresh_for_tests<T>(run: impl FnOnce() -> T) -> T {
    STUB_STARTUP_AUDIO_REFRESH_FOR_TESTS.with(|enabled| enabled.set(true));
    STARTUP_AUDIO_REFRESH_COUNT_FOR_TESTS.with(|count| count.set(0));
    let result = run();
    STUB_STARTUP_AUDIO_REFRESH_FOR_TESTS.with(|enabled| enabled.set(false));
    result
}

/// Return how many deferred startup audio refreshes were requested in the current test scope.
pub(crate) fn startup_audio_refresh_count_for_tests() -> usize {
    STARTUP_AUDIO_REFRESH_COUNT_FOR_TESTS.with(std::cell::Cell::get)
}

/// Record a stubbed startup audio refresh and return whether probing should be skipped.
pub(crate) fn record_startup_audio_refresh_for_tests() -> bool {
    if !STUB_STARTUP_AUDIO_REFRESH_FOR_TESTS.with(std::cell::Cell::get) {
        return false;
    }
    STARTUP_AUDIO_REFRESH_COUNT_FOR_TESTS.with(|count| {
        count.set(count.get().saturating_add(1));
    });
    true
}
