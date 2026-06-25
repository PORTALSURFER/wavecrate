use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Instant,
};

use radiant::prelude as ui;
use wavecrate::sample_sources::{Rating, SourceDatabase};

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::folder_browser_actions::file_navigation_reveal_direction;
use crate::native_app::sample_library::sample_list::{
    SAMPLE_BROWSER_LIST_ID, SAMPLE_BROWSER_ROW_HEIGHT, SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS,
};
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
        let advance_visible_ids = self.rating_advance_visible_ids_before_adjustment();
        let advance_previous_index = advance_visible_ids.as_ref().and_then(|_| {
            self.library
                .folder_browser
                .selected_audio_file_index_matching_tags(&self.metadata.tags_by_file)
        });
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
            if let Some(visible_ids) = advance_visible_ids {
                self.advance_after_rating_in_visible_order(
                    &visible_ids,
                    advance_previous_index,
                    context,
                );
            } else {
                self.navigate_browser(1, false, false, context);
            }
        }
        if applied > 0 {
            self.library
                .folder_browser
                .retain_visible_file_selection_after_tag_filter(&self.metadata.tags_by_file);
        }
    }

    fn rating_advance_visible_ids_before_adjustment(&self) -> Option<Vec<String>> {
        if !self.ui.settings.persisted.controls.advance_after_rating
            || self.library.folder_browser.random_navigation_enabled()
        {
            return None;
        }
        Some(
            self.library
                .folder_browser
                .selected_audio_files_matching_tags(&self.metadata.tags_by_file)
                .into_iter()
                .map(|file| file.id.clone())
                .collect(),
        )
    }

    fn advance_after_rating_in_visible_order(
        &mut self,
        visible_ids_before_rating: &[String],
        previous_index: Option<usize>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let previous_selection = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        let Some(path) = self.library.folder_browser.navigate_selected_file_in_ids(
            1,
            false,
            visible_ids_before_rating,
        ) else {
            return;
        };
        let Some(path) =
            self.rating_advance_visible_target(visible_ids_before_rating, previous_index, &path)
        else {
            return;
        };

        if self.library.folder_browser.selected_file_id() != previous_selection.as_deref() {
            self.cancel_metadata_tag_entry();
            self.metadata.selected_tag = None;
        }
        if self.library.folder_browser.selected_file_id() != Some(path.as_str()) {
            self.library
                .folder_browser
                .focus_file_preserving_selection_matching_tags(
                    path.clone(),
                    &self.metadata.tags_by_file,
                );
        }
        if let Some(index) = self
            .library
            .folder_browser
            .selected_audio_file_index_matching_tags(&self.metadata.tags_by_file)
        {
            let reveal_direction = file_navigation_reveal_direction(previous_index, index, 1);
            context.scroll_fixed_row_into_view(
                SAMPLE_BROWSER_LIST_ID,
                index,
                SAMPLE_BROWSER_ROW_HEIGHT,
                SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS,
                SAMPLE_BROWSER_SELECTION_CONTEXT_ROWS,
                reveal_direction,
            );
        }
        self.load_navigation_sample(path, context);
    }

    fn rating_advance_visible_target(
        &self,
        visible_ids_before_rating: &[String],
        previous_index: Option<usize>,
        candidate: &str,
    ) -> Option<String> {
        let visible_ids_after_rating = self
            .library
            .folder_browser
            .selected_audio_files_matching_tags(&self.metadata.tags_by_file)
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        if visible_ids_after_rating.iter().any(|id| id == candidate) {
            return Some(candidate.to_owned());
        }
        let visible_after = visible_ids_after_rating
            .iter()
            .map(String::as_str)
            .collect::<std::collections::HashSet<_>>();
        let primary_index = previous_index.unwrap_or(0);
        visible_ids_before_rating
            .iter()
            .skip(primary_index.saturating_add(1))
            .find(|id| visible_after.contains(id.as_str()))
            .or_else(|| {
                visible_ids_before_rating
                    .iter()
                    .take(primary_index)
                    .rev()
                    .find(|id| visible_after.contains(id.as_str()))
            })
            .cloned()
            .or_else(|| visible_ids_after_rating.first().cloned())
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
