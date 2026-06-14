use super::*;

/// Which browser-search refresh path should run after a mutation.
pub(super) enum RefreshPolicy {
    Full,
    TriageFilter,
}

/// Side effects required after a browser-search state mutation.
#[derive(Default)]
pub(super) struct MutationEffects {
    cancel_similarity_rebuild: bool,
    mark_projection_dirty: bool,
    refresh: Option<RefreshPolicy>,
}

impl MutationEffects {
    pub(super) fn none() -> Self {
        Self::default()
    }

    pub(super) fn projection_only() -> Self {
        Self {
            mark_projection_dirty: true,
            ..Self::default()
        }
    }

    pub(super) fn refresh(refresh: RefreshPolicy) -> Self {
        Self {
            cancel_similarity_rebuild: true,
            mark_projection_dirty: true,
            refresh: Some(refresh),
        }
    }
}

pub(super) fn refresh_effects(changed: bool, refresh: RefreshPolicy) -> MutationEffects {
    if changed {
        MutationEffects::refresh(refresh)
    } else {
        MutationEffects::none()
    }
}

pub(super) fn apply_mutation_effects(controller: &mut AppController, effects: MutationEffects) {
    if effects.cancel_similarity_rebuild {
        crate::app::controller::library::wavs::cancel_pending_similarity_filter_rebuild(controller);
    }
    if effects.mark_projection_dirty {
        controller.mark_browser_search_projection_revision_dirty();
    }
    match effects.refresh {
        Some(RefreshPolicy::Full) => refresh_browser_search_results(controller),
        Some(RefreshPolicy::TriageFilter) => refresh_browser_triage_filter_results(controller),
        None => {}
    }
}

/// Refresh browser rows through the authoritative async worker or the retained sync path.
fn refresh_browser_search_results(controller: &mut AppController) {
    if controller.should_dispatch_browser_search_async() {
        controller.dispatch_search_job();
    } else {
        controller.rebuild_browser_lists();
    }
}

/// Refresh a simple triage-filter change without creating async churn when cached rows suffice.
fn refresh_browser_triage_filter_results(controller: &mut AppController) {
    if controller.can_refresh_triage_filter_locally() {
        if controller.should_dispatch_browser_search_async()
            || controller.ui.browser.search.search_busy
        {
            controller.invalidate_async_browser_search_for_local_projection();
        }
        controller.rebuild_browser_lists_retained();
    } else {
        refresh_browser_search_results(controller);
    }
}
