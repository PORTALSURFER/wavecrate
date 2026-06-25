use crate::app::controller::StatusTone;
use crate::app::controller::jobs::{DropTargetTransferKind, DropTargetTransferResult};
use crate::app::controller::ui::drag_drop_controller::DragDropController;
use crate::sample_sources::WavEntry;
use tracing::{info, warn};

impl DragDropController<'_> {
    /// Apply a completed drop-target copy or move job back onto controller state.
    pub(crate) fn apply_drop_target_transfer_result(&mut self, result: DropTargetTransferResult) {
        let Some(target_source) = self
            .library
            .sources
            .iter()
            .find(|source| source.id == result.target_source_id)
            .cloned()
        else {
            self.set_status("Target source not available for drop", StatusTone::Error);
            return;
        };
        for entry in &result.transferred {
            if result.kind == DropTargetTransferKind::Move
                && let Some(source) = self
                    .library
                    .sources
                    .iter()
                    .find(|source| source.id == entry.source_id)
                    .cloned()
            {
                self.prune_cached_sample(&source, &entry.source_relative);
            }
            self.insert_cached_entry(
                &target_source,
                WavEntry {
                    relative_path: entry.target_relative.clone(),
                    file_size: entry.file_size,
                    modified_ns: entry.modified_ns,
                    content_hash: None,
                    tag: entry.tag,
                    looped: entry.looped,
                    sound_type: entry.sound_type,
                    locked: entry.locked,
                    missing: false,
                    last_played_at: entry.last_played_at,
                    last_curated_at: entry.last_curated_at,
                    user_tag: entry.user_tag.clone(),
                    tag_named: false,
                    normal_tags: entry.normal_tags.clone(),
                },
            );
            self.ui_cache
                .browser
                .normal_tags
                .entry(target_source.id.clone())
                .or_default()
                .insert(
                    entry.target_relative.clone(),
                    entry
                        .normal_tags
                        .iter()
                        .map(|label| crate::sample_sources::db::SourceTag {
                            id: 0,
                            display_label: label.clone(),
                            normalized_text: label
                                .split_whitespace()
                                .collect::<Vec<_>>()
                                .join(" ")
                                .to_ascii_lowercase(),
                        })
                        .collect(),
                );
        }
        self.set_drop_target_transfer_status(&result);
        for err in &result.errors {
            warn!(
                error = %err,
                action = result.kind.action_past_tense(),
                transferred = result.transferred.len(),
                target = %result.target_label,
                cancelled = result.cancelled,
                "Drop target transfer error"
            );
        }
        info!(
            action = result.kind.action_past_tense(),
            transferred = result.transferred.len(),
            errors = result.errors.len(),
            target = %result.target_label,
            "Drop target transfer completed"
        );
    }

    /// Translate drop-target transfer counts into one user-facing status line.
    fn set_drop_target_transfer_status(&mut self, result: &DropTargetTransferResult) {
        let transferred = result.transferred.len();
        if transferred == 0 && result.errors.is_empty() {
            let message = match (result.kind, result.cancelled) {
                (DropTargetTransferKind::Copy, true) => "Copy cancelled".to_string(),
                (DropTargetTransferKind::Move, true) => "Move cancelled".to_string(),
                (DropTargetTransferKind::Copy, false) => "No samples copied".to_string(),
                (DropTargetTransferKind::Move, false) => "No samples moved".to_string(),
            };
            self.set_status(message, StatusTone::Warning);
            return;
        }
        let tone = if result.errors.is_empty() && !result.cancelled {
            StatusTone::Info
        } else {
            StatusTone::Warning
        };
        let mut message = if transferred == 1 && result.errors.is_empty() && !result.cancelled {
            format!(
                "{} to {}",
                result.kind.action_past_tense(),
                result.target_label
            )
        } else {
            format!(
                "{} {transferred} sample(s) to {}",
                result.kind.action_past_tense(),
                result.target_label
            )
        };
        if !result.errors.is_empty() {
            message.push_str(&format!(" with {} error(s)", result.errors.len()));
        }
        if result.cancelled {
            message.push_str(" (cancelled)");
        }
        self.set_status(message, tone);
    }
}
