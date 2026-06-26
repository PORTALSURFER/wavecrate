use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};

type ReleaseUpdateCheckResult = Result<Option<wavecrate::updater::PublicReleaseInfo>, String>;

impl NativeAppState {
    pub(in crate::native_app) fn maybe_start_release_update_check(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if !self.ui.startup.release_update_check_pending {
            return;
        }
        self.ui.startup.release_update_check_pending = false;
        if !self.ui.settings.persisted.updates.check_on_startup {
            return;
        }
        if self.background.release_update_check_task.active().is_some() {
            return;
        }

        let ticket = self.background.release_update_check_task.begin();
        let current_build_number = current_build_number();
        self.ui.release_update.begin_check();
        context
            .business()
            .background("gui-release-update-check")
            .run(
                move |_| ui::TaskCompletion {
                    ticket,
                    output: run_release_update_check(current_build_number),
                },
                GuiMessage::ReleaseUpdateCheckFinished,
            );
    }

    pub(in crate::native_app) fn finish_release_update_check(
        &mut self,
        completion: ui::TaskCompletion<ReleaseUpdateCheckResult>,
    ) {
        let Some(result) = self
            .background
            .release_update_check_task
            .finish_completion(completion)
        else {
            return;
        };
        if let Err(error) = &result {
            tracing::debug!("Release update check failed: {error}");
        }
        self.ui.release_update.finish(result);
    }

    pub(in crate::native_app) fn open_release_download_page(&mut self) {
        let url = self
            .ui
            .release_update
            .latest
            .as_ref()
            .map(|release| release.download_page_url.as_str())
            .unwrap_or(wavecrate::updater::PUBLIC_RELEASE_PAGE_URL);
        if let Err(error) = wavecrate::updater::open_release_page(url) {
            self.ui.status.sample = format!("Could not open release page: {error}");
        }
    }
}

fn run_release_update_check(current_build_number: u64) -> ReleaseUpdateCheckResult {
    wavecrate::updater::check_public_release_catalog(
        wavecrate::updater::PublicReleaseCheckRequest::current(current_build_number),
    )
    .map_err(|err| err.to_string())
}

fn current_build_number() -> u64 {
    env!("WAVECRATE_BUILD_NUMBER")
        .parse::<u64>()
        .unwrap_or_default()
}
