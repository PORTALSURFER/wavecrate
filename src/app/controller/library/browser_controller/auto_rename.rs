//! Deterministic sample-browser auto-rename helpers.
//!
//! V1 keeps the naming contract intentionally small and stable so the browser
//! context action can batch-rename samples without guessing at tag ordering.

/// Required metadata snapshot used to build one auto-rename target basename.
#[derive(Debug, Clone)]
pub(crate) struct AutoRenameInput {
    /// App-level default creator or artist identifier.
    pub(crate) identifier: String,
    /// Whether the sample is marked as looped.
    pub(crate) looped: bool,
    /// Canonical sound classification for the sample.
    pub(crate) sound_type: Option<crate::sample_sources::SampleSoundType>,
    /// Stored sample BPM metadata.
    pub(crate) bpm: Option<f32>,
}

/// Built naming parts for one auto-rename request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AutoRenameStem {
    /// Fully tagged basename when all required metadata is available.
    pub(crate) tagged_basename: Option<String>,
    /// Sanitized identifier used for fallback numbering.
    pub(crate) fallback_identifier: String,
}

/// Canonical fallback identifier used when settings or sanitization produce no
/// visible characters.
pub(crate) const DEFAULT_AUTO_RENAME_IDENTIFIER: &str = "portal";

/// Build deterministic naming parts for one auto-rename request.
pub(crate) fn build_auto_rename_stem(input: &AutoRenameInput) -> AutoRenameStem {
    let identifier = sanitize_identifier(&input.identifier)
        .unwrap_or_else(|| String::from(DEFAULT_AUTO_RENAME_IDENTIFIER));
    let tagged_basename = input.sound_type.and_then(|sound_type| {
        input
            .bpm
            .filter(|bpm| bpm.is_finite() && *bpm > 0.0)
            .map(|bpm| {
                let shot_type = if input.looped { "loop" } else { "SS" };
                format!(
                    "{identifier}_{shot_type}_{}_{:.0}",
                    sound_type.token(),
                    bpm.round()
                )
            })
    });
    AutoRenameStem {
        tagged_basename,
        fallback_identifier: identifier,
    }
}

fn sanitize_identifier(identifier: &str) -> Option<String> {
    let mut output = String::new();
    for ch in identifier.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
        }
    }
    (!output.is_empty()).then_some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> AutoRenameInput {
        AutoRenameInput {
            identifier: String::from("Artist Name"),
            looped: false,
            sound_type: Some(crate::sample_sources::SampleSoundType::Kick),
            bpm: Some(129.6),
        }
    }

    #[test]
    fn builds_shot_type_and_stable_order() {
        let one_shot = build_auto_rename_stem(&input());
        assert_eq!(
            one_shot.tagged_basename.as_deref(),
            Some("artistname_SS_kick_130")
        );
        assert_eq!(one_shot.fallback_identifier, "artistname");

        let mut looped = input();
        looped.looped = true;
        assert_eq!(
            build_auto_rename_stem(&looped).tagged_basename.as_deref(),
            Some("artistname_loop_kick_130")
        );
    }

    #[test]
    fn missing_sound_type_or_bpm_uses_identifier_fallback() {
        let mut missing_sound = input();
        missing_sound.sound_type = None;
        assert_eq!(
            build_auto_rename_stem(&missing_sound),
            AutoRenameStem {
                tagged_basename: None,
                fallback_identifier: String::from("artistname"),
            }
        );

        let mut missing_bpm = input();
        missing_bpm.bpm = None;
        assert_eq!(
            build_auto_rename_stem(&missing_bpm),
            AutoRenameStem {
                tagged_basename: None,
                fallback_identifier: String::from("artistname"),
            }
        );
    }

    #[test]
    fn invalid_or_empty_identifier_falls_back_to_portal() {
        let mut invalid_identifier = input();
        invalid_identifier.identifier = String::from("!!!");
        invalid_identifier.sound_type = None;
        invalid_identifier.bpm = None;
        assert_eq!(
            build_auto_rename_stem(&invalid_identifier),
            AutoRenameStem {
                tagged_basename: None,
                fallback_identifier: String::from(DEFAULT_AUTO_RENAME_IDENTIFIER),
            }
        );

        let mut empty_identifier = input();
        empty_identifier.identifier.clear();
        assert_eq!(
            build_auto_rename_stem(&empty_identifier).fallback_identifier,
            DEFAULT_AUTO_RENAME_IDENTIFIER
        );
    }
}
