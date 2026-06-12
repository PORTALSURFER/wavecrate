use std::ptr::copy_nonoverlapping;

use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::System::DataExchange::{GetClipboardData, IsClipboardFormatAvailable};
use windows::Win32::System::Memory::GlobalSize;

use super::handle::{ClipboardReader, GlobalLockGuard, OwnedHGlobal};

pub(super) const CF_UNICODETEXT: u32 = 13;

pub(super) fn create_unicode_text(text: &str) -> Result<OwnedHGlobal, String> {
    let wide = encode_unicode_text(text);
    let bytes = unicode_payload_bytes(wide.len());
    let owned = OwnedHGlobal::new(bytes)?;
    let lock = unsafe { GlobalLockGuard::new(owned.handle()) }?;
    unsafe {
        copy_nonoverlapping(wide.as_ptr() as *const u8, lock.ptr() as *mut u8, bytes);
    }
    Ok(owned)
}

pub(super) fn read_text() -> Result<String, String> {
    if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) }.is_err() {
        return Ok(String::new());
    }
    let _clipboard = ClipboardReader::new()?;
    let handle = unsafe { GetClipboardData(CF_UNICODETEXT) }
        .map_err(|err| format!("GetClipboardData(CF_UNICODETEXT) failed: {err}"))?;
    let lock = unsafe { GlobalLockGuard::new(HGLOBAL(handle.0)) }?;
    let ptr = lock.ptr() as *const u16;
    if ptr.is_null() {
        return Ok(String::new());
    }
    let size_bytes = unsafe { GlobalSize(HGLOBAL(handle.0)) };
    if size_bytes == 0 {
        return Err("GlobalSize failed for clipboard text".to_string());
    }
    let max_u16 = size_bytes / std::mem::size_of::<u16>();
    if max_u16 == 0 {
        return Ok(String::new());
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, max_u16) };
    let len = bounded_utf16_len(slice)?;
    Ok(String::from_utf16_lossy(&slice[..len]))
}

fn encode_unicode_text(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn unicode_payload_bytes(units: usize) -> usize {
    units * std::mem::size_of::<u16>()
}

fn bounded_utf16_len(slice: &[u16]) -> Result<usize, String> {
    slice
        .iter()
        .position(|&ch| ch == 0)
        .ok_or_else(|| "Clipboard text missing terminator".to_string())
}

#[cfg(test)]
mod tests {
    use super::{bounded_utf16_len, encode_unicode_text, unicode_payload_bytes};

    #[test]
    fn bounded_utf16_len_finds_terminator() {
        let data = [b'H' as u16, b'i' as u16, 0, b'X' as u16];
        let len = bounded_utf16_len(&data).expect("expected terminator");
        assert_eq!(len, 2);
    }

    #[test]
    fn bounded_utf16_len_errors_without_terminator() {
        let data = [b'H' as u16, b'i' as u16];
        let err = bounded_utf16_len(&data).expect_err("expected error");
        assert!(err.contains("missing terminator"));
    }

    #[test]
    fn unicode_text_payload_is_null_terminated() {
        let encoded = encode_unicode_text("Hi");
        assert_eq!(encoded, vec![b'H' as u16, b'i' as u16, 0]);
        assert_eq!(unicode_payload_bytes(encoded.len()), 6);
    }
}
