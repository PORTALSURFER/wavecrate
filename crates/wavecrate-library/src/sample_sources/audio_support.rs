use std::path::Path;

/// Supported audio extensions for sample sources (lowercase, without dots).
pub const SUPPORTED_AUDIO_EXTENSIONS: [&str; 1] = ["wav"];

/// Return true if the path has a supported audio extension.
pub fn is_supported_audio(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    SUPPORTED_AUDIO_EXTENSIONS
        .iter()
        .any(|supported| ext.eq_ignore_ascii_case(supported))
}

/// Build a SQL WHERE clause that filters for supported audio extensions.
pub fn supported_audio_where_clause() -> String {
    let exts: Vec<String> = SUPPORTED_AUDIO_EXTENSIONS
        .iter()
        .map(|ext| format!("'{}'", ext))
        .collect();
    format!("extension IN ({})", exts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_audio_is_wav_first_only() {
        assert!(is_supported_audio(Path::new("kick.wav")));
        assert!(is_supported_audio(Path::new("KICK.WAV")));

        for unsupported in ["loop.aif", "loop.aiff", "loop.flac", "loop.mp3"] {
            assert!(
                !is_supported_audio(Path::new(unsupported)),
                "{unsupported} should not be treated as supported audio"
            );
        }
    }

    #[test]
    fn supported_audio_sql_filter_is_wav_only() {
        assert_eq!(supported_audio_where_clause(), "extension IN ('wav')");
    }
}
