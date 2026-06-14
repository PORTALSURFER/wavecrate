use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use radiant::prelude as ui;
use wavecrate::sample_sources::SourceDatabase;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct LastPlayedPersistResult {
    pub(in crate::native_app) file_id: String,
    pub(in crate::native_app) result: Result<(), String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LastPlayedPersistRequest {
    file_id: String,
    source_root: PathBuf,
    relative_path: PathBuf,
    played_at: i64,
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
        context
            .business()
            .background("gui-last-played-persist")
            .run(
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

fn persist_last_played(request: LastPlayedPersistRequest) -> LastPlayedPersistResult {
    let result = persist_last_played_inner(&request);
    LastPlayedPersistResult {
        file_id: request.file_id,
        result,
    }
}

fn persist_last_played_inner(request: &LastPlayedPersistRequest) -> Result<(), String> {
    let (file_size, modified_ns) =
        file_metadata(&request.source_root.join(&request.relative_path))?;
    let db = SourceDatabase::open_for_user_metadata_write(&request.source_root)
        .map_err(|err| err.to_string())?;
    let mut batch = db.write_batch().map_err(|err| err.to_string())?;
    batch
        .upsert_file(&request.relative_path, file_size, modified_ns)
        .map_err(|err| err.to_string())?;
    batch
        .set_last_played_at(&request.relative_path, request.played_at)
        .map_err(|err| err.to_string())?;
    batch.commit().map_err(|err| err.to_string())
}

fn file_metadata(path: &Path) -> Result<(u64, i64), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let modified_ns = metadata
        .modified()
        .map_err(|err| format!("Missing modified time for {}: {err}", path.display()))?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| String::from("File modified time is before epoch"))?
        .as_nanos() as i64;
    Ok((metadata.len(), modified_ns))
}

fn now_unix_secs() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    i64::try_from(secs).unwrap_or(i64::MAX)
}
