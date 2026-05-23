use super::DOWNLOAD_URL;
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

pub(super) fn show_update_required_if_needed(build_id: &str, reason: &str) -> bool {
    if !is_update_required_reason(reason) {
        return false;
    }
    let description = format!(
        "This Wavecrate build is not registered for access or has expired.\n\nBuild: {build_id}\n\nDownload the latest Wavecrate version from portalsurfer.org and try again.\n\nIf this is a new test build, make sure it has been registered on the server before running it.\n\nOpen the download page now?"
    );
    if MessageDialog::new()
        .set_level(MessageLevel::Info)
        .set_title("Wavecrate update required")
        .set_description(&description)
        .set_buttons(MessageButtons::YesNo)
        .show()
        == MessageDialogResult::Yes
        && let Err(err) = open::that(DOWNLOAD_URL)
    {
        show_error(
            "Wavecrate update required",
            &format!("Could not open the download page: {err}\n\n{DOWNLOAD_URL}"),
        );
    }
    true
}

fn is_update_required_reason(reason: &str) -> bool {
    reason.contains("unknown_build") || reason.contains("build_expired")
}

pub(super) fn confirm_activation_request(build_id: &str, reason: &str) -> bool {
    let description = format!(
        "This Wavecrate build needs server access before it can launch.\n\nBuild: {build_id}\n\nReason: {reason}\n\nRequest access for this computer now?"
    );
    MessageDialog::new()
        .set_level(MessageLevel::Info)
        .set_title("Wavecrate access")
        .set_description(&description)
        .set_buttons(MessageButtons::OkCancel)
        .show()
        == MessageDialogResult::Ok
}

pub(super) fn show_info(title: &str, description: &str) {
    let _ = MessageDialog::new()
        .set_level(MessageLevel::Info)
        .set_title(title)
        .set_description(description)
        .set_buttons(MessageButtons::Ok)
        .show();
}

pub(super) fn show_error(title: &str, description: &str) {
    let _ = MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title(title)
        .set_description(description)
        .set_buttons(MessageButtons::Ok)
        .show();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_required_reasons_are_build_registration_failures() {
        assert!(is_update_required_reason(
            r#"server returned HTTP 400: {"error":"unknown_build"}"#
        ));
        assert!(is_update_required_reason(
            r#"server returned HTTP 400: {"error":"build_expired"}"#
        ));
        assert!(!is_update_required_reason("network error: timed out"));
    }
}
