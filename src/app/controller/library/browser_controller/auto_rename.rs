//! Deterministic sample-browser auto-rename helpers.
//!
//! V1 keeps the naming contract intentionally small and stable so the browser
//! context action can batch-rename samples without guessing at tag ordering.

/// Metadata snapshot used to build one auto-rename target basename.
#[derive(Debug, Clone)]
pub(crate) struct AutoRenameInput {
    /// App-level default creator or artist identifier.
    pub(crate) identifier: String,
    /// Whether the sample is marked as looped.
    pub(crate) looped: bool,
    /// Canonical sound classification for the sample.
    pub(crate) sound_type: Option<crate::sample_sources::SampleSoundType>,
    /// Optional single custom metadata tag authored for the sample.
    pub(crate) user_tag: Option<String>,
    /// Normal library tags assigned to the sample.
    pub(crate) normal_tags: Vec<String>,
    /// Stored sample BPM metadata, appended when available.
    pub(crate) bpm: Option<f32>,
}

/// Built naming parts for one auto-rename request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AutoRenameStem {
    /// Fully tagged basename when rename metadata yields a stable name.
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
    let mut metadata_tokens = Vec::with_capacity(3);
    if input.normal_tags.is_empty() {
        if let Some(sound_type) = input.sound_type {
            metadata_tokens.push(sound_type.token().to_string());
        }
        if let Some(user_tag) = input.user_tag.as_deref().and_then(sanitize_identifier) {
            metadata_tokens.push(user_tag);
        }
    } else {
        for tag in &input.normal_tags {
            if let Some(token) = sanitize_identifier(tag) {
                metadata_tokens.push(token);
            }
        }
    }
    if let Some(bpm) = input.bpm.filter(|bpm| bpm.is_finite() && *bpm > 0.0) {
        metadata_tokens.push(format!("{:.0}", bpm.round()));
    }
    let shot_type = if input.looped { "loop" } else { "SS" };
    let tagged_basename = Some(if metadata_tokens.is_empty() {
        format!("{identifier}_{shot_type}")
    } else {
        format!("{identifier}_{shot_type}_{}", metadata_tokens.join("_"))
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
            user_tag: None,
            normal_tags: Vec::new(),
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
    fn appends_sanitized_custom_tag_when_present() {
        let mut tagged = input();
        tagged.user_tag = Some(String::from("Vintage FX!"));
        assert_eq!(
            build_auto_rename_stem(&tagged).tagged_basename.as_deref(),
            Some("artistname_SS_kick_vintagefx_130")
        );
    }

    #[test]
    fn uses_custom_tag_when_sound_type_is_missing() {
        let mut tagged = input();
        tagged.sound_type = None;
        tagged.user_tag = Some(String::from("Vintage FX!"));
        assert_eq!(
            build_auto_rename_stem(&tagged).tagged_basename.as_deref(),
            Some("artistname_SS_vintagefx_130")
        );
    }

    #[test]
    fn uses_sound_type_without_bpm() {
        let mut tagged = input();
        tagged.bpm = None;
        assert_eq!(
            build_auto_rename_stem(&tagged).tagged_basename.as_deref(),
            Some("artistname_SS_kick")
        );
    }

    #[test]
    fn uses_custom_tag_without_bpm() {
        let mut tagged = input();
        tagged.sound_type = None;
        tagged.bpm = None;
        tagged.user_tag = Some(String::from("Vintage FX!"));
        assert_eq!(
            build_auto_rename_stem(&tagged).tagged_basename.as_deref(),
            Some("artistname_SS_vintagefx")
        );
    }

    #[test]
    fn normal_tags_replace_legacy_sound_type_and_custom_tag_tokens() {
        let mut tagged = input();
        tagged.user_tag = Some(String::from("Vintage FX!"));
        tagged.normal_tags = vec![String::from("Deep Kick"), String::from("Layer")];
        assert_eq!(
            build_auto_rename_stem(&tagged).tagged_basename.as_deref(),
            Some("artistname_SS_deepkick_layer_130")
        );
    }

    #[test]
    fn missing_sound_type_user_tag_and_bpm_uses_identifier_fallback() {
        let mut missing_sound = input();
        missing_sound.sound_type = None;
        missing_sound.bpm = None;
        assert_eq!(
            build_auto_rename_stem(&missing_sound),
            AutoRenameStem {
                tagged_basename: Some(String::from("artistname_SS")),
                fallback_identifier: String::from("artistname"),
            }
        );

        let mut blank_user_tag = input();
        blank_user_tag.sound_type = None;
        blank_user_tag.bpm = None;
        blank_user_tag.user_tag = Some(String::from("!!!"));
        assert_eq!(
            build_auto_rename_stem(&blank_user_tag),
            AutoRenameStem {
                tagged_basename: Some(String::from("artistname_SS")),
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
                tagged_basename: Some(String::from("portal_SS")),
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
