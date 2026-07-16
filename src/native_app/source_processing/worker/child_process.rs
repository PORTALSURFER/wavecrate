//! Shared cancellation and join policy for CPU-owning native child processes.

use std::{
    process::Child,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

const CHILD_POLL_INTERVAL: Duration = Duration::from_millis(10);

pub(in crate::native_app) fn wait_for_cancellable_child(
    mut child: Child,
    cancel: &AtomicBool,
    operation: &str,
) -> Result<Option<Child>, String> {
    loop {
        if cancel.load(Ordering::Acquire) {
            if let Err(error) = child.kill()
                && child
                    .try_wait()
                    .map_err(|poll_error| {
                        format!(
                            "Cancel {operation} process failed: {error}; poll failed: {poll_error}"
                        )
                    })?
                    .is_none()
            {
                return Err(format!("Cancel {operation} process failed: {error}"));
            }
            child
                .wait()
                .map_err(|error| format!("Join cancelled {operation} failed: {error}"))?;
            return Ok(None);
        }
        if child
            .try_wait()
            .map_err(|error| format!("Poll {operation} process failed: {error}"))?
            .is_some()
        {
            return Ok(Some(child));
        }
        std::thread::sleep(CHILD_POLL_INTERVAL);
    }
}
