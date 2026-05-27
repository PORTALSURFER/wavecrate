//! Installer progress events and removed interactive entrypoint.

use std::sync::mpsc;

/// Events emitted by installer worker paths.
pub(crate) enum InstallerEvent {
    Started { total_files: usize },
    FileCopied { copied_files: usize, name: String },
    Log(String),
    Finished,
}

/// Shared sender type used by installer tasks and helper modules.
pub(crate) type InstallerSender = mpsc::Sender<InstallerEvent>;

/// Compatibility entrypoint for the removed interactive installer.
pub(crate) fn removed_interactive_installer_entrypoint() -> Result<(), String> {
    Err(String::from(
        "The deprecated native installer UI has been removed; use --dry-run or --uninstall.",
    ))
}

/// Report the total number of files the installer expects to copy.
pub(crate) fn send_started(
    sender: &mpsc::Sender<InstallerEvent>,
    total_files: usize,
) -> Result<(), String> {
    sender
        .send(InstallerEvent::Started { total_files })
        .map_err(|err| format!("Failed to report install start: {err}"))
}

/// Report one copied file and the current completion count.
pub(crate) fn send_file_copied(
    sender: &mpsc::Sender<InstallerEvent>,
    copied_files: usize,
    name: String,
) -> Result<(), String> {
    sender
        .send(InstallerEvent::FileCopied { copied_files, name })
        .map_err(|err| format!("Failed to report install progress: {err}"))
}

/// Report a successful installer completion event.
pub(crate) fn send_finished(sender: &mpsc::Sender<InstallerEvent>) -> Result<(), String> {
    sender
        .send(InstallerEvent::Finished)
        .map_err(|err| format!("Failed to report completion: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_interactive_installer_ui_returns_actionable_error() {
        let err = removed_interactive_installer_entrypoint()
            .expect_err("interactive UI should be removed");
        assert!(err.contains("deprecated native installer UI has been removed"));
    }
}
