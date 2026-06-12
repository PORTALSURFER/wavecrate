//! Test-only controls for folder projection execution mode.

use std::{cell::Cell, thread_local};

thread_local! {
    static FOLDER_PROJECTION_ASYNC_OVERRIDE: Cell<Option<bool>> = const { Cell::new(None) };
}

/// Resolve whether folder projection should run through the async path.
pub(super) fn folder_projection_async_enabled() -> bool {
    folder_projection_async_override_for_tests().unwrap_or(false)
}

fn folder_projection_async_override_for_tests() -> Option<bool> {
    FOLDER_PROJECTION_ASYNC_OVERRIDE.with(|value| value.get())
}

/// Run a closure with folder projection async behavior overridden for tests.
pub(crate) fn with_folder_projection_async_enabled_for_tests<T>(
    enabled: bool,
    run: impl FnOnce() -> T,
) -> T {
    struct Reset<'a> {
        cell: &'a Cell<Option<bool>>,
        previous: Option<bool>,
    }

    impl Drop for Reset<'_> {
        fn drop(&mut self) {
            self.cell.set(self.previous);
        }
    }

    FOLDER_PROJECTION_ASYNC_OVERRIDE.with(|value| {
        let previous = value.replace(Some(enabled));
        let _reset = Reset {
            cell: value,
            previous,
        };
        run()
    })
}
