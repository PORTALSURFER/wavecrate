use super::*;

impl AppController {
    pub(in crate::app::controller::library::background_jobs::polling::runtime_handlers) fn rollback_normal_tag_metadata(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        normalized_text: &str,
        display_label: &str,
        before_present: bool,
        expected_present: bool,
        active_source_matches: bool,
    ) {
        if active_source_matches && let Some(index) = self.wav_index_for_path(relative_path) {
            let _ = self.ensure_wav_page_loaded(index);
            if let Some(wav) = self.wav_entries.entry_mut(index) {
                rollback_normal_tag_labels(
                    &mut wav.normal_tags,
                    normalized_text,
                    display_label,
                    before_present,
                    expected_present,
                );
            }
        }
        if let Some(cache) = self.cache.wav.entries.get_mut(source_id)
            && let Some(index) = cache.lookup.get(relative_path).copied()
            && let Some(wav) = cache.entry_mut(index)
        {
            rollback_normal_tag_labels(
                &mut wav.normal_tags,
                normalized_text,
                display_label,
                before_present,
                expected_present,
            );
        }
        self.rollback_normal_tag_ui_cache(
            source_id,
            relative_path,
            normalized_text,
            display_label,
            before_present,
            expected_present,
        );
    }

    fn rollback_normal_tag_ui_cache(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        normalized_text: &str,
        display_label: &str,
        before_present: bool,
        expected_present: bool,
    ) {
        let tags = self
            .ui_cache
            .browser
            .normal_tags
            .entry(source_id.clone())
            .or_default()
            .entry(relative_path.to_path_buf())
            .or_default();
        let current_present = tags
            .iter()
            .any(|tag| tag.normalized_text == normalized_text);
        if current_present != expected_present {
            return;
        }
        if before_present {
            if !current_present {
                tags.push(crate::sample_sources::db::SourceTag {
                    id: 0,
                    display_label: display_label.to_string(),
                    normalized_text: normalized_text.to_string(),
                });
                tags.sort_by(|left, right| {
                    left.display_label
                        .to_ascii_lowercase()
                        .cmp(&right.display_label.to_ascii_lowercase())
                        .then_with(|| left.normalized_text.cmp(&right.normalized_text))
                });
            }
        } else {
            tags.retain(|tag| tag.normalized_text != normalized_text);
        }
    }
}

fn rollback_normal_tag_labels(
    labels: &mut Vec<String>,
    normalized_text: &str,
    display_label: &str,
    before_present: bool,
    expected_present: bool,
) {
    let current_present = labels
        .iter()
        .any(|label| label.to_ascii_lowercase() == normalized_text);
    if current_present != expected_present {
        return;
    }
    if before_present {
        if !current_present {
            labels.push(display_label.to_string());
            labels.sort_by_key(|label| label.to_ascii_lowercase());
        }
    } else {
        labels.retain(|label| label.to_ascii_lowercase() != normalized_text);
    }
}
