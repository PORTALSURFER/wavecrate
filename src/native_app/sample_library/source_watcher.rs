use std::time::Duration;

mod classification;
mod debounce;
mod handle;
mod path_mapping;
mod roots;
mod state;

const WATCHER_POLL_INTERVAL: Duration = Duration::from_millis(200);
const SOURCE_CHANGE_DEBOUNCE: Duration = Duration::from_millis(400);
const MAX_PENDING_PATHS_PER_SOURCE: usize = 512;

pub(in crate::native_app) use handle::GuiSourceWatcherHandle;

#[cfg(test)]
#[path = "source_watcher/tests.rs"]
mod tests;
