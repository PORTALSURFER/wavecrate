use super::planning::run_background_auto_rename_request;
use super::*;

impl BrowserController<'_> {
    pub(in crate::app::controller::library::browser_controller::actions) fn rename_browser_sample_action(
        &mut self,
        row: usize,
        new_name: &str,
    ) -> Result<(), String> {
        let ctx = self.resolve_browser_sample(row)?;
        if self.warn_if_any_browser_context_busy(std::slice::from_ref(&ctx), "renaming") {
            return Ok(());
        }
        let result = self.try_rename_browser_sample(row, new_name);
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }

    pub(crate) fn auto_rename_browser_sample_paths_action(
        &mut self,
        paths: &[PathBuf],
    ) -> Result<(), String> {
        self.dispatch_auto_rename_browser_sample_paths_action(paths, cfg!(test))
    }

    #[cfg(test)]
    pub(crate) fn auto_rename_browser_sample_paths_background_for_tests(
        &mut self,
        paths: &[PathBuf],
    ) -> Result<(), String> {
        self.dispatch_auto_rename_browser_sample_paths_action(paths, false)
    }

    fn dispatch_auto_rename_browser_sample_paths_action(
        &mut self,
        paths: &[PathBuf],
        run_inline: bool,
    ) -> Result<(), String> {
        #[cfg(test)]
        let dispatch_started_at = std::time::Instant::now();
        if paths.is_empty() {
            return Ok(());
        }
        let Some(source) = self.current_source() else {
            return Err(String::from("No source selected"));
        };
        let snapshot = self.capture_auto_rename_background_request(&source, paths);
        let intent_key = auto_rename_intent_key(&source.id, paths);
        if self.runtime.jobs.file_ops_in_progress() {
            let pending = PendingBrowserAutoRenameIntent {
                key: intent_key.clone(),
                source_id: source.id.clone(),
                paths: paths.to_vec(),
            };
            return match self
                .runtime
                .source_lane
                .mutations
                .handle_busy_browser_auto_rename_intent(intent_key, pending)
            {
                BrowserRenameBusyDecision::Collapsed => {
                    self.set_file_op_status("Auto rename already in progress...", StatusTone::Busy);
                    Ok(())
                }
                BrowserRenameBusyDecision::Queued => {
                    self.set_file_op_status(
                        "Auto rename queued after current rename...",
                        StatusTone::Busy,
                    );
                    Ok(())
                }
                BrowserRenameBusyDecision::UnrelatedFileOp => {
                    Err("File operation already in progress".to_string())
                }
            };
        }
        let requested_paths = paths.to_vec();
        self.runtime
            .source_lane
            .mutations
            .begin_browser_rename_intent(intent_key);
        self.begin_pending_file_mutation(&source.id, requested_paths.clone());
        #[cfg(test)]
        crate::app::controller::batch_latency::record(
            crate::app::controller::batch_latency::BatchLatencySample::new(
                crate::app::controller::batch_latency::BatchLatencyPhase::AutoRenameDispatch,
                requested_paths.len(),
                dispatch_started_at.elapsed(),
            ),
        );
        if run_inline {
            self.preload_bpm_values_for_paths(paths);
            let requests = match self.prepare_auto_rename_requests(&source, paths) {
                Ok(requests) => requests,
                Err(err) => {
                    self.runtime
                        .source_lane
                        .mutations
                        .clear_browser_rename_intent();
                    self.finish_pending_file_mutation(&source.id, requested_paths);
                    return Err(err);
                }
            };
            if requests.is_empty() {
                self.runtime
                    .source_lane
                    .mutations
                    .clear_browser_rename_intent();
                self.finish_pending_file_mutation(&source.id, requested_paths);
                return Ok(());
            }
            let result = run_sample_auto_rename_job(
                source.clone(),
                requests,
                Arc::new(AtomicBool::new(false)),
            );
            self.apply_file_op_result(FileOpResult::SampleAutoRename(result));
            return Ok(());
        }
        let title = String::from("Preparing auto rename");
        self.set_file_op_status(format!("{title}..."), StatusTone::Busy);
        self.show_status_progress(
            crate::app::state::ProgressTaskKind::FileOps,
            title,
            requested_paths.len().max(1),
            true,
        );
        let pending_source_id = source.id.clone();
        if let Err(err) = self.runtime.jobs.begin_one_shot_file_op(move |cancel| {
            FileOpResult::SampleAutoRename(run_background_auto_rename_request(snapshot, cancel))
        }) {
            self.runtime
                .source_lane
                .mutations
                .clear_browser_rename_intent();
            self.finish_pending_file_mutation(&pending_source_id, requested_paths);
            return Err(err);
        }
        Ok(())
    }
}

fn auto_rename_intent_key(
    source_id: &crate::sample_sources::SourceId,
    paths: &[PathBuf],
) -> BrowserRenameIntentKey {
    BrowserRenameIntentKey::new(
        source_id.clone(),
        paths
            .iter()
            .map(|path| (path.clone(), path.clone()))
            .collect(),
    )
}
