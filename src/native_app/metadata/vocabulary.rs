use super::types::MetadataTagCommit;
use std::collections::BTreeSet;

pub(super) const METADATA_TAG_CATEGORIES: [(&str, &str); 5] = [
    ("playback-type", "Playback Type"),
    ("sound-type", "Sound Type"),
    ("character", "Character"),
    ("prefix", "Prefix"),
    ("tuning-scale", "Tuning/Scale"),
];

pub(super) const USER_EXTENSIBLE_METADATA_TAG_CATEGORIES: [(&str, &str); 4] = [
    ("sound-type", "Sound Type"),
    ("character", "Character"),
    ("prefix", "Prefix"),
    ("tuning-scale", "Tuning/Scale"),
];

pub(super) const DEFAULT_METADATA_TAGS: &[&str] = &["one-shot", "loop"];

const PLAYBACK_TYPE_TAGS: &[&str] = &["loop", "one shot", "oneshot"];
const SOUND_TYPE_TAGS: &[&str] = &[
    "kick",
    "snare",
    "clap",
    "hat",
    "bass",
    "stab",
    "texture",
    "vocal",
    "percussion",
    "ambience",
    "effect",
    "fx",
    "drum loop",
    "synth loop",
];
const CHARACTER_TAGS: &[&str] = &[
    "warm",
    "harsh",
    "clean",
    "noisy",
    "distorted",
    "punchy",
    "soft",
    "metallic",
    "dark",
    "bright",
    "wide",
    "dry",
    "wet",
    "raw",
    "polished",
];
const TUNING_SCALE_TAGS: &[&str] = &[
    "major",
    "minor",
    "dorian",
    "phrygian",
    "lydian",
    "mixolydian",
    "locrian",
    "pentatonic",
    "chromatic",
    "microtonal",
];

pub(in crate::native_app) fn commit_metadata_tag_text(value: &str) -> MetadataTagCommit {
    let parts = value
        .split(['\n', ',', ';'])
        .map(str::trim)
        .collect::<Vec<_>>();
    MetadataTagCommit {
        tags: parts
            .into_iter()
            .filter_map(normalize_metadata_tag)
            .collect(),
        remainder: String::new(),
    }
}

pub(in crate::native_app) fn normalize_metadata_tag(value: &str) -> Option<String> {
    let mut normalized = String::new();
    let mut previous_separator = false;
    for ch in value.trim().chars() {
        let next = if ch.is_ascii_alphanumeric() {
            previous_separator = false;
            ch.to_ascii_lowercase()
        } else if ch == '_' {
            previous_separator = false;
            ch
        } else if ch.is_whitespace() || ch == '-' || ch.is_ascii_punctuation() {
            if previous_separator || normalized.is_empty() {
                previous_separator = true;
                continue;
            }
            previous_separator = true;
            '-'
        } else {
            continue;
        };
        normalized.push(next);
    }
    let normalized = normalized.trim_matches('-').to_string();
    (!normalized.is_empty()).then_some(normalized)
}

#[cfg(test)]
pub(in crate::native_app) fn metadata_tag_completion<'a>(
    value: &str,
    known_tags: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let prefix = normalize_metadata_tag(value)?;
    metadata_tag_completions_for_prefix(prefix.as_str(), known_tags)
        .into_iter()
        .next()
}

pub(in crate::native_app) fn metadata_tag_completions_for_prefix<'a>(
    prefix: &str,
    known_tags: impl IntoIterator<Item = &'a str>,
) -> Vec<String> {
    known_tags
        .into_iter()
        .filter(|tag| tag.starts_with(prefix))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
pub(in crate::native_app) fn metadata_tag_category_id(tag: &str) -> &'static str {
    inferred_metadata_tag_category_id(tag)
}

pub(in crate::native_app) fn inferred_metadata_tag_category_id(tag: &str) -> &'static str {
    let normalized = normalize_metadata_category_match(tag);
    if PLAYBACK_TYPE_TAGS.contains(&normalized.as_str()) {
        "playback-type"
    } else if SOUND_TYPE_TAGS.contains(&normalized.as_str()) {
        "sound-type"
    } else if CHARACTER_TAGS.contains(&normalized.as_str()) {
        "character"
    } else if TUNING_SCALE_TAGS.contains(&normalized.as_str()) {
        "tuning-scale"
    } else if has_metadata_category_prefix(&normalized, "prefix")
        || has_metadata_category_prefix(&normalized, "artist")
        || has_metadata_category_prefix(&normalized, "pack")
        || has_metadata_category_prefix(&normalized, "project")
    {
        "prefix"
    } else {
        "character"
    }
}

pub(in crate::native_app) fn metadata_tag_category_order(category_id: &str) -> usize {
    METADATA_TAG_CATEGORIES
        .iter()
        .position(|(id, _label)| *id == category_id)
        .unwrap_or(METADATA_TAG_CATEGORIES.len())
}

pub(in crate::native_app) fn inferred_metadata_tag_category_id_for_name(tag: &str) -> &'static str {
    inferred_metadata_tag_category_id(tag)
}

pub(in crate::native_app) fn metadata_tag_category_label_for_id(
    category_id: &str,
) -> Option<&'static str> {
    METADATA_TAG_CATEGORIES
        .iter()
        .find_map(|(id, label)| (*id == category_id).then_some(*label))
}

pub(in crate::native_app) fn static_metadata_tag_category_id(
    category_id: &str,
) -> Option<&'static str> {
    METADATA_TAG_CATEGORIES
        .iter()
        .find_map(|(id, _label)| (*id == category_id).then_some(*id))
}

pub(in crate::native_app) fn metadata_tag_category_is_locked(category_id: &str) -> bool {
    category_id == "playback-type"
}

fn has_metadata_category_prefix(value: &str, prefix: &str) -> bool {
    value == prefix
        || value
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with(':') || rest.starts_with(' '))
}

fn normalize_metadata_category_match(value: &str) -> String {
    value
        .split(|ch: char| ch == '-' || ch == '_' || ch.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

pub(in crate::native_app) fn normalize_metadata_category_query(value: &str) -> Option<String> {
    normalize_metadata_tag(value).map(|normalized| normalized.replace('_', "-"))
}
