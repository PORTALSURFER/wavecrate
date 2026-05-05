use super::payload::{build_dropfiles_payload, encode_drag_paths};
use super::*;
use std::os::windows::ffi::OsStringExt;
use windows::Win32::UI::Shell::DROPFILES;

fn decode_drag_paths(bytes: &[u8]) -> Vec<String> {
    let utf16 = bytes
        .chunks_exact(std::mem::size_of::<u16>())
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    let mut paths = Vec::new();
    let mut start = 0usize;
    while start < utf16.len() {
        let Some(end) = utf16[start..].iter().position(|value| *value == 0) else {
            panic!("drag path payload must be null terminated");
        };
        if end == 0 {
            break;
        }
        paths.push(
            std::ffi::OsString::from_wide(&utf16[start..start + end])
                .to_string_lossy()
                .into_owned(),
        );
        start += end + 1;
    }
    paths
}

#[test]
fn normalize_path_strips_windows_verbatim_prefix() {
    let normalized = normalize_path(std::path::Path::new(r"\\?\C:\samples\kick.wav"));
    assert_eq!(normalized, PathBuf::from(r"C:\samples\kick.wav"));
}

#[test]
fn encode_drag_paths_double_null_terminates_multi_path_payload() {
    let encoded = encode_drag_paths(&[
        PathBuf::from(r"C:\samples\kick.wav"),
        PathBuf::from(r"D:\packs\snare.wav"),
    ]);

    assert!(encoded.ends_with(&[0, 0, 0, 0]));
    assert_eq!(
        decode_drag_paths(&encoded),
        vec![
            String::from(r"C:\samples\kick.wav"),
            String::from(r"D:\packs\snare.wav"),
        ]
    );
}

#[test]
fn build_dropfiles_payload_prepends_wide_dropfiles_header() {
    let payload = build_dropfiles_payload(&[PathBuf::from(r"C:\samples\hat.wav")]);
    let header_len = std::mem::size_of::<DROPFILES>();
    let header = unsafe { std::ptr::read_unaligned(payload.as_ptr().cast::<DROPFILES>()) };

    assert_eq!(header.pFiles as usize, header_len);
    assert!(header.fWide.as_bool());
    assert!(!header.fNC.as_bool());
    assert_eq!(
        decode_drag_paths(&payload[header_len..]),
        vec![String::from(r"C:\samples\hat.wav")]
    );
}
