use std::{collections::BTreeSet, path::PathBuf, time::Instant};

use super::MAX_PENDING_PATHS_PER_SOURCE;

#[derive(Debug)]
pub(super) struct PendingGuiSourceWatch {
    pub(super) last_event: Instant,
    pub(super) paths: BTreeSet<PathBuf>,
    pub(super) overflowed: bool,
}

impl PendingGuiSourceWatch {
    pub(super) fn new(last_event: Instant, path: Option<PathBuf>) -> Self {
        let mut pending = Self {
            last_event,
            paths: BTreeSet::new(),
            overflowed: false,
        };
        pending.add_path(path);
        pending
    }

    pub(super) fn add_path(&mut self, path: Option<PathBuf>) {
        let Some(path) = path else {
            self.overflowed = true;
            self.paths.clear();
            return;
        };
        if self.paths.len() >= MAX_PENDING_PATHS_PER_SOURCE {
            self.overflowed = true;
            self.paths.clear();
            return;
        }
        self.paths.insert(path);
    }
}

#[derive(Debug)]
pub(super) struct GuiSourceWatchEvent {
    pub(super) source_id: String,
    pub(super) paths: Vec<PathBuf>,
    pub(super) overflowed: bool,
}
