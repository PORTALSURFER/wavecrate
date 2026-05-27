use crate::events;

pub(crate) fn ensure_downloads(sender: &events::InstallerSender) -> Result<(), String> {
    send_log(sender, "Using bundled ML model")?;
    Ok(())
}

fn send_log(sender: &events::InstallerSender, message: &str) -> Result<(), String> {
    sender
        .send(events::InstallerEvent::Log(message.to_string()))
        .map_err(|err| format!("Failed to send log update: {err}"))
}
