use std::collections::HashSet;

use super::FileEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::native_app) enum BrowserListingRevealReason {
    DestructiveEditReload,
    HistoryNavigation,
    LoadedFileFocus,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct BrowserListingRevealState {
    reveal: Option<BrowserListingReveal>,
}

impl BrowserListingRevealState {
    pub(super) fn set(&mut self, file_id: String, reason: BrowserListingRevealReason) {
        self.reveal = Some(BrowserListingReveal { file_id, reason });
    }

    pub(super) fn clear(&mut self) -> bool {
        self.reveal.take().is_some()
    }

    pub(super) fn active_file_id_for_focus(&self, focused_id: Option<&str>) -> Option<&str> {
        let reveal = self.reveal.as_ref()?;
        (focused_id == Some(reveal.file_id.as_str())).then_some(reveal.file_id.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BrowserListingReveal {
    file_id: String,
    reason: BrowserListingRevealReason,
}

pub(in crate::native_app) struct BrowserListingSnapshot<'a> {
    rows: Vec<&'a FileEntry>,
    ids: Vec<String>,
    id_set: HashSet<String>,
}

impl<'a> BrowserListingSnapshot<'a> {
    pub(super) fn new(rows: Vec<&'a FileEntry>) -> Self {
        let ids = rows.iter().map(|file| file.id.clone()).collect::<Vec<_>>();
        let id_set = ids.iter().cloned().collect();
        Self { rows, ids, id_set }
    }

    pub(in crate::native_app) fn rows(&self) -> &[&'a FileEntry] {
        &self.rows
    }

    pub(in crate::native_app) fn ids(&self) -> &[String] {
        &self.ids
    }

    pub(in crate::native_app) fn len(&self) -> usize {
        self.rows.len()
    }

    pub(in crate::native_app) fn contains(&self, file_id: &str) -> bool {
        self.id_set.contains(file_id)
    }

    pub(in crate::native_app) fn index_of(&self, file_id: &str) -> Option<usize> {
        self.ids.iter().position(|id| id == file_id)
    }

    pub(in crate::native_app) fn target_after_removed_or_hidden(
        &self,
        previous_index: usize,
    ) -> Option<&str> {
        self.ids
            .get(previous_index)
            .or_else(|| self.ids.first())
            .map(String::as_str)
    }
}
