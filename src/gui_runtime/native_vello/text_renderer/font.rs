//! Fallback native-font discovery helpers for the Vello text renderer.

use super::*;

pub(super) fn load_native_font() -> Option<FontData> {
    for path in native_font_candidates() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        return Some(FontData::new(Blob::from(bytes), 0));
    }
    None
}

pub(super) fn native_font_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(path) = std::env::var("RADIANT_NATIVE_FONT_PATH") {
        candidates.push(PathBuf::from(path));
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(windir) = std::env::var("WINDIR") {
            let base = PathBuf::from(windir).join("Fonts");
            // Prefer fixed-pitch UI glyph advances so dense rows stay visually even.
            candidates.push(base.join("consola.ttf"));
            candidates.push(base.join("segoeui.ttf"));
            candidates.push(base.join("arial.ttf"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        // Prefer fixed-pitch fonts for deterministic row text spacing.
        candidates.push(PathBuf::from("/System/Library/Fonts/SFNSMono.ttf"));
        candidates.push(PathBuf::from(
            "/System/Library/Fonts/Supplemental/Menlo.ttc",
        ));
        candidates.push(PathBuf::from("/System/Library/Fonts/SFNS.ttf"));
        candidates.push(PathBuf::from(
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ));
        candidates.push(PathBuf::from("/Library/Fonts/Arial.ttf"));
    }
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        // Prefer fixed-pitch fonts for deterministic row text spacing.
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        ));
        candidates.push(PathBuf::from("/usr/share/fonts/dejavu/DejaVuSansMono.ttf"));
        candidates.push(PathBuf::from("/usr/share/fonts/TTF/DejaVuSansMono.ttf"));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        ));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ));
        candidates.push(PathBuf::from("/usr/share/fonts/dejavu/DejaVuSans.ttf"));
        candidates.push(PathBuf::from("/usr/share/fonts/TTF/DejaVuSans.ttf"));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ));
    }

    candidates
}
