use radiant::prelude as ui;

use crate::native_app::app::{
    GlobalStorageUsageState, GuiMessage, NativeAppState, SettingsMessage,
};

impl NativeAppState {
    pub(in crate::native_app) fn queue_global_storage_usage_refresh(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.ui.settings.ui.global_storage_usage = GlobalStorageUsageState::Loading;
        context
            .business()
            .blocking_io("gui-global-storage-usage")
            .latest(&mut self.background.global_storage_usage_task)
            .run(
                |_| wavecrate::app_dirs::global_storage_usage(),
                |completion| {
                    GuiMessage::Settings(SettingsMessage::GlobalStorageUsageFinished(completion))
                },
            );
    }

    pub(in crate::native_app) fn finish_global_storage_usage_refresh(
        &mut self,
        completion: ui::TaskCompletion<Result<wavecrate::app_dirs::GlobalStorageUsage, String>>,
    ) {
        let Some(result) = self
            .background
            .global_storage_usage_task
            .finish_completion(completion)
        else {
            return;
        };
        match result {
            Ok(usage) => {
                self.ui.settings.ui.global_storage_usage = GlobalStorageUsageState::Ready(usage);
            }
            Err(error) => {
                tracing::warn!(
                    target: "wavecrate::debug::storage",
                    event = "global_storage_usage.failed",
                    error = error.as_str(),
                    "Failed to measure global database and cache usage"
                );
                self.ui.settings.ui.global_storage_usage = GlobalStorageUsageState::Unavailable;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_completion_ignores_stale_measurements() {
        let mut state =
            crate::native_app::test_support::state::NativeAppStateFixture::default().build();
        let stale_ticket = state.background.global_storage_usage_task.begin();
        let current_ticket = state.background.global_storage_usage_task.begin();

        state.finish_global_storage_usage_refresh(ui::TaskCompletion {
            ticket: stale_ticket,
            output: Ok(wavecrate::app_dirs::GlobalStorageUsage {
                database_bytes: 1,
                cache_bytes: 2,
            }),
        });
        assert_eq!(
            state.ui.settings.ui.global_storage_usage,
            GlobalStorageUsageState::NotLoaded
        );

        state.finish_global_storage_usage_refresh(ui::TaskCompletion {
            ticket: current_ticket,
            output: Ok(wavecrate::app_dirs::GlobalStorageUsage {
                database_bytes: 3,
                cache_bytes: 5,
            }),
        });
        assert_eq!(
            state.ui.settings.ui.global_storage_usage,
            GlobalStorageUsageState::Ready(wavecrate::app_dirs::GlobalStorageUsage {
                database_bytes: 3,
                cache_bytes: 5,
            })
        );
    }
}
