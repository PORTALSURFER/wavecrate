//! Browser-controller wrapper and shared sample context types.

use super::*;

/// Helper wrapper that groups browser-specific controller flows without widening `AppController`.
pub(crate) struct BrowserController<'a> {
    pub(super) controller: &'a mut AppController,
}

impl<'a> BrowserController<'a> {
    /// Build a browser helper bound to the current controller instance.
    pub(crate) fn new(controller: &'a mut AppController) -> Self {
        Self { controller }
    }
}

impl std::ops::Deref for BrowserController<'_> {
    type Target = AppController;

    fn deref(&self) -> &Self::Target {
        self.controller
    }
}

impl std::ops::DerefMut for BrowserController<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.controller
    }
}

/// Resolved source/sample context used by browser-side destructive actions.
pub(crate) struct TriageSampleContext {
    pub(crate) source: SampleSource,
    pub(crate) entry: WavEntry,
    pub(crate) absolute_path: PathBuf,
}

/// Browser focus target to restore after deleting one or more visible rows.
#[derive(Clone, Debug)]
pub(crate) struct DeleteBrowserFocusPlan {
    pub(crate) preferred_path: Option<PathBuf>,
    pub(crate) fallback_visible_row: Option<usize>,
}
