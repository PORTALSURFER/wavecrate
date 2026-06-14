use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Instant,
};

use radiant::prelude as ui;
use wavecrate::sample_sources::{Rating, SourceDatabase};

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::transaction_history::TransactionContext;

#[derive(Clone, Debug, PartialEq, Eq)]
struct RatingUpdate {
    root: PathBuf,
    relative_path: PathBuf,
    absolute_path: PathBuf,
    previous_rating: Rating,
    previous_locked: bool,
    rating: Rating,
    locked: bool,
}

#[derive(Debug, Default)]
struct RatingAdjustmentPlan {
    updates: Vec<RatingUpdate>,
    auto_trash_paths: Vec<PathBuf>,
}

impl NativeAppState {
    pub(in crate::native_app) fn adjust_selected_rating(
        &mut self,
        delta: i8,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let plan = self.rating_adjustment_plan_for_selected_files(delta);
        if plan.is_empty() {
            self.ui.status.sample = String::from("Select a sample to rate");
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

        let applied = match self.apply_rating_update_states(&plan.updates, RatingUpdateMode::After)
        {
            Ok(applied) => applied,
            Err(error) => {
                self.ui.status.sample = format!("Rating failed: {error}");
                emit_gui_action(
                    "browser.rating.adjust",
                    Some("browser"),
                    Some(direction_label(delta)),
                    "error",
                    started_at,
                    Some(self.ui.status.sample.as_str()),
                );
                return;
            }
        };

        if applied > 0 {
            self.ui.status.sample = format!(
                "Rated {applied} sample{}",
                if applied == 1 { "" } else { "s" }
            );
        }
        emit_gui_action(
            "browser.rating.adjust",
            Some("browser"),
            Some(direction_label(delta)),
            "success",
            started_at,
            None,
        );

        if applied > 0 {
            self.register_rating_transaction(delta, plan.updates);
        }

        if !plan.auto_trash_paths.is_empty() {
            self.move_negative_threshold_files_to_trash(plan.auto_trash_paths, started_at, context);
            return;
        }

        if applied > 0 && self.ui.settings.persisted.controls.advance_after_rating {
            self.navigate_browser(1, false, context);
        }
    }

    fn rating_adjustment_plan_for_selected_files(&self, delta: i8) -> RatingAdjustmentPlan {
        if delta == 0 {
            return RatingAdjustmentPlan::default();
        }
        let mut plan = RatingAdjustmentPlan::default();
        for candidate in self
            .library
            .folder_browser
            .selected_file_rating_candidates()
            .into_iter()
            .filter(|candidate| !candidate.locked)
        {
            if should_auto_trash_on_rating(candidate.rating, delta) {
                plan.auto_trash_paths.push(candidate.path);
                continue;
            }
            let Some((root, relative_path)) = self
                .library
                .folder_browser
                .source_relative_file_path(&candidate.path)
            else {
                continue;
            };
            let Some((rating, locked)) = next_rating_state(candidate.rating, delta) else {
                continue;
            };
            plan.updates.push(RatingUpdate {
                root,
                relative_path,
                absolute_path: candidate.path,
                previous_rating: candidate.rating,
                previous_locked: candidate.locked,
                rating,
                locked,
            });
        }
        plan
    }

    fn register_rating_transaction(&mut self, delta: i8, updates: Vec<RatingUpdate>) {
        let label = format!("Rate {}", if delta < 0 { "down" } else { "up" });
        let undo_updates = updates.clone();
        let redo_updates = updates;
        self.begin_transaction(label);
        self.register_transaction_action(
            "Apply rating changes",
            move |transaction| {
                transaction
                    .apply_rating_update_states(&undo_updates, RatingUpdateMode::Before)
                    .map(|_| ())
            },
            move |transaction| {
                transaction
                    .apply_rating_update_states(&redo_updates, RatingUpdateMode::After)
                    .map(|_| ())
            },
        );
        self.commit_transaction();
    }

    fn apply_rating_update_states(
        &mut self,
        updates: &[RatingUpdate],
        mode: RatingUpdateMode,
    ) -> Result<usize, String> {
        let mut applied = 0usize;
        for (root, source_updates) in group_updates_by_source(
            updates
                .iter()
                .cloned()
                .map(|update| update.for_mode(mode))
                .collect(),
        ) {
            persist_rating_updates(&root, &source_updates)?;
            for update in source_updates {
                if self.library.folder_browser.set_file_rating_state(
                    &update.absolute_path,
                    update.rating,
                    update.locked,
                ) {
                    applied += 1;
                }
            }
        }
        Ok(applied)
    }
}

impl TransactionContext<'_> {
    fn apply_rating_update_states(
        &mut self,
        updates: &[RatingUpdate],
        mode: RatingUpdateMode,
    ) -> Result<usize, String> {
        self.state.apply_rating_update_states(updates, mode)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RatingUpdateMode {
    Before,
    After,
}

impl RatingUpdate {
    fn for_mode(mut self, mode: RatingUpdateMode) -> Self {
        if mode == RatingUpdateMode::Before {
            self.rating = self.previous_rating;
            self.locked = self.previous_locked;
        }
        self
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

fn should_auto_trash_on_rating(current: Rating, delta: i8) -> bool {
    current == Rating::TRASH_3 && delta < 0
}

impl RatingAdjustmentPlan {
    fn is_empty(&self) -> bool {
        self.updates.is_empty() && self.auto_trash_paths.is_empty()
    }
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

    #[test]
    fn fourth_negative_rating_triggers_auto_trash_threshold() {
        assert!(should_auto_trash_on_rating(Rating::TRASH_3, -1));
        assert!(!should_auto_trash_on_rating(Rating::new(-2), -1));
        assert!(!should_auto_trash_on_rating(Rating::TRASH_3, 1));
    }
}
