use crate::native_app::{app::ReleaseUpdateStatus, test_support::state::NativeAppStateFixture};

#[test]
fn release_update_state_marks_available_release() {
    let mut state = NativeAppStateFixture::default().build();
    let ticket = state.background.release_update_check_task.begin();
    state.finish_release_update_check(radiant::prelude::TaskCompletion {
        ticket,
        output: Ok(Some(release_info(999))),
    });

    assert_eq!(
        state.ui.release_update.status,
        ReleaseUpdateStatus::Available
    );
    assert_eq!(
        state
            .ui
            .release_update
            .latest
            .as_ref()
            .map(|release| release.build_number),
        Some(999)
    );
}

#[test]
fn release_update_state_ignores_stale_completion() {
    let mut state = NativeAppStateFixture::default().build();
    let stale = state.background.release_update_check_task.begin();
    let current = state.background.release_update_check_task.begin();
    state.finish_release_update_check(radiant::prelude::TaskCompletion {
        ticket: stale,
        output: Ok(Some(release_info(998))),
    });

    assert_eq!(state.ui.release_update.status, ReleaseUpdateStatus::Idle);

    state.finish_release_update_check(radiant::prelude::TaskCompletion {
        ticket: current,
        output: Ok(None),
    });
    assert_eq!(
        state.ui.release_update.status,
        ReleaseUpdateStatus::UpToDate
    );
}

fn release_info(build_number: u64) -> wavecrate::updater::PublicReleaseInfo {
    wavecrate::updater::PublicReleaseInfo {
        build_id: format!("wavecrate-nightly-b{build_number}"),
        build_number,
        version: "nightly".to_string(),
        released_at: "2026-06-25T20:13:25.000Z".to_string(),
        download_page_url: wavecrate::updater::PUBLIC_RELEASE_PAGE_URL.to_string(),
    }
}
