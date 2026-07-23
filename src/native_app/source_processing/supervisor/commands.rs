use super::{
    Arc, BTreeMap, CommittedSourceDelta, MAX_VISIBLE_PRIORITY_PATHS, SampleSource,
    SourceProcessingBudgetHandle, SourceProcessingSupervisor, register_source_for_scan_locked,
};

impl SourceProcessingSupervisor {
    /// Admit a newly configured source before its first external scan starts.
    ///
    /// This deliberately only grows the configured set. Full replacement also
    /// retires removed lifecycle epochs and is owned by the configuration path.
    pub(in crate::native_app) fn register_source_for_scan(
        &self,
        source: SampleSource,
    ) -> Result<u64, String> {
        let _replacement = match self.shared.source_replacement.try_lock() {
            Ok(replacement) => replacement,
            Err(std::sync::TryLockError::Poisoned(poison)) => poison.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => {
                return Err("Configured sources are currently being replaced".to_string());
            }
        };
        register_source_for_scan_locked(self.shared.as_ref(), source)
    }

    pub(in crate::native_app) fn budget_handle(&self) -> SourceProcessingBudgetHandle {
        SourceProcessingBudgetHandle {
            shared: Arc::clone(&self.shared),
        }
    }

    pub(in crate::native_app) fn lifecycle_generations(&self) -> BTreeMap<String, u64> {
        self.shared.control().source_lifecycle_generations.clone()
    }

    /// Re-arm the authoritative source audits after the watcher stream is live.
    ///
    /// The initial audit and this watcher-ready request coalesce while an audit
    /// is in flight. If it already completed, this request runs one final audit
    /// that closes the gap between its snapshot and native event delivery.
    pub(in crate::native_app) fn request_manifest_audits(&self, reason: &'static str) {
        let mut control = self.shared.control();
        let active_source_ids = control
            .sources
            .keys()
            .filter(|source_id| control.source_is_active(source_id))
            .cloned()
            .collect::<Vec<_>>();
        if active_source_ids.is_empty() {
            return;
        }
        control
            .force_manifest_audit_sources
            .extend(active_source_ids.iter().cloned());
        control.dirty_sources.extend(active_source_ids);
        control.notify(reason);
        drop(control);
        self.shared.wake.notify_one();
    }

    pub(in crate::native_app) fn wake_source(&self, source_id: &str, reason: &'static str) {
        let mut control = self.shared.control();
        if !control.source_is_active(source_id) {
            return;
        }
        let bounded_delta_pending = control.pending_readiness_deltas.contains_key(source_id);
        if !bounded_delta_pending {
            control.cancel_source_work(source_id);
        }
        control.mark_source_dirty(source_id, reason);
        drop(control);
        if !bounded_delta_pending {
            self.shared
                .cancel_external_scans(|registration| registration.source_id == source_id);
            self.shared.budget_wake.notify_all();
        }
        self.shared.wake.notify_one();
    }

    /// Force complete source reconciliation when a bounded delta cannot describe all changes.
    pub(in crate::native_app) fn wake_source_for_full_reconciliation(
        &self,
        source_id: &str,
        reason: &'static str,
    ) {
        self.shared
            .control()
            .pending_readiness_deltas
            .remove(source_id);
        self.wake_source(source_id, reason);
    }

    pub(in crate::native_app) fn cancel_foreground_source_scan(
        &self,
        source_id: &str,
        reason: &'static str,
    ) {
        let mut control = self.shared.control();
        if !control.source_is_active(source_id) {
            return;
        }
        control.cancel_source_work(source_id);
        control.notify(reason);
        drop(control);
        self.shared
            .cancel_external_scans(|registration| registration.source_id == source_id);
        self.shared.budget_wake.notify_all();
        self.shared.wake.notify_one();
    }

    /// Reconcile a source without invalidating work that already owns its
    /// current lifecycle generation.
    ///
    /// UI projection refreshes and completed foreground scans can arrive well
    /// after the source database commit that scheduled them. Treating those
    /// delayed notifications as a new mutation would kill a valid long-running
    /// finalizer and immediately start the same work again.
    pub(in crate::native_app) fn request_source_processing(
        &self,
        source_id: &str,
        reason: &'static str,
    ) {
        let mut control = self.shared.control();
        if !control.source_is_active(source_id) {
            return;
        }
        control.mark_source_dirty(source_id, reason);
        drop(control);
        self.shared.wake.notify_one();
    }

    /// Publish the affected identity set from one authoritative committed manifest delta.
    pub(in crate::native_app) fn request_source_delta(
        &self,
        source_id: &str,
        delta: &CommittedSourceDelta,
        reason: &'static str,
    ) {
        if delta.is_empty() {
            return;
        }
        let mut control = self.shared.control();
        if !control.source_is_active(source_id) {
            return;
        }
        control
            .pending_readiness_deltas
            .entry(source_id.to_string())
            .or_default()
            .merge(delta);
        control.mark_source_dirty(source_id, reason);
        drop(control);
        self.shared.wake.notify_one();
    }

