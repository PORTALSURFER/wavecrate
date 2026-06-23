use crate::native_app::audio::playback::tagged_playback_mode_for_tag;

pub(super) fn replace_other_playback_type_tags(
    file_tags: &mut Vec<String>,
    incoming: &str,
    added: &mut Vec<String>,
) -> Vec<String> {
    if tagged_playback_mode_for_tag(incoming).is_none() {
        return Vec::new();
    }
    let mut removed = Vec::new();
    file_tags.retain(|existing| {
        let replaced = playback_type_tag_replaced_by(existing, incoming);
        if replaced && !added.iter().any(|added_tag| added_tag == existing) {
            push_unique(&mut removed, existing.clone());
        }
        !replaced
    });
    added.retain(|existing| !playback_type_tag_replaced_by(existing, incoming));
    removed
}

pub(super) fn playback_type_replacement_present(tags: &[String], incoming: &str) -> bool {
    tagged_playback_mode_for_tag(incoming).is_some()
        && tags
            .iter()
            .any(|existing| playback_type_tag_replaced_by(existing, incoming))
}

pub(super) fn sanitize_playback_type_tags(tags: &mut Vec<String>) -> bool {
    let before_len = tags.len();
    let mut saw_playback_type = false;
    tags.retain(|tag| {
        if tagged_playback_mode_for_tag(tag).is_none() {
            return true;
        }
        if saw_playback_type {
            return false;
        }
        saw_playback_type = true;
        true
    });
    tags.len() != before_len
}

fn playback_type_tag_replaced_by(existing: &str, incoming: &str) -> bool {
    tagged_playback_mode_for_tag(existing).is_some() && existing != incoming
}

fn push_unique(tags: &mut Vec<String>, tag: String) {
    if !tags.iter().any(|existing| existing == &tag) {
        tags.push(tag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incoming_playback_type_replaces_opposite_and_synonym_tags() {
        let mut tags = vec![
            String::from("one-shot"),
            String::from("oneshot"),
            String::from("warm"),
        ];
        let mut added = Vec::new();

        let removed = replace_other_playback_type_tags(&mut tags, "loop", &mut added);

        assert_eq!(tags, vec![String::from("warm")]);
        assert_eq!(
            removed,
            vec![String::from("one-shot"), String::from("oneshot")]
        );
    }

    #[test]
    fn persisted_playback_type_sanitizer_keeps_first_playback_type_only() {
        let mut tags = vec![
            String::from("loop"),
            String::from("one-shot"),
            String::from("warm"),
        ];

        assert!(sanitize_playback_type_tags(&mut tags));

        assert_eq!(tags, vec![String::from("loop"), String::from("warm")]);
    }
}
