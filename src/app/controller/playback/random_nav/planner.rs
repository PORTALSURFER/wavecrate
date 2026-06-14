use super::super::*;
use rand::Rng;
use rand::seq::IteratorRandom;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

/// Resolved random-navigation target chosen from the visible browser rows.
pub(super) struct RandomVisibleTarget {
    /// Source that owns the chosen browser row.
    pub(super) source_id: SourceId,
    /// Visible browser row selected for the next random jump.
    pub(super) visible_row: usize,
    /// Source-relative path for the chosen sample.
    pub(super) path: PathBuf,
}

#[derive(Clone)]
pub(super) struct RandomVisibleRow {
    pub(super) visible_row: usize,
    pub(super) path: PathBuf,
}

pub(super) struct RandomVisibleList {
    pub(super) source_id: SourceId,
    pub(super) fingerprint: u64,
    visible_len: usize,
    rows: Vec<RandomVisibleRow>,
}

impl RandomVisibleList {
    pub(super) fn from_controller(controller: &mut AppController, source_id: SourceId) -> Self {
        let visible_len = controller.visible_browser_len();
        let rows = visible_rows(controller, visible_len);
        let fingerprint = fingerprint(&source_id, visible_len, &rows);
        Self {
            source_id,
            fingerprint,
            visible_len,
            rows,
        }
    }

    pub(super) fn available_rows<'a>(
        &'a self,
        played_paths: impl Fn(&PathBuf) -> bool,
        current_path: Option<&std::path::Path>,
    ) -> Vec<&'a RandomVisibleRow> {
        let exclude_current = current_path.is_some() && self.visible_len > 1;
        self.rows
            .iter()
            .filter(|row| !played_paths(&row.path))
            .filter(|row| {
                !exclude_current || current_path.is_none_or(|selected| selected != row.path)
            })
            .collect()
    }

    pub(super) fn choose_target<R: Rng + ?Sized>(
        &self,
        rows: &[&RandomVisibleRow],
        rng: &mut R,
    ) -> Option<RandomVisibleTarget> {
        let row = rows.iter().choose(rng)?;
        Some(RandomVisibleTarget {
            source_id: self.source_id.clone(),
            visible_row: row.visible_row,
            path: row.path.clone(),
        })
    }
}

pub(super) fn current_path(controller: &AppController) -> Option<PathBuf> {
    controller
        .sample_view
        .wav
        .selected_wav
        .clone()
        .or_else(|| controller.ui.browser.selection.last_focused_path.clone())
}

fn visible_rows(controller: &mut AppController, visible_len: usize) -> Vec<RandomVisibleRow> {
    let mut rows = Vec::new();
    for visible_row in 0..visible_len {
        let Some(entry_index) = controller.visible_browser_index(visible_row) else {
            continue;
        };
        let Some(path) = controller
            .wav_entry(entry_index)
            .map(|entry| entry.relative_path.clone())
        else {
            continue;
        };
        rows.push(RandomVisibleRow { visible_row, path });
    }
    rows
}

fn fingerprint(source_id: &SourceId, visible_len: usize, rows: &[RandomVisibleRow]) -> u64 {
    let mut hasher = DefaultHasher::new();
    source_id.as_str().hash(&mut hasher);
    visible_len.hash(&mut hasher);
    for row in rows {
        row.path.hash(&mut hasher);
    }
    hasher.finish()
}
