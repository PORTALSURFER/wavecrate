use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};

mod worker;
use worker::{LastPlayedPersistRequest, persist_last_played};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct LastPlayedPersistResult {
    pub(in crate::native_app) file_id: String,
    pub(in crate::native_app) result: Result<(), String>,
}

impl NativeAppState {
    pub(in crate::native_app) fn record_selected_sample_last_played(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(file_id) = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned)
        else {
            return;
        };
        self.record_sample_last_played(file_id, context);
    }

    pub(in crate::native_app) fn record_sample_last_played(
        &mut self,
        file_id: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let absolute_path = PathBuf::from(&file_id);
        let Some((source_root, relative_path)) = self
            .library
            .folder_browser
            .source_relative_file_path(&absolute_path)
        else {
            return;
        };
        let played_at = now_unix_secs();
        self.library
            .folder_browser
            .set_file_last_played_at(&absolute_path, played_at);
        let request = LastPlayedPersistRequest {
            file_id,
            source_root,
            relative_path,
            played_at,
        };
        context.business().idle("gui-last-played-persist").run(
            move |_| persist_last_played(request),
            GuiMessage::LastPlayedPersisted,
        );
    }

    pub(in crate::native_app) fn finish_last_played_persist(
        &mut self,
        result: LastPlayedPersistResult,
    ) {
        if let Err(error) = result.result {
            self.ui.status.sample = format!("Last played not saved: {error}");
            emit_gui_action(
                "playback.last_played.persist",
                Some("browser"),
                Some(result.file_id.as_str()),
                "error",
                std::time::Instant::now(),
                Some(&error),
            );
        }
    }
}

fn now_unix_secs() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    i64::try_from(secs).unwrap_or(i64::MAX)
}
