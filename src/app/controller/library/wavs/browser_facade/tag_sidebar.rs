use super::super::*;
use crate::app::state::StatusTone;
use crate::sample_sources::{SampleSoundType, SampleSource};
use std::path::PathBuf;

impl AppController {
    /// Toggle whether sidebar metadata edits should auto-rename edited samples.
    pub(crate) fn toggle_browser_tag_sidebar_auto_rename(&mut self) {
        self.ui.browser.tag_sidebar_auto_rename = !self.ui.browser.tag_sidebar_auto_rename;
        if !self.ui.browser.tag_sidebar_auto_rename {
            self.set_status("Auto rename off", StatusTone::Info);
            return;
        }
        let target_paths = self.browser_tag_sidebar_target_paths();
        if target_paths.is_empty() {
            self.set_status("Auto rename on", StatusTone::Info);
            return;
        }
        if let Err(err) = self.auto_rename_after_tag_sidebar_change(&target_paths) {
            self.set_status(err, StatusTone::Error);
        }
    }

    /// Store the current draft value for the browser metadata tag input.
    pub(crate) fn set_browser_tag_sidebar_input(&mut self, value: String) {
        self.ui.browser.tag_sidebar_input = value;
    }

    /// Apply the current tag input draft to focused/selected browser rows.
    pub(crate) fn commit_browser_tag_sidebar_input(&mut self) -> Result<(), String> {
        let value = self.ui.browser.tag_sidebar_input.clone();
        self.apply_browser_tag_sidebar_normal_tag_tokens(&value)?;
        self.ui.browser.tag_sidebar_input.clear();
        Ok(())
    }

    /// Apply one playback-type value to the focused/selected browser rows.
    pub(crate) fn apply_browser_tag_sidebar_looped(&mut self, looped: bool) -> Result<(), String> {
        let Some(source) = self.current_source() else {
            return Err(String::from("No source selected"));
        };
        let target_paths = self.browser_tag_sidebar_target_paths();
        self.set_sample_looped_for_source_batch(&source, &target_paths, looped, false)?;
        self.auto_rename_after_tag_sidebar_change(&target_paths)?;
        Ok(())
    }

    /// Apply one sound-type value to the focused/selected browser rows.
    pub(crate) fn apply_browser_tag_sidebar_sound_type(
        &mut self,
        sound_type: Option<SampleSoundType>,
    ) -> Result<(), String> {
        match sound_type {
            Some(sound_type) => self.apply_browser_tag_sidebar_normal_tag(sound_type.token()),
            None => Ok(()),
        }
    }

    /// Compatibility path for legacy callers that now assigns a normal tag.
    pub(crate) fn apply_browser_tag_sidebar_user_tag(
        &mut self,
        user_tag: Option<String>,
    ) -> Result<(), String> {
        let Some(user_tag) = user_tag else {
            return Ok(());
        };
        self.apply_browser_tag_sidebar_normal_tag(&user_tag)
    }

    /// Assign one normal tag to the focused/selected browser rows.
    pub(crate) fn apply_browser_tag_sidebar_normal_tag(
        &mut self,
        label: &str,
    ) -> Result<(), String> {
        self.apply_browser_tag_sidebar_normal_tag_tokens(label)
    }

    /// Assign one or more comma-delimited normal tags to the focused/selected browser rows.
    pub(crate) fn apply_browser_tag_sidebar_normal_tag_tokens(
        &mut self,
        input: &str,
    ) -> Result<(), String> {
        let Some(source) = self.current_source() else {
            return Err(String::from("No source selected"));
        };
        let tokens = browser_tag_sidebar_tokens(input);
        if tokens.is_empty() {
            return Ok(());
        }
        let mut resolved_labels = Vec::<String>::new();
        for token in tokens {
            let resolved_label = self.resolve_browser_normal_tag_label(&source, &token)?;
            if !resolved_labels
                .iter()
                .any(|label| label.eq_ignore_ascii_case(&resolved_label))
            {
                resolved_labels.push(resolved_label);
            }
        }
        let target_paths = self.browser_tag_sidebar_target_paths();
        for resolved_label in resolved_labels {
            self.set_normal_tag_for_source_batch(&source, &target_paths, &resolved_label, true)?;
        }
        self.auto_rename_after_tag_sidebar_change(&target_paths)?;
        Ok(())
    }

    /// Remove one normal tag from the focused/selected browser rows.
    pub(crate) fn remove_browser_tag_sidebar_normal_tag(
        &mut self,
        label: &str,
    ) -> Result<(), String> {
        let Some(source) = self.current_source() else {
            return Err(String::from("No source selected"));
        };
        let target_paths = self.browser_tag_sidebar_target_paths();
        self.set_normal_tag_for_source_batch(&source, &target_paths, label, false)?;
        self.auto_rename_after_tag_sidebar_change(&target_paths)?;
        Ok(())
    }

    fn resolve_browser_normal_tag_label(
        &mut self,
        source: &SampleSource,
        label: &str,
    ) -> Result<String, String> {
        let trimmed = label.split_whitespace().collect::<Vec<_>>().join(" ");
        if trimmed.is_empty() {
            return Err(String::from("Tag label cannot be empty"));
        }
        let matches = self
            .database_for(source)
            .map_err(|err| err.to_string())?
            .search_tags(&trimmed, 1)
            .map_err(|err| err.to_string())?;
        Ok(matches
            .first()
            .map(|usage| usage.tag.display_label.clone())
            .unwrap_or(trimmed))
    }

    fn auto_rename_after_tag_sidebar_change(
        &mut self,
        target_paths: &[PathBuf],
    ) -> Result<(), String> {
        if !self.ui.browser.tag_sidebar_auto_rename || target_paths.is_empty() {
            return Ok(());
        }
        self.browser()
            .auto_rename_browser_sample_paths_action(target_paths)
    }
}

fn browser_tag_sidebar_tokens(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|token| token.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|token| !token.is_empty())
        .collect()
}
