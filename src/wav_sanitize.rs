//! Compatibility exports for reusable WAV sanitization helpers.

pub use reson::wav_sanitize::{
    SanitizedWavReader, open_sanitized_wav, read_sanitized_wav_bytes, sanitize_wav_bytes,
};
