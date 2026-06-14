use super::*;
use std::path::PathBuf;
use std::time::Duration;

impl AppController {
    pub(super) fn request_auto_watcher_sync_for_source_if_due(
        &mut self,
        source_id: &SourceId,
        paths: Vec<PathBuf>,
        overflowed: bool,
        min_interval: Duration,
    ) {
        if overflowed || paths.is_empty() {
            self.request_auto_quick_sync_for_source_if_due(source_id, min_interval);
            return;
        }
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| &source.id == source_id)
            .cloned()
        else {
            return;
        };
        self.request_auto_targeted_sync_for_source(source, paths, min_interval);
    }
}
