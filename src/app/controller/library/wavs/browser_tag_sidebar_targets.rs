use super::*;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Stable identity for one browser tag-sidebar target.
///
/// Sidebar targeting prefers explicit selected/focused paths, but it can fall
/// back to the currently focused visible row before a durable focused path has
/// been committed. This enum keeps that resolution explicit so projection and
/// mutation can share one authoritative snapshot.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) enum BrowserTagSidebarTargetRef {
    /// Target identified directly by its relative sample path.
    Path(PathBuf),
    /// Target identified by the absolute wav-entry index of the focused row.
    EntryIndex(usize),
}

/// Shared sidebar-target snapshot reused by browser projection and mutation.
///
/// The snapshot captures the exact identity set, the focused primary target,
/// and stable hashes for retained projection keys. Callers resolve paths or
/// entries from this snapshot instead of reconstructing sidebar targets ad hoc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BrowserTagSidebarTargetSnapshot {
    targets: Vec<BrowserTagSidebarTargetRef>,
    primary_target: Option<BrowserTagSidebarTargetRef>,
    target_identity_hash: u64,
    primary_identity_hash: u64,
}

impl BrowserTagSidebarTargetSnapshot {
    /// Build one sidebar-target snapshot from browser selection state.
    pub(crate) fn from_controller(controller: &AppController) -> Self {
        let selected_paths = &controller.ui.browser.selection.selected_paths;
        let last_focused_path = controller.ui.browser.selection.last_focused_path.clone();
        let targets = if !selected_paths.is_empty() {
            selected_paths
                .iter()
                .cloned()
                .map(BrowserTagSidebarTargetRef::Path)
                .collect()
        } else if let Some(path) = last_focused_path.clone() {
            vec![BrowserTagSidebarTargetRef::Path(path)]
        } else {
            controller
                .ui
                .browser
                .selection
                .selected_visible
                .and_then(|visible_row| controller.visible_browser_index(visible_row))
                .map(BrowserTagSidebarTargetRef::EntryIndex)
                .into_iter()
                .collect()
        };
        let primary_target = if !selected_paths.is_empty() {
            last_focused_path
                .filter(|path| selected_paths.iter().any(|selected| selected == path))
                .map(BrowserTagSidebarTargetRef::Path)
                .or_else(|| targets.first().cloned())
        } else {
            targets.first().cloned()
        };
        Self {
            target_identity_hash: hash_sidebar_targets(&targets),
            primary_identity_hash: primary_target
                .as_ref()
                .map(hash_sidebar_target)
                .unwrap_or(0),
            targets,
            primary_target,
        }
    }

    /// Return the ordered sidebar target identity set.
    pub(crate) fn targets(&self) -> &[BrowserTagSidebarTargetRef] {
        &self.targets
    }

    /// Return the focused primary target, when any.
    pub(crate) fn primary_target(&self) -> Option<&BrowserTagSidebarTargetRef> {
        self.primary_target.as_ref()
    }

    /// Return the number of selected sidebar targets.
    pub(crate) fn selected_count(&self) -> usize {
        self.targets.len()
    }

    /// Return a stable hash of the full target identity set.
    pub(crate) fn target_identity_hash(&self) -> u64 {
        self.target_identity_hash
    }

    /// Return a stable hash of the focused primary target identity.
    pub(crate) fn primary_identity_hash(&self) -> u64 {
        self.primary_identity_hash
    }

    /// Resolve the snapshot into concrete relative paths for browser mutations.
    pub(crate) fn resolve_paths(&self, controller: &mut AppController) -> Vec<PathBuf> {
        self.targets
            .iter()
            .filter_map(|target| resolve_sidebar_target_path(controller, target))
            .collect()
    }

    /// Resolve the snapshot into wav entries for sidebar projection.
    pub(crate) fn resolve_entries(&self, controller: &mut AppController) -> Vec<WavEntry> {
        self.targets
            .iter()
            .filter_map(|target| resolve_sidebar_target_entry(controller, target))
            .collect()
    }
}

impl AppController {
    /// Capture the shared browser tag-sidebar target snapshot.
    pub(crate) fn browser_tag_sidebar_target_snapshot(&self) -> BrowserTagSidebarTargetSnapshot {
        BrowserTagSidebarTargetSnapshot::from_controller(self)
    }

    /// Resolve sidebar-target paths from the shared snapshot for mutations.
    pub(crate) fn browser_tag_sidebar_target_paths(&mut self) -> Vec<PathBuf> {
        self.browser_tag_sidebar_target_snapshot()
            .resolve_paths(self)
    }

    /// Resolve sidebar-target entries from the shared snapshot for projection.
    pub(crate) fn browser_tag_sidebar_target_entries(&mut self) -> Vec<WavEntry> {
        self.browser_tag_sidebar_target_snapshot()
            .resolve_entries(self)
    }
}

fn resolve_sidebar_target_path(
    controller: &mut AppController,
    target: &BrowserTagSidebarTargetRef,
) -> Option<PathBuf> {
    match target {
        BrowserTagSidebarTargetRef::Path(path) => Some(path.clone()),
        BrowserTagSidebarTargetRef::EntryIndex(index) => controller
            .wav_entry(*index)
            .map(|entry| entry.relative_path.clone()),
    }
}

fn resolve_sidebar_target_entry(
    controller: &mut AppController,
    target: &BrowserTagSidebarTargetRef,
) -> Option<WavEntry> {
    let path = resolve_sidebar_target_path(controller, target)?;
    let index = controller.wav_index_for_path(Path::new(&path))?;
    controller.wav_entry(index).cloned()
}

fn hash_sidebar_targets(targets: &[BrowserTagSidebarTargetRef]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    targets.hash(&mut hasher);
    hasher.finish()
}

fn hash_sidebar_target(target: &BrowserTagSidebarTargetRef) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    target.hash(&mut hasher);
    hasher.finish()
}
