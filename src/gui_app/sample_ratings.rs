use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Instant,
};

use radiant::prelude as ui;
use wavecrate::sample_sources::{Rating, SourceDatabase};

use super::{GuiAppState, GuiMessage, emit_gui_action};

#[derive(Clone, Debug, PartialEq, Eq)]
struct RatingUpdate {
    root: PathBuf,
    relative_path: PathBuf,
    absolute_path: PathBuf,
    rating: Rating,
    locked: bool,
}

impl GuiAppState {
    pub(super) fn adjust_selected_rating(
        &mut self,
        delta: i8,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let updates = self.rating_updates_for_selected_files(delta);
        if updates.is_empty() {
            self.sample_status = String::from("Select a sample to rate");
            emit_gui_action(
                "browser.rating.adjust",
                Some("browser"),
                Some(direction_label(delta)),
                "empty",
                started_at,
                None,
            );
            return;
        }

        let mut applied = 0usize;
        let mut last_error = None;
        for (root, source_updates) in group_updates_by_source(updates) {
            match persist_rating_updates(&root, &source_updates) {
                Ok(()) => {
                    for update in source_updates {
                        if self.folder_browser.set_file_rating_state(
                            &update.absolute_path,
                            update.rating,
                            update.locked,
                        ) {
                            applied += 1;
                        }
                    }
                }
                Err(error) => last_error = Some(error),
            }
        }

        if let Some(error) = last_error {
            self.sample_status = format!("Rating failed: {error}");
            emit_gui_action(
                "browser.rating.adjust",
                Some("browser"),
                Some(direction_label(delta)),
                "error",
                started_at,
                Some(self.sample_status.as_str()),
            );
            return;
        }

        self.sample_status = format!(
            "Rated {applied} sample{}",
            if applied == 1 { "" } else { "s" }
        );
        emit_gui_action(
            "browser.rating.adjust",
            Some("browser"),
            Some(direction_label(delta)),
            "success",
            started_at,
            None,
        );

        if applied > 0 && self.persisted_settings.controls.advance_after_rating {
            self.navigate_browser(1, false, context);
        }
    }

    fn rating_updates_for_selected_files(&self, delta: i8) -> Vec<RatingUpdate> {
        if delta == 0 {
            return Vec::new();
        }
        self.folder_browser
            .selected_file_rating_candidates()
            .into_iter()
            .filter(|candidate| !candidate.locked)
            .filter_map(|candidate| {
                let (root, relative_path) = self
                    .folder_browser
                    .source_relative_file_path(&candidate.path)?;
                let (rating, locked) = next_rating_state(candidate.rating, delta)?;
                Some(RatingUpdate {
                    root,
                    relative_path,
                    absolute_path: candidate.path,
                    rating,
                    locked,
                })
            })
            .collect()
    }
}

fn next_rating_state(current: Rating, delta: i8) -> Option<(Rating, bool)> {
    if current == Rating::KEEP_3 && delta > 0 {
        return Some((Rating::KEEP_3, true));
    }
    if current == Rating::TRASH_3 && delta < 0 {
        return None;
    }

    let mut new_value = current.val() + delta.signum();
    if current.val() != 0 && new_value == 0 {
        new_value += delta.signum();
    }
    let rating = Rating::new(new_value.clamp(-3, 3));
    (rating != current).then_some((rating, false))
}

fn group_updates_by_source(updates: Vec<RatingUpdate>) -> BTreeMap<PathBuf, Vec<RatingUpdate>> {
    let mut by_source: BTreeMap<PathBuf, Vec<RatingUpdate>> = BTreeMap::new();
    for update in updates {
        by_source
            .entry(update.root.clone())
            .or_default()
            .push(update);
    }
    by_source
}

fn persist_rating_updates(root: &Path, updates: &[RatingUpdate]) -> Result<(), String> {
    let db = SourceDatabase::open_for_user_metadata_write(root).map_err(|err| err.to_string())?;
    let mut batch = db.write_batch().map_err(|err| err.to_string())?;
    for update in updates {
        let (file_size, modified_ns) = file_metadata(&update.absolute_path)?;
        batch
            .upsert_file(&update.relative_path, file_size, modified_ns)
            .map_err(|err| err.to_string())?;
        batch
            .set_tag(&update.relative_path, update.rating)
            .map_err(|err| err.to_string())?;
        batch
            .set_locked(&update.relative_path, update.locked)
            .map_err(|err| err.to_string())?;
    }
    batch.commit().map_err(|err| err.to_string())
}

fn direction_label(delta: i8) -> &'static str {
    if delta < 0 { "down" } else { "up" }
}

fn file_metadata(path: &Path) -> Result<(u64, i64), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let modified_ns = metadata
        .modified()
        .map_err(|err| format!("Missing modified time for {}: {err}", path.display()))?
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map_err(|_| String::from("File modified time is before epoch"))?
        .as_nanos() as i64;
    Ok((metadata.len(), modified_ns))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_rating_skips_neutral_when_changing_direction() {
        assert_eq!(
            next_rating_state(Rating::KEEP_1, -1),
            Some((Rating::TRASH_1, false))
        );
        assert_eq!(
            next_rating_state(Rating::TRASH_1, 1),
            Some((Rating::KEEP_1, false))
        );
    }

    #[test]
    fn next_rating_locks_keep_three_on_fourth_keep() {
        assert_eq!(
            next_rating_state(Rating::KEEP_3, 1),
            Some((Rating::KEEP_3, true))
        );
    }

    #[test]
    fn next_rating_stops_at_trash_three_without_trash_move() {
        assert_eq!(next_rating_state(Rating::TRASH_3, -1), None);
    }
}