    #[cfg(test)]
    pub(in crate::native_app) fn pending_source_delta_contains_identity_for_tests(
        &self,
        source_id: &str,
        identity: &str,
    ) -> bool {
        self.shared
            .control()
            .pending_readiness_deltas
            .get(source_id)
            .is_some_and(|delta| delta.scope_ids.contains(identity))
    }

    /// Requeue exact current feature, embedding, and similarity targets after
    /// an explicit user request.
    pub(in crate::native_app) fn request_source_reanalysis(
        &self,
        source_id: &str,
        reason: &'static str,
    ) {
        let mut control = self.shared.control();
        if !control.source_is_active(source_id) {
            return;
        }
        control.cancel_source_work(source_id);
        control
            .force_reanalysis_sources
            .insert(source_id.to_string());
        control.mark_source_dirty(source_id, reason);
        drop(control);
        self.shared
            .cancel_external_scans(|registration| registration.source_id == source_id);
        self.shared.budget_wake.notify_all();
        self.shared.wake.notify_one();
    }

    pub(in crate::native_app) fn finish_foreground_source_refresh(
        &self,
        source_id: &str,
        reason: &'static str,
    ) {
        let mut control = self.shared.control();
        if !control.source_is_active(source_id) {
            return;
        }
        let was_awaiting = control
            .awaiting_foreground_refresh_sources
            .remove(source_id);
        let bounded_delta_pending = control.pending_readiness_deltas.contains_key(source_id);
        if was_awaiting || bounded_delta_pending {
            control.mark_source_dirty(source_id, reason);
        }
        drop(control);
        if was_awaiting || bounded_delta_pending {
            self.shared.wake.notify_one();
        }
    }

    pub(in crate::native_app) fn set_selected_source(&self, source_id: Option<&str>) {
        let mut control = self.shared.control();
        let selected = source_id.map(str::to_string);
        if control.priority.selected_source != selected {
            control.priority.selected_source = selected;
            control.notify("selected_source_changed");
            self.shared.wake.notify_one();
        }
    }

    #[cfg(test)]
    pub(in crate::native_app) fn selected_source_priority_for_tests(&self) -> Option<String> {
        self.shared.control().priority.selected_source.clone()
    }

    pub(in crate::native_app) fn prioritize_path(
        &self,
        source_id: &str,
        relative_path: &str,
        immediate: bool,
    ) {
        let mut control = self.shared.control();
        let key = (source_id.to_string(), relative_path.to_string());
        let priorities = if immediate {
            control.priority.immediate_paths.clear();
            &mut control.priority.immediate_paths
        } else {
            &mut control.priority.visible_paths
        };
        if priorities.insert(key) {
            control.notify("interactive_path_priority");
            self.shared.wake.notify_one();
        }
    }

    pub(in crate::native_app) fn set_visible_paths<I>(&self, paths: I)
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let visible_paths = paths.into_iter().take(MAX_VISIBLE_PRIORITY_PATHS).collect();
        let mut control = self.shared.control();
        if control.priority.visible_paths != visible_paths {
            control.priority.visible_paths = visible_paths;
            control.notify("visible_paths_changed");
            self.shared.wake.notify_one();
        }
    }

    pub(in crate::native_app) fn set_current_folder(&self, source_id: &str, relative_path: &str) {
        let mut control = self.shared.control();
        let current = Some((source_id.to_string(), relative_path.to_string()));
        if control.priority.current_folder != current {
            control.priority.current_folder = current;
            control.notify("current_folder_changed");
            self.shared.wake.notify_one();
        }
    }

    pub(in crate::native_app) fn set_playback_active(&self, active: bool) {
        let mut control = self.shared.control();
        if control.playback_active != active {
            control.playback_active = active;
            control.notify("playback_activity_changed");
            tracing::debug!(
                target: "wavecrate::source_processing",
                event = "source_processing.playback_activity_changed",
                active,
                "Playback activity changed without pausing source processing"
            );
            drop(control);
            self.shared.wake.notify_all();
        }
    }

    pub(in crate::native_app) fn set_foreground_activity(&self, active: bool) {
        let mut control = self.shared.control();
        if control.foreground_active == active {
            return;
        }
        control.foreground_active = active;
        control.notify("foreground_activity_changed");
        tracing::debug!(
            target: "wavecrate::source_processing",
            event = "source_processing.foreground_activity_changed",
            active,
            "Foreground loading activity changed without pausing source processing"
        );
        drop(control);
        self.shared.wake.notify_all();
    }
}
