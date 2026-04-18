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

/// Reason why one sample could not receive an auto-generated filename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AutoRenameSkipReason {
    /// No app-level identifier is configured.
    MissingIdentifier,
    /// No canonical sound classification is available.
    MissingSoundType,
    /// BPM metadata is missing or invalid.
    MissingBpm,
    /// Sanitization removed all visible identifier characters.
    InvalidIdentifier,
}

impl AutoRenameSkipReason {
    /// User-facing explanation for skipped rename items.
    pub(crate) const fn message(&self) -> &'static str {
        match self {
            Self::MissingIdentifier => "missing default identifier",
            Self::MissingSoundType => "missing sound type",
            Self::MissingBpm => "missing BPM",
            Self::InvalidIdentifier => "identifier is invalid after sanitization",
        }
    }
}

/// Build the deterministic V1 auto-rename basename without a file extension.
pub(crate) fn build_auto_rename_basename(
    input: &AutoRenameInput,
) -> Result<String, AutoRenameSkipReason> {
    let identifier = sanitize_identifier(&input.identifier)
        .ok_or_else(|| {
            if input.identifier.trim().is_empty() {
                AutoRenameSkipReason::MissingIdentifier
            } else {
                AutoRenameSkipReason::InvalidIdentifier
            }
        })?;
    let sound_type = input
        .sound_type
        .ok_or(AutoRenameSkipReason::MissingSoundType)?
        .token();
    let bpm = input
        .bpm
        .filter(|bpm| bpm.is_finite() && *bpm > 0.0)
        .map(|bpm| bpm.round() as i32)
        .ok_or(AutoRenameSkipReason::MissingBpm)?;
    let shot_type = if input.looped { "loop" } else { "SS" };
    Ok(format!("{identifier}_{shot_type}_{sound_type}_{bpm}"))
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
        let one_shot = build_auto_rename_basename(&input()).unwrap();
        assert_eq!(one_shot, "artistname_SS_kick_130");

        let mut looped = input();
        looped.looped = true;
        assert_eq!(
            build_auto_rename_basename(&looped).unwrap(),
            "artistname_loop_kick_130"
        );
    }

    #[test]
    fn rejects_missing_required_segments() {
        let mut missing_identifier = input();
        missing_identifier.identifier.clear();
        assert_eq!(
            build_auto_rename_basename(&missing_identifier),
            Err(AutoRenameSkipReason::MissingIdentifier)
        );

        let mut missing_sound = input();
        missing_sound.sound_type = None;
        assert_eq!(
            build_auto_rename_basename(&missing_sound),
            Err(AutoRenameSkipReason::MissingSoundType)
        );

        let mut missing_bpm = input();
        missing_bpm.bpm = None;
        assert_eq!(
            build_auto_rename_basename(&missing_bpm),
            Err(AutoRenameSkipReason::MissingBpm)
        );
    }
}
