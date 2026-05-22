use super::*;

impl SourceHydrationRuntime {
    /// Return the pending hydration request for `kind`, when one exists.
    pub(crate) fn pending(&self, kind: SourceHydrationKind) -> Option<&PendingSourceHydration> {
        match kind {
            SourceHydrationKind::ActiveSelection => self.pending_active.as_ref(),
            SourceHydrationKind::InactivePane => self.pending_inactive.as_ref(),
        }
    }

    /// Return the mutable pending hydration request for `kind`, when one exists.
    pub(crate) fn pending_mut(
        &mut self,
        kind: SourceHydrationKind,
    ) -> Option<&mut PendingSourceHydration> {
        match kind {
            SourceHydrationKind::ActiveSelection => self.pending_active.as_mut(),
            SourceHydrationKind::InactivePane => self.pending_inactive.as_mut(),
        }
    }

    /// Store a new pending hydration request in the lane identified by `kind`.
    pub(crate) fn set_pending(
        &mut self,
        kind: SourceHydrationKind,
        pending: PendingSourceHydration,
    ) {
        match kind {
            SourceHydrationKind::ActiveSelection => self.pending_active = Some(pending),
            SourceHydrationKind::InactivePane => self.pending_inactive = Some(pending),
        }
    }

    /// Clear the pending hydration request for `kind`.
    pub(crate) fn clear_pending(&mut self, kind: SourceHydrationKind) {
        match kind {
            SourceHydrationKind::ActiveSelection => self.pending_active = None,
            SourceHydrationKind::InactivePane => self.pending_inactive = None,
        }
    }

    /// Return the source id currently owning a visible loading state, when any.
    pub(crate) fn loading_source_id(&self) -> Option<SourceId> {
        self.pending_active
            .as_ref()
            .map(|pending| pending.source_id.clone())
            .or_else(|| {
                self.pending_inactive
                    .as_ref()
                    .map(|pending| pending.source_id.clone())
            })
    }
}
