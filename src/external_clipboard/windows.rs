use std::path::PathBuf;

use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::DataExchange::{RegisterClipboardFormatW, SetClipboardData};
use windows::Win32::System::Ole::{CF_HDROP, DROPEFFECT_COPY};
use windows::core::w;

mod handle;
mod hdrop;
mod text;

use handle::{ClipboardWriter, OwnedHGlobal};

pub(super) fn copy_file_paths(paths: &[PathBuf]) -> Result<(), String> {
    let _clipboard = ClipboardWriter::new()?;
    let hdrop = hdrop::create_hdrop(paths)?;
    let drop_effect = hdrop::create_drop_effect(DROPEFFECT_COPY.0)?;
    let effect_format = preferred_drop_effect_format()?;
    set_clipboard_hglobal(effect_format as u32, drop_effect, "Preferred DropEffect")?;
    set_clipboard_hglobal(CF_HDROP.0 as u32, hdrop, "CF_HDROP")?;
    Ok(())
}

pub(super) fn copy_text(text: &str) -> Result<(), String> {
    let _clipboard = ClipboardWriter::new()?;
    let text_payload = text::create_unicode_text(text)?;
    set_clipboard_hglobal(text::CF_UNICODETEXT, text_payload, "text")?;
    Ok(())
}

pub(super) fn read_file_paths() -> Result<Vec<PathBuf>, String> {
    hdrop::read_file_paths()
}

pub(super) fn read_text() -> Result<String, String> {
    text::read_text()
}

fn set_clipboard_hglobal(format: u32, payload: OwnedHGlobal, label: &str) -> Result<(), String> {
    // SAFETY: The clipboard is open; ownership of the HGLOBAL transfers to the
    // system when SetClipboardData succeeds.
    unsafe { SetClipboardData(format, Some(HANDLE(payload.handle().0))) }
        .map_err(|err| format!("SetClipboardData({label}) failed: {err}"))?;
    let _ = payload.release();
    Ok(())
}

fn preferred_drop_effect_format() -> Result<u16, String> {
    static FORMAT: std::sync::OnceLock<Result<u16, String>> = std::sync::OnceLock::new();
    FORMAT
        .get_or_init(|| {
            let fmt = unsafe { RegisterClipboardFormatW(w!("Preferred DropEffect")) };
            if fmt == 0 {
                Err("RegisterClipboardFormatW failed".to_string())
            } else {
                Ok(fmt as u16)
            }
        })
        .clone()
}
