use crate::native_app::app::NativeAppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum TaggedPlaybackMode {
    OneShot,
    Loop,
}

impl TaggedPlaybackMode {
    pub(in crate::native_app) fn label(self) -> &'static str {
        match self {
            Self::OneShot => "One-shot",
            Self::Loop => "Loop",
        }
    }

    pub(in crate::native_app) fn loop_playback(self) -> bool {
        matches!(self, Self::Loop)
    }
}

impl NativeAppState {
    pub(in crate::native_app) fn prepare_playback_mode_for_loaded_sample(&mut self) {
        self.prepare_playback_mode_for_loaded_sample_with_fallback(None);
    }

    pub(in crate::native_app) fn prepare_playback_mode_for_path(&mut self, path: &str) {
        self.prepare_playback_mode_for_path_with_fallback(path, None);
    }

    pub(in crate::native_app) fn loop_playback_for_path_after_policy(&self, path: &str) -> bool {
        if self.audio.loop_playback_manual_override_path.as_deref() == Some(path) {
            return self.audio.loop_playback;
        }
        self.tagged_playback_mode_for_file(path)
            .map(TaggedPlaybackMode::loop_playback)
            .unwrap_or(self.audio.loop_playback)
    }

    pub(in crate::native_app) fn mark_loop_playback_manual_override_for_loaded_sample(&mut self) {
        self.audio.loop_playback_manual_override_path = self.loaded_sample_id();
    }

    pub(in crate::native_app) fn reconcile_playback_mode_after_metadata_tag_change(
        &mut self,
        file_id: &str,
    ) {
        if self.loaded_sample_id().as_deref() != Some(file_id) {
            return;
        }
        let was_looping = self.audio.loop_playback;
        self.audio.loop_playback_manual_override_path = None;
        self.prepare_playback_mode_for_path(file_id);
        self.restart_current_playback_if_loop_policy_changed(was_looping);
    }

    fn prepare_playback_mode_for_loaded_sample_with_fallback(&mut self, fallback: Option<bool>) {
        let Some(path) = self.loaded_sample_id() else {
            return;
        };
        self.prepare_playback_mode_for_path_with_fallback(path.as_str(), fallback);
    }

    fn prepare_playback_mode_for_path_with_fallback(&mut self, path: &str, fallback: Option<bool>) {
        if self.audio.loop_playback_manual_override_path.as_deref() == Some(path) {
            return;
        }
        self.audio.loop_playback_manual_override_path = None;
        if let Some(mode) = self.tagged_playback_mode_for_file(path) {
            self.audio.loop_playback = mode.loop_playback();
        } else if let Some(loop_playback) = fallback {
            self.audio.loop_playback = loop_playback;
        }
    }

    fn tagged_playback_mode_for_file(&self, file_id: &str) -> Option<TaggedPlaybackMode> {
        tagged_playback_mode_for_tags(self.metadata.tags_by_file.get(file_id).map(Vec::as_slice))
    }

    fn loaded_sample_id(&self) -> Option<String> {
        self.waveform
            .current
            .has_loaded_sample()
            .then(|| self.waveform.current.path().display().to_string())
    }

    fn restart_current_playback_if_loop_policy_changed(&mut self, was_looping: bool) {
        if was_looping == self.audio.loop_playback || !self.waveform.current.is_playing() {
            return;
        }
        let Some((start, end)) = self.audio.current_playback_span else {
            return;
        };
        let current = self.current_audio_progress_ratio().unwrap_or(start);
        let result = if self.audio.loop_playback {
            self.start_playback_span(start, end, Some(current))
        } else {
            self.start_playback_current_span(current.clamp(start, end), end)
        };
        if let Err(err) = result {
            self.audio.loop_playback = was_looping;
            self.ui.status.sample = format!("Playback mode update failed: {err}");
        }
    }
}

pub(in crate::native_app) fn tagged_playback_mode_for_tags(
    tags: Option<&[String]>,
) -> Option<TaggedPlaybackMode> {
    tags?
        .iter()
        .find_map(|tag| tagged_playback_mode_for_tag(tag))
}

pub(in crate::native_app) fn tagged_playback_mode_for_tag(tag: &str) -> Option<TaggedPlaybackMode> {
    match normalized_playback_tag(tag).as_str() {
        "loop" => Some(TaggedPlaybackMode::Loop),
        "oneshot" | "one shot" => Some(TaggedPlaybackMode::OneShot),
        _ => None,
    }
}

fn normalized_playback_tag(tag: &str) -> String {
    tag.split(|ch: char| ch == '-' || ch == '_' || ch.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_tag_policy_recognizes_loop_and_one_shot_spellings() {
        assert_eq!(
            tagged_playback_mode_for_tags(Some(&[String::from("loop")])),
            Some(TaggedPlaybackMode::Loop)
        );
        assert_eq!(
            tagged_playback_mode_for_tags(Some(&[String::from("one-shot")])),
            Some(TaggedPlaybackMode::OneShot)
        );
        assert_eq!(
            tagged_playback_mode_for_tags(Some(&[String::from("oneshot")])),
            Some(TaggedPlaybackMode::OneShot)
        );
        assert_eq!(
            tagged_playback_mode_for_tags(Some(&[String::from("warm")])),
            None
        );
    }
}
